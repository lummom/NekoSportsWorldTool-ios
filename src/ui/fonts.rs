//! 中文字体安装。
//!
//! 移动端（Android / iOS）用打包进二进制的思源黑体；桌面从 Windows 字体目录
//! 加载微软雅黑等，找不到返回 false（不崩溃，界面中文会显示为方块）。

#[cfg(not(any(target_os = "android", target_os = "ios")))]
use std::path::PathBuf;

#[cfg(any(target_os = "android", target_os = "ios"))]
const BUNDLED_FONT: &[u8] = include_bytes!("../../assets/fonts/NotoSansSC-VF.ttf");

/// 字体候选（按优先级）。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
const CANDIDATES: [&str; 4] = ["msyh.ttc", "msyh.ttf", "simhei.ttf", "simsun.ttc"];

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn fonts_dir() -> PathBuf {
    std::env::var("WINDIR")
        .map(|w| PathBuf::from(w).join("Fonts"))
        .unwrap_or_else(|_| PathBuf::from(r"C:\Windows\Fonts"))
}

/// 安装中文字体到 egui。返回实际加载的字体名（None = 全部失败）。
pub fn install(ctx: &egui::Context) -> Option<String> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let name = "Noto Sans SC".to_owned();
        let mut fonts = egui::FontDefinitions::default();
        fonts
            .font_data
            .insert(name.clone(), egui::FontData::from_static(BUNDLED_FONT));
        if let Some(list) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            list.insert(0, name.clone());
        }
        if let Some(list) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            list.push(name.clone());
        }
        ctx.set_fonts(fonts);
        return Some(name);
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
    let dir = fonts_dir();
    for name in CANDIDATES {
        let path = dir.join(name);
        let Ok(data) = std::fs::read(&path) else {
            continue;
        };
        let mut fonts = egui::FontDefinitions::default();
        // ttc 集合取 index 0（msyh.ttc[0] = Microsoft YaHei）
        fonts
            .font_data
            .insert(name.to_string(), egui::FontData::from_owned(data));
        // proportional：插到最前（中文优先由雅黑渲染）
        if let Some(list) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            list.insert(0, name.to_string());
        }
        // monospace：追加（数字/ASCII 仍用默认等宽，中文回落雅黑）
        if let Some(list) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            list.push(name.to_string());
        }
        ctx.set_fonts(fonts);
        return Some(name.to_string());
    }
    None
    }
}
