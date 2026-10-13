#!/usr/bin/env bash
# 构建 iOS app（真机 / 模拟器），并做结构与架构校验。
#
# 用法: ios/build.sh <device|simulator> [release|debug]
# 产物: dist/NekoSportsWorldTool.app（device 另出 dist/NekoSportsWorldTool-ios-arm64.ipa）
set -euo pipefail

KIND="${1:-device}"
PROFILE="${2:-release}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/dist"
NAME="NekoSportsWorldTool"

case "$KIND" in
  device)    TRIPLE="aarch64-apple-ios";     SDK="iphoneos";        SWIFT_TARGET="arm64-apple-ios14.0";           MIN_FLAG="-mios-version-min=14.0" ;;
  simulator) TRIPLE="aarch64-apple-ios-sim"; SDK="iphonesimulator"; SWIFT_TARGET="arm64-apple-ios14.0-simulator"; MIN_FLAG="-mios-simulator-version-min=14.0" ;;
  *) echo "用法: $0 <device|simulator> [release|debug]" >&2; exit 2 ;;
esac

# 两件事必须显式指定（否则 CI 会分别在链接和部署目标上踩坑）：
#   1) Rust 侧调用的 4 个 Swift 函数（见 src/ios.rs）在 cargo 链接 cdylib 副产物时还不存在，
#      iOS 的 staticlib 才是真正交给 Swift 链接的东西 → 允许 .dylib 留未定义符号。
#   2) rustc 对 aarch64-apple-ios 默认按 10.0 链接，而 ring 等 C 代码按当前 SDK(26.x) 编译
#      → 统一部署目标，避免 "built for newer iOS version" 链接警告与潜在兼容问题。
export IPHONEOS_DEPLOYMENT_TARGET=14.0
export RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-Wl,-undefined,dynamic_lookup -C link-arg=$MIN_FLAG"

if [ "$PROFILE" = "release" ]; then CARGO_FLAGS="--release"; PROFILE_DIR="release"; else CARGO_FLAGS=""; PROFILE_DIR="debug"; fi

rustup target add "$TRIPLE" >/dev/null 2>&1 || true
cargo build --target "$TRIPLE" --features ios --lib $CARGO_FLAGS

LIB="$ROOT/target/$TRIPLE/$PROFILE_DIR/libnekosportsworldtool.a"
if [ ! -f "$LIB" ]; then echo "缺少静态库: $LIB" >&2; exit 1; fi

APP="$OUT/$NAME.app"
rm -rf "$APP"
mkdir -p "$APP"

SDK_PATH="$(xcrun --sdk "$SDK" --show-sdk-path)"
xcrun --sdk "$SDK" swiftc -O -sdk "$SDK_PATH" -target "$SWIFT_TARGET" \
  -Xlinker -syslibroot -Xlinker "$SDK_PATH" \
  "$ROOT/ios/main.swift" \
  -L "$(dirname "$LIB")" -lnekosportsworldtool \
  -framework UIKit -framework Foundation -framework CoreGraphics -framework QuartzCore \
  -framework Metal -framework CoreText -framework Security -framework UniformTypeIdentifiers \
  -lc++ \
  -o "$APP/$NAME"

cp "$ROOT/ios/Info.plist" "$APP/Info.plist"

# 模拟器不强制签名，但签个 ad-hoc 更接近真机
if [ "$KIND" = "simulator" ]; then
  codesign --force --sign - "$APP" || true
fi

echo "=== 结构校验 ==="
plutil -lint "$APP/Info.plist"
file "$APP/$NAME"
lipo -info "$APP/$NAME"
otool -l "$APP/$NAME" | grep -A4 "LC_BUILD_VERSION" | head -8 || true
test -x "$APP/$NAME"
echo "APP: $APP"

if [ "$KIND" = "device" ]; then
  IPA="$OUT/$NAME-ios-arm64.ipa"
  rm -f "$IPA"
  rm -rf "$OUT/Payload"
  mkdir -p "$OUT/Payload"
  cp -R "$APP" "$OUT/Payload/"
  ( cd "$OUT" && zip -qry "$IPA" Payload )
  rm -rf "$OUT/Payload"
  echo "IPA: $IPA"
  unzip -l "$IPA" | head -12
fi
