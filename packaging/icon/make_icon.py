"""Draws the app icon from the design tokens (accent + paper) and writes every platform's sizes.

Run from the repository root: python3 packaging/icon/make_icon.py
A notebook page with ruled lines and a bookmark: no letters, so the codename can change (§1).
"""
from pathlib import Path
from PIL import Image, ImageDraw

ACCENT = (0x22, 0x50, 0x5E, 255)
PAPER = (0xF7, 0xF4, 0xEE, 255)
S = 1024
ROOT = Path(__file__).resolve().parents[2]


def draw(size=S, rounded=False, inset=0.0):
    k = 4  # supersample
    n = size * k
    img = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    pad = int(n * inset)
    if rounded:
        d.rounded_rectangle([pad, pad, n - pad - 1, n - pad - 1], radius=int(n * 0.225), fill=ACCENT)
    else:
        d.rectangle([pad, pad, n - 1 - pad, n - 1 - pad], fill=ACCENT)
    u = (n - 2 * pad) / S  # design units
    def box(x0, y0, x1, y1):
        return [pad + x0 * u, pad + y0 * u, pad + x1 * u, pad + y1 * u]
    d.rounded_rectangle(box(292, 212, 732, 812), radius=int(36 * u), fill=PAPER)
    for y in (420, 520, 620):
        d.rounded_rectangle(box(372, y - 10, 652, y + 10), radius=int(10 * u), fill=ACCENT)
    # Bookmark ribbon hanging from the top edge.
    x0, x1, top, bottom, notch = 556, 636, 212, 352, 316
    d.polygon([(pad + x0 * u, pad + top * u), (pad + x1 * u, pad + top * u), (pad + x1 * u, pad + bottom * u),
               (pad + (x0 + x1) / 2 * u, pad + notch * u), (pad + x0 * u, pad + bottom * u)], fill=ACCENT)
    return img.resize((size, size), Image.LANCZOS)


def save(img, path, size):
    path.parent.mkdir(parents=True, exist_ok=True)
    img.resize((size, size), Image.LANCZOS).save(path)


def main():
    square = draw()
    rounded = draw(rounded=True, inset=0.04)
    # Android legacy launcher icons.
    for folder, px in {"mdpi": 48, "hdpi": 72, "xhdpi": 96, "xxhdpi": 144, "xxxhdpi": 192}.items():
        save(rounded, ROOT / f"app/android/app/src/main/res/mipmap-{folder}/ic_launcher.png", px)
    # iOS: the system masks the corners, so the square full-bleed art.
    ios = ROOT / "app/ios/Runner/Assets.xcassets/AppIcon.appiconset"
    for f in ios.glob("*.png"):
        with Image.open(f) as old:
            px = old.size[0]
        save(square.convert("RGB"), f, px)
    # macOS: the art carries its own rounded square.
    mac = ROOT / "app/macos/Runner/Assets.xcassets/AppIcon.appiconset"
    for f in mac.glob("*.png"):
        with Image.open(f) as old:
            px = old.size[0]
        save(rounded, f, px)
    # Windows .ico and the Linux/Flatpak icon.
    rounded.save(ROOT / "app/windows/runner/resources/app_icon.ico",
                 sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    save(rounded, ROOT / "packaging/linux/dev.daftar.Daftar.png", 512)
    # MSIX logos.
    for name, px in {"Square44x44Logo": 44, "Square150x150Logo": 150, "StoreLogo": 50}.items():
        save(rounded, ROOT / f"packaging/windows/Assets/{name}.png", px)
    save(rounded, ROOT / "packaging/icon/icon-1024.png", 1024)


if __name__ == "__main__":
    main()
