"""Run: python tools/gen_mini.py

Chibi full-body heroes for small stage panes (compact layout): big head, small body,
12x12 with the outline, facing right. Every class keeps its look (headgear, colors,
weapon) and gets idle, walk, attack and sit. Writes mini_<class>.txt."""
from pixel import Grid, write

# Body colors per class: (main, shade, legs, boots).
BODY = {
    "knight": ("b", "n", "S", "d"),
    "mage": ("b", "n", "n", "d"),
    "rogue": ("p", "d", "d", "p"),
    "ranger": ("g", "t", "t", "d"),
    "cleric": ("w", "s", "s", "S"),
    "bard": ("o", "r", "n", "d"),
    "sahur_knight": ("s", "S", "S", "d"),
    "wild_card": ("r", "y", "r", "k"),
}


def head(g, cls, y, eyes):
    """Headgear and face, top row `y` (the face sits at y+2..y+4)."""
    e = "k" if eyes else None
    def face(c="o"):
        g.rect(4, y + 2, 8, y + 4, c)
        if e:
            g.set(6, y + 3, e)
            g.set(8, y + 3, e)
    if cls == "knight":
        g.rect(3, y + 1, 8, y + 4, "s")
        g.rect(8, y + 1, 8, y + 4, "S")
        g.set(4, y + 1, "w")
        g.rect(4, y + 3, 8, y + 3, "k")              # visor
        if eyes:
            g.set(6, y + 3, "w")
            g.set(8, y + 3, "w")
        g.rect(5, y, 6, y, "r")                      # plume
        g.set(4, y - 1 if y else 0, "r")
    elif cls == "mage":
        face()
        g.rect(4, y + 5, 8, y + 5, "w")              # beard
        g.rect(3, y + 1, 9, y + 1, "n")              # brim
        g.rect(5, y + 1, 7, y + 1, "y")              # band
        g.rect(5, y, 7, y, "b")                      # hat
        g.set(4, max(0, y - 1), "b")                 # floppy tip
    elif cls == "rogue":
        g.rect(3, y + 1, 8, y + 4, "d")
        g.set(4, y + 1, "S")
        g.rect(5, y + 2, 8, y + 3, "k")
        if eyes:
            g.set(6, y + 3, "C")
            g.set(8, y + 3, "C")
        g.rect(3, y + 4, 8, y + 4, "r")              # scarf
    elif cls == "ranger":
        g.rect(3, y + 1, 8, y + 4, "g")
        g.rect(5, y + 2, 8, y + 4, "o")
        if eyes:
            g.set(6, y + 3, "k")
            g.set(8, y + 3, "k")
        g.set(6, y, "G")                             # leaf
        g.set(7, max(0, y - 1), "G")
    elif cls == "cleric":
        g.rect(3, y + 1, 8, y + 4, "w")
        g.rect(5, y + 2, 8, y + 4, "o")
        if eyes:
            g.set(6, y + 3, "k")
            g.set(8, y + 3, "k")
        g.rect(4, y + 1, 8, y + 1, "y")              # circlet
        g.rect(4, max(0, y - 1), 7, max(0, y - 1), "y")   # halo
    elif cls == "bard":
        face()
        g.rect(3, y + 2, 3, y + 4, "y")              # hair
        g.rect(3, y + 1, 8, y + 1, "r")              # cap
        g.rect(4, y, 7, y, "r")
        g.set(9, y + 1, "p")
        g.set(2, max(0, y - 1), "G")                 # feather
        g.set(3, y, "G")
    elif cls == "sahur_knight":
        g.rect(3, y, 8, y + 4, "o")                  # the drum helmet
        g.rect(4, y, 7, y, "y")
        g.rect(3, y + 1, 8, y + 1, "s")              # band
        g.rect(4, y + 3, 8, y + 3, "k")              # slit
        if eyes:
            g.set(6, y + 3, "y")
            g.set(8, y + 3, "y")
        g.set(5, max(0, y - 1), "r")                 # plume
    elif cls == "wild_card":
        g.rect(3, y + 2, 8, y + 4, "w")              # mask
        g.set(3, y + 3, "s")
        g.set(9, y + 3, "w")                         # nose
        if eyes:
            g.set(6, y + 3, "k")
            g.set(8, y + 3, "k")
        g.set(6, y + 4, "b")                         # tear
        g.rect(3, y + 1, 8, y + 1, "k")              # hat band
        g.rect(2, y, 4, y, "r")                      # two points with bells
        g.set(1, max(0, y - 1), "y")
        g.rect(7, y, 9, y, "y")
        g.set(10, max(0, y - 1), "y")


