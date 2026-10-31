"""Package the approved raster symbol into desktop icon formats.

Run: python3 scripts/build_icons.py (requires Pillow).
The source artwork and original generation references remain untouched.
"""

from pathlib import Path
import json

from PIL import Image


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "assets/branding/cownter-symbol.png"
DEST = ROOT / "assets/icons"
SIZES = (16, 20, 24, 32, 40, 48, 64, 128, 256, 512, 1024)


def main():
    DEST.mkdir(parents=True, exist_ok=True)
    with Image.open(SOURCE) as original:
        master = original.convert("RGBA")
    if master.width != master.height:
        raise ValueError("App icon source must be square")
    if master.getchannel("A").getextrema() != (0, 255):
        raise ValueError("App icon source must include transparent and opaque pixels")

    for size in SIZES:
        icon = master.resize((size, size), Image.Resampling.LANCZOS)
        icon.save(DEST / f"{size}x{size}.png")

    icon = master.resize((1024, 1024), Image.Resampling.LANCZOS)
    icon.save(DEST / "icon.png")
    icon.save(DEST / "icon.ico", sizes=[(s, s) for s in (16, 24, 32, 48, 64, 128, 256)])
    icon.save(DEST / "icon.icns", sizes=[(s, s) for s in (16, 32, 64, 128, 256, 512, 1024)])

    # Native tray templates use the approved symbol's alpha as a monochrome mask.
    for color, rgb in (("dark", (36, 44, 53)), ("light", (255, 255, 255))):
        for size in (16, 20, 22, 24, 32, 44, 48, 64):
            template = Image.new("RGBA", (size, size), rgb + (0,))
            template.putalpha(master.getchannel("A").resize((size, size), Image.Resampling.LANCZOS))
            template.save(DEST / f"tray-{color}-{size}.png")

    manifest = {
        "name": "Cownter",
        "logo": "../branding/cownter-logo.png",
        "symbol": "../branding/cownter-symbol.png",
        "app": {"png": "icon.png", "windows": "icon.ico", "macos": "icon.icns"},
        "png_sizes": list(SIZES),
        "tray": {
            "dark": "tray-dark-32.png",
            "light": "tray-light-32.png",
            "macos_template_1x": "tray-dark-22.png",
            "macos_template_2x": "tray-dark-44.png",
        },
    }
    (DEST / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Generated desktop and tray icons in {DEST}")


if __name__ == "__main__":
    main()
