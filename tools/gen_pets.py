"""Run: python tools/gen_pets.py

Pets (TASKS M14): cat, dog, dragon, owl. Each has a baby (8x7) and a grown-up (11x9)
look, facing right, with idle (2 frames), walk (2) and sit (1).
Writes pet_<id>_<baby|grown>.txt."""
from pixel import Grid, write


def frame(w, h, draw, **pose):
    g = Grid(w, h)
    draw(g, w, h, **pose)
    g.outline()
    return g.rows()


def legs4(g, y, xs, c, step):
    """Four little legs; `step` swaps which pair is forward."""
    for k, x in enumerate(xs):
        dx = (1 if k % 2 else -1) * step
        g.set(x + dx, y, c)


# ------------------------------------------------------------------ cat
def cat(g, w, h, big=False, bob=0, step=0, sit=False, blink=False):
    b = h - 2 + (1 if sit else 0)              # body bottom row (legs below)
    top = b - (3 if big else 2) + bob
    x0, x1 = 1, w - 4
    g.rect(x0, top, x1, b, "o")                 # body
    for x in range(x0 + 1, x1, 2):
        g.set(x, top, "r")                       # stripes
    g.rect(x0, b, x1, b, "y")                    # pale belly
    hx = w - 4                                   # head
    hy = top - (3 if big else 2)
    g.rect(hx, hy, w - 1, hy + (3 if big else 2), "o")
    g.set(hx, hy - 1, "o")                       # ears
    g.set(w - 1, hy - 1, "o")
    g.set(hx, hy - 1 if not big else hy - 2, "r")
    e = "o" if blink else "k"
    g.set(hx + 1, hy + 1, e)
    g.set(w - 1, hy + 1, e)
    g.set(w - 2, hy + 2, "r")                    # nose
    # tail curling up
    g.line([(0, top), (0, top - 1), (1, top - 2)] if not sit else [(0, b), (1, b + 1)], "o")
    if not sit:
        legs4(g, b + 1, [x0, x0 + 1, x1 - 1, x1], "o", step)


# ------------------------------------------------------------------ dog
def dog(g, w, h, big=False, bob=0, step=0, sit=False, blink=False):
    b = h - 2 + (1 if sit else 0)
    top = b - (3 if big else 2) + bob
    x0, x1 = 1, w - 4
    g.rect(x0, top, x1, b, "y")
    g.rect(x0 + 1, top, x0 + 2, top + 1, "o")   # a patch
    hx, hy = w - 4, top - (3 if big else 2)
    g.rect(hx, hy, w - 1, hy + (3 if big else 2), "y")
    g.rect(hx, hy, hx, hy + 2, "o")             # floppy ear
    e = "y" if blink else "k"
    g.set(hx + 2, hy + 1, e)
    g.set(w - 1, hy + 2, "k")                    # nose
    g.set(w - 2, hy + (3 if big else 2) + 1, "r")   # tongue out
    g.line([(0, top - 1 + bob), (0, top)] if not sit else [(0, b)], "o")   # wagging tail
    if not sit:
        legs4(g, b + 1, [x0, x0 + 1, x1 - 1, x1], "y", step)


# ------------------------------------------------------------------ dragon
def dragon(g, w, h, big=False, bob=0, step=0, sit=False, blink=False):
    b = h - 2 + (1 if sit else 0)
    top = b - (3 if big else 2) + bob
    x0, x1 = 2, w - 4
    g.rect(x0, top, x1, b, "g")
    g.rect(x0, b, x1, b, "y")                    # belly scales
    g.rect(x0 + 1, top - 2 + bob, x0 + 3, top - 1, "t")   # wing
    g.set(x0 + 2, top - 3 + bob + step, "t")    # wing flaps when walking
    hx, hy = w - 4, top - (3 if big else 2)
    g.rect(hx, hy, w - 1, hy + (3 if big else 2), "g")
    g.set(hx, hy - 1, "w")                       # horns
    if big:
        g.set(hx + 2, hy - 1, "w")
    e = "g" if blink else "y"
    g.set(hx + 2, hy + 1, e)
    g.set(w - 1, hy + 2, "G")
    g.line([(1, b), (0, b - 1)], "g")            # tail
    if big:
        g.set(0, b - 2, "r")                     # tail tip
    if not sit:
        legs4(g, b + 1, [x0, x0 + 1, x1 - 1, x1], "t", step)


# ------------------------------------------------------------------ owl
def owl(g, w, h, big=False, bob=0, step=0, sit=False, blink=False):
    b = h - 1
    top = (1 if big else 1) + bob + (1 if sit else 0)
    x0, x1 = (2, w - 2) if big else (1, w - 2)
    g.rect(x0, top, x1, b - 1, "S")              # body
    g.rect(x0 + 1, top + (4 if big else 3), x1 - 1, b - 1, "s")   # chest
    g.set(x0, top - 1, "S")                      # ear tufts
    g.set(x1, top - 1, "S")
    ey = top + 1
    for x in (x0 + 1, x1 - 1):                   # big eyes
        g.set(x, ey, "y")
        g.set(x, ey + (1 if big else 0), "k" if not blink else "y")
    g.set((x0 + x1) // 2, ey + 1, "o")           # beak
    wing = "d"
    g.rect(x0 - 1, top + 2, x0 - 1, b - 2 - (step if not sit else 0), wing)
    g.rect(x1 + 1, top + 2, x1 + 1, b - 2 - (step if not sit else 0), wing)
    g.set(x0 + 1, b, "o")                        # feet
    g.set(x1 - 1, b, "o")


def anims(draw, w, h, big):
    F = lambda **k: frame(w, h, draw, big=big, **k)
    return {
        "idle 400": [F(), F(bob=1, blink=True)],
        "walk 150": [F(step=0), F(step=1, bob=1)],
        "sit 900": [F(sit=True, blink=True)],
    }


PETS = {"cat": cat, "dog": dog, "dragon": dragon, "owl": owl}

if __name__ == "__main__":
    for name, draw in PETS.items():
        write(f"pet_{name}_baby", anims(draw, 8, 7, False))
        write(f"pet_{name}_grown", anims(draw, 11, 9, True))
    print("ok")
