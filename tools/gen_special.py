"""Run: python tools/gen_special.py

The Sahur Knight: a hero with its own body, not the shared knight template. A proper
knight whose helmet is a wooden kentongan drum (the slit of the drum is the visor, with
glowing eyes behind it), steel armor, a red cape, and a big wooden bat. Original design.

Writes hero_/face_/portrait_sahur_knight.txt with every animation tag the game uses."""
import os

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "placeholders")


class Grid:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.px = [["."] * w for _ in range(h)]

    def set(self, x, y, c):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y][x] = c

    def rect(self, x0, y0, x1, y1, c):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.set(x, y, c)

    def line(self, pts, c):
        for x, y in pts:
            self.set(x, y, c)

    def outline(self):
        src = [r[:] for r in self.px]
        for y in range(self.h):
            for x in range(self.w):
                if src[y][x] != ".":
                    continue
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    nx, ny = x + dx, y + dy
                    if 0 <= nx < self.w and 0 <= ny < self.h and src[ny][nx] not in ".k":
                        self.px[y][x] = "k"
                        break

    def rows(self):
        return ["".join(r) for r in self.px]


def hero(bob=0, legs="stand", arms="hold", bat="upright", eyes="open", lift=0, cape=0):
    """One 16x16 frame of a proper knight: the HELMET is a wooden kentongan drum with a
    visor slit and glowing eyes; steel armor, red cape, a big wooden bat.
    `bob` drops the upper body 1 px, `lift` raises everything (jumps), `cape` flutters."""
    g = Grid(16, 16)
    dy = bob - lift
    # --- red cape behind (left side), flutters while running
    g.rect(3, 8 + dy, 4, 12 + dy - cape, "r")
    g.set(2, 12 + dy - cape, "p")
    g.set(3, 13 + dy - cape, "p")
    # --- legs: steel greaves
    ly = 12 - lift
    if legs == "stand":
        g.rect(5, ly, 6, ly + 3, "S")
        g.rect(9, ly, 10, ly + 3, "S")
    elif legs == "strideA":
        g.rect(4, ly, 5, ly + 2, "S")
        g.set(3, ly + 3, "S")
        g.rect(10, ly, 11, ly + 2, "S")
        g.set(12, ly + 3, "S")
    elif legs == "strideB":
        g.rect(6, ly, 7, ly + 3, "S")
        g.rect(8, ly, 9, ly + 2, "S")
    elif legs == "sit":
        g.rect(5, 14, 12, 15, "S")
    sit = 2 if legs == "sit" else 0
    # --- steel torso, pauldrons, wooden belt, a little gold emblem
    ty = 8 + dy + sit
    g.rect(5, ty, 10, ty + 3, "s")
    g.rect(9, ty, 10, ty + 3, "S")
    g.rect(4, ty, 4, ty + 1, "S")
    g.rect(11, ty, 11, ty + 1, "S")
    g.rect(5, ty + 3, 10, ty + 3, "o")
    g.set(7, ty + 1, "y")
    # --- the kentongan drum helmet: wooden cylinder, rim, visor slit, glowing eyes, plume
    hy = 2 + dy + sit
    g.rect(4, hy, 11, hy + 5, "o")
    g.rect(10, hy, 11, hy + 5, "r")
    g.rect(4, hy + 5, 11, hy + 5, "y")       # rim
    g.rect(5, hy + 3, 10, hy + 3, "k")       # the drum slit = visor
    if eyes == "open":
        g.set(7, hy + 3, "y")
        g.set(9, hy + 3, "y")
    elif eyes == "angry":
        g.set(7, hy + 3, "r")
        g.set(9, hy + 3, "r")
    g.set(6, hy + 1, "r")                    # wood grain
    g.set(8, hy + 2, "r")
    g.rect(7, hy - 2, 8, hy - 1, "r")        # plume
    g.set(6, hy - 2, "r")
    # --- front arm and the bat
    ay = ty + 1
    if arms == "hold":
        g.rect(11, ay, 12, ay, "S")
    elif arms == "fwd":
        g.rect(11, ay, 13, ay, "S")
    elif arms == "up":
        g.rect(11, ay - 2, 12, ay - 1, "S")
    elif arms == "raise":
        g.rect(12, ay - 4, 12, ay - 1, "S")
    if bat == "upright":   # held like a sword at rest
        g.rect(13, ay - 6, 14, ay - 3, "y")
        g.rect(13, ay - 2, 13, ay + 1, "o")
    elif bat == "windup":  # raised back over the helmet
        g.rect(4, max(0, hy - 4), 12, max(0, hy - 3), "y")
        g.rect(12, max(0, hy - 3), 13, hy - 1, "o")
    elif bat == "swing":   # straight out, full reach
        g.rect(13, ay - 1, 15, ay, "y")
        g.rect(12, ay, 12, ay, "o")
    elif bat == "low":     # follow-through
        g.rect(13, ay + 1, 14, ay + 2, "o")
        g.rect(14, ay + 3, 15, ay + 4, "y")
    elif bat == "high":    # victory: raised to the sky
        g.rect(12, max(0, hy - 5), 13, ay - 3, "y")
        g.rect(12, ay - 2, 12, ay - 1, "o")
    elif bat == "ground":  # resting beside a sitting knight
        g.rect(12, 13, 15, 13, "y")
    g.outline()
    return g.rows()


