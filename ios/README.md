# iOS 版（未签名侧载包）

把现有 egui 界面原样搬到 iPhone/iPad：**渲染走 `eframe` 的 wgpu 后端**（`eframe 0.29` 自带
iOS 分支；iOS 上 `glow/glutin` 没有后端，所以 `Cargo.toml` 里 iOS 目标单独开 `wgpu`），
UIKit 缺的那几样（系统键盘输入框、文件选择器、常亮、安全区、剪贴板）由 `ios/main.swift`
提供，经 C ABI 与 `src/ios.rs` 对接。

## 怎么构建

**只能在 macOS 上编译**（需要 Xcode 的 iOS SDK）。两条路：

```bash
# 1) GitHub Actions（本仓库自带 .github/workflows/ios.yml）
#    push 后自动跑：check / desktop 回归 / ipa / 模拟器验收

# 2) 有 Mac 时本地跑
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
bash ios/build.sh simulator release   # 给模拟器：dist/NekoSportsWorldTool.app
bash ios/build.sh device release      # 给真机：dist/NekoSportsWorldTool-ios-arm64.ipa
```

没有 Mac 的 Windows 机器上，也能对 iOS 代码做**类型检查**（`src/ios.rs` 会在桌面 host 上一起编译）：

```powershell
cargo check --features ios --lib        # 只查 iOS 桥接层
cargo check --features gui              # 顺带确认桌面没被改坏
```

## 怎么装到 iPhone（没有 Mac 也能做）

产物是**未签名** `.ipa`，需要自己用 Apple ID 签一次：

1. Windows 装 [Sideloadly](https://sideloadly.io/)（另有 iTunes/Apple Devices 驱动要求），插上 iPhone；
2. 把 `NekoSportsWorldTool-ios-arm64.ipa` 拖进去，填自己的 Apple ID，Start；
3. 手机上：设置 → 通用 → VPN与设备管理 → 信任该开发者；
4. **免费 Apple ID 签的包 7 天过期**，到期重签一次即可（数据不会丢，签名不变时可直接覆盖）。

注意：免费账号有 3 个 app / 10 个 App ID 的限制；换 Apple ID 或换机器签出来的包不能互相覆盖安装。

## 已验证 / 未验证

- 已验证（CI，模拟器）：能启动、egui 画得出来、换页重绘正常（自检截图对比）、无 panic。
- **未验证**：真机侧载安装、软键盘中文输入法、真机性能 —— 这些必须有 iPhone 才能测。
- 已知裁剪：iOS 上没有应用内自更新（关于页会提示去 Release 页下载）；桌面版的自替换逻辑在 iOS 不编译。
