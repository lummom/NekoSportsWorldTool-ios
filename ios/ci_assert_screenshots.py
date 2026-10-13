#!/usr/bin/env python3
"""CI 断言：模拟器两张截图必须非空且确实变了（换页重绘生效）。

用法: python3 ios/ci_assert_screenshots.py <证据目录>
依赖: Pillow（CI 里 pip 装）
"""
import os
import sys

from PIL import Image, ImageChops

MIN_BYTES = 20_000        # 低于这个大小基本是纯色/空白
MIN_CHANGED_RATIO = 0.005  # 换页前后至少要变 0.5% 的像素

directory = sys.argv[1] if len(sys.argv) > 1 else "dist/evidence"
before = os.path.join(directory, "shot-before.png")
after = os.path.join(directory, "shot-after.png")

for path in (before, after):
    size = os.path.getsize(path)
    print(f"{path}: {size} 字节")
    if size < MIN_BYTES:
        sys.exit(f"截图疑似空白: {path}")

a = Image.open(before).convert("RGB")
b = Image.open(after).convert("RGB")
if a.size != b.size:
    sys.exit(f"两张截图尺寸不一致: {a.size} vs {b.size}")

diff = ImageChops.difference(a, b)
changed = sum(1 for pixel in diff.getdata() if sum(pixel) > 24)
ratio = changed / (a.size[0] * a.size[1])
print(f"换页前后变化像素占比: {ratio:.4%}（阈值 {MIN_CHANGED_RATIO:.2%}）")
if ratio < MIN_CHANGED_RATIO:
    sys.exit("换页前后画面几乎没变 —— egui 没渲染或没重绘")

print("渲染 + 重绘 OK")