def weapon(g, cls, y, pose):
    """The class weapon at the front hand (x 9-10); `pose` rest/up/strike."""
    ax = 9
    if cls in ("knight", "sahur_knight"):
        c, tip = ("s", "w") if cls == "knight" else ("y", "y")
        if pose == "strike":
            g.rect(ax, y + 6, ax + 1, y + 6, c)
            g.set(ax + 1, y + 6, tip)
        elif pose == "up":
            g.rect(ax, y, ax, y + 4, c)
            g.set(ax, y, tip)
        else:
            g.rect(ax, y + 2, ax, y + 6, c)
            g.set(ax, y + 2, tip)
            if cls == "knight":
                g.set(ax, y + 6, "y")                # guard
    elif cls == "mage":
        top = y if pose != "strike" else y + 3
        g.rect(ax, top + 1, ax, y + 9, "o")
        g.set(ax, top, "C")
        if pose == "strike":
            g.set(ax + 1, top, "y")                  # spark
    elif cls == "rogue":
        if pose == "strike":
            g.set(ax, y + 6, "s")
            g.set(ax + 1, y + 6, "w")
        else:
            g.set(ax, y + 5, "s")
            g.set(ax, y + 4 if pose == "rest" else y + 3, "w")
    elif cls == "ranger":
        g.line([(ax, y + 2), (ax + 1, y + 3), (ax + 1, y + 4), (ax + 1, y + 5),
                (ax + 1, y + 6), (ax, y + 7)], "o")
        g.rect(ax, y + 3, ax, y + 6, "w" if pose != "up" else "s")
        if pose == "strike":
            g.set(ax + 2 if ax + 2 < 12 else ax + 1, y + 4, "w")   # arrow away
    elif cls == "cleric":
        hy = y + 2 if pose != "strike" else y + 5
        g.rect(ax, hy + 1, ax, y + 7, "S")
        g.rect(ax, hy, ax + 1, hy, "y")
    elif cls == "bard":
        g.rect(ax - 1, y + 6, ax, y + 7, "y")        # lute body
        g.set(ax + 1, y + 5 if pose != "strike" else y + 6, "o")   # neck
        if pose == "up":
            g.set(ax + 1, y + 2, "y")                # a note
    elif cls == "wild_card":
        if pose == "strike":
            g.rect(ax + 1, y + 5, ax + 1, y + 6, "w")   # card thrown
        else:
            g.rect(ax, y + 5, ax, y + 6, "w")
            g.set(ax, y + 5, "r")


def figure(cls, bob=0, legs="stand", pose="rest", eyes=True, lunge=0):
    g = Grid(12, 12)
    main, shade, leg, boot = BODY[cls]
    sit = legs == "sit"
    y = 1 + bob + (1 if sit else 0)                  # head top
    # body (motley for the Wild Card)
    for by in range(y + 5, y + 8):
        for bx in range(3, 8):
            c = main if bx < 7 else shade
            if cls == "wild_card":
                c = "r" if (bx + by) % 2 else "y"
            g.set(bx + lunge, by, c)
    # legs
    ly = y + 8
    if sit:
        g.rect(3, 11, 9, 11, leg)
    else:
        xs = {"stand": (4, 6), "strideA": (3, 7), "strideB": (5, 6)}[legs]
        for x in xs:
            g.set(x, min(ly, 10), leg)
            g.set(x, min(ly + 1, 11), boot)
    head(g, cls, y, eyes)
    weapon(g, cls, y + (1 if sit else 0), pose)
    g.outline()
    return g.rows()


def anims(cls):
    F = lambda **k: figure(cls, **k)
    return {
        "idle 300": [F(), F(bob=1), F(), F(eyes=False)],
        "walk 120": [F(legs="strideA"), F(legs="strideB", bob=1), F(legs="strideA"),
                     F(legs="strideB", bob=1)],
        "attack 100": [F(pose="up"), F(pose="strike", lunge=1), F(pose="rest")],
        "sit 900": [F(legs="sit", eyes=False), F(legs="sit", eyes=False, bob=1)],
    }


if __name__ == "__main__":
    for cls in BODY:
        write(f"mini_{cls}", anims(cls))
    print("ok")
