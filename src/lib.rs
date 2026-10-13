//! Shared desktop and Android application.

mod api;
pub mod cli;
mod crypto;
pub mod location;
pub mod platform;
pub mod template;
mod relaunch;
mod textlog;
mod track;
mod update;

#[cfg(feature = "gui")]
mod ui;

#[cfg(all(target_os = "android", feature = "android"))]
mod android;

// iOS 宿主层：Android/iOS 目标编译；桌面 host 带 --features ios 时也编译，
// 这样没有 Mac 也能在本地做类型检查（见 ios/README.md）。
#[cfg(any(target_os = "ios", feature = "ios"))]
mod ios;

#[cfg(feature = "gui")]
/// iOS 入口：由 ios/main.swift 调用。winit 的 iOS 后端会在内部调用
/// UIApplicationMain 并接管生命周期（因此这里必须在 UIApplication 初始化前被调用）。
#[cfg(all(target_os = "ios", feature = "ios"))]
#[no_mangle]
pub extern "C" fn start_ios_app() {
    let options = eframe::NativeOptions {
        run_and_return: false,
        ..Default::default()
    };
    if let Err(error) = run_gui(options) {
        eprintln!("iOS GUI 启动失败: {error}");
    }
}

#[cfg(feature = "gui")]
pub fn run_gui(options: eframe::NativeOptions) -> eframe::Result<()> {
    eframe::run_native(
        "NekoSportsWorldTool",
        options,
        Box::new(|cc| {
            #[cfg(target_os = "android")]
            android::set_context(&cc.egui_ctx);
            Ok(Box::new(ui::App::new(cc)))
        }),
    )
}
