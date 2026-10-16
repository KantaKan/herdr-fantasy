"""Run: python tools/gen_pets.py

Pets (TASKS M14): cat, dog, dragon, owl, facing right. A baby (13x8) and a grown-up
(19x10, with room in front for the trick), animated so they read as alive:
  idle  4 frames  breathing, blinking, tail/ear moves
  walk  4 frames  a real leg cycle with a bounce
  sit   2 frames  lying down, breathing
  trick 4 frames  dragon puffs fire, cat stretches, dog wags and barks, owl flaps
Writes pet_<id>_<baby|grown>.txt."""
from pixel import Grid, write

# Leg x-offsets for the 4 walk phases: (front pair, back pair).
STRIDE = [(1, -1), (0, 0), (-1, 1), (0, 0)]


def leg(g, x, y0, y1, c, foot):
    g.rect(x, y0, x, y1 - 1, c)
    g.set(x, y1, foot)


# ------------------------------------------------------------------ dragon
def dragon(g, big, bob=0, phase=None, blink=False, wing=0, tail=0, fire=0, sit=False):
    """Side view: round body, raised bat wing, horned head with a snout, spade tail."""
    s = 1 if big else 0
    gy = 9 if big else 7                         # feet row
    by = gy - (3 + s) + bob + (1 if sit else 0)  # body top
    bx0, bx1 = 3, 7 + s                          # body x
    g.rect(bx0, by, bx1, gy - 2 + (1 if sit else 0), "g")
    g.rect(bx0 + 1, gy - 2 + (1 if sit else 0), bx1 - 1, gy - 2 + (1 if sit else 0), "y")  # belly
    g.set(bx0, by, "G")                          # lit back
    # tail: from the hip back and up to a red spade; `tail` lifts the tip
    ty = by + 2
    g.line([(2, ty), (1, ty - tail), (0, ty - 1 - tail)], "g")
    g.set(0, ty - 2 - tail, "r")
    g.set(1, ty - 2 - tail, "r")
    # neck and head
    hx = bx1 + 1
    hy = by - (3 if big else 2)
    g.rect(hx - 1, hy + 2, hx, by + 1, "g")      # neck
    g.rect(hx, hy, hx + 2 + s, hy + 2, "g")      # head
    g.rect(hx + 2 + s, hy + 1, hx + 3 + s, hy + 2, "G")   # snout
    g.set(hx + 1, hy - 1, "w")                   # horns
    if big:
        g.set(hx, hy - 1, "w")
    g.set(hx + 1, hy + 1, "g" if blink else "y")    # eye
    if fire:
        # open mouth, then a flame cone that grows: yellow core, orange, red edge
        mx, my = hx + 4 + s, hy + 2
        g.set(mx - 1, my, "r")
        for k in range([0, 2, 5, 4][fire] + s):
            half = k // 2
            for d in range(-half, half + 1):
                c = "y" if abs(d) < max(1, half - 1) else ("o" if abs(d) < half else "r")
                g.set(mx + k, my + d, c)
    # bat wing over the back: `wing` 0 = up, 1 = half, 2 = folded
    wy = by - (3 - wing) - (1 if big else 0)
    g.line([(bx0 + 1, by), (bx0 + 2, wy), (bx0 + 3 + s, wy - 1 + wing // 2)], "G")   # wing bone
    for x in range(bx0 + 1, bx0 + 4 + s):
        for y in range(wy + 1, by):
            if (x - bx0) + (y - wy) >= 2:
                g.set(x, y, "t")                 # membrane
    # legs
    if sit:
        g.rect(bx0, gy, bx1 + 1, gy, "t")
        return
    fp, bp = STRIDE[phase] if phase is not None else (0, 0)
    foot = "t"
    leg(g, bx0 + 1 + bp, gy - 1, gy, "g", foot)
    leg(g, bx1 - 1 + fp, gy - 1, gy, "g", foot)


# ------------------------------------------------------------------ cat
def cat(g, big, bob=0, phase=None, blink=False, wing=0, tail=0, fire=0, sit=False):
    """Orange tabby: pointed ears, stripes, a tail that curls up and swishes."""
    s = 1 if big else 0
    gy = 9 if big else 7
    by = gy - (2 + s) + bob + (1 if sit else 0)
    bx0, bx1 = 2, 7 + s
    stretch = fire                               # trick: a long stretch, bum up
    g.rect(bx0, by - (1 if stretch == 2 else 0), bx1, gy - 2 + (1 if sit else 0), "o")
    for x in range(bx0 + 1, bx1, 2):
        g.set(x, by, "r")                        # stripes
    g.rect(bx0 + 1, gy - 2 + (1 if sit else 0), bx1 - 1, gy - 2 + (1 if sit else 0), "y")
    hx = bx1 + 1
    hy = by - (2 if big else 1) + (1 if stretch == 2 else 0)
    g.rect(hx - 1, hy, hx + 2 + s, hy + 2 + s, "o")
    g.set(hx - 1, hy - 1, "o")                   # ears
    g.set(hx + 1 + s, hy - 1, "o")
    g.set(hx - 1, hy - 2 if big else hy - 1, "r")
    g.set(hx + 1 + s, hy - 2 if big else hy - 1, "r")
    e = "o" if blink else "k"
    g.set(hx, hy + 1, e)
    g.set(hx + 2 + s, hy + 1, e)
    g.set(hx + 1 + s, hy + 2 + s, "r")           # nose
    g.set(hx + 3 + s, hy + 2, "w")               # whisker
    # tail curls up from the back; `tail` swishes it
    g.line([(bx0 - 1, by + 1), (bx0 - 2 + tail, by), (bx0 - 2 + tail, by - 1),
            (bx0 - 1 + tail, by - 2)], "o")
    g.set(bx0 - 1 + tail, by - 2, "r")
    if sit:
        return
    fp, bp = STRIDE[phase] if phase is not None else (0, 0)
    if stretch:
        fp = 2                                   # front paws reach forward
    for x in (bx0 + bp, bx0 + 1 - bp):
        leg(g, x, gy - 1, gy, "o", "y")
    for x in (bx1 - 1 + fp, bx1 - fp // 2):
        leg(g, x, gy - 1, gy, "o", "y")


# ------------------------------------------------------------------ dog
def dog(g, big, bob=0, phase=None, blink=False, wing=0, tail=0, fire=0, sit=False):
    """Tan pup: floppy ear, a brown patch, tongue out, a tail that wags."""
    s = 1 if big else 0
    gy = 9 if big else 7
    by = gy - (2 + s) + bob + (1 if sit else 0)
    bx0, bx1 = 2, 7 + s
    g.rect(bx0, by, bx1, gy - 2 + (1 if sit else 0), "y")
    g.rect(bx0 + 1, by, bx0 + 2 + s, by + 1, "o")   # patch
    hx = bx1 + 1
    hy = by - (3 if big else 2) - (1 if fire == 2 else 0)   # bark: head up
    g.rect(hx - 1, hy, hx + 2 + s, hy + 2 + s, "y")
    g.rect(hx + 2 + s, hy + 1 + s, hx + 3 + s, hy + 2 + s, "y")   # muzzle
    g.set(hx + 3 + s, hy + 1 + s, "k")           # nose
    g.rect(hx - 1, hy, hx - 1, hy + 2 + s, "o")  # floppy ear
    g.set(hx - 2, hy + 1, "o")
    g.set(hx + 1, hy + 1, "y" if blink else "k")
    if fire:
        g.set(hx + 3 + s, hy + 3 + s, "r")       # tongue / bark
        if fire >= 2:
            g.set(hx + 4 + s, hy - 1, "w")       # "!" bark
            g.set(hx + 4 + s, hy, "w")
    else:
        g.set(hx + 2 + s, hy + 3 + s, "r")       # tongue out
    # wagging tail
    g.line([(bx0 - 1, by + 1), (bx0 - 1 - (tail % 2), by - tail)], "o")
    if sit:
        g.rect(bx0, gy, bx1 + 1, gy, "y")
        return
    fp, bp = STRIDE[phase] if phase is not None else (0, 0)
    for x in (bx0 + bp, bx0 + 1 - bp):
        leg(g, x, gy - 1, gy, "y", "o")
    for x in (bx1 - 1 + fp, bx1 - fp):
        leg(g, x, gy - 1, gy, "y", "o")


# ------------------------------------------------------------------ owl
def owl(g, big, bob=0, phase=None, blink=False, wing=0, tail=0, fire=0, sit=False):
    """Round grey owl: ear tufts, big yellow eyes, beak; hops instead of walking,
    flaps its wings and tilts its head for its trick."""
    s = 1 if big else 0
    gy = 9 if big else 7
    hop = 1 if phase in (1, 3) else 0
    top = 1 + bob - hop + (1 if sit else 0)
    x0, x1 = 3, 8 + s
    tilt = 1 if fire == 2 else 0                 # head tilt
    g.rect(x0, top + tilt, x1, gy - 1 - hop, "S")
    g.rect(x0 + 1, top + 3 + s, x1 - 1, gy - 1 - hop, "s")   # chest
    for x in range(x0 + 1, x1, 2):
        g.set(x, top + 4 + s, "S")               # chest feathers
    g.set(x0, top - 1 + tilt, "S")               # ear tufts
    g.set(x1, top - 1, "S")
    ey = top + 1 + tilt
    for x in (x0 + 1, x1 - 1):
        g.set(x, ey, "y")
        if big:
            g.set(x + (1 if x == x0 + 1 else -1), ey, "y")
        g.set(x, ey + (1 if big else 0), "y" if blink else "k")
    g.set((x0 + x1) // 2 + (1 if big else 0), ey + 1 + s, "o")   # beak
    # wings at the sides; flapping lifts them out
    if fire in (1, 3):
        g.line([(x0 - 1, top + 2), (x0 - 2, top + 1), (x0 - 3, top + 2)], "d")
        g.line([(x1 + 1, top + 2), (x1 + 2, top + 1), (x1 + 3, top + 2)], "d")
    else:
        g.rect(x0 - 1, top + 2, x0 - 1, gy - 2 - hop, "d")
        g.rect(x1 + 1, top + 2, x1 + 1, gy - 2 - hop, "d")
    g.set(x0 + 1, gy - hop, "o")                 # feet
    g.set(x1 - 1, gy - hop, "o")


def frame(draw, big, **pose):
    g = Grid(19, 10) if big else Grid(13, 8)   # room in front for fire and barks
    draw(g, big, **pose)
    g.outline()
    return g.rows()


def anims(draw, big):
    F = lambda **k: frame(draw, big, **k)
    return {
        "idle 300": [F(), F(bob=1, tail=1, wing=1), F(tail=2), F(blink=True, tail=1)],
        "walk 110": [F(phase=0, wing=1), F(phase=1, bob=1, tail=1, wing=2),
                     F(phase=2, wing=1), F(phase=3, bob=1, tail=1)],
        "sit 900": [F(sit=True, blink=True, wing=2), F(sit=True, blink=True, bob=1, wing=2)],
        "trick 150": [F(fire=1, tail=1), F(fire=2, wing=1, tail=2),
                      F(fire=3, wing=0, tail=1), F(fire=1, bob=1)],
    }


PETS = {"cat": cat, "dog": dog, "dragon": dragon, "owl": owl}

if __name__ == "__main__":
    for name, draw in PETS.items():
        write(f"pet_{name}_baby", anims(draw, False))
        write(f"pet_{name}_grown", anims(draw, True))
    print("ok")
