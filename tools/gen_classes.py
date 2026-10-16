"""Run: python tools/gen_classes.py

The six standard classes (ART.md §4), each with its own body, outfit and weapon:
Knight (sword + shield), Mage (staff + fireball), Rogue (twin daggers + smoke),
Ranger (bow + arrow), Cleric (mace + sun shield), Bard (lute + music notes).

Every class is one draw function over a shared pose `P`; `anims()` turns it into all
the tags the game plays. Writes hero_/face_/portrait_<class>.txt."""
from pixel import Grid, sparkle, write


def pose(bob=0, legs="stand", arms="hold", wpn="rest", eyes="open", lift=0, flap=0):
    """Where things go in a 16x16 frame. The hero faces right: weapon hand at x 11-12,
    weapon at x 12-15; the back side (cape, quiver, shield) is x 1-4."""
    dy = bob - lift
    sit = 2 if legs == "sit" else 0
    return dict(hy=2 + dy + sit, ty=8 + dy + sit, ay=9 + dy + sit, ly=12 - lift,
                legs=legs, arms=arms, wpn=wpn, eyes=eyes, flap=flap)


def legs(g, P, c, hi, boot):
    """Two legs (trousers `c`, knee `hi`, boots), striding or sitting."""
    ly, L = P["ly"], P["legs"]

    def leg(x, top, h):
        g.rect(x, top, x + 1, top + h - 1, c)
        g.set(x, top + 1, hi)
        g.rect(x, top + h - 1, x + 1, top + h - 1, boot)
    if L == "stand":
        leg(5, ly, 4)
        leg(9, ly, 4)
    elif L == "strideA":
        leg(4, ly, 3)
        g.set(3, ly + 3, boot)
        leg(10, ly, 3)
        g.set(12, ly + 3, boot)
    elif L == "strideB":
        leg(6, ly, 4)
        leg(8, ly, 3)
    elif L == "sit":
        g.rect(5, 14, 12, 14, c)
        g.set(8, 14, hi)
        g.rect(5, 15, 12, 15, boot)


def robe(g, P, c, shade, feet):
    """A floor-length robe hem instead of legs; the feet peek out and stride."""
    ly, L = P["ly"], P["legs"]
    if L == "sit":
        g.rect(4, 14, 12, 15, c)
        g.rect(10, 14, 12, 15, shade)
        g.set(12, 15, feet)
        return
    g.rect(4, ly, 11, ly + 2, c)
    g.rect(9, ly, 11, ly + 2, shade)
    g.set(5, ly + 1, shade)                  # a fold
    feet_x = {"stand": (5, 9), "strideA": (3, 11), "strideB": (6, 8)}[L]
    for x in feet_x:
        g.rect(x, ly + 3, x + 1, ly + 3, feet)
    if L == "strideA":
        g.set(3, ly + 2, c)                  # the hem swings
        g.set(12, ly + 2, shade)


def front_hand(g, P, sleeve, hand):
    """The weapon arm: down at the side, forward, up (wind-up) or raised high."""
    ay, A = P["ay"], P["arms"]
    if A == "hold":
        g.set(11, ay, sleeve)
        g.set(12, ay, hand)
    elif A == "fwd":
        g.rect(11, ay, 12, ay, sleeve)
        g.set(13, ay, hand)
    elif A == "up":
        g.set(11, ay - 1, sleeve)
        g.set(12, ay - 2, hand)
    elif A == "raise":
        g.rect(12, ay - 3, 12, ay - 1, sleeve)
        g.set(12, ay - 4, hand)


def eye(P, open_c, off_c, angry_c):
    return {"open": open_c, "off": off_c, "angry": angry_c}[P["eyes"]]


