//! Platform services shared by the existing UI and the mobile hosts.

#[derive(Clone, Copy, Debug)]
pub enum InputKind {
    Text,
    Password,
    Integer,
    Decimal,
}

pub fn data_dir() -> std::path::PathBuf {
    #[cfg(test)]
    if let Some(path) = TEST_DATA_DIR.with(|p| p.borrow().clone()) {
        return path;
    }
    #[cfg(target_os = "android")]
    {
        crate::android::data_dir()
    }
    #[cfg(target_os = "ios")]
    {
        crate::ios::data_dir()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."))
    }
}

pub fn edit_text(id: u64, value: &str, kind: InputKind) {
    #[cfg(target_os = "android")]
    crate::android::edit_text(id, value, kind);
    #[cfg(target_os = "ios")]
    crate::ios::edit_text(id, value, kind);
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let _ = (id, value, kind);
}

pub fn take_edited_text(id: u64) -> Option<String> {
    #[cfg(target_os = "android")]
    {
        crate::android::take_edited_text(id)
    }
    #[cfg(target_os = "ios")]
    {
        crate::ios::take_edited_text(id)
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let _ = id;
        None
    }
}

pub fn set_keep_screen_on(enabled: bool) {
    #[cfg(target_os = "android")]
    crate::android::set_keep_screen_on(enabled);
    #[cfg(target_os = "ios")]
    crate::ios::set_keep_screen_on(enabled);
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let _ = enabled;
}

/// 当前程序版本：桌面读 Cargo 版本，Android 读 manifest versionName（形如 0.2.5-android.3），
/// iOS 读 Info.plist 的 CFBundleShortVersionString（形如 0.3.0-ios.1，由 main.swift 传入）。
pub fn version_name() -> String {
    #[cfg(target_os = "android")]
    {
        crate::android::version_name()
    }
    #[cfg(target_os = "ios")]
    {
        crate::ios::version_name()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

/// Android：把已下载的 APK 交给系统安装器（用户在系统弹窗确认安装）。
/// iOS：不允许自装，留空实现（关于页在 iOS 上没有可下载的自更新产物）。
pub fn install_apk(path: &str) {
    #[cfg(target_os = "android")]
    crate::android::install_apk(path);
    #[cfg(not(target_os = "android"))]
    let _ = path;
}

#[cfg(feature = "gui")]
pub fn sync_clipboard(context: &egui::Context) {
    #[cfg(target_os = "android")]
    {
        let text = context.output(|output| output.copied_text.clone());
        if !text.is_empty() { crate::android::copy_text(&text); }
    }
    #[cfg(target_os = "ios")]
    {
        let text = context.output(|output| output.copied_text.clone());
        if !text.is_empty() { crate::ios::copy_text(&text); }
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let _ = context;
}

#[cfg(feature = "gui")]
pub fn apply_safe_area(context: &egui::Context, input: &mut egui::RawInput) {
    #[cfg(target_os = "android")]
    if let Some(rect) = input.screen_rect.as_mut() {
        let [left, top, right, bottom] = crate::android::safe_insets();
        let scale = context.pixels_per_point();
        rect.min.x += left as f32 / scale;
        rect.min.y += top as f32 / scale;
        rect.max.x -= right as f32 / scale;
        rect.max.y -= bottom as f32 / scale;
    }
    #[cfg(target_os = "ios")]
    if let Some(rect) = input.screen_rect.as_mut() {
        // iOS 的刘海/底部小白条：Swift 侧把 safeAreaInsets（点）推过来，这里按缩放换算
        let [left, top, right, bottom] = crate::ios::safe_insets();
        let scale = context.pixels_per_point();
        rect.min.x += left / scale;
        rect.min.y += top / scale;
        rect.max.x -= right / scale;
        rect.max.y -= bottom / scale;
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let _ = (context, input);
}

#[cfg(test)]
thread_local! {
    pub(crate) static TEST_DATA_DIR: std::cell::RefCell<Option<std::path::PathBuf>> = const { std::cell::RefCell::new(None) };
}
