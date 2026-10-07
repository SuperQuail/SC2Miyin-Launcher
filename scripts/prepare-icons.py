"""由弥音立绘生成 Tauri 所需的应用图标。

用法：
    python scripts/prepare-icons.py

输出到 src-tauri/icons/。
"""

from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
PORTRAIT = ROOT / "assets-staging" / "miyin" / "portrait.png"
OUT = ROOT / "src-tauri" / "icons"

PNG_SIZES = {"32x32.png": 32, "128x128.png": 128, "128x128@2x.png": 256, "icon.png": 512}
ICO_SIZES = [16, 24, 32, 48, 64, 128, 256]

# 聚焦面部：从中心略偏上开始裁一个正方形
FOCUS_Y = 0.34


def square(size: int) -> Image.Image:
    """把立绘裁成正方形并缩放到指定尺寸。"""
    source = Image.open(PORTRAIT).convert("RGB")

    side = min(source.width, source.height)
    left = (source.width - side) // 2
    top = max(0, min(source.height - side, round(source.height * FOCUS_Y - side / 2)))
    cropped = source.crop((left, top, left + side, top + side))

    return cropped.resize((size, size), Image.LANCZOS)


def main() -> int:
    if not PORTRAIT.is_file():
        raise SystemExit(f"缺少立绘：{PORTRAIT}")

    OUT.mkdir(parents=True, exist_ok=True)

    for name, size in PNG_SIZES.items():
        square(size).save(OUT / name, "PNG", optimize=True)
        print(f"{name:<18}{size}x{size}")

    square(256).save(OUT / "icon.ico", sizes=[(s, s) for s in ICO_SIZES])
    print(f"{'icon.ico':<18}{len(ICO_SIZES)} sizes")

    return 0


if __name__ == "__main__":
    sys.exit(main())