# ---------------------------------------------------------------- Knight
def knight(g, P):
    """Rounded steel helmet with a red plume and a visor slit (two white eye dots),
    blue tunic over steel, red kite shield on the back arm, short sword."""
    hy, ty, ay, W = P["hy"], P["ty"], P["ay"], P["wpn"]
    # kite shield on the back arm: red field, gold cross
    g.rect(2, ty, 4, ty + 2, "r")
    g.set(3, ty + 3, "r")
    g.rect(3, ty, 3, ty + 2, "y")
    g.set(2, ty + 1, "y")
    g.set(2, ty, "o")                        # lit corner
    legs(g, P, "S", "s", "d")
    # tunic with a steel collar and a skirt
    g.rect(5, ty, 10, ty + 3, "b")
    g.rect(9, ty, 10, ty + 3, "n")
    g.rect(5, ty, 10, ty, "s")               # mail collar
    g.set(10, ty, "S")
    g.rect(5, ty + 2, 10, ty + 2, "o")       # belt
    g.set(7, ty + 2, "y")
    g.set(4, ty + 3, "b")                    # skirt flares
    g.set(11, ty + 3, "n")
    g.set(11, ty, "S")                       # pauldron
    # rounded helmet: highlight top-left, shadow right
    g.rect(6, hy, 9, hy, "s")
    g.rect(5, hy + 1, 10, hy + 5, "s")
    g.rect(9, hy + 1, 10, hy + 5, "S")
    g.set(6, hy + 1, "w")
    g.set(5, hy + 5, "S")
    g.rect(6, hy + 3, 10, hy + 3, "k")       # visor slit
    e = eye(P, "w", "k", "y")
    g.set(7, hy + 3, e)
    g.set(9, hy + 3, e)
    g.set(8, hy + 4, "S")                    # breathing holes
    g.set(9, hy + 5, "d")
    # plume sweeping back
    g.rect(7, hy - 1, 8, hy - 1, "r")
    g.set(8, hy - 2, "o")
    g.set(7, hy - 2, "r")
    g.set(6, hy - 2, "r")
    g.set(5, hy - 1 + P["flap"], "p")
    front_hand(g, P, "b", "s")

    def guard(x, y):                         # crossguard + grip
        g.rect(x - 1, y, x + 1, y, "y")
    if W == "rest":                          # sword upright at the side
        g.rect(13, ay - 5, 13, ay - 2, "s")
        g.set(13, ay - 5, "w")
        g.set(14, ay - 4, "S")               # the blade's dark edge
        g.rect(14, ay - 3, 14, ay - 2, "S")
        guard(13, ay - 1)
        g.set(13, ay, "o")
    elif W in ("windup", "cast"):            # raised high
        g.rect(12, max(0, ay - 9), 12, ay - 4, "s")
        g.set(12, max(0, ay - 9), "w")
        g.rect(13, max(0, ay - 8), 13, ay - 4, "S")
        guard(12, ay - 3)
    elif W == "strike":                      # thrust out, with a swoosh
        g.rect(13, ay, 15, ay, "s")
        g.set(15, ay, "w")
        g.rect(13, ay + 1, 15, ay + 1, "S")
        g.rect(12, ay - 1, 12, ay + 1, "y")
        g.line([(14, ay - 3), (15, ay - 2)], "w")
    elif W == "follow":                      # cut through, point down
        g.line([(13, ay + 1), (14, ay + 2), (15, ay + 3)], "s")
        g.line([(14, ay + 1), (15, ay + 2)], "S")
        g.set(15, ay + 4, "w")
        g.set(13, ay, "y")
    elif W == "ground":
        g.rect(11, 13, 14, 13, "s")
        g.set(15, 13, "w")
        g.rect(10, 12, 10, 14, "y")
    elif W == "stow":                        # sheathed at the hip
        g.line([(11, ty + 2), (12, ty + 3), (13, ty + 4)], "d")
        g.set(11, ty + 1, "y")


def knight_face(blink=False, bob=0):
    g = Grid(8, 8)
    t = 2 + bob
    g.rect(2, t, 5, t, "s")
    g.rect(1, t + 1, 6, 6, "s")
    g.rect(5, t + 1, 6, 6, "S")
    g.set(2, t + 1, "w")
    g.rect(2, t + 2, 6, t + 2, "k")
    if not blink:
        g.set(3, t + 2, "w")
        g.set(5, t + 2, "w")
    g.rect(3, t - 1, 4, t - 1, "r")
    g.set(2, t - 2, "r")
    g.outline()
    return g.rows()


def knight_portrait(talk):
    g = Grid(32, 32)
    g.rect(2, 26, 29, 31, "b")               # tunic shoulders
    g.rect(20, 26, 29, 31, "n")
    g.rect(2, 25, 9, 28, "s")                # pauldrons
    g.rect(22, 25, 29, 28, "S")
    g.rect(3, 25, 6, 25, "w")
    g.rect(11, 26, 20, 27, "s")              # mail collar
    g.rect(9, 7, 22, 25, "s")                # helmet dome
    g.rect(11, 5, 20, 6, "s")
    g.rect(18, 7, 22, 25, "S")
    g.rect(11, 8, 13, 12, "w")               # shine
    g.rect(15, 5, 16, 25, "S")               # the ridge down the middle
    g.rect(15, 5, 15, 25, "s")
    for x, y in ((9, 7), (10, 7), (9, 8), (22, 7), (21, 7), (22, 8), (11, 5), (20, 5)):
        g.set(x, y, ".")                     # round the dome
    g.rect(10, 15, 21, 17, "k")              # visor slit
    g.rect(12, 16, 13, 16, "w")              # eye dots
    g.rect(18, 16, 19, 16, "w")
    for y in (20, 22) if not talk else (20, 22, 24):
        for x in (12, 14, 17, 19):
            g.set(x, y, "d")                 # breathing holes
    g.rect(14, 1, 17, 4, "r")                # plume
    g.rect(14, 1, 15, 2, "o")
    g.rect(9, 2, 13, 3, "r")
    g.rect(5, 3, 8, 5, "p")
    g.outline()
    return g.rows()


