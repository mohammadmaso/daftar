"""Writes the logo and wordmark SVGs from the bundled fonts, with the name as outlines.

Run from the repository root: python3 packaging/icon/make_logo.py   (needs fonttools, uharfbuzz)
The mark is the app icon (make_icon.py); the lockups mirror `Wordmark` in
app/lib/design/components/logo.dart: mark, a gap of 0.32 × mark, the name at 0.62 × mark.
"""
from pathlib import Path

import uharfbuzz as hb
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parents[2]
FONTS = ROOT / "app/assets/fonts"
OUT = ROOT / "packaging/icon"
INK, PAPER, TEXT = "#22505E", "#F7F4EE", "#1D1C1A"
NAME_EN, NAME_FA = "Daftar", "دفتر"  # app/lib/app/identity.dart


def mark(x=0.0, s=1024.0):
    """The icon's tile, page, rules and bookmark, placed at x with side s."""
    u = s / 1024

    def rr(l, t, r, b, rad, fill):
        return (f'<rect x="{x + l * u:.2f}" y="{t * u:.2f}" width="{(r - l) * u:.2f}" '
                f'height="{(b - t) * u:.2f}" rx="{rad * u:.2f}" fill="{fill}"/>')

    parts = [rr(0, 0, 1024, 1024, 230, INK), rr(292, 212, 732, 812, 36, PAPER)]
    parts += [rr(372, y - 10, 652, y + 10, 10, INK) for y in (420, 520, 620)]
    pts = [(556, 212), (636, 212), (636, 352), (596, 316), (556, 352)]
    parts.append('<path d="M' + " L".join(f"{x + a * u:.2f} {b * u:.2f}" for a, b in pts)
                 + f' Z" fill="{INK}"/>')
    return "\n  ".join(parts)


def text_path(text, font_file, size, x, baseline, rtl=False):
    """Shapes text with HarfBuzz and returns (svg path d, advance width)."""
    data = (FONTS / font_file).read_bytes()
    face = hb.Face(data)
    font = hb.Font(face)
    buf = hb.Buffer()
    buf.add_str(text)
    buf.guess_segment_properties()
    if rtl:
        buf.direction = "rtl"
    hb.shape(font, buf, {"kern": True, "liga": True})
    tt = TTFont(FONTS / font_file)
    glyphs = tt.getGlyphSet()
    order = tt.getGlyphOrder()
    k = size / tt["head"].unitsPerEm
    pen = SVGPathPen(glyphs)
    pen_x = 0
    for info, pos in zip(buf.glyph_infos, buf.glyph_positions):
        gx = x + (pen_x + pos.x_offset) * k
        gy = baseline - pos.y_offset * k
        glyphs[order[info.codepoint]].draw(TransformPen(pen, (k, 0, 0, -k, gx, gy)))
        pen_x += pos.x_advance
    return pen.getCommands(), pen_x * k


def lockup(name, font_file, persian=False):
    s = 1024.0
    size = s * (0.66 if persian else 0.62)
    gap = s * 0.32
    tt = TTFont(FONTS / font_file)
    # Centre the name's x-height (Latin) or letter body (Persian) on the mark.
    cap = (tt["OS/2"].sxHeight or 500) / tt["head"].unitsPerEm * size
    baseline = s / 2 + (cap * 0.5 if not persian else size * 0.14)
    if persian:  # the mark sits at the start of the line, which is the right in Persian
        d, width = text_path(name, font_file, size, 0, baseline, rtl=True)
        total = width + gap + s
        body = f'<path d="{d}" fill="{TEXT}"/>\n  ' + mark(width + gap)
    else:
        d, width = text_path(name, font_file, size, s + gap, baseline)
        total = s + gap + width
        body = mark() + f'\n  <path d="{d}" fill="{TEXT}"/>'
    pad = s * 0.06
    return svg(total + 2 * pad, s + 2 * pad, body, pad)


def svg(w, h, body, pad=0.0):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{-pad:.0f} {-pad:.0f} {w:.0f} {h:.0f}">\n'
            f"  {body}\n</svg>\n")


def main():
    (OUT / "logo.svg").write_text(svg(1024, 1024, mark()))
    (OUT / "wordmark-en-sans.svg").write_text(lockup(NAME_EN, "Inter-SemiBold.ttf"))
    (OUT / "wordmark-en-serif.svg").write_text(lockup(NAME_EN, "SourceSerif4-SemiBold.ttf"))
    (OUT / "wordmark-fa.svg").write_text(lockup(NAME_FA, "Vazirmatn-Bold.ttf", persian=True))
    print("wrote logo.svg and three wordmarks")


if __name__ == "__main__":
    main()
