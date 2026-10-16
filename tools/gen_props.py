"""Run: python tools/gen_props.py

Prop placeholders for hero actions (SPEC §5.4): book, scroll, spyglass, bird, seal,
pigeon, coin, fireball, anvil, spellbook, chest. Each is 8x8 with auto outline, drawn
from shapes like the bosses. Also appends the `hold` pose to hero.txt (once)."""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gen_bosses import Canvas  # noqa: E402

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "placeholders")


def frame(draw, f):
    c = Canvas(8, 8)
    inner = Canvas(6, 6)
    draw(inner, f)
    for y in range(6):
        for x in range(6):
            if inner.px[y][x] != ".":
                c.set(x + 1, y + 1, inner.px[y][x])
    c.outline()
    return c.rows()


def book(c, f):
    c.rect(0, 0.2, 0.45, 1, "w", "s")
    c.rect(0.55, 0.2, 1, 1, "w", "s")
    c.rect(0.45, 0.2, 0.55, 1, "r")
    if f:
        c.tri([(0.55, 0.2), (0.9, 0.0), (0.9, 0.6)], "s")  # page turning


def scroll(c, f):
    c.rect(0.1, 0.1, 0.9, 0.9 if f else 0.6, "y", "o")
    c.rect(0, 0, 1, 0.15, "o")
    c.rect(0.25, 0.4, 0.75, 0.45, "k")


def spyglass(c, f):
    c.rect(0, 0.3, 0.7, 0.7, "y", "o")
    c.rect(0.7, 0.2, 1, 0.8, "S")
    c.dot(0.9, 0.5, "C" if f else "w")


def bird(c, f):
    c.ellipse(0.5, 0.6, 0.35, 0.25, "w", "s")
    c.dot(0.85, 0.45, "o")
    wing = [(0.2, 0.5), (0.6, 0.5), (0.4, 0.0 if f else 0.9)]
    c.tri(wing, "s")


def seal(c, f):
    c.rect(0, 0.4, 1, 1, "y", "o")
    c.ellipse(0.5, 0.65 if f else 0.3, 0.25, 0.25, "r", "p")


def pigeon(c, f):
    c.ellipse(0.45, 0.6, 0.4, 0.3, "s", "S")
    c.ellipse(0.8, 0.35, 0.18, 0.18, "s")
    c.dot(0.95, 0.35, "o")
    c.tri([(0.1, 0.5), (0.6, 0.5), (0.3, 0.0 if f else 1.0)], "S")
    c.dot(0.4, 0.6, "w")  # the letter


def coin(c, f):
    if f:
        c.rect(0.4, 0.1, 0.6, 0.9, "y", "o")
    else:
        c.ellipse(0.5, 0.5, 0.45, 0.45, "y", "o", "w")


def fireball(c, f):
    c.ellipse(0.6, 0.5, 0.35, 0.35, "o", "r", "y")
    c.tri([(0.3, 0.3), (0.3, 0.7), (0.0, 0.5 + (0.2 if f else -0.2))], "r")


def anvil(c, f):
    c.rect(0.0, 0.3, 1.0, 0.5, "S", "d")
    c.rect(0.3, 0.5, 0.7, 1.0, "S", "d")
    if f:
        c.dot(0.5, 0.0, "y")
        c.dot(0.8, 0.1, "w")


def spellbook(c, f):
    c.rect(0, 0.2, 1, 1, "p", "k")
    c.rect(0.1, 0.3, 0.9, 0.9, "b")
    c.ellipse(0.5, 0.6, 0.2, 0.2, "C" if f else "c")
    c.dot(0.5, 0.0, "C" if f else "w")


def chest(c, f):
    c.rect(0, 0.4, 1, 1, "o", "r")
    c.rect(0, 0.1 if not f else 0.0, 1, 0.4 if not f else 0.2, "o", "r")
    c.rect(0.4, 0.45, 0.6, 0.65, "y")
    if f:
        c.rect(0.15, 0.25, 0.85, 0.4, "y")  # treasure glint


PROPS = {
    "book": book, "scroll": scroll, "spyglass": spyglass, "bird": bird, "seal": seal,
    "pigeon": pigeon, "coin": coin, "fireball": fireball, "anvil": anvil,
    "spellbook": spellbook, "chest": chest,
}

for name, fn in PROPS.items():
    lines = ["# tag a 220"]
    for f in (0, 1):
        lines += frame(fn, f) + [""]
    open(os.path.join(OUT, f"prop_{name}.txt"), "w", newline="\n").write("\n".join(lines))

# Hero `hold` pose: arms forward at chest height, holding whatever prop is drawn there.
hero_path = os.path.join(OUT, "hero.txt")
hero = open(hero_path).read()
if "# tag hold" not in hero:
    stand = [line for line in hero.split("# tag idle 250\n", 1)[1].split("\n\n")[0].split("\n")]
    rows = [list(r) for r in stand]
    rows[11][4] = "M"                      # hands leave the sides...
    rows[11][11] = "M"
    for x in (12, 13):                     # ...and reach forward
        rows[10][x] = "y"
    rows[10][14] = "k"
    hold = ["".join(r) for r in rows]
    bob = ["." * 16] + hold[0:8] + hold[9:]
    hero = hero.rstrip("\n") + "\n\n# tag hold 400\n" + "\n".join(hold) + "\n\n" + "\n".join(bob) + "\n"
    open(hero_path, "w", newline="\n").write(hero)
print("ok")