# ---------------------------------------------------------------- Mage
def mage(g, P):
    """Tall floppy hat and robe in blue with gold trim, white beard, staff with a cyan
    orb; attacks hurl a fireball."""
    hy, ty, ay, W = P["hy"], P["ty"], P["ay"], P["wpn"]
    robe(g, P, "b", "n", "d")
    g.rect(5, ty, 10, ty + 3, "b")
    g.rect(9, ty, 10, ty + 3, "n")
    g.rect(5, ty + 3, 10, ty + 3, "y")       # gold rope belt
    g.set(4, ty + 1, "b")                    # back sleeve
    g.set(4, ty + 2, "n")
    # face and beard
    g.rect(6, hy + 3, 10, hy + 4, "o")
    e = eye(P, "k", "r", "C")
    g.set(8, hy + 3, e)
    g.set(10, hy + 3, e)
    g.rect(6, hy + 5, 10, hy + 6, "w")       # beard over the robe
    g.rect(9, hy + 5, 10, hy + 6, "s")
    g.rect(7, hy + 7, 8, hy + 7, "w")
    g.set(8, hy + 8, "s")
    # floppy hat: wide brim, gold band, a star, the tip flopping back
    g.rect(4, hy + 2, 11, hy + 2, "n")
    g.rect(5, hy + 1, 10, hy + 1, "y")
    g.rect(6, hy, 9, hy, "b")
    g.set(7, hy, "y")                        # star
    g.rect(7, hy - 1, 9, hy - 1, "b")
    g.set(9, hy - 1, "n")
    g.rect(5, hy - 2, 7, hy - 2, "b")
    g.set(4, hy - 1 + P["flap"], "n")        # the floppy tip
    front_hand(g, P, "b", "o")
    if W == "rest":                          # staff planted beside him
        g.rect(13, ay - 4, 13, min(15, P["ly"] + 3), "o")
        g.set(13, ay - 1, "r")
        g.rect(13, ay - 6, 14, ay - 5, "C")
        g.set(14, ay - 5, "c")
        g.set(12, ay - 5, "c")
    elif W in ("windup", "cast"):            # staff held up, orb blazing
        g.rect(12, ay - 5, 12, ay + 2, "o")
        g.rect(12, max(0, ay - 8), 13, ay - 7, "C")
        g.set(12, max(0, ay - 8), "w")
        g.set(11, ay - 6, "c")
        g.set(13, ay - 6, "c")
    elif W == "strike":                      # staff thrust, fireball away
        g.rect(13, ay - 1, 13, ay + 3, "o")
        g.rect(13, ay - 3, 13, ay - 2, "C")
        g.rect(14, ay - 2, 15, ay, "o")      # fireball
        g.set(15, ay - 1, "y")
        g.set(14, ay - 1, "r")
        g.set(15, ay - 3, "y")
    elif W == "follow":
        g.rect(13, ay - 4, 13, min(15, P["ly"] + 3), "o")
        g.rect(13, ay - 6, 14, ay - 5, "c")
        g.set(15, ay - 7, "y")               # a last spark
    elif W == "ground":
        g.rect(9, 13, 14, 13, "o")
        g.rect(15, 12, 15, 13, "C")
    elif W == "stow":                        # staff on the back
        g.rect(3, ay - 4, 3, min(15, P["ly"] + 2), "o")
        g.rect(2, ay - 6, 3, ay - 5, "C")


def mage_face(blink=False, bob=0):
    g = Grid(8, 8)
    t = 3 + bob
    g.rect(2, t, 6, t + 1, "o")
    g.set(4, t, "r" if blink else "k")
    g.set(6, t, "r" if blink else "k")
    g.rect(2, t + 2, 6, 7, "w")
    g.rect(5, t + 2, 6, 7, "s")
    g.rect(1, t - 1, 7, t - 1, "n")
    g.rect(2, t - 2, 5, t - 2, "b")
    g.set(3, t - 2, "y")
    g.rect(1, max(0, t - 3), 3, max(0, t - 3), "b")
    g.outline()
    return g.rows()


def mage_portrait(talk):
    g = Grid(32, 32)
    g.rect(1, 26, 30, 31, "b")               # robe shoulders, gold trim
    g.rect(21, 26, 30, 31, "n")
    g.rect(1, 26, 30, 26, "y")
    g.rect(9, 14, 22, 22, "o")               # face
    g.rect(20, 14, 22, 22, "r")
    g.rect(11, 15, 13, 15, "w")              # bushy brows
    g.rect(18, 15, 20, 15, "w")
    g.rect(12, 16, 13, 17, "k")
    g.rect(18, 16, 19, 17, "k")
    g.set(12, 16, "w")
    g.set(18, 16, "w")
    g.rect(15, 18, 16, 19, "r")              # nose
    g.rect(8, 20, 23, 29, "w")               # the great beard
    g.rect(19, 20, 23, 29, "s")
    g.rect(11, 30, 20, 31, "w")
    g.rect(13, 21, 18, 21, "s")              # moustache line
    if talk:
        g.rect(14, 23, 17, 24, "k")
    g.rect(4, 12, 27, 13, "n")               # brim
    g.rect(7, 10, 24, 11, "y")               # gold band
    g.rect(15, 10, 16, 11, "C")              # gem
    g.rect(9, 5, 22, 9, "b")                 # cone
    g.rect(19, 5, 22, 9, "n")
    g.rect(12, 2, 19, 4, "b")
    g.rect(5, 0, 12, 2, "b")                 # flopping tip
    g.rect(3, 1, 5, 3, "n")
    g.rect(13, 6, 14, 7, "y")                # star
    g.outline()
    return g.rows()