def sparkle(rows, pts, c="C"):
    g = [list(r) for r in rows]
    for x, y in pts:
        if 0 <= y < len(g) and 0 <= x < len(g[0]) and g[y][x] == ".":
            g[y][x] = c
    return ["".join(r) for r in g]


H = hero
ANIMS = {
    "idle 250": [H(), H(bob=1), H(), H(eyes="off")],
    "sit 900": [H(legs="sit", arms="hold", bat="ground", eyes="off"),
                H(legs="sit", arms="hold", bat="ground", eyes="off", bob=1)],
    "walk 100": [H(legs="strideA", cape=1), H(legs="strideB", lift=1),
                 H(legs="strideA", bob=1, cape=1), H(legs="strideB", lift=1)],
    "attack 80": [H(arms="up", bat="windup", eyes="angry"),
                  H(arms="fwd", bat="swing", eyes="angry"),
                  H(arms="fwd", bat="low")],
    "cast 100": [sparkle(H(arms="up", bat="high", eyes="angry"), [(2, 1), (15, 2)]),
                 sparkle(H(arms="up", bat="high", eyes="angry", bob=1), [(1, 3), (15, 0)])],
    "open 120": [H(arms="fwd", bat="none", bob=1), H(arms="fwd", bat="none")],
    "hold 400": [H(arms="fwd", bat="none"), H(arms="fwd", bat="none", bob=1)],
    "raise_hand 300": [H(arms="raise", bat="none"), H(arms="raise", bat="none", bob=1)],
    "victory 150": [H(arms="up", bat="high"), H(arms="up", bat="high", lift=1, cape=1)],
}


def face(blink=False, bob=0):
    """8x8 for compact mode: the drum helmet with its slit and glowing eyes."""
    g = Grid(8, 8)
    t = 2 + bob
    g.rect(1, t, 6, 6, "o")
    g.rect(5, t, 6, 6, "r")
    g.rect(1, 6, 6, 6, "y")
    g.rect(2, t + 2, 5, t + 2, "k")
    if not blink:
        g.set(3, t + 2, "y")
        g.set(4, t + 2, "y")
    g.rect(3, t - 2, 4, t - 1, "r")
    g.outline()
    return g.rows()


def portrait(talk):
    """32x32 close-up: the drum helmet head-on, glowing eyes, steel collar, plume."""
    g = Grid(32, 32)
    g.rect(3, 26, 28, 31, "s")               # steel shoulders
    g.rect(22, 26, 28, 31, "S")
    g.rect(6, 6, 25, 25, "o")                # drum helmet
    g.rect(21, 6, 25, 25, "r")
    for (x, y) in ((9, 9), (13, 11), (18, 20), (10, 21)):
        g.rect(x, y, x + 2, y, "r")          # grain
    g.rect(6, 23, 25, 25, "y")               # rim
    g.rect(8, 14, 23, 17, "k")               # the slit / visor
    g.rect(11, 15, 13, 16, "y")              # glowing eyes
    g.rect(18, 15, 20, 16, "y")
    if talk:
        g.rect(13, 19, 18, 21, "k")          # the drum speaks through a lower slit
    g.rect(14, 0, 17, 5, "r")                # plume
    g.rect(12, 1, 13, 3, "r")
    g.outline()
    return g.rows()


def write(name, anims):
    lines = []
    for tag, frames in anims.items():
        lines.append(f"# tag {tag}")
        for f in frames:
            assert all(len(r) == len(f[0]) for r in f) and len(f) == len(frames[0]), name
            lines += f + [""]
    open(os.path.join(OUT, name + ".txt"), "w", newline="\n").write("\n".join(lines))


write("hero_sahur_knight", ANIMS)
write("face_sahur_knight", {"idle 250": [face(), face(bob=1), face(), face(blink=True)]})
write("portrait_sahur_knight", {"talk 180": [portrait(False), portrait(True)], "idle 1000": [portrait(False)]})
print("ok")
