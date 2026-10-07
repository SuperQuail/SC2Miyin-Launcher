"""把原始美术素材压缩为 Web 尺寸，输出到 ui/public/。

用法：
    python scripts/prepare-assets.py

- 战役美术来自 assets-staging/campaigns/ 与 _candidates/（Blizzard 版权，仅作标识性使用）
- 弥音立绘来自 assets-staging/miyin/
原始素材不入库；ui/public/ 下的产出才是应用实际引用的文件。
"""

from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
STAGING = ROOT / "assets-staging"
OUT = ROOT / "ui" / "public"

# slug -> (源文件, 最大宽度)
CARDS = {
    "wol": ("campaigns/wol.png", 960),
    "hots": ("campaigns/hots.png", 960),
    "lotv": ("campaigns/lotv.jpg", 960),
    "nco": ("campaigns/nco.png", 960),
    "lotv-prologue": ("campaigns/lotv-prologue.png", 960),
    "sc-remastered": ("campaigns/sc-remastered.png", 960),
}

# 应用背景：星际争霸 II 主视觉
BACKDROP = ("_candidates/c11_MarineHydraliskZealot_SC2_Art1.webp", 1920)
# 顶部 Logo：保留透明通道
LOGO = ("campaigns/sc2-logo.png", 512)

# 弥音立绘：输出名 -> (源文件, 最大宽度)
MIYIN = {
    "wink": ("miyin/wink.png", 680),
    "chibi": ("miyin/chibi.png", 440),
    "portrait": ("miyin/portrait.png", 560),
    "cry": ("miyin/cry.jpg", 400),
}


def load(relative: str) -> Image.Image:
    path = STAGING / relative
    if not path.is_file():
        raise SystemExit(f"缺少素材：{path}")
    return Image.open(path)


def save(image: Image.Image, relative: str, max_width: int, quality: int = 86) -> tuple[int, int, int]:
    """等比缩放到最大宽度并保存；PNG 保留透明，JPEG 铺深色底。"""
    target = OUT / relative
    target.parent.mkdir(parents=True, exist_ok=True)

    if image.width > max_width:
        ratio = max_width / image.width
        image = image.resize((max_width, round(image.height * ratio)), Image.LANCZOS)

    if target.suffix.lower() == ".png":
        image.convert("RGBA").save(target, "PNG", optimize=True)
    else:
        if image.mode in ("RGBA", "LA", "P"):
            background = Image.new("RGB", image.size, (12, 10, 24))
            rgba = image.convert("RGBA")
            background.paste(rgba, mask=rgba.split()[-1])
            image = background
        else:
            image = image.convert("RGB")
        image.save(target, "JPEG", quality=quality, optimize=True, progressive=True)

    return image.width, image.height, target.stat().st_size


def main() -> int:
    if not STAGING.is_dir():
        raise SystemExit(f"缺少原始素材目录：{STAGING}")

    outputs: list[tuple[str, int, int, int]] = []

    for slug, (source, max_width) in CARDS.items():
        outputs.append((f"campaigns/{slug}.jpg", *save(load(source), f"campaigns/{slug}.jpg", max_width)))

    source, max_width = BACKDROP
    outputs.append(("backdrop.jpg", *save(load(source), "backdrop.jpg", max_width, quality=80)))

    source, max_width = LOGO
    outputs.append(("campaigns/sc2-logo.png", *save(load(source), "campaigns/sc2-logo.png", max_width)))

    for name, (source, max_width) in MIYIN.items():
        outputs.append((f"miyin/{name}.png", *save(load(source), f"miyin/{name}.png", max_width)))

    print(f"{'file':<34}{'size':<14}{'bytes':>10}")
    for name, width, height, size in outputs:
        print(f"{name:<34}{f'{width}x{height}':<14}{size:>10,}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