# ---------------------------------------------------------------- Rogue
def rogue(g, P):
    """Dark hood with glowing cyan eyes in the shadow, red scarf that streams behind,
    leather jerkin, twin daggers; vanishes in smoke to cast."""
    hy, ty, ay, W = P["hy"], P["ty"], P["ay"], P["wpn"]
    f = P["flap"]
    g.set(4, ty, "r")                        # scarf tail
    g.set(3, ty + f, "r")
    g.set(2, ty + 1 - f, "p")
    legs(g, P, "d", "S", "p")
    g.rect(5, ty, 10, ty + 3, "p")
    g.rect(9, ty, 10, ty + 3, "d")
    g.line([(5, ty + 1), (6, ty + 2), (7, ty + 3)], "o")   # strap
    g.rect(8, ty + 3, 10, ty + 3, "o")       # belt
    g.set(4, ty + 2, "p")                    # back arm
    # the hood: a point at the back, a shadowed opening at the front
    g.rect(6, hy, 9, hy, "d")
    g.rect(5, hy + 1, 10, hy + 5, "d")
    g.set(4, hy + 2, "d")
    g.set(4, hy + 3, "d")
    g.set(6, hy, "S")                        # lit edge
    g.set(5, hy + 1, "S")
    g.rect(7, hy + 2, 10, hy + 4, "k")
    e = eye(P, "C", "n", "r")
    g.set(8, hy + 3, e)
    g.set(10, hy + 3, e)
    g.rect(5, hy + 5, 10, hy + 5, "r")       # scarf over the mouth
    g.set(10, hy + 5, "p")
    front_hand(g, P, "p", "d")
    A = P["arms"]
    back = {"hold": (4, ay + 1), "fwd": (4, ay), "up": (4, ay - 2), "raise": (4, ay + 1)}[A]

    def dagger(x, y, dx, dy):                # grip at (x, y), blade 2 px along (dx, dy)
        g.set(x, y, "o")
        g.set(x + dx, y + dy, "s")
        g.set(x + 2 * dx, y + 2 * dy, "w")
    if W == "rest":
        dagger(13, ay, 0, -1)
        dagger(back[0] - 1, back[1], 0, -1)
    elif W in ("windup", "cast"):
        dagger(13, ay - 2, 1, -1)
        dagger(3, ay - 2, -1, -1)
    elif W == "strike":                      # double lunge, two slash marks
        g.rect(13, ay, 14, ay, "s")
        g.set(15, ay, "w")
        g.set(12, ay + 1, "d")
        g.set(13, ay + 1, "s")
        g.set(14, ay + 1, "w")
        g.line([(15, ay - 3), (14, ay - 2)], "w")
        g.set(15, ay + 3, "w")
    elif W == "follow":
        dagger(13, ay + 1, 1, 1)
        dagger(3, ay + 1, -1, 1)
    elif W == "ground":                      # stuck in the ground
        g.rect(12, 12, 12, 13, "s")
        g.set(12, 11, "o")
        g.rect(14, 12, 14, 13, "s")
        g.set(14, 11, "o")
    elif W == "stow":
        g.set(4, ty + 3, "o")
        g.set(3, ty + 4, "s")
        g.set(11, ty + 3, "o")
        g.set(12, ty + 4, "s")


def rogue_face(blink=False, bob=0):
    g = Grid(8, 8)
    t = 2 + bob
    g.rect(2, t, 5, t, "d")
    g.rect(1, t + 1, 6, 6, "d")
    g.set(2, t, "S")
    g.rect(2, t + 2, 6, t + 3, "k")
    if not blink:
        g.set(3, t + 2, "C")
        g.set(5, t + 2, "C")
    g.rect(1, 6, 6, 7, "r")
    g.set(0, 7, "r")
    g.outline()
    return g.rows()


def rogue_portrait(talk):
    g = Grid(32, 32)
    g.rect(2, 26, 29, 31, "p")               # jerkin
    g.rect(21, 26, 29, 31, "d")
    g.line([(6 + i, 26 + i) for i in range(6)], "o")     # strap
    g.rect(6, 4, 25, 25, "d")                # hood
    g.rect(9, 2, 22, 3, "d")
    g.rect(8, 4, 12, 6, "S")                 # lit edge
    g.rect(6, 6, 7, 12, "S")
    g.rect(9, 9, 22, 22, "k")                # the dark opening
    g.rect(11, 14, 14, 15, "c")              # glowing eyes
    g.rect(18, 14, 21, 15, "c")
    g.rect(12, 14, 13, 14, "C")
    g.rect(19, 14, 20, 14, "C")
    g.rect(5, 20, 26, 26, "r")               # scarf up to the nose
    g.rect(20, 20, 26, 26, "p")
    g.rect(8, 22, 23, 22, "p")               # folds
    g.rect(10, 24, 21, 24, "p")
    if talk:
        g.rect(14, 21, 17, 21, "p")          # the scarf moves when it talks
    g.outline()
    return g.rows()


