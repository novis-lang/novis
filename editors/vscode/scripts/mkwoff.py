"""Build `media/novis-icons.woff` from the monogram, so the font is not an orphan binary.

    python -m pip install fonttools
    python scripts/mkwoff.py ../../website/media/novis-logo-black-monogram.svg \
        media/novis-icons.woff --proof /tmp/proof.svg

Nothing runs this automatically: the font changes when the mark does, which is a design
decision and not a build step. `fonttools` is its one dependency and is deliberately not in
`package.json` -- it builds an asset rather than the extension.

VS Code renders a contributed `$(name)` with the same CSS it uses for its own codicons, so
the font has to agree with codicon.ttf: 1000 units per em, ascent 1000, descent 0, and the
artwork filling that box above the baseline. The design grid is the SVG's own viewBox,
mapped onto it and flipped, because SVG counts y downward and a font counts it up. What the
glyph fills is measured from the artwork rather than the viewBox, so whatever padding the
drawing was exported with does not decide how large the mark sits beside the text.

Outlines only. A stroked path encloses no area, so it would compile to an empty glyph --
the checks below refuse one rather than shipping a blank chip. `--proof` writes the compiled
glyph back out of the finished WOFF as an SVG, which is the only view that shows what the
font actually holds rather than what went into it.
"""

import argparse
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.misc.transform import Transform
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.t2CharStringPen import T2CharStringPen
from fontTools.pens.transformPen import TransformPen
from fontTools.svgLib import SVGPath

SVG_NS = "{http://www.w3.org/2000/svg}"
UPM = 1000
NUM = r"[-+]?[0-9]*\.?[0-9]+(?:[eE][-+]?[0-9]+)?"


def prop(el, name, default):
    """A presentation attribute, whether it was written as an attribute or in `style`."""
    style = el.get("style", "")
    hit = re.search(rf"(?:^|;)\s*{name}\s*:\s*([^;]+)", style)
    return hit.group(1).strip() if hit else el.get(name, default)


def parse_transform(text):
    """The subset Affinity, Illustrator and Inkscape actually emit."""
    out = Transform()
    for name, args in re.findall(rf"(\w+)\s*\(([^)]*)\)", text or ""):
        v = [float(n) for n in re.findall(NUM, args)]
        if name == "matrix":
            out = out.transform(v)
        elif name == "translate":
            out = out.translate(v[0], v[1] if len(v) > 1 else 0)
        elif name == "scale":
            out = out.scale(v[0], v[1] if len(v) > 1 else v[0])
        elif name == "rotate" and len(v) == 3:
            out = out.translate(v[1], v[2]).rotate(v[0] * 3.141592653589793 / 180).translate(-v[1], -v[2])
        elif name == "rotate":
            out = out.rotate(v[0] * 3.141592653589793 / 180)
        else:
            sys.exit(f"unsupported transform {name}() -- flatten it in the drawing app")
    return out


def viewbox(root):
    vb = root.get("viewBox")
    if not vb:
        sys.exit("the SVG needs a viewBox; without one there is no design grid to map from")
    x, y, w, h = (float(n) for n in re.split(r"[,\s]+", vb.strip()))
    if x or y:
        sys.exit(f"viewBox origin must be 0 0, got {x} {y}")
    return w, h


def collect(el, parent, out):
    """Every <path>, paired with the transform its ancestors put it under."""
    here = parent.transform(parse_transform(el.get("transform")))
    tag = el.tag.split("}")[-1]
    if tag == "path":
        d = el.get("d")
        if d:
            if prop(el, "stroke", "none") != "none" and prop(el, "fill", "black") in ("none", ""):
                sys.exit(
                    "a path here is stroked, not filled, so it encloses no area and would "
                    "compile to a blank glyph. Expand the stroke to outlines first "
                    "(in Affinity: Layer > Expand Stroke)"
                )
            out.append((d, here))
    elif tag in ("rect", "circle", "ellipse", "polygon", "polyline", "line"):
        sys.exit(
            f"<{tag}> is not read; convert every shape to a path first "
            "(in Affinity: Layer > Convert to Curves)"
        )
    for child in el:
        collect(child, here, out)
    return out


def draw_all(items, make_pen):
    for d, local in items:
        svg = f'<svg xmlns="http://www.w3.org/2000/svg"><path d="{d}"/></svg>'
        SVGPath.fromstring(svg).draw(make_pen(local))


