"""Run: python tools/gen_special.py

The Sahur Knight: a hero with its own body, not the shared knight template. A walking
wooden kentongan drum — a tall log with grain, googly eyes, a wide mouth — wearing a
little steel knight's cap with a red plume, swinging a thick wooden bat. Original design.

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


def hero(bob=0, legs="stand", arms="down", bat="shoulder", eyes="open", mouth="smile", lift=0):
    """One 16x16 frame. `bob` drops the body, `lift` raises the whole hero (jumps)."""
    g = Grid(16, 16)
    top = 5 + bob - lift
    # --- legs (stubby, dark wood)
    leg_y = 13 - lift
    if legs == "stand":
        g.rect(5, leg_y, 6, leg_y + 2, "r")
        g.rect(9, leg_y, 10, leg_y + 2, "r")
    elif legs == "strideA":
        g.rect(4, leg_y, 5, leg_y + 1, "r")
        g.set(3, leg_y + 2, "r")
        g.rect(10, leg_y, 11, leg_y + 1, "r")
        g.set(12, leg_y + 2, "r")
    elif legs == "strideB":
        g.rect(6, leg_y, 9, leg_y + 1, "r")
        g.rect(7, leg_y + 2, 8, leg_y + 2, "r")
    elif legs == "sit":
        g.rect(4, 14, 11, 15, "r")
    # --- the log body (kentongan): wood with a darker right side and grain
    body_bottom = (13 if legs != "sit" else 13) + bob - lift
    if legs == "sit":
        top += 2
    g.rect(4, top, 11, body_bottom, "o")
    g.rect(10, top, 11, body_bottom, "r")
    for gy in range(top + 6, body_bottom, 3):
        g.line([(5, gy), (6, gy)], "r")  # grain
    g.rect(5, body_bottom, 10, body_bottom, "y")  # cut ring at the bottom
    # --- steel cap with a red plume
    g.rect(4, top - 1, 11, top, "s")
    g.rect(5, top - 2, 10, top - 2, "S")
    g.rect(7, top - 4, 8, top - 3, "r")
    # --- googly eyes
    ey = top + 2
    if eyes == "open":
        g.rect(5, ey, 6, ey + 1, "w")
        g.rect(8, ey, 9, ey + 1, "w")
        g.set(6, ey + 1, "k")
        g.set(9, ey + 1, "k")
    elif eyes == "wide":
        g.rect(5, ey - 1, 6, ey + 1, "w")
        g.rect(8, ey - 1, 9, ey + 1, "w")
        g.set(6, ey, "k")
        g.set(9, ey, "k")
    elif eyes == "blink":
        g.line([(5, ey + 1), (6, ey + 1), (8, ey + 1), (9, ey + 1)], "k")
    elif eyes == "sleepy":
        g.line([(5, ey + 1), (6, ey + 1), (8, ey + 1), (9, ey + 1)], "k")
        g.set(6, ey, "w")
        g.set(9, ey, "w")
    # --- mouth
    my = ey + 4
    if mouth == "smile":
        g.line([(5, my - 1), (6, my), (7, my), (8, my), (9, my - 1)], "k")
    elif mouth == "open":
        g.rect(5, my - 1, 9, my + 1, "k")
        g.rect(6, my + 1, 8, my + 1, "p")
    elif mouth == "flat":
        g.line([(6, my), (7, my), (8, my)], "k")
    # --- arms (sticks) and the bat
    ay = top + 4
    if arms == "down":
        g.line([(3, ay), (3, ay + 1)], "o")
        g.line([(12, ay), (12, ay + 1)], "o")
    elif arms == "fwd":
        g.line([(3, ay), (3, ay + 1)], "o")
        g.line([(12, ay), (13, ay)], "o")
    elif arms == "up":
        g.line([(3, ay - 1), (3, ay - 2)], "o")
        g.line([(12, ay - 1), (12, ay - 2)], "o")
    elif arms == "raise":
        g.line([(3, ay), (3, ay + 1)], "o")
        g.line([(12, ay - 1), (12, ay - 3), (13, ay - 4)], "o")
    if bat == "shoulder":   # resting on the shoulder, pointing back-up
        g.line([(2, ay - 1), (1, ay - 2), (1, ay - 3), (0, ay - 4)], "y")
        g.line([(2, ay - 2), (1, ay - 4), (0, ay - 5)], "o")
    elif bat == "up":       # wound up overhead
        g.rect(12, top - 4, 13, ay - 1, "y")
        g.rect(13, top - 5, 14, top - 3, "o")
    elif bat == "swing":    # straight out in front
        g.rect(12, ay - 1, 15, ay, "y")
        g.set(15, ay - 2, "o")
    elif bat == "low":      # follow-through
        g.line([(13, ay + 1), (14, ay + 2), (15, ay + 3)], "y")
        g.line([(14, ay + 1), (15, ay + 2)], "o")
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
    "idle 250": [H(), H(bob=1), H(), H(eyes="blink")],
    "sit 900": [H(legs="sit", arms="down", bat="none", eyes="sleepy"),
                H(legs="sit", arms="down", bat="none", eyes="sleepy", bob=1)],
    "walk 100": [H(legs="strideA"), H(legs="strideB", lift=1), H(legs="strideA", bob=1), H(legs="strideB", lift=1)],
    "attack 80": [H(arms="up", bat="up", eyes="wide", mouth="open"),
                  H(arms="fwd", bat="swing", eyes="wide", mouth="open"),
                  H(arms="fwd", bat="low", mouth="smile")],
    "cast 100": [sparkle(H(arms="up", bat="none", eyes="wide", mouth="open"), [(2, 1), (13, 0)]),
                 sparkle(H(arms="up", bat="none", eyes="wide", mouth="open", bob=1), [(1, 2), (14, 1)])],
    "open 120": [H(arms="fwd", bat="none", bob=1), H(arms="fwd", bat="none", eyes="wide")],
    "hold 400": [H(arms="fwd", bat="none"), H(arms="fwd", bat="none", bob=1)],
    "raise_hand 300": [H(arms="raise", bat="none", mouth="open"), H(arms="raise", bat="none", mouth="flat", bob=1)],
    "victory 150": [H(arms="up", bat="up", eyes="wide", mouth="open"),
                    H(arms="up", bat="up", eyes="wide", mouth="open", lift=1)],
}


def face(mouth_open=False, bob=0):
    """8x8: the top of the log with cap, eyes and mouth."""
    g = Grid(8, 8)
    t = 2 + bob
    g.rect(1, t, 6, 7, "o")
    g.rect(5, t, 6, 7, "r")
    g.rect(1, t - 1, 6, t - 1, "s")
    g.rect(3, t - 2, 4, t - 2, "r")
    g.set(2, t + 1, "w")
    g.set(4, t + 1, "w")
    if mouth_open:
        g.rect(2, t + 3, 4, t + 4, "k")
    else:
        g.line([(2, t + 3), (3, t + 4), (4, t + 3)], "k")
    g.outline()
    return g.rows()


def portrait(talk):
    """32x32 close-up for the ask popup and character sheet."""
    g = Grid(32, 32)
    g.rect(5, 9, 26, 31, "o")
    g.rect(22, 9, 26, 31, "r")
    for y in (17, 22, 27):
        g.rect(7, y, 11, y, "r")             # grain
    g.rect(4, 6, 27, 9, "s")                 # cap
    g.rect(6, 4, 25, 5, "S")
    g.rect(14, 0, 17, 3, "r")                # plume
    g.rect(15, 0, 18, 1, "r")
    g.rect(8, 12, 13, 17, "w")               # eyes
    g.rect(17, 12, 22, 17, "w")
    g.rect(11, 15, 12, 16, "k")
    g.rect(18, 15, 19, 16, "k")
    if talk:
        g.rect(10, 21, 21, 26, "k")
        g.rect(12, 25, 19, 26, "p")
    else:
        g.line([(10, 22), (11, 23), (12, 24), (13, 24), (14, 24), (15, 24),
                (16, 24), (17, 24), (18, 24), (19, 23), (20, 22)], "k")
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
write("face_sahur_knight", {"idle 250": [face(), face(bob=1)]})
write("portrait_sahur_knight", {"talk 180": [portrait(False), portrait(True)], "idle 1000": [portrait(False)]})
print("ok")
