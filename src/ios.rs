//! iOS 宿主薄层。
//!
//! 渲染/事件循环由 eframe（winit 的 iOS 后端 + wgpu Metal）负责；UIKit 侧那些
//! eframe 没有的能力（系统键盘输入框、文件选择器、常亮、安全区、剪贴板）放在
//! ios/main.swift 里，用 C ABI 双向对接：
//!
//! ```text
//! Rust → Swift: nswt_ios_present_text / nswt_ios_present_picker /
//!               nswt_ios_set_idle_disabled / nswt_ios_copy_text
//! Swift → Rust: nswt_ios_set_home / nswt_ios_set_version / nswt_ios_set_insets /
//!               nswt_ios_text_result / nswt_ios_file_result / nswt_ios_log /
//!               nswt_ios_start（入口）
//! ```
//!
//! 回调一律只在主线程发生（winit 的事件循环就跑在主线程），Rust 侧只写缓冲，
//! UI 每帧轮询取走——与 android.rs 的 take_edited_text / take_device_info 同构。

use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::platform::InputKind;

static HOME_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);
static VERSION: Mutex<Option<String>> = Mutex::new(None);
static INSETS: Mutex<[f32; 4]> = Mutex::new([0.0; 4]);
static EDITS: OnceLock<Mutex<HashMap<u64, Option<String>>>> = OnceLock::new();
static PICKED: Mutex<Option<String>> = Mutex::new(None);
static SCREEN_ON: AtomicBool = AtomicBool::new(false);
static FRAME: AtomicUsize = AtomicUsize::new(0);
static SELFTEST_DONE: AtomicBool = AtomicBool::new(false);
static START: OnceLock<std::time::Instant> = OnceLock::new();

/// 未读到 Info.plist 时的兜底版本号。
const FALLBACK_VERSION: &str = "0.3.0-ios.1";

// ── Swift 侧实现（ios/main.swift）───────────────────────────────────────
unsafe extern "C" {
    fn nswt_ios_present_text(id: u64, value: *const c_char, kind: u32);
    fn nswt_ios_present_picker();
    fn nswt_ios_set_idle_disabled(disabled: i32);
    fn nswt_ios_copy_text(text: *const c_char);
}

// ── Rust → Swift 调用入口（供 platform.rs 用）──────────────────────────
pub(crate) fn edit_text(id: u64, value: &str, kind: InputKind) {
    let kind = match kind {
        InputKind::Text => 0,
        InputKind::Password => 1,
        InputKind::Integer => 2,
        InputKind::Decimal => 3,
    };
    // 内嵌 NUL 不可能出现在界面输入里；真出现就退化成空串，不 panic
    let value = std::ffi::CString::new(value).unwrap_or_default();
    unsafe { nswt_ios_present_text(id, value.as_ptr(), kind) };
}

pub(crate) fn take_edited_text(id: u64) -> Option<String> {
    EDITS
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .remove(&id)
        .flatten()
}

pub(crate) fn pick_osm_file() {
    *PICKED.lock().unwrap() = None;
    unsafe { nswt_ios_present_picker() };
}

/// 用户选完文件（Swift 已把文件拷进沙盒）后返回其路径。
pub(crate) fn take_picked_file() -> Option<String> {
    PICKED.lock().unwrap().take()
}

pub(crate) fn set_keep_screen_on(enabled: bool) {
    // update() 每帧都调，只有变化时才过桥
    if SCREEN_ON.swap(enabled, Ordering::Relaxed) != enabled {
        unsafe { nswt_ios_set_idle_disabled(i32::from(enabled)) };
    }
}

pub(crate) fn copy_text(text: &str) {
    let text = std::ffi::CString::new(text).unwrap_or_default();
    unsafe { nswt_ios_copy_text(text.as_ptr()) };
}

pub(crate) fn version_name() -> String {
    VERSION
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| FALLBACK_VERSION.to_string())
}

pub(crate) fn safe_insets() -> [f32; 4] {
    *INSETS.lock().unwrap()
}

/// 应用沙盒内的数据目录（Swift 启动时给 Application Support；缺省回退 Documents）。
pub(crate) fn data_dir() -> PathBuf {
    if let Some(dir) = HOME_DIR.lock().unwrap().clone() {
        return dir;
    }
    let fallback = std::env::var("HOME")
        .map(|home| PathBuf::from(home).join("Documents").join("NekoSportsWorldTool"))
        .unwrap_or_else(|_| PathBuf::from("."));
    let _ = std::fs::create_dir_all(&fallback);
    fallback
}

/// 自检钩子：CI 里带 NSWT_SELFTEST=1 启动时，跑满 8 秒后切一次标签页，
/// 让 CI 能比较切换前后的截图（证明"能起、能画、能重绘"）。
pub(crate) fn maybe_selftest(tab: &mut usize) {
    if std::env::var("NSWT_SELFTEST").as_deref() != Ok("1") {
        return;
    }
    let started = *START.get_or_init(std::time::Instant::now);
    let _ = FRAME.fetch_add(1, Ordering::Relaxed);
    if started.elapsed() >= std::time::Duration::from_secs(8)
        && !SELFTEST_DONE.swap(true, Ordering::Relaxed)
    {
        *tab = 1;
        eprintln!("SELFTEST: switched tab to 1");
    }
}

// ── Swift → Rust 回调 ──────────────────────────────────────────────────
/// # Safety
/// 指针来自 Swift 的 `withCString`，调用期间有效；null 表示"用户取消"。
unsafe fn read_cstr(pointer: *const c_char) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    Some(CStr::from_ptr(pointer).to_string_lossy().into_owned())
}

#[no_mangle]
pub unsafe extern "C" fn nswt_ios_set_home(path: *const c_char) {
    if let Some(dir) = read_cstr(path) {
        let dir = PathBuf::from(dir);
        let _ = std::fs::create_dir_all(&dir);
        *HOME_DIR.lock().unwrap() = Some(dir);
    }
}

#[no_mangle]
pub unsafe extern "C" fn nswt_ios_set_version(text: *const c_char) {
    if let Some(version) = read_cstr(text) {
        if !version.is_empty() {
            *VERSION.lock().unwrap() = Some(version);
        }
    }
}

#[no_mangle]
pub extern "C" fn nswt_ios_set_insets(left: f32, top: f32, right: f32, bottom: f32) {
    *INSETS.lock().unwrap() = [left.max(0.0), top.max(0.0), right.max(0.0), bottom.max(0.0)];
}

#[no_mangle]
pub unsafe extern "C" fn nswt_ios_text_result(id: u64, text: *const c_char) {
    // 未预置 key 也能收（insert 覆盖），None = 取消 → UI 保留原值
    EDITS
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .insert(id, read_cstr(text));
}

#[no_mangle]
pub unsafe extern "C" fn nswt_ios_file_result(path: *const c_char) {
    *PICKED.lock().unwrap() = read_cstr(path);
}

#[no_mangle]
pub unsafe extern "C" fn nswt_ios_log(text: *const c_char) {
    if let Some(line) = read_cstr(text) {
        eprintln!("[ios] {line}");
    }
}
