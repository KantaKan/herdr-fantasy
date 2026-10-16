"""Run: python tools/gen_jokes.py

Joke Class placeholders for Summons (ART.md §4b): 12x12 chibis, `idle` 2 frames.
Each is a shared little body plus a costume drawn from shapes. Original archetypes only."""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gen_bosses import Canvas  # noqa: E402  (importing regenerates bosses; harmless)

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "placeholders")


def body(c, shirt, shirt_dark, skin="y"):
    c.rect(0.25, 0.55, 0.75, 0.95, shirt, shirt_dark)
    c.ellipse(0.5, 0.32, 0.34, 0.3, skin, "o")
    c.dot(0.38, 0.32, "k")
    c.dot(0.62, 0.32, "k")


def tenx(c, f):
    body(c, "n", "k")
    c.rect(0.0, 0.15, 0.15, 0.5, "d")                  # stacked headphones
    c.rect(0.85, 0.15, 1.0, 0.5, "d")
    c.rect(0.1, 0.0, 0.9, 0.08, "c")
    c.rect(0.0, 0.55 if f else 0.6, 0.2, 0.7, "S")     # keyboards in both hands
    c.rect(0.8, 0.6 if f else 0.55, 1.0, 0.75, "S")


def jester(c, f):
    body(c, "p", "y")
    c.tri([(0.1, 0.12), (0.5, 0.12), (0.0, 0.0)], "r")
    c.tri([(0.5, 0.12), (0.9, 0.12), (1.0, 0.0)], "y")
    c.dot(0.0, 0.0 if f else 0.08, "y")                 # bells
    c.dot(1.0, 0.08 if f else 0.0, "r")


def night_owl(c, f):
    c.tri([(0.0, 1.0), (1.0, 1.0), (0.5, 0.25)], "n")   # cape
    body(c, "k", "n", skin="k")
    c.tri([(0.2, 0.15), (0.32, 0.0), (0.4, 0.12)], "k")  # ears
    c.tri([(0.6, 0.12), (0.68, 0.0), (0.8, 0.15)], "k")
    c.dot(0.38, 0.32, "y")
    c.dot(0.62, 0.32, "y" if f else "o")


def intern(c, f):
    body(c, "w", "s")
    c.rect(0.2, 0.0, 0.8, 0.12, "o")                     # cardboard crown
    c.tri([(0.2, 0.05), (0.3, 0.05), (0.25, -0.1)], "o")
    c.rect(0.48, 0.55, 0.52, 0.8, "r")                   # lanyard
    c.rect(0.8, 0.5, 1.0, 0.6 if f else 0.65, "o")       # coffee tray


def pilgrim(c, f):
    body(c, "S", "d")
    c.rect(0.78, 0.35, 1.0, 1.0 if f else 0.85, "w", "s")  # scroll of copied answers


def vibe(c, f):
    body(c, "C", "c")
    c.rect(0.25, 0.28, 0.75, 0.36, "k")                  # sunglasses
    c.rect(0.0, 0.2, 0.12, 0.45, "r")                    # headphones
    c.rect(0.88, 0.2, 1.0, 0.45, "r")
    c.dot(0.9 if f else 0.95, 0.6, "y")                  # vague gesture


def senior(c, f):
    body(c, "t", "k")
    c.ellipse(0.5, 0.48, 0.28, 0.14, "s")                # grey beard
    c.rect(0.82, 0.6, 1.0, 0.78, "w")                    # mug
    c.dot(0.9, 0.5 if f else 0.45, "s")                  # steam


def yaml(c, f):
    body(c, "b", "n")
    c.tri([(0.2, 0.12), (0.8, 0.12), (0.55, -0.05 if f else 0.0)], "b")  # pointy hat
    for k, y in enumerate((0.62, 0.74, 0.86)):           # indentation stripes
        c.rect(0.3 + 0.08 * k, y, 0.72, y + 0.03, "c")


def pm(c, f):
    body(c, "d", "k")
    c.rect(0.78, 0.25 if f else 0.3, 0.84, 0.7, "y")     # roadmap pointer
    c.rect(0.4, 0.55, 0.6, 0.6, "w")                     # collar


def paladin(c, f):
    body(c, "s", "S")
    c.rect(0.3, 0.3, 0.7, 0.34, "k")                     # droopy eyes
    c.dot(0.35, 0.7, "r" if f else "k")                  # pagers blinking
    c.dot(0.65, 0.8, "k" if f else "r")


def cultist(c, f):
    c.ellipse(0.5, 0.3, 0.42, 0.34, "y", "o")            # hood
    body(c, "y", "o")
    c.dot(0.5, 0.7, "o" if f else "r")                   # duck emblem


def twins(c, f):
    body(c, "G", "g")
    c.rect(0.5, 0.55, 0.75, 0.95, "r", "p")              # the other half disagrees
    c.rect(0.5, 0.05, 0.84, 0.6, "o")
    c.dot(0.62, 0.32, "k")
    c.dot(0.2 if f else 0.8, 0.0, "w")                   # <<<<<<< / >>>>>>>


JOKES = {
    "tenx": tenx, "jester": jester, "night_owl": night_owl, "intern": intern,
    "pilgrim": pilgrim, "vibe": vibe, "senior": senior, "yaml": yaml, "pm": pm,
    "paladin": paladin, "cultist": cultist, "twins": twins,
}


def framed(draw, f):
    c = Canvas(12, 12)
    inner = Canvas(10, 10)
    draw(inner, f)
    for y in range(10):
        for x in range(10):
            if inner.px[y][x] != ".":
                c.set(x + 1, y + 1, inner.px[y][x])
    c.outline()
    return c.rows()


for name, draw in JOKES.items():
    lines = ["# tag idle 300"]
    for f in (0, 1):
        lines += framed(draw, f) + [""]
    open(os.path.join(OUT, f"joke_{name}.txt"), "w", newline="\n").write("\n".join(lines))
print("ok")
