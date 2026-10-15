"""Run: python tools/gen_bosses.py

Parametric boss placeholders: each species is drawn from shapes in normalized
coordinates, then rasterized at three tiers with auto outline. Output:
assets/placeholders/boss_<species>_<tier>.txt"""
import os

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "placeholders")
TIERS = {"little": (14, 12), "normal": (24, 20), "big": (34, 28)}


class Canvas:
    def __init__(s, w, h):
        s.w, s.h = w, h
        s.px = [["."] * w for _ in range(h)]

    def X(s, u):
        return u * (s.w - 1)

    def Y(s, v):
        return v * (s.h - 1)

    def set(s, x, y, c):
        if 0 <= x < s.w and 0 <= y < s.h:
            s.px[y][x] = c

    def ellipse(s, cx, cy, rx, ry, c, shade=None, hi=None):
        cx, cy, rx, ry = s.X(cx), s.Y(cy), max(s.X(rx), 0.6), max(s.Y(ry), 0.6)
        for y in range(s.h):
            for x in range(s.w):
                dx, dy = (x - cx) / rx, (y - cy) / ry
                d = dx * dx + dy * dy
                if d <= 1.0:
                    col = c
                    if shade and dy > 0.35:
                        col = shade
                    if hi and dx < -0.2 and dy < -0.2 and d < 0.5:
                        col = hi
                    s.set(x, y, col)

    def rect(s, x0, y0, x1, y1, c, shade=None):
        X0, Y0, X1, Y1 = round(s.X(x0)), round(s.Y(y0)), round(s.X(x1)), round(s.Y(y1))
        for y in range(Y0, Y1 + 1):
            for x in range(X0, X1 + 1):
                s.set(x, y, shade if shade and y > Y0 + (Y1 - Y0) * 0.6 else c)

    def dot(s, u, v, c, size=1):
        x, y = round(s.X(u)), round(s.Y(v))
        for dy in range(size):
            for dx in range(size):
                s.set(x + dx, y + dy, c)

    def tri(s, pts, c):
        (ax, ay), (bx, by), (cx_, cy_) = [(s.X(u), s.Y(v)) for u, v in pts]
        for y in range(s.h):
            for x in range(s.w):
                def side(px, py, qx, qy, rx, ry):
                    return (px - rx) * (qy - ry) - (qx - rx) * (py - ry)
                d1 = side(x, y, ax, ay, bx, by)
                d2 = side(x, y, bx, by, cx_, cy_)
                d3 = side(x, y, cx_, cy_, ax, ay)
                if not ((d1 < 0 or d2 < 0 or d3 < 0) and (d1 > 0 or d2 > 0 or d3 > 0)):
                    s.set(x, y, c)

    def outline(s):
        src = [r[:] for r in s.px]
        for y in range(s.h):
            for x in range(s.w):
                if src[y][x] != ".":
                    continue
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    nx, ny = x + dx, y + dy
                    if 0 <= nx < s.w and 0 <= ny < s.h and src[ny][nx] not in ".k":
                        s.px[y][x] = "k"
                        break

    def rows(s):
        return ["".join(r) for r in s.px]


def eyes(c, y, xs, col, size):
    for x in xs:
        c.dot(x, y, col, size)