# ---------------------------------------------------------------- Ranger
def ranger(g, P):
    """Green cloak and hood with a leaf, quiver on the back, longbow; attacks draw and
    loose an arrow."""
    hy, ty, ay, W = P["hy"], P["ty"], P["ay"], P["wpn"]
    g.rect(3, ty - 2, 4, ty + 1, "r")        # quiver
    g.set(3, ty - 3, "w")                    # fletching
    g.set(4, ty - 3, "y")
    g.set(4, ty - 4, "w")
    legs(g, P, "t", "g", "d")
    g.rect(5, ty, 10, ty + 3, "g")           # tunic
    g.rect(9, ty, 10, ty + 3, "t")
    g.rect(5, ty + 3, 10, ty + 3, "o")       # belt
    g.set(8, ty + 3, "y")
    g.rect(4, ty + 1, 4, ty + 3, "t")        # cloak hem
    g.set(3, ty + 4 - P["flap"], "t")
    # hood with a face, a point at the back and a leaf on top
    g.rect(6, hy, 9, hy, "g")
    g.rect(5, hy + 1, 10, hy + 5, "g")
    g.set(4, hy + 2, "g")
    g.set(3, hy + 3, "t")
    g.set(6, hy, "G")
    g.rect(5, hy + 5, 10, hy + 5, "t")
    g.rect(7, hy + 2, 10, hy + 4, "o")
    e = eye(P, "k", "r", "k")
    g.set(8, hy + 3, e)
    g.set(10, hy + 3, e)
    if P["eyes"] == "angry":
        g.rect(8, hy + 2, 10, hy + 2, "t")   # a squint along the arrow
    g.set(8, hy - 1, "G")                    # leaf
    g.set(9, hy - 1, "G")
    g.set(9, hy - 2, "g")
    front_hand(g, P, "g", "o")

    def bow(pull):                           # stave ")" at x 11-13, string at 11 - pull
        g.line([(11, ay - 4), (12, ay - 3), (13, ay - 2), (13, ay - 1), (13, ay),
                (13, ay + 1), (13, ay + 2), (12, ay + 3), (11, ay + 4)], "o")
        g.set(11, ay - 4, "y")
        g.set(11, ay + 4, "y")
        if pull:
            g.line([(11, ay - 3), (10, ay - 2), (9, ay - 1), (9, ay), (9, ay + 1),
                    (10, ay + 2), (11, ay + 3)], "w")
        else:
            g.rect(11, ay - 3, 11, ay + 3, "w")
    if W == "rest":
        bow(False)
    elif W == "windup":                      # drawn, arrow nocked
        bow(True)
        g.rect(10, ay, 14, ay, "S")
        g.set(15, ay, "w")
        g.set(9, ay, "r")
    elif W == "strike":                      # loosed: the arrow streaks off
        bow(False)
        g.rect(14, ay, 15, ay, "w")
        g.set(15, ay - 2, "s")
        g.set(14, ay + 2, "s")
    elif W == "follow":
        bow(False)
        g.set(12, ay - 1, "w")               # the string still hums
        g.set(12, ay + 1, "w")
    elif W == "cast":                        # bow raised to the sky
        g.line([(11, ay - 7), (12, ay - 6), (13, ay - 5), (14, ay - 4), (15, ay - 3)], "o")
        g.line([(12, ay - 7), (15, ay - 4)], "w")
    elif W == "ground":
        g.rect(10, 13, 15, 13, "o")
        g.rect(11, 12, 14, 12, "w")
    elif W == "stow":                        # bow across the back
        g.rect(2, ty - 3, 2, ty + 3, "o")


def ranger_face(blink=False, bob=0):
    g = Grid(8, 8)
    t = 2 + bob
    g.rect(2, t, 5, t, "g")
    g.rect(1, t + 1, 6, 6, "g")
    g.set(2, t, "G")
    g.rect(2, t + 2, 6, t + 3, "o")
    g.set(3, t + 2, "r" if blink else "k")
    g.set(5, t + 2, "r" if blink else "k")
    g.rect(1, 6, 6, 6, "t")
    g.set(4, t - 1, "G")
    g.set(5, max(0, t - 2), "G")
    g.outline()
    return g.rows()


