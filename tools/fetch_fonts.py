"""Fetches the optional reading fonts (ADR-0027) and writes static 400/500/600/700 cuts.

Run from the repository root: python3 tools/fetch_fonts.py   (needs `pip install fonttools brotli`)
Sources are the google/fonts OFL directory. Variable fonts are instanced at fixed weights (and a
text optical size) because the app bundles one file per weight, like Inter and Vazirmatn. Latin
faces are subset to Latin, Latin Extended and common punctuation to keep the bundle small.
"""
import io
import urllib.request
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "app/assets/fonts"
BASE = "https://raw.githubusercontent.com/google/fonts/main/ofl"
WEIGHTS = {400: "Regular", 500: "Medium", 600: "SemiBold", 700: "Bold"}
LATIN = (
    "U+0000-024F,U+0259,U+02B0-02FF,U+0300-036F,U+1E00-1EFF,U+2000-206F,U+20A0-20CF,"
    "U+2100-218F,U+2190-21FF,U+2212,U+2215,U+FB00-FB06,U+FEFF,U+FFFD"
)

# family name in the app, google/fonts dir, file, fixed axes, subset to Latin
FONTS = [
    ("SourceSerif4", "sourceserif4", "SourceSerif4[opsz,wght].ttf", {"opsz": 16}, True),
    ("Literata", "literata", "Literata[opsz,wght].ttf", {"opsz": 14}, True),
    ("AtkinsonNext", "atkinsonhyperlegiblenext", "AtkinsonHyperlegibleNext[wght].ttf", {}, True),
    ("MarkaziText", "markazitext", "MarkaziText[wght].ttf", {}, False),
    ("NotoNaskhArabic", "notonaskharabic", "NotoNaskhArabic[wght].ttf", {}, False),
]
STATIC = [("IBMPlexSansArabic", "ibmplexsansarabic", "IBMPlexSansArabic-{style}.ttf")]


def fetch(path):
    with urllib.request.urlopen(f"{BASE}/{path}", timeout=120) as r:
        return r.read()


def subset_latin(font):
    opts = subset.Options()
    opts.layout_features = ["*"]
    opts.name_IDs = ["*"]
    opts.notdef_outline = True
    s = subset.Subsetter(opts)
    s.populate(unicodes=subset.parse_unicodes(LATIN))
    s.subset(font)


def main():
    for family, folder, file, axes, latin in FONTS:
        data = fetch(f"{folder}/{file}")
        for w, style in WEIGHTS.items():
            font = TTFont(io.BytesIO(data))
            font = instancer.instantiateVariableFont(font, {"wght": w, **axes}, updateFontNames=False)
            if latin:
                subset_latin(font)
            font.save(OUT / f"{family}-{style}.ttf")
        (OUT / f"{family}-OFL.txt").write_bytes(fetch(f"{folder}/OFL.txt"))
        print("wrote", family)
    for family, folder, pattern in STATIC:
        for style in WEIGHTS.values():
            (OUT / f"{family}-{style}.ttf").write_bytes(fetch(f"{folder}/{pattern.format(style=style)}"))
        (OUT / f"{family}-OFL.txt").write_bytes(fetch(f"{folder}/OFL.txt"))
        print("wrote", family)


if __name__ == "__main__":
    main()
