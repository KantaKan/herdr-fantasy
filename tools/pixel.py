"""Shared helpers for the sprite generators: a pixel Grid (palette keys from ART.md),
`sparkle`, and `write`, which saves animations as assets/placeholders/<name>.txt."""
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


def sparkle(rows, pts, c="C"):
    g = [list(r) for r in rows]
    for x, y in pts:
        if 0 <= y < len(g) and 0 <= x < len(g[0]) and g[y][x] == ".":
            g[y][x] = c
    return ["".join(r) for r in g]


def write(name, anims):
    lines = []
    for tag, frames in anims.items():
        lines.append(f"# tag {tag}")
        for f in frames:
            assert all(len(r) == len(f[0]) for r in f) and len(f) == len(frames[0]), name
            lines += f + [""]
    open(os.path.join(OUT, name + ".txt"), "w", newline="\n").write("\n".join(lines))