def build(svg_path, out_path, codepoint, family, glyph_name, fit):
    root = ET.parse(svg_path).getroot()
    w, h = viewbox(root)
    items = collect(root, Transform(), [])
    if not items:
        sys.exit("no <path> elements found in the SVG")

    # The artwork's own extent decides the size, not the viewBox: whatever padding the
    # drawing was saved with, the glyph fills the same fraction of the em as a codicon
    # does, so it sits at the weight of the text beside it.
    flip = Transform().translate(0, UPM).scale(UPM / max(w, h), -UPM / max(w, h))
    measure = BoundsPen(None)
    draw_all(items, lambda local: TransformPen(measure, flip.transform(local)))
    if measure.bounds is None:
        sys.exit("the paths enclose no area -- an outline-only or empty drawing")
    x0, y0, x1, y1 = measure.bounds

    k = UPM * fit / max(x1 - x0, y1 - y0)
    place = (
        Transform()
        .translate((UPM - (x1 - x0) * k) / 2, (UPM - (y1 - y0) * k) / 2)
        .scale(k)
        .translate(-x0, -y0)
    )

    charstring_pen = T2CharStringPen(UPM, None)
    draw_all(items, lambda local: TransformPen(charstring_pen, place.transform(flip).transform(local)))
    charstring = charstring_pen.getCharString()

    fb = FontBuilder(UPM, isTTF=False)
    fb.setupGlyphOrder([".notdef", glyph_name])
    fb.setupCharacterMap({codepoint: glyph_name})
    fb.setupCFF(
        family,
        {"FullName": family, "FamilyName": family, "Weight": "Regular"},
        {".notdef": T2CharStringPen(UPM, None).getCharString(), glyph_name: charstring},
        {},
    )
    fb.setupHorizontalMetrics({".notdef": (UPM, 0), glyph_name: (UPM, 0)})
    fb.setupHorizontalHeader(ascent=UPM, descent=0, lineGap=0)
    fb.setupNameTable(
        {
            "familyName": family,
            "styleName": "Regular",
            "uniqueFontIdentifier": f"{family} Regular",
            "fullName": family,
            "psName": family.replace(" ", ""),
            "version": "Version 1.0",
        }
    )
    fb.setupOS2(sTypoAscender=UPM, sTypoDescender=0, usWinAscent=UPM, usWinDescent=0)
    fb.setupPost()
    # `head` records when the font was built, which would make every rebuild a different file and
    # the committed binary impossible to check against its source. Pinned to the epoch instead, so
    # rebuilding from an unchanged SVG produces the same bytes.
    fb.font["head"].created = fb.font["head"].modified = 0
    fb.font.flavor = "woff"
    fb.save(out_path)

    glyphs = fb.font.getGlyphSet()
    bp = BoundsPen(glyphs)
    glyphs[glyph_name].draw(bp)
    return bp.bounds


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("svg")
    ap.add_argument("out")
    ap.add_argument("--codepoint", default="0xE001")
    ap.add_argument("--family", default="novis-icons")
    ap.add_argument("--glyph", default="novis-mark")
    ap.add_argument("--fit", type=float, default=0.92,
                    help="fraction of the em the artwork fills (codicons sit near 0.9)")
    ap.add_argument("--proof", help="write the compiled glyph back out as an SVG to eyeball")
    a = ap.parse_args()
    cp = int(a.codepoint, 16)
    bounds = build(a.svg, a.out, cp, a.family, a.glyph, a.fit)
    print(f"wrote {a.out}  glyph={a.glyph}  U+{cp:04X}  bounds={bounds}")
    print(f"      {Path(a.out).stat().st_size} bytes")

    if a.proof:
        # Read the outline back out of the compiled WOFF, so what gets rasterised is what
        # the font actually contains rather than what went into it.
        from fontTools.pens.svgPathPen import SVGPathPen
        from fontTools.ttLib import TTFont

        font = TTFont(a.out)
        assert font.flavor == "woff", font.flavor
        assert font.getBestCmap()[cp] == a.glyph, font.getBestCmap()
        glyphs = font.getGlyphSet()
        svg_pen = SVGPathPen(glyphs)
        glyphs[a.glyph].draw(TransformPen(svg_pen, Transform().translate(0, UPM).scale(1, -1)))
        Path(a.proof).write_text(
            f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {UPM} {UPM}">'
            f'<path fill="#B20038" d="{svg_pen.getCommands()}"/></svg>\n',
            encoding="utf-8",
        )
        print(f"      proof -> {a.proof}")
