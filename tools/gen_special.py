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
    visor slit and glowing eyes; steel armor, red cape, a big wooden bat. Light comes from
    the top-left: every material gets a highlight and a shadow.
    `bob` drops the upper body 1 px, `lift` raises everything (jumps), `cape` flutters."""
    g = Grid(16, 16)
    dy = bob - lift
    sit = 2 if legs == "sit" else 0
    ty = 8 + dy + sit                       # torso top
    hy = 2 + dy + sit                       # helmet top
    # --- red cape behind: folds (p) and a lighter edge (o); flutters while running
    for y in range(ty, ty + 5 - cape):
        g.set(3, y, "p" if y % 2 else "r")
        g.set(4, y, "r")
    g.set(2, ty + 4 - cape, "p")
    g.set(3, ty + 5 - cape, "r")
    # --- legs: steel greaves, a bright knee plate, dark boots
    ly = 12 - lift

    def leg(x, top, h):
        g.rect(x, top, x + 1, top + h - 1, "S")
        g.set(x, top + 1, "s")              # knee plate catches the light
        g.rect(x, top + h - 1, x + 1, top + h - 1, "d")
    if legs == "stand":
        leg(5, ly, 4)
        leg(9, ly, 4)
    elif legs == "strideA":
        leg(4, ly, 3)
        g.set(3, ly + 3, "d")
        leg(10, ly, 3)
        g.set(12, ly + 3, "d")
    elif legs == "strideB":
        leg(6, ly, 4)
        leg(8, ly, 3)
    elif legs == "sit":
        g.rect(5, 14, 12, 14, "S")
        g.rect(5, 15, 12, 15, "d")
        g.set(8, 14, "s")
    # --- torso: steel plate with a shine, shadowed right side, sun emblem, belt + buckle
    g.rect(5, ty, 10, ty + 2, "s")
    g.rect(9, ty, 10, ty + 2, "S")
    g.set(6, ty, "w")                       # shine
    g.set(7, ty + 1, "y")                   # sun emblem
    g.set(8, ty + 1, "o")
    g.set(7, ty + 2, "o")
    g.rect(5, ty + 3, 10, ty + 3, "o")      # leather belt
    g.set(7, ty + 3, "y")                   # buckle
    g.set(10, ty + 3, "r")
    g.set(4, ty, "s")                       # pauldrons
    g.set(4, ty + 1, "S")
    g.set(11, ty, "S")
    g.set(11, ty + 1, "d")
    # --- the kentongan drum helmet
    g.rect(4, hy, 11, hy + 5, "o")
    g.rect(5, hy, 10, hy, "y")              # cut top of the drum, lit
    g.set(5, hy + 1, "y")                   # highlight strip on the lit side
    g.set(5, hy + 4, "y")
    g.rect(10, hy + 1, 11, hy + 4, "r")     # shadow side
    g.rect(4, hy + 2, 11, hy + 2, "s")      # steel band (where the drum's rope goes)
    g.rect(10, hy + 2, 11, hy + 2, "S")
    g.rect(4, hy + 5, 11, hy + 5, "S")      # lower band
    g.set(4, hy + 5, "s")
    g.rect(5, hy + 3, 10, hy + 3, "k")      # the drum slit = visor
    if eyes == "open":                      # two-tone glow
        g.set(6, hy + 3, "o")
        g.set(7, hy + 3, "y")
        g.set(9, hy + 3, "y")
    elif eyes == "angry":
        g.set(6, hy + 3, "p")
        g.set(7, hy + 3, "r")
        g.set(9, hy + 3, "r")
    g.set(7, hy + 4, "r")                   # wood grain
    g.set(8, hy + 1, "r")
    # plume: highlight, body, and a tail sweeping back
    g.rect(7, hy - 2, 8, hy - 1, "r")
    g.set(7, hy - 2, "o")
    g.set(6, hy - 1, "r")
    g.set(5, hy - 1, "p")
    # --- front arm (steel gauntlet) and the bat (wood with a grip band and a shine)
    ay = ty + 1

    def gauntlet(x, y):
        g.set(x, y, "s")
        g.set(x + 1, y, "S")
    if arms == "hold":
        gauntlet(11, ay)
    elif arms == "fwd":
        gauntlet(11, ay)
        g.set(13, ay, "S")
    elif arms == "up":
        gauntlet(11, ay - 2)
        g.set(11, ay - 1, "S")
    elif arms == "raise":
        g.rect(12, ay - 4, 12, ay - 1, "S")
        g.set(12, ay - 4, "s")
    if bat == "upright":   # held like a sword at rest
        g.rect(13, ay - 6, 14, ay - 3, "y")
        g.set(13, ay - 6, "w")
        g.set(14, ay - 4, "o")
        g.rect(13, ay - 2, 13, ay + 1, "o")
        g.set(13, ay, "r")                  # grip band
    elif bat == "windup":  # raised back over the helmet
        g.rect(4, max(0, hy - 4), 11, max(0, hy - 3), "y")
        g.set(4, max(0, hy - 4), "w")
        g.set(7, max(0, hy - 3), "o")
        g.rect(12, max(0, hy - 3), 13, hy - 1, "o")
        g.set(12, hy - 2, "r")
    elif bat == "swing":   # straight out, full reach
        g.rect(13, ay - 1, 15, ay, "y")
        g.set(15, ay - 1, "w")
        g.set(14, ay, "o")
        g.set(12, ay, "r")
    elif bat == "low":     # follow-through
        g.rect(13, ay + 1, 13, ay + 2, "o")
        g.set(14, ay + 2, "r")
        g.rect(14, ay + 3, 15, ay + 4, "y")
        g.set(15, ay + 4, "o")
    elif bat == "high":    # raised to the sky
        g.rect(12, max(0, hy - 5), 13, ay - 3, "y")
        g.set(12, max(0, hy - 5), "w")
        g.set(13, ay - 4, "o")
        g.rect(12, ay - 2, 12, ay - 1, "o")
        g.set(12, ay - 2, "r")
    elif bat == "ground":  # resting beside a sitting knight
        g.rect(12, 13, 15, 13, "y")
        g.set(15, 13, "w")
        g.set(12, 13, "r")
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
    """8x8 for compact mode: the drum helmet with its band, slit and glowing eyes."""
    g = Grid(8, 8)
    t = 2 + bob
    g.rect(1, t, 6, 6, "o")
    g.rect(2, t, 5, t, "y")
    g.rect(5, t + 1, 6, 6, "r")
    g.rect(1, 6, 6, 6, "S")
    g.rect(2, t + 2, 5, t + 2, "k")
    if not blink:
        g.set(2, t + 2, "o")
        g.set(3, t + 2, "y")
        g.set(5, t + 2, "y")
    g.set(3, t - 1, "r")
    g.set(4, t - 1, "o")
    g.set(2, t - 1, "p")
    g.outline()
    return g.rows()


def portrait(talk):
    """32x32 close-up: the drum helmet head-on — grain, rope bands with rivets, a deep
    glowing visor slit, a flowing plume, a steel collar and the cape behind."""
    g = Grid(32, 32)
    g.rect(1, 27, 30, 31, "r")               # cape behind the shoulders
    g.rect(1, 29, 3, 31, "p")
    g.rect(28, 29, 30, 31, "p")
    g.rect(4, 26, 27, 31, "s")               # steel collar / shoulders
    g.rect(20, 26, 27, 31, "S")
    g.rect(5, 26, 9, 26, "w")                # shine
    for x in (8, 15, 23):
        g.set(x, 29, "d")                    # rivets
    g.rect(14, 28, 17, 30, "y")              # sun emblem
    g.rect(15, 29, 16, 29, "o")
    g.rect(6, 6, 25, 25, "o")                # the drum helmet
    g.rect(7, 6, 24, 7, "y")                 # cut top, lit
    g.rect(8, 8, 9, 22, "y")                 # highlight strip
    g.rect(21, 8, 25, 25, "r")               # shadow side
    g.rect(24, 9, 25, 24, "p")
    for (x, y, n) in ((12, 9, 4), (16, 12, 3), (11, 21, 5), (17, 22, 3)):
        g.rect(x, y, x + n, y, "r")          # wood grain
    for y in (10, 23):                       # steel rope bands with rivets
        g.rect(6, y, 25, y + 1, "s")
        g.rect(21, y, 25, y + 1, "S")
        for x in (9, 15, 21):
            g.set(x, y, "w")
    g.rect(8, 15, 23, 18, "k")               # the visor slit
    g.rect(9, 16, 22, 17, "p")               # depth inside the slit
    g.rect(11, 16, 13, 17, "o")              # glowing eyes: halo
    g.rect(18, 16, 20, 17, "o")
    g.set(12, 16, "y")
    g.set(19, 16, "y")
    g.set(12, 17, "w")
    g.set(19, 17, "w")
    if talk:
        g.rect(13, 19, 18, 21, "k")          # the drum speaks through a lower slit
        g.rect(14, 20, 17, 20, "p")
    g.rect(14, 1, 17, 5, "r")                # plume
    g.rect(14, 1, 15, 3, "o")                # plume highlight
    g.rect(10, 2, 13, 4, "r")                # tail sweeping back
    g.rect(7, 4, 9, 5, "p")
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

# ---------------------------------------------------------------------------------------
# The Wild Card: an original harlequin trickster (the joker of a card deck, not any
# comic character). Two-pointed jester hat with bells, white mask with a diamond tear,
# ruffled collar, red/yellow diamond motley, two-tone tights, curly shoes; throws cards.


def wild(bob=0, legs="stand", arms="down", card="none", eyes="open", mouth="grin",
         lift=0, juggle=None, bells=0):
    g = Grid(16, 16)
    dy = bob - lift
    sit = 2 if legs == "sit" else 0
    hy = 4 + dy + sit                        # top of the face
    ty = 9 + dy + sit                        # torso top
    # --- legs: one red, one yellow, curly pointed shoes
    ly = 12 - lift
    if legs == "stand":
        g.rect(5, ly, 6, ly + 2, "r")
        g.rect(9, ly, 10, ly + 2, "y")
        g.rect(4, ly + 3, 6, ly + 3, "k")
        g.set(3, ly + 2, "k")               # curly toe
        g.rect(9, ly + 3, 11, ly + 3, "k")
        g.set(12, ly + 2, "k")
    elif legs == "strideA":
        g.rect(4, ly, 5, ly + 2, "r")
        g.rect(10, ly, 11, ly + 2, "y")
        g.rect(2, ly + 3, 4, ly + 3, "k")
        g.rect(11, ly + 3, 13, ly + 3, "k")
        g.set(13, ly + 2, "k")
    elif legs == "strideB":
        g.rect(6, ly, 7, ly + 2, "r")
        g.rect(8, ly, 9, ly + 2, "y")
        g.rect(6, ly + 3, 10, ly + 3, "k")
        g.set(10, ly + 2, "k")
    elif legs == "sit":                      # cross-legged
        g.rect(4, 14, 7, 14, "r")
        g.rect(8, 14, 11, 14, "y")
        g.rect(3, 15, 12, 15, "k")
    # --- motley body: red/yellow diamonds, buttons, ruffled collar
    for y in range(ty, ty + 3):
        for x in range(5, 11):
            g.set(x, y, "r" if (x + y) % 2 else "y")
    g.set(7, ty + 1, "w")                    # pom-pom buttons
    g.set(8, ty + 2, "w")
    for x in range(4, 12):                   # ruffle: zigzag white/steel
        g.set(x, ty - 1, "w" if x % 2 else "s")
    # --- the mask
    g.rect(5, hy, 10, hy + 3, "w")
    g.rect(9, hy, 10, hy + 3, "s")           # shadow side
    if eyes == "open":
        g.set(6, hy + 1, "k")
        g.set(9, hy + 1, "k")
    elif eyes == "wink":
        g.set(6, hy + 1, "k")
        g.line([(8, hy + 1), (9, hy + 1)], "k")
    elif eyes == "closed":
        g.line([(6, hy + 1), (8, hy + 1), (9, hy + 1)], "k")
    elif eyes == "glint":
        g.set(6, hy + 1, "k")
        g.set(9, hy + 1, "C")
    g.set(6, hy + 2, "b")                    # the diamond tear
    if mouth == "grin":
        g.line([(6, hy + 3), (7, hy + 3), (8, hy + 3), (9, hy + 2)], "r")
    elif mouth == "laugh":
        g.rect(6, hy + 3, 9, hy + 3, "k")
        g.set(7, hy + 3, "r")
    # --- two-pointed jester hat with bells (they bounce)
    g.rect(5, hy - 1, 10, hy - 1, "k")       # band
    g.rect(4, hy - 2, 6, hy - 2, "r")
    g.rect(3, hy - 3, 4, hy - 3, "r")
    g.set(2, hy - 3 + bells, "y")            # left bell
    g.rect(9, hy - 2, 11, hy - 2, "y")
    g.rect(11, hy - 3, 12, hy - 3, "y")
    g.set(13, hy - 3 + (1 - bells), "y")     # right bell
    g.set(7, hy - 2, "r")
    g.set(8, hy - 2, "y")
    # --- arms (motley sleeves) and the cards
    ay = ty + 1
    if arms == "down":
        g.set(4, ay, "y")
        g.set(11, ay, "r")
        g.set(11, ay + 1, "w")              # glove
    elif arms == "back":                     # wind-up, card hand behind the head
        g.set(4, ay, "y")
        g.set(11, ay - 2, "r")
        g.set(12, ay - 3, "w")
    elif arms == "fwd":
        g.set(4, ay, "y")
        g.rect(11, ay, 12, ay, "r")
        g.set(13, ay, "w")
    elif arms == "up":
        g.set(4, ay - 2, "y")
        g.set(3, ay - 3, "w")
        g.set(11, ay - 2, "r")
        g.set(12, ay - 3, "w")
    elif arms == "bow":
        g.set(4, ay + 1, "y")
        g.rect(11, ay, 13, ay, "r")
        g.set(13, ay - 1, "w")
    elif arms == "raise":
        g.set(4, ay, "y")
        g.rect(12, ay - 4, 12, ay - 1, "r")
        g.set(12, ay - 5, "w")

    def a_card(x, y):
        g.rect(x, y, x + 1, y + 2, "w")
        g.set(x, y + 1, "r")                 # the red diamond pip
    if card == "hand":
        a_card(12, ay - 5)
    elif card == "fly":
        a_card(14, ay - 2)
    elif card == "fan":                      # a fan of cards at rest
        a_card(12, ay)
        g.set(14, ay, "w")
    if juggle:
        for x, y, c in juggle:
            g.set(x, y, c)
    g.outline()
    return g.rows()


W = wild
BALLS_A = [(4, 0, "r"), (8, 1, "y"), (12, 0, "C")]
BALLS_B = [(5, 1, "C"), (9, 0, "r"), (12, 2, "y")]
WILD = {
    "idle 250": [W(card="fan"), W(card="fan", bob=1, bells=1), W(card="fan"), W(card="fan", eyes="wink")],
    "sit 900": [W(legs="sit", eyes="closed", mouth="grin"), W(legs="sit", eyes="closed", bob=1, bells=1)],
    "walk 100": [W(legs="strideA", card="fan"), W(legs="strideB", card="fan", lift=1, bells=1),
                 W(legs="strideA", card="fan", bob=1), W(legs="strideB", card="fan", lift=1, bells=1)],
    "attack 80": [W(arms="back", card="hand", eyes="glint"),
                  W(arms="fwd", card="fly", eyes="glint", mouth="laugh"),
                  W(arms="fwd", mouth="laugh", bells=1)],
    "cast 100": [W(arms="up", juggle=BALLS_A, mouth="laugh"), W(arms="up", juggle=BALLS_B, mouth="laugh", bells=1)],
    "open 120": [W(arms="fwd", bob=1), W(arms="fwd")],
    "hold 400": [W(arms="fwd"), W(arms="fwd", bob=1, bells=1)],
    "raise_hand 300": [W(arms="raise"), W(arms="raise", bob=1, bells=1)],
    "victory 150": [W(arms="bow", bob=1, eyes="wink"), W(arms="up", lift=1, mouth="laugh", bells=1)],
}


def wild_face(wink=False, bob=0):
    g = Grid(8, 8)
    t = 3 + bob
    g.rect(2, t, 5, 6, "w")
    g.rect(5, t, 5, 6, "s")
    g.set(2, t + 1, "k")
    g.set(4, t + 1, "k" if not wink else "w")
    g.set(2, t + 2, "b")
    g.rect(2, t + 3, 4, t + 3, "r")
    g.rect(1, t - 1, 2, t - 1, "r")          # hat points
    g.rect(5, t - 1, 6, t - 1, "y")
    g.set(0, t - 2, "y")
    g.set(7, t - 2, "y")
    g.outline()
    return g.rows()


def wild_portrait(talk):
    g = Grid(32, 32)
    for y in range(26, 32):                  # motley shoulders
        for x in range(4, 28):
            g.set(x, y, "r" if ((x // 3) + (y // 3)) % 2 else "y")
    for x in range(3, 29):                   # big ruffled collar
        g.set(x, 24 + (x % 2), "w")
        g.set(x, 25 - (x % 2), "s")
    g.rect(8, 9, 23, 23, "w")                # mask
    g.rect(20, 9, 23, 23, "s")
    g.rect(10, 13, 13, 15, "k")              # eyes
    g.rect(18, 13, 21, 15, "k")
    g.set(11, 13, "w")
    g.set(19, 13, "w")
    g.rect(11, 17, 12, 18, "b")              # diamond tear
    g.set(11, 19, "b")
    g.set(12, 16, "b")
    if talk:                                 # laughing
        g.rect(12, 19, 19, 22, "k")
        g.rect(13, 21, 18, 22, "r")
    else:                                    # sly grin
        g.line([(12, 20), (13, 21), (14, 21), (15, 21), (16, 21), (17, 21), (18, 20), (19, 19)], "r")
    g.rect(7, 7, 24, 8, "k")                 # hat band
    g.rect(4, 4, 14, 6, "r")                 # left point
    g.rect(1, 2, 5, 4, "r")
    g.rect(17, 4, 27, 6, "y")                # right point
    g.rect(26, 2, 30, 4, "y")
    g.rect(0, 4, 1, 5, "y")                  # bells
    g.rect(30, 4, 31, 5, "r")
    g.set(15, 5, "r")
    g.set(16, 5, "y")
    g.outline()
    return g.rows()


write("hero_wild_card", WILD)
write("face_wild_card", {"idle 250": [wild_face(), wild_face(bob=1), wild_face(), wild_face(wink=True)]})
write("portrait_wild_card", {"talk 180": [wild_portrait(False), wild_portrait(True)], "idle 1000": [wild_portrait(False)]})

print("ok")
