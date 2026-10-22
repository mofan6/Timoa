"""Subset official Sarasa UI masters without changing retained outlines/hinting.

Usage: python tools/subset-ui-fonts.py --master-dir PATH_TO_FULL_FONT_DIRECTORY
Requires fonttools. Keep full upstream fonts outside release sources.
"""
import argparse
import hashlib
import json
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parents[1]
FAMILY = "Timoa Sarasa UI SC"
MASTERS = {
    "Regular": "81fc5857cbc9b05a16a75c7ad3e848032df52dddd29192f14b48d4b449c973f7",
    "Bold": "b3e6ef6ce5df339d103fcfa9a146d8338c67eb48333308397861d97b4408aca9",
}


def coverage():
    # Complete UI/catalog coverage plus a modest editable reserve, rather
    # than thousands of unused GB2312/Big5 glyphs. Other user text falls back
    # through the platform's font matching.
    chars = set((ROOT / 'tools/font-common-characters.txt').read_text(encoding='utf-8'))
    # Latin, punctuation, currency, arrows, CJK punctuation and fullwidth.
    for start, end in [(0x20, 0x250), (0x2000, 0x2070), (0x20A0, 0x20D0),
                       (0x2100, 0x2200), (0x3000, 0x3040), (0xFF01, 0xFF61)]:
        chars.update(map(chr, range(start, end)))
    for folder, suffix in [("ui", ".slint"), ("src", ".rs"), ("i18n", ".json")]:
        for path in (ROOT / folder).rglob("*" + suffix):
            chars.update(path.read_text(encoding="utf-8"))
    # JSON escape sequences are decoded too; generated translations must
    # retain their actual glyphs even if the catalog serialization changes.
    catalog = json.loads((ROOT / "i18n/messages.json").read_text(encoding="utf-8"))
    for source, translations in catalog.items():
        chars.update(source)
        for translated in translations:
            chars.update(translated)
    return {ord(char) for char in chars if ord(char) >= 32}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--master-dir", type=Path, required=True)
    args = parser.parse_args()
    directory = ROOT / "assets/fonts"
    requested = coverage()
    manifest = json.loads((directory / "manifest.json").read_text(encoding="utf-8"))
    manifest.update(subsetted=True, family=FAMILY, subsetTool="fontTools " + __import__("fontTools").__version__,
                    coverage="All UI/catalog/application characters + modest common-character reserve + Latin/punctuation",
                    hintingPreserved=True, fonts=[])
    for style, expected_hash in MASTERS.items():
        filename = f"SarasaUiSC-{style}.ttf"
        master = args.master_dir / filename
        master_bytes = master.read_bytes()
        if hashlib.sha256(master_bytes).hexdigest() != expected_hash:
            raise ValueError(f"Full upstream master checksum differs: {master}")
        font = TTFont(master, recalcTimestamp=False)
        available = set(font.getBestCmap())
        retained = requested & available
        options = subset.Options()
        options.hinting = True
        options.layout_features = ["*"]
        options.name_IDs = ["*"]
        options.name_legacy = True
        options.name_languages = ["*"]
        options.notdef_glyph = True
        options.notdef_outline = True
        options.recommended_glyphs = True
        job = subset.Subsetter(options=options)
        job.populate(unicodes=retained)
        job.subset(font)
        # Distinct internal names avoid colliding with installed full masters.
        replacements = {1: FAMILY, 2: style, 3: f"Timoa-Sarasa-1.0.42-{style}-subset",
                        4: f"{FAMILY} {style}", 6: f"TimoaSarasaUiSC-{style}",
                        16: FAMILY, 17: style}
        for entry in font["name"].names:
            if entry.nameID in replacements:
                entry.string = replacements[entry.nameID].encode(entry.getEncoding(), errors="replace")
        mapped_count = len(font.getBestCmap())
        font.save(directory / filename)
        font.close()
        result = (directory / filename).read_bytes()
        manifest["fonts"].append(dict(file=filename, family=FAMILY, weight=400 if style == "Regular" else 700,
                                      bytes=len(result), sha256=hashlib.sha256(result).hexdigest(),
                                      masterSha256=expected_hash, masterBytes=len(master_bytes),
                                      requestedUnicodeCount=len(retained), unicodeCount=mapped_count, unmodified=False))
    manifest["totalBytes"] = sum(font["bytes"] for font in manifest["fonts"])
    # Build-time coverage check and automatic regeneration use this compact
    # text file; it is not embedded into the executable.
    (directory / 'coverage.txt').write_text(''.join(map(chr, sorted(requested))) + '\n', encoding='utf-8')
    (directory / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(manifest, ensure_ascii=True, indent=2))


if __name__ == "__main__":
    main()
