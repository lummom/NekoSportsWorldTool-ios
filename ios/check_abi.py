#!/usr/bin/env python3
"""检查 Rust ↔ Swift 的 C ABI 符号名是否一一对应。

这条检查是"没 Mac 也能防住盲写错误"的关键：符号名对不上时 linker 只会报
"Undefined symbols"，要等一轮 CI 才知道；本地跑这个脚本 0.1 秒就能发现。

用法: python3 ios/check_abi.py
"""
import io
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LIB_RS = os.path.join(ROOT, "src", "lib.rs")
IOS_RS = os.path.join(ROOT, "src", "ios.rs")
SWIFT = os.path.join(ROOT, "ios", "main.swift")

lib_rs = io.open(LIB_RS, encoding="utf-8").read()
ios_rs = io.open(IOS_RS, encoding="utf-8").read()
swift = io.open(SWIFT, encoding="utf-8").read()

# Rust 导出给 Swift 用的入口（#[no_mangle] pub extern "C" fn ...）
rust_exports = set(re.findall(r'#\[no_mangle\]\s*\npub (?:unsafe )?extern "C" fn (\w+)', lib_rs + "\n" + ios_rs))
# Rust 里声明、由 Swift 实现的外部符号（只在 extern "C" 块里找，否则会把 Rust 自己的定义也算进来）
block = re.search(r'unsafe extern "C" \{(.*?)\n\}', ios_rs, re.S)
rust_imports = set(re.findall(r'fn (nswt_ios_\w+)\(', block.group(1))) if block else set()
# Swift 里声明的 Rust 符号 / Swift 自己实现的符号
swift_wants = set(re.findall(r'@_silgen_name\("(\w+)"\)', swift))
swift_provides = set(re.findall(r'@_cdecl\("(\w+)"\)', swift))

problems = []
for name in sorted(swift_wants - rust_exports):
    problems.append(f"Swift 要调用 {name}，但 Rust 没有导出（#[no_mangle] 名字不一致？）")
for name in sorted(rust_exports - swift_wants):
    problems.append(f"Rust 导出了 {name}，但 Swift 没有声明（多余/名字不一致）")
for name in sorted(rust_imports - swift_provides):
    problems.append(f"Rust 要调用 {name}，但 Swift 没有实现（@_cdecl 名字不一致？）")

print(f"Rust 导出 {len(rust_exports)} 个、Swift 声明 {len(swift_wants)} 个；"
      f"Rust 需要 {len(rust_imports)} 个、Swift 实现 {len(swift_provides)} 个")
print("  Rust 导出:", ", ".join(sorted(rust_exports)))
print("  Swift 实现:", ", ".join(sorted(swift_provides)))

if problems:
    for item in problems:
        print("  ✗", item, file=sys.stderr)
    sys.exit(1)
print("ABI 符号名一致 ✓")
