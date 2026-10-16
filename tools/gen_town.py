"""Run: python tools/gen_town.py

Town placeholders (ART.md §7): one building per theme in three tiers (16/20/24 px) and
the six commit-count decorations, drawn from shapes like the bosses."""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gen_bosses import Canvas  # noqa: E402  (importing regenerates bosses; harmless)

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "placeholders")


def framed(draw, w, h, f):
    c = Canvas(w, h)
    inner = Canvas(w - 2, h - 2)
    draw(inner, f)
    for y in range(inner.h):
        for x in range(inner.w):
            if inner.px[y][x] != ".":
                c.set(x + 1, y + 1, inner.px[y][x])
    c.outline()
    return c.rows()


def house(wall, wall_dark, roof, roof_dark, door, extra=None):
    def draw(c, f):
        c.rect(0.08, 0.45, 0.92, 1.0, wall, wall_dark)              # walls
        c.tri([(0.0, 0.48), (1.0, 0.48), (0.5, 0.05)], roof)       # roof
        c.rect(0.0, 0.44, 1.0, 0.5, roof_dark)
        c.rect(0.42, 0.72, 0.58, 1.0, door)                        # door
        c.rect(0.18, 0.6, 0.32, 0.72, "y" if f else "o")           # window glow flickers
        c.rect(0.68, 0.6, 0.82, 0.72, "o" if f else "y")
        if extra:
            extra(c, f)
    return draw


def forge_fire(c, f):
    c.rect(0.72, 0.08, 0.84, 0.4, "S", "d")                      # chimney
    c.dot(0.78, 0.02 if f else 0.06, "o")                         # sparks


def lantern(c, f):
    c.rect(0.06, 0.5, 0.1, 0.62, "k")
    c.dot(0.08, 0.64, "y" if f else "o")


def harbor_pier(c, f):
    c.rect(0.0, 0.95, 1.0, 1.0, "o")


def tree_house(c, f):
    pass


THEMES = {
    "forge": house("S", "d", "d", "k", "o", forge_fire),
    "lantern": house("o", "r", "r", "p", "p", lantern),
    "harbor": house("w", "s", "b", "n", "t", harbor_pier),
    "meadow": house("w", "s", "y", "o", "o"),
}


def serpent(c, f):
    c.rect(0.42, 0.55, 0.58, 1.0, "o", "r")                       # trunk
    c.ellipse(0.5, 0.35, 0.48, 0.32, "g", "t", "G")                # canopy
    c.rect(0.3, 0.3, 0.7, 0.55, "o", "r")                          # house in the tree
    c.rect(0.44, 0.38, 0.56, 0.48, "y" if f else "o")


THEMES["serpent"] = serpent

DECOS = {
    "tree": (10, 14, lambda c, f: (c.rect(0.4, 0.6, 0.6, 1.0, "o", "r"), c.ellipse(0.5, 0.35, 0.48, 0.35, "g", "t", "G"))),
    "well": (12, 10, lambda c, f: (c.rect(0.1, 0.5, 0.9, 1.0, "S", "d"), c.rect(0.1, 0.0, 0.2, 0.5, "o"), c.rect(0.8, 0.0, 0.9, 0.5, "o"), c.rect(0.0, 0.0, 1.0, 0.12, "r"), c.rect(0.3, 0.55, 0.7, 0.65, "b" if f else "c"))),
    "lanterns": (8, 12, lambda c, f: (c.rect(0.4, 0.2, 0.6, 1.0, "k"), c.rect(0.15, 0.0, 0.85, 0.3, "y" if f else "o"))),
    "windmill": (14, 18, lambda c, f: (c.rect(0.3, 0.4, 0.7, 1.0, "w", "s"), c.tri([(0.25, 0.42), (0.75, 0.42), (0.5, 0.25)], "r"),
                                       c.rect(0.45, 0.0, 0.55, 0.5, "S") if f else c.rect(0.05, 0.22, 0.95, 0.3, "S"))),
    "statue": (10, 16, lambda c, f: (c.rect(0.1, 0.8, 0.9, 1.0, "S", "d"), c.rect(0.3, 0.35, 0.7, 0.8, "s", "S"), c.ellipse(0.5, 0.2, 0.2, 0.18, "s", "S"), c.dot(0.75, 0.1 if f else 0.15, "y"))),
    "castle": (24, 20, lambda c, f: (c.rect(0.0, 0.35, 1.0, 1.0, "S", "d"), c.rect(0.0, 0.15, 0.2, 0.35, "S"), c.rect(0.8, 0.15, 1.0, 0.35, "S"),
                                     c.rect(0.4, 0.05, 0.6, 0.35, "s"), c.rect(0.45, 0.7, 0.55, 1.0, "k"), c.rect(0.5, 0.0, 0.52, 0.05, "k"), c.rect(0.52, 0.0, 0.62 if f else 0.58, 0.04, "r"))),
}

for theme, draw in THEMES.items():
    for tier, size in enumerate((16, 20, 24)):
        lines = ["# tag a 600"]
        for f in (0, 1):
            lines += framed(draw, size, size, f) + [""]
        open(os.path.join(OUT, f"town_{theme}_{tier}.txt"), "w", newline="\n").write("\n".join(lines))
for name, (w, h, draw) in DECOS.items():
    lines = ["# tag a 700"]
    for f in (0, 1):
        lines += framed(draw, w, h, f) + [""]
    open(os.path.join(OUT, f"deco_{name}.txt"), "w", newline="\n").write("\n".join(lines))
print("ok")