def ranger_portrait(talk):
    g = Grid(32, 32)
    g.rect(24, 14, 26, 26, "r")              # quiver over the shoulder
    g.rect(25, 10, 25, 13, "o")
    g.rect(24, 9, 26, 9, "w")
    g.rect(27, 8, 27, 10, "y")
    g.rect(1, 25, 30, 31, "t")               # cloak
    g.rect(1, 25, 12, 31, "g")
    g.rect(14, 27, 17, 29, "G")              # leaf brooch
    g.set(15, 28, "g")
    g.rect(6, 5, 23, 24, "g")                # hood
    g.rect(9, 3, 20, 4, "g")
    g.rect(8, 5, 11, 7, "G")
    g.rect(19, 5, 23, 24, "t")
    g.rect(9, 10, 21, 23, "o")               # face
    g.rect(19, 10, 21, 23, "r")
    g.rect(11, 13, 14, 13, "t")              # brows
    g.rect(17, 13, 20, 13, "t")
    g.rect(12, 14, 13, 16, "k")
    g.rect(18, 14, 19, 16, "k")
    g.set(12, 14, "w")
    g.set(18, 14, "w")
    g.rect(15, 17, 16, 18, "r")              # nose
    if talk:
        g.rect(13, 20, 17, 21, "k")
    else:
        g.rect(13, 20, 17, 20, "r")
    g.rect(15, 0, 18, 2, "G")                # leaf on top
    g.set(14, 2, "g")
    g.outline()
    return g.rows()


# ---------------------------------------------------------------- Cleric
def cleric(g, P):
    """White and gold robe, hood with a gold circlet, a floating halo, round shield with
    a sun on the back arm, a gold mace; casts in holy light."""
    hy, ty, ay, W = P["hy"], P["ty"], P["ay"], P["wpn"]
    robe(g, P, "w", "s", "S")
    g.rect(5, ty, 10, ty + 3, "w")
    g.rect(9, ty, 10, ty + 3, "s")
    g.rect(7, ty, 7, ty + 3, "y")            # gold stole
    g.set(8, ty, "y")
    # hood, face, circlet, halo
    g.rect(6, hy, 9, hy, "w")
    g.rect(5, hy + 1, 10, hy + 5, "w")
    g.rect(5, hy + 4, 5, hy + 5, "s")
    g.rect(7, hy + 2, 10, hy + 4, "o")
    e = eye(P, "k", "r", "y")
    g.set(8, hy + 3, e)
    g.set(10, hy + 3, e)
    g.rect(6, hy + 1, 10, hy + 1, "y")       # circlet
    g.set(9, hy + 1, "C")
    g.rect(6, hy - 2, 9, hy - 2, "y")        # halo
    # round shield with a sun, strapped to the back arm
    g.rect(2, ty, 3, ty + 3, "s")
    g.rect(1, ty + 1, 4, ty + 2, "s")
    g.rect(2, ty + 1, 3, ty + 2, "y")
    g.set(2, ty + 1, "w")
    g.set(1, ty + 1, "w")
    front_hand(g, P, "w", "o")

    def mace(hx, hy_, x0, y0, x1, y1):       # head box + handle line
        g.rect(hx, hy_, hx + 1, hy_ + 1, "y")
        g.set(hx, hy_, "w")
        g.line([(x, y) for x in range(x0, x1 + 1) for y in range(y0, y1 + 1)], "S")
    if W == "rest":
        mace(13, ay - 4, 13, ay - 2, 13, ay + 1)
    elif W in ("windup", "cast"):
        mace(12, max(0, ay - 7), 12, ay - 5, 12, ay - 3)
    elif W == "strike":                      # bonk, with a star
        mace(14, ay - 1, 12, ay, 13, ay)
        g.set(15, ay - 3, "y")
        g.set(14, ay - 2, "w")
    elif W == "follow":
        mace(13, ay + 3, 13, ay + 1, 13, ay + 2)
    elif W == "ground":
        mace(14, 12, 10, 13, 13, 13)
    elif W == "stow":
        mace(11, ty + 4, 11, ty + 2, 11, ty + 3)


def cleric_face(blink=False, bob=0):
    g = Grid(8, 8)
    t = 2 + bob
    g.rect(2, t, 5, t, "w")
    g.rect(1, t + 1, 6, 6, "w")
    g.rect(1, t + 1, 6, t + 1, "y")
    g.rect(2, t + 2, 6, 6, "o")
    g.set(3, t + 3, "r" if blink else "k")
    g.set(5, t + 3, "r" if blink else "k")
    g.rect(1, 7, 6, 7, "y")
    g.rect(2, max(0, t - 2), 5, max(0, t - 2), "y")
    g.outline()
    return g.rows()


