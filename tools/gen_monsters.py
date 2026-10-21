"""Run: python tools/gen_monsters.py

Field monsters beside the slimes (TASKS M23): bat, skeleton, goblin, mushroom, 12x10,
facing right (the stage flips them to face the party). Tags: idle (2 frames), hurt.
Writes mob_<id>.txt."""
from pixel import Grid, write


def bat(g, f=0, hurt=False):
    """Purple bat; wings up / down; it flies, so it sits high in the frame."""
    y = 3 + f
    g.rect(5, y, 7, y + 2, "p")                  # body
    g.set(5, y - 1, "p")                         # ears
    g.set(7, y - 1, "p")
    e = "r" if not hurt else "k"
    g.set(6, y + 1, e)
    g.set(7, y + 1, e)
    g.set(6, y + 3, "w")                         # fang
    if f == 0:                                   # wings up
        g.line([(4, y), (3, y - 1), (2, y - 2), (1, y - 1), (0, y)], "d")
        g.line([(8, y), (9, y - 1), (10, y - 2), (11, y - 1)], "d")
        g.rect(2, y - 1, 4, y, "p")
        g.rect(8, y - 1, 10, y, "p")
    else:                                        # wings down
        g.line([(4, y + 1), (3, y + 2), (2, y + 3), (1, y + 2)], "d")
        g.line([(8, y + 1), (9, y + 2), (10, y + 3), (11, y + 2)], "d")
        g.rect(2, y + 1, 4, y + 2, "p")
        g.rect(8, y + 1, 10, y + 2, "p")


def skeleton(g, f=0, hurt=False):
    """Bone-white skeleton with a bone club."""
    y = f
    g.rect(4, y + 1, 7, y + 3, "w")              # skull
    g.set(7, y + 3, "s")
    g.set(5, y + 2, "k" if not hurt else "s")    # eye holes
    g.set(7, y + 2, "k" if not hurt else "s")
    g.rect(5, y + 4, 6, y + 4, "s")              # jaw
    g.set(5, y + 5, "w")                         # spine
    g.rect(4, y + 6, 7, y + 6, "w")              # ribs
    g.set(6, y + 5, "s")
    g.rect(4, y + 7, 7, y + 7, "s")
    g.set(4, 8, "w")                             # legs
    g.set(4, 9, "w")
    g.set(7, 8, "w")
    g.set(7, 9, "w")
    g.set(8, y + 6, "w")                         # arm and bone club
    g.rect(9, y + 3, 9, y + 6, "w")
    g.rect(9, y + 2, 10, y + 3, "s")


def goblin(g, f=0, hurt=False):
    """Green goblin: big ears, red eyes, loincloth, a wooden club."""
    y = 1 + f
    g.rect(4, y + 1, 7, y + 4, "g")              # head
    g.set(4, y + 1, "G")
    g.set(3, y + 2, "g")                         # ears
    g.set(2, y + 1, "g")
    g.set(8, y + 2, "g")
    g.set(9, y + 1, "g")
    e = "r" if not hurt else "k"
    g.set(5, y + 2, e)
    g.set(7, y + 2, e)
    g.rect(5, y + 4, 7, y + 4, "k")              # grin
    g.set(6, y + 4, "w")
    g.rect(4, y + 5, 7, y + 6, "g")              # body
    g.rect(4, y + 6, 7, y + 6, "o")              # loincloth
    g.set(4, 8, "t")                             # legs
    g.set(4, 9, "t")
    g.set(7, 8, "t")
    g.set(7, 9, "t")
    g.set(8, y + 5, "g")                         # arm and club
    g.rect(9, y + 2, 9, y + 5, "o")
    g.rect(9, y + 1, 10, y + 2, "r")


def mushroom(g, f=0, hurt=False):
    """Red-capped mushroom with white spots and a sleepy face; it squishes."""
    squish = f
    top = 1 + squish
    g.rect(2, top + 1, 9, top + 3, "r")          # cap
    g.rect(3, top, 8, top, "r")
    g.set(4, top + 1, "w")                       # spots
    g.set(7, top + 2, "w")
    g.set(5, top, "w")
    g.rect(4, top + 4, 7, 9, "y")                # stem
    g.rect(7, top + 4, 7, 9, "o")
    e = "k" if not hurt else "r"
    g.set(5, top + 5, e)
    g.set(6, top + 5, e)
    g.set(3, 9, "y")                             # feet
    g.set(8, 9, "o")


def frames(draw):
    def fr(**k):
        g = Grid(12, 10)
        draw(g, **k)
        g.outline()
        return g.rows()
    return {"idle 300": [fr(f=0), fr(f=1)], "hurt 100": [fr(f=0, hurt=True)]}


if __name__ == "__main__":
    for name, draw in [("bat", bat), ("skeleton", skeleton), ("goblin", goblin), ("mushroom", mushroom)]:
        write(f"mob_{name}", frames(draw))
    print("ok")