# Each species: draw(canvas, size_px, mood) where mood is "idle" or "laugh".
def golem(c, z, m):
    c.rect(0.12, 0.42, 0.88, 0.78, "S", "d")           # torso
    c.ellipse(0.13, 0.42, 0.13, 0.13, "s", "S")        # shoulders
    c.ellipse(0.87, 0.42, 0.13, 0.13, "s", "S")
    c.rect(0.02, 0.48, 0.16, 0.8, "s", "S")            # arms
    c.rect(0.84, 0.48, 0.98, 0.8, "s", "S")
    c.rect(0.24, 0.8, 0.42, 0.98, "S", "d")            # legs
    c.rect(0.58, 0.8, 0.76, 0.98, "S", "d")
    c.rect(0.3, 0.1, 0.7, 0.42, "s", "S")              # head
    c.ellipse(0.5, 0.6, 0.1, 0.1, "y" if m == "idle" else "r")  # glowing core
    eyes(c, 0.24, (0.38, 0.58), "y" if m == "idle" else "r", 1 + z // 20)
    if m == "laugh":
        c.rect(0.4, 0.34, 0.6, 0.38, "k")


def ghost(c, z, m):
    c.ellipse(0.5, 0.42, 0.42, 0.4, "w", "s", "C")
    c.rect(0.08, 0.42, 0.92, 0.82, "w", "s")
    for i in range(5):                                  # wavy hem
        c.ellipse(0.16 + i * 0.17, 0.86, 0.09, 0.09, "s")
    eyes(c, 0.36, (0.32, 0.6), "k" if m == "idle" else "p", 1 + z // 14)
    if m == "laugh":
        c.ellipse(0.5, 0.6, 0.14, 0.08, "p")
    c.dot(0.14, 0.14, "C")
    c.dot(0.86, 0.22, "C")


def mimic(c, z, m):
    c.rect(0.06, 0.48, 0.94, 0.96, "o", "r")           # chest body
    c.rect(0.06, 0.48, 0.94, 0.54, "y")                # trim
    c.rect(0.44, 0.6, 0.56, 0.74, "y")                 # lock
    lid = 0.08 if m == "laugh" else 0.22
    c.rect(0.06, lid, 0.94, 0.4, "o", "r")             # lid
    c.rect(0.06, 0.4, 0.94, 0.48, "p")                 # mouth
    for i in range(6):                                  # teeth
        c.tri([(0.1 + i * 0.14, 0.4), (0.18 + i * 0.14, 0.4), (0.14 + i * 0.14, 0.47)], "w")
    if m == "laugh":
        c.ellipse(0.5, 0.46, 0.12, 0.06, "r")          # tongue
    eyes(c, lid + 0.06, (0.3, 0.66), "y", 1 + z // 16)


def wyrm(c, z, m):
    c.ellipse(0.55, 0.78, 0.42, 0.2, "p", "k")         # coil
    c.ellipse(0.45, 0.6, 0.3, 0.16, "r", "p")
    c.ellipse(0.32, 0.36, 0.14, 0.2, "r", "p")         # neck
    c.ellipse(0.3, 0.18, 0.22, 0.14, "r", "p")         # head
    c.tri([(0.18, 0.06), (0.26, 0.0), (0.28, 0.1)], "y")   # horns
    c.tri([(0.36, 0.06), (0.46, 0.0), (0.44, 0.1)], "y")
    for i in range(4):                                  # gear scales
        c.dot(0.4 + i * 0.12, 0.74, "y")
    eyes(c, 0.16, (0.2,), "y" if m == "idle" else "w", 1 + z // 16)
    if m == "laugh":
        c.tri([(0.08, 0.24), (0.3, 0.24), (0.1, 0.34)], "o")   # fire breath


def kraken(c, z, m):
    for i in range(5):
        x = 0.12 + i * 0.19
        c.rect(x - 0.05, 0.5, x + 0.05, 0.9, "p")
        c.ellipse(x + (0.05 if i % 2 else -0.05), 0.92, 0.07, 0.06, "p")
    c.ellipse(0.5, 0.36, 0.4, 0.32, "r", "p", "o")
    eyes(c, 0.34, (0.3, 0.62), "y" if m == "idle" else "w", 1 + z // 12)
    c.dot(0.33, 0.37, "k")
    c.dot(0.65, 0.37, "k")
    if m == "laugh":
        c.ellipse(0.5, 0.56, 0.12, 0.06, "k")


def hydra(c, z, m):
    c.ellipse(0.5, 0.8, 0.4, 0.2, "g", "t")
    for x, top in ((0.18, 0.2), (0.5, 0.06), (0.82, 0.2)):
        c.rect(x - 0.05, top + 0.1, x + 0.05, 0.7, "g", "t")
        c.ellipse(x, top + 0.08, 0.13, 0.1, "G", "g")
        c.dot(x - 0.04, top + 0.06, "y" if m == "idle" else "r")
        if m == "laugh":
            c.dot(x, top + 0.14, "k")


def imp(c, z, m):
    c.ellipse(0.5, 0.66, 0.26, 0.24, "o", "r")
    c.ellipse(0.5, 0.3, 0.3, 0.24, "o", "r", "y")
    c.tri([(0.22, 0.14), (0.3, 0.0), (0.36, 0.14)], "r")
    c.tri([(0.64, 0.14), (0.7, 0.0), (0.78, 0.14)], "r")
    c.rect(0.8, 0.3, 0.86, 0.8, "r")                   # red pen
    c.dot(0.83, 0.26, "w")
    eyes(c, 0.28, (0.38, 0.58), "w" if m == "idle" else "y", 1 + z // 18)
    if m == "laugh":
        c.rect(0.4, 0.4, 0.6, 0.44, "k")


def gremlin(c, z, m):
    c.tri([(0.0, 0.2), (0.3, 0.3), (0.26, 0.46)], "G")  # ears
    c.tri([(1.0, 0.2), (0.7, 0.3), (0.74, 0.46)], "G")
    c.ellipse(0.5, 0.4, 0.3, 0.28, "g", "t", "G")
    c.ellipse(0.5, 0.8, 0.24, 0.2, "g", "t")
    eyes(c, 0.34, (0.38, 0.58), "y", 1 + z // 14)
    c.rect(0.36, 0.52, 0.64, 0.56 if m == "idle" else 0.62, "w")   # grin


def slime_king(c, z, m):
    c.ellipse(0.5, 0.66, 0.48, 0.34, "g", "t", "G")
    c.rect(0.02, 0.66, 0.98, 0.98, "g", "t")
    c.rect(0.28, 0.12, 0.72, 0.28, "y")                # crown
    for x in (0.28, 0.5, 0.72):
        c.tri([(x - 0.08, 0.14), (x, 0.0), (x + 0.08, 0.14)], "y")
    c.dot(0.5, 0.18, "r")
    eyes(c, 0.56, (0.32, 0.62), "w", 1 + z // 12)
    if m == "laugh":
        c.ellipse(0.5, 0.78, 0.16, 0.06, "k")


SPECIES = {
    "test_golem": golem, "flaky_ghost": ghost, "snapshot_mimic": mimic,
    "build_wyrm": wyrm, "linker_kraken": kraken, "dependency_hydra": hydra,
    "lint_imp": imp, "typo_gremlin": gremlin, "slime_king": slime_king,
}


def frame(fn, w, h, mood, dx=0, dy=0):
    c = Canvas(w, h)
    inner = Canvas(w - 2, h - 2)                        # leave room for outline
    fn(inner, h, mood)
    for y in range(inner.h):
        for x in range(inner.w):
            if inner.px[y][x] != ".":
                c.set(x + 1 + dx, y + 1 + dy, inner.px[y][x])
    c.outline()
    return c.rows()


def crumble(rows, keep):
    h, w = len(rows), len(rows[0])
    cut = h - max(1, int(h * keep))
    out = ["." * w] * cut + rows[cut:]
    return out


os.makedirs(OUT, exist_ok=True)
for name, fn in SPECIES.items():
    for tier, (w, h) in TIERS.items():
        idle = frame(fn, w, h, "idle")
        anims = {
            "idle 400": [idle, frame(fn, w, h, "idle", dy=1)],
            "laugh 150": [frame(fn, w, h, "laugh", dx=-1), frame(fn, w, h, "laugh", dx=1)],
            "die 300": [crumble(frame(fn, w, h, "laugh", dy=2), 0.6), crumble(idle, 0.2)],
        }
        lines = []
        for tag, frames in anims.items():
            lines.append(f"# tag {tag}")
            for f in frames:
                assert all(len(r) == w for r in f) and len(f) == h
                lines += f + [""]
        open(os.path.join(OUT, f"boss_{name}_{tier}.txt"), "w", newline="\n").write("\n".join(lines))
print("ok")