def cleric_portrait(talk):
    g = Grid(32, 32)
    g.rect(3, 4, 28, 4, "y")                 # halo
    g.rect(5, 3, 26, 3, "y")
    g.rect(1, 25, 30, 31, "w")               # robe
    g.rect(21, 25, 30, 31, "s")
    g.rect(9, 25, 11, 31, "y")               # stole
    g.rect(20, 25, 22, 31, "y")
    g.rect(14, 27, 17, 30, "y")              # sun pendant
    g.rect(15, 28, 16, 29, "w")
    g.rect(6, 6, 25, 25, "w")                # hood
    g.rect(21, 6, 25, 25, "s")
    g.rect(8, 6, 12, 8, "w")
    g.rect(9, 11, 22, 24, "o")               # face
    g.rect(20, 11, 22, 24, "r")
    g.rect(7, 9, 24, 10, "y")                # circlet
    g.rect(15, 8, 16, 10, "C")               # gem
    g.rect(12, 14, 13, 16, "k")
    g.rect(18, 14, 19, 16, "k")
    g.set(12, 14, "w")
    g.set(18, 14, "w")
    g.rect(15, 18, 16, 19, "r")
    if talk:
        g.rect(13, 21, 17, 22, "k")
    else:
        g.line([(13, 21), (14, 22), (15, 22), (16, 22), (17, 21)], "r")   # kind smile
    g.outline()
    return g.rows()


# ---------------------------------------------------------------- Bard
def bard(g, P):
    """Red feathered cap, golden hair, orange vest over a white shirt, a lute; attacks
    with a lute bonk, casts by singing up music notes."""
    hy, ty, ay, W = P["hy"], P["ty"], P["ay"], P["wpn"]
    legs(g, P, "n", "b", "d")
    g.rect(5, ty, 10, ty + 3, "w")           # shirt
    g.rect(5, ty, 6, ty + 3, "o")            # vest
    g.rect(9, ty, 10, ty + 3, "o")
    g.set(10, ty + 1, "r")
    g.set(10, ty + 2, "r")
    g.set(7, ty + 1, "s")                    # laces
    g.set(8, ty + 2, "s")
    g.rect(5, ty + 3, 10, ty + 3, "p")       # sash
    g.set(4, ty + 1, "w")                    # back sleeve
    # face and hair
    g.rect(5, hy + 2, 6, hy + 5, "y")        # golden hair
    g.rect(7, hy + 2, 10, hy + 5, "o")
    sing = P["eyes"] == "angry" or W == "cast"
    e = eye(P, "k", "r", "k")
    g.set(8, hy + 3, e)
    g.set(10, hy + 3, e)
    g.set(9, hy + 5, "k" if sing else "r")   # sings when it fights
    # cap with a long feather
    g.rect(5, hy, 10, hy + 1, "r")
    g.rect(9, hy, 10, hy + 1, "p")
    g.rect(6, hy - 1, 8, hy - 1, "r")
    g.set(11, hy + 1, "p")                   # brim
    g.set(5, hy - 1, "G")
    g.set(4, hy - 2 + P["flap"], "G")
    g.set(3, hy - 2, "g")
    front_hand(g, P, "w", "o")

    def lute(bx, by, nx, ny, ndx, ndy):      # body box at (bx, by), neck from (nx, ny)
        g.rect(bx, by, bx + 2, by + 2, "y")  # light wood, so it stands out on the vest
        g.rect(bx + 2, by, bx + 2, by + 2, "o")
        g.set(bx, by, "w")
        g.set(bx + 1, by + 1, "k")           # sound hole
        g.line([(nx + i * ndx, ny + i * ndy) for i in range(3)], "o")
        g.set(nx + 3 * ndx, ny + 3 * ndy, "r")   # pegbox
    if W == "rest":                          # held across the body
        lute(8, ay + 1, 11, ay, 1, -1)
    elif W == "windup":                      # raised to bonk
        lute(11, max(0, ay - 8), 12, ay - 5, 0, 1)
    elif W == "strike":
        lute(13, ay - 1, 12, ay, -1, 0)
        g.set(15, ay - 3, "w")
        g.set(14, ay - 2, "y")
    elif W == "follow":
        lute(13, ay + 2, 13, ay + 1, -1, -1)
    elif W == "cast":                        # strumming
        lute(8, ay + 1, 11, ay, 1, -1)
    elif W == "ground":
        lute(12, 12, 11, 13, -1, 0)
    elif W == "stow":
        lute(2, ty, 3, ty - 1, 0, -1)


def bard_face(blink=False, bob=0):
    g = Grid(8, 8)
    t = 3 + bob
    g.rect(1, t, 2, 6, "y")
    g.rect(3, t, 6, 6, "o")
    g.set(4, t + 1, "r" if blink else "k")
    g.set(6, t + 1, "r" if blink else "k")
    g.set(5, t + 3, "r")
    g.rect(1, t - 2, 6, t - 1, "r")
    g.rect(5, t - 2, 6, t - 1, "p")
    g.set(0, max(0, t - 3), "G")
    g.set(1, max(0, t - 3), "G")
    g.outline()
    return g.rows()


