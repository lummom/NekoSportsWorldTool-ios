// NekoSportsWorldTool 的 iOS 入口 + UIKit 桥。
//
// 两个职责：
//  1) 在 UIApplication 初始化之前把控制权交给 Rust —— winit 的 iOS 后端会自己调用
//     UIApplicationMain（它刻意不注册 app delegate），所以这里必须是进程的 main。
//  2) 实现 Rust 侧声明的 UIKit 能力：系统键盘输入框、文件选择器、常亮、剪贴板、安全区。
//
// Rust 侧对应实现见 src/ios.rs；符号名两边必须一致。

import UIKit
import UniformTypeIdentifiers

// ── Rust 导出（src/ios.rs / src/lib.rs）────────────────────────────────
@_silgen_name("nswt_ios_start")        func nswt_ios_start()
@_silgen_name("nswt_ios_set_home")     func nswt_ios_set_home(_ path: UnsafePointer<CChar>?)
@_silgen_name("nswt_ios_set_version")  func nswt_ios_set_version(_ text: UnsafePointer<CChar>?)
@_silgen_name("nswt_ios_set_insets")   func nswt_ios_set_insets(_ l: Float, _ t: Float, _ r: Float, _ b: Float)
@_silgen_name("nswt_ios_text_result")  func nswt_ios_text_result(_ id: UInt64, _ text: UnsafePointer<CChar>?)
@_silgen_name("nswt_ios_file_result")  func nswt_ios_file_result(_ path: UnsafePointer<CChar>?)
@_silgen_name("nswt_ios_log")          func nswt_ios_log(_ text: UnsafePointer<CChar>?)

// ── 小工具 ────────────────────────────────────────────────────────────
func nswtLog(_ message: String) {
    message.withCString { nswt_ios_log($0) }
}

/// 取 key window（winit 创建的那个），用于计算安全区与展示弹窗。
func nswtKeyWindow() -> UIWindow? {
    for case let scene as UIWindowScene in UIApplication.shared.connectedScenes {
        if let window = scene.windows.first(where: { $0.isKeyWindow }) {
            return window
        }
        if let window = scene.windows.first {
            return window
        }
    }
    return nil
}

func nswtTopViewController() -> UIViewController? {
    var controller = nswtKeyWindow()?.rootViewController
    while let presented = controller?.presentedViewController {
        controller = presented
    }
    return controller
}

func nswtDocumentsDir() -> URL {
    let base = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first
        ?? URL(fileURLWithPath: NSTemporaryDirectory())
    return base
}

/// 安全区（刘海 / 底部小白条）推给 Rust；变化才过桥，免得每帧都调。
var nswtLastInsets: UIEdgeInsets?

func nswtPushInsets() {
    guard let window = nswtKeyWindow() else { return }
    let insets = window.safeAreaInsets
    if let last = nswtLastInsets, last == insets { return }
    nswtLastInsets = insets
    nswt_ios_set_insets(Float(insets.left), Float(insets.top), Float(insets.right), Float(insets.bottom))
    nswtLog("insets \(insets.top)/\(insets.bottom)")
}

// ── Rust → Swift：系统键盘输入框（对标 Android 的原生 EditText 弹窗）──
@_cdecl("nswt_ios_present_text")
public func nswt_ios_present_text(_ id: UInt64, _ value: UnsafePointer<CChar>?, _ kind: UInt32) {
    let initial = value.map { String(cString: $0) } ?? ""
    DispatchQueue.main.async {
        guard let host = nswtTopViewController() else {
            nswtLog("present_text: no view controller")
            return
        }
        let alert = UIAlertController(title: nil, message: nil, preferredStyle: .alert)
        alert.addTextField { field in
            field.text = initial
            field.clearButtonMode = .whileEditing
            field.autocorrectionType = .no
            field.autocapitalizationType = .none
            switch kind {
            case 1: field.isSecureTextEntry = true          // Password
            case 2: field.keyboardType = .numberPad         // Integer
            case 3: field.keyboardType = .decimalPad        // Decimal
            default: field.keyboardType = .default
            }
        }
        alert.addAction(UIAlertAction(title: "取消", style: .cancel) { _ in
            nswt_ios_text_result(id, nil)                    // 取消 = 保留原值
        })
        alert.addAction(UIAlertAction(title: "确定", style: .default) { _ in
            let text = alert.textFields?.first?.text ?? ""
            text.withCString { nswt_ios_text_result(id, $0) }
        })
        host.present(alert, animated: true)
    }
}

// ── Rust → Swift：OSM 文件选择器（rfd 无 iOS 后端）─────────────────────
final class NswtPickerDelegate: NSObject, UIDocumentPickerDelegate {
    static let shared = NswtPickerDelegate()

    func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
        guard let url = urls.first else {
            nswt_ios_file_result(nil)
            return
        }
        let directory = nswtDocumentsDir().appendingPathComponent("osm", isDirectory: true)
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let destination = directory.appendingPathComponent(url.lastPathComponent)
        try? FileManager.default.removeItem(at: destination)
        do {
            try FileManager.default.copyItem(at: url, to: destination)   // asCopy: true → 已是沙盒内副本
        } catch {
            nswtLog("copy osm failed: \(error)")
            nswt_ios_file_result(nil)
            return
        }
        nswtLog("osm picked: \(destination.path)")
        destination.path.withCString { nswt_ios_file_result($0) }
    }

    func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
        nswt_ios_file_result(nil)
    }
}

@_cdecl("nswt_ios_present_picker")
public func nswt_ios_present_picker() {
    DispatchQueue.main.async {
        guard let host = nswtTopViewController() else {
            nswtLog("present_picker: no view controller")
            return
        }
        let picker = UIDocumentPickerViewController(forOpeningContentTypes: [UTType.data], asCopy: true)
        picker.delegate = NswtPickerDelegate.shared
        picker.allowsMultipleSelection = false
        host.present(picker, animated: true)
    }
}

// ── Rust → Swift：常亮 / 剪贴板 ────────────────────────────────────────
@_cdecl("nswt_ios_set_idle_disabled")
public func nswt_ios_set_idle_disabled(_ disabled: Int32) {
    DispatchQueue.main.async {
        UIApplication.shared.isIdleTimerDisabled = disabled != 0
    }
}

@_cdecl("nswt_ios_copy_text")
public func nswt_ios_copy_text(_ text: UnsafePointer<CChar>?) {
    guard let text else { return }
    UIPasteboard.general.string = String(cString: text)
}

// ── 进程入口 ──────────────────────────────────────────────────────────
let version = (Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String) ?? ""
version.withCString { nswt_ios_set_version($0) }

let dataDirectory = nswtDocumentsDir().appendingPathComponent("NekoSportsWorldTool", isDirectory: true)
try? FileManager.default.createDirectory(at: dataDirectory, withIntermediateDirectories: true)
dataDirectory.path.withCString { nswt_ios_set_home($0) }

// 安全区要在 winit 建好窗口之后才读得到，用 1s 定时器兜底（portrait 锁定，实测只变一次）
Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { _ in nswtPushInsets() }

nswtLog("launching rust (version \(version))")
nswt_ios_start()   // Rust 接管：内部调用 UIApplicationMain，不会返回