def bard_portrait(talk):
    g = Grid(32, 32)
    g.rect(1, 26, 30, 31, "o")               # vest
    g.rect(22, 26, 30, 31, "r")
    g.rect(11, 25, 20, 31, "w")              # shirt and laces
    for y in (27, 29):
        g.rect(14, y, 17, y, "s")
    g.rect(6, 10, 9, 24, "y")                # golden hair, curling at the ends
    g.rect(5, 18, 6, 25, "y")
    g.rect(7, 13, 7, 22, "o")
    g.rect(22, 10, 24, 20, "y")
    g.set(24, 21, "y")
    g.rect(14, 6, 15, 7, "y")                # a gold pin on the cap
    g.rect(9, 11, 22, 24, "o")               # face
    g.rect(20, 11, 22, 24, "r")
    g.rect(12, 15, 13, 17, "k")
    g.rect(18, 15, 19, 17, "k")
    g.set(12, 15, "w")
    g.set(18, 15, "w")
    g.rect(15, 18, 16, 19, "r")
    if talk:                                 # singing
        g.rect(13, 21, 18, 23, "k")
        g.rect(14, 22, 17, 23, "r")
        g.rect(27, 12, 28, 13, "y")          # a note
        g.rect(29, 8, 29, 13, "y")
    else:
        g.line([(12, 21), (13, 22), (14, 22), (15, 22), (16, 22), (17, 22), (18, 21)], "r")
    g.rect(5, 6, 25, 10, "r")                # cap
    g.rect(20, 6, 25, 10, "p")
    g.rect(8, 4, 20, 5, "r")
    g.rect(24, 10, 27, 11, "p")              # brim
    g.line([(7, 4), (6, 3), (5, 2), (4, 1), (3, 0)], "G")   # long feather
    g.line([(8, 4), (7, 3), (6, 2), (5, 1)], "G")
    g.line([(6, 4), (5, 3), (4, 2)], "g")
    g.outline()
    return g.rows()


# ---------------------------------------------------------------- animations
def frame(draw, fx=(), fx_c="C", **kw):
    g = Grid(16, 16)
    draw(g, pose(**kw))
    g.outline()
    return sparkle(g.rows(), fx, fx_c)


def anims(draw, cast_fx, cast_c):
    """The tags the game plays (ART.md §5), the same poses for every class."""
    F = lambda **k: frame(draw, **k)
    return {
        "idle 250": [F(), F(bob=1), F(), F(eyes="off")],
        "sit 900": [F(legs="sit", wpn="ground", eyes="off"),
                    F(legs="sit", wpn="ground", eyes="off", bob=1)],
        "walk 100": [F(legs="strideA", flap=1), F(legs="strideB", lift=1),
                     F(legs="strideA", bob=1, flap=1), F(legs="strideB", lift=1)],
        "attack 80": [F(arms="up", wpn="windup", eyes="angry"),
                      F(arms="fwd", wpn="strike", eyes="angry"),
                      F(arms="fwd", wpn="follow")],
        "cast 100": [F(arms="up", wpn="cast", eyes="angry", fx=cast_fx[0], fx_c=cast_c),
                     F(arms="up", wpn="cast", eyes="angry", bob=1, fx=cast_fx[1], fx_c=cast_c)],
        "open 120": [F(arms="fwd", wpn="stow", bob=1), F(arms="fwd", wpn="stow")],
        "hold 400": [F(arms="fwd", wpn="stow"), F(arms="fwd", wpn="stow", bob=1)],
        "raise_hand 300": [F(arms="raise", wpn="stow"), F(arms="raise", wpn="stow", bob=1)],
        "victory 150": [F(arms="up", wpn="cast"), F(arms="up", wpn="cast", lift=1, flap=1)],
    }


STARS = [[(1, 2), (15, 1), (14, 5)], [(2, 0), (15, 3), (0, 5)]]
NOTES = [[(14, 2), (15, 1), (1, 4), (15, 5)], [(15, 0), (14, 4), (2, 2), (0, 6)]]
SMOKE = [[(1, 12), (2, 13), (14, 13), (13, 14), (0, 9)], [(0, 11), (3, 14), (15, 12), (1, 7)]]
LEAVES = [[(1, 3), (14, 1), (15, 6)], [(2, 1), (15, 2), (0, 6)]]
CLASSES = [
    ("knight", knight, knight_face, knight_portrait, STARS, "y"),
    ("mage", mage, mage_face, mage_portrait, STARS, "C"),
    ("rogue", rogue, rogue_face, rogue_portrait, SMOKE, "s"),
    ("ranger", ranger, ranger_face, ranger_portrait, LEAVES, "G"),
    ("cleric", cleric, cleric_face, cleric_portrait, STARS, "y"),
    ("bard", bard, bard_face, bard_portrait, NOTES, "y"),
]

if __name__ == "__main__":
    for name, draw, face, portrait, fx, fx_c in CLASSES:
        write(f"hero_{name}", anims(draw, fx, fx_c))
        write(f"face_{name}", {"idle 250": [face(), face(bob=1), face(), face(blink=True)]})
        write(f"portrait_{name}", {"talk 180": [portrait(False), portrait(True)],
                                   "idle 1000": [portrait(False)]})
    print("ok")
