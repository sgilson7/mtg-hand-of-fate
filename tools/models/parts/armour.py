"""Armour pieces for the armourer's turning pedestal: four slots for the
wanderer's starter set and for each guild. Each stands alone on y = 0,
centred on x = 0, z = 0, facing +z. Sizes: a chest piece ~70 cm, a helm
~35 cm, a pair of gauntlets ~35 cm, a pair of boots ~40 cm. Round and
chunky, to sit on the hero figure (parts/hero.py)."""
import math

from common import ball, box, cyl, part, mirror

GLOW = "#6fd8ff"

# The chest: a round cuirass centred at y = 38 (y 10..66, 67 wide, 45 deep).
TY, TR, TSX, TSY, TSZ = 38, 28, 1.2, 1.0, 0.8


def fz(x, y, cy=TY, r=TR, sx=TSX, sy=TSY, sz=TSZ):
    """The front surface z of an ellipsoid at (x, y)."""
    q = 1 - (x / (r * sx)) ** 2 - ((y - cy) / (r * sy)) ** 2
    return sz * r * math.sqrt(max(q, 0))


def ring(cx, cy, cz, rad, n, c, w=3, d=3, start=0, end=1000, g=0, mesh="cube"):
    """Segments around a circle in the xy plane (a halo, a band seen face on)."""
    out = []
    for i in range(n):
        a = start + (end - start) * (i + 0.5) / n
        ang = a / 1000 * 2 * math.pi
        seg = 2 * math.pi * rad * (end - start) / 1000 / n * 1.15
        out.append(part(mesh, (cx + rad * math.cos(ang), cy + rad * math.sin(ang), cz), (w, seg, d), c, t=(0, a, 0), g=g))
    return out


def hoop(cy, rad, n, c, size=3, cz=0, sz=1, g=0, mesh="sphere"):
    """Beads around a horizontal circle (a crown band, a chain)."""
    out = []
    for i in range(n):
        ang = 2 * math.pi * i / n
        out.append(part(mesh, (rad * math.sin(ang), cy, cz + rad * sz * math.cos(ang)), (size, size, size), c, g=g))
    return out


def chain(x0, y0, z0, x1, y1, z1, n, c, r=2.2):
    """Links from one point to another, alternating flat and edge on."""
    out = []
    for i in range(n):
        f = i / max(n - 1, 1)
        x, y, z = x0 + (x1 - x0) * f, y0 + (y1 - y0) * f, z0 + (z1 - z0) * f
        s = (r * 2.4, r * 1.4, r * 1.4) if i % 2 else (r * 1.4, r * 1.4, r * 2.4)
        out.append(part("sphere", (x, y, z), s, c))
    return out


# ---------------------------------------------------------------- wanderer
W_LEATHER, W_DARK, W_CLOTH, W_CLOTH_D, W_WRAP, W_BRASS = "#7a5236", "#4e3324", "#6b7a4a", "#4f5c38", "#c9b48a", "#b39a62"


def wanderer_head():
    out = [
        part("frustum", (0, 5, 0), (44, 10, 38), W_CLOTH_D),            # the mantle round the neck
        ball(18, W_CLOTH, 0, 19, -1, 1.05, 1.0, 1.05),                   # the hood
        ball(13, W_CLOTH, 0, 27, -8, 1.0, 0.9, 1.0),                    # the hood's crown
        part("cone", (0, 30, -18), (12, 16, 12), W_CLOTH, t=(-330, 0, 0)),  # its tail falling back
        ball(12, "#2c2019", 0, 17, 9, 1.0, 1.05, 0.7),                  # the shadowed opening
        ball(13, W_CLOTH_D, 0, 17, 10, 1.08, 1.1, 0.45),                # the hood's rim
        ball(11, "#2c2019", 0, 17, 13, 0.92, 0.95, 0.4),
        ball(2.2, "#f2d27a", -4.5, 18, 17, g=500), ball(2.2, "#f2d27a", 4.5, 18, 17, g=500),  # eyes in the dark
        part("cyl", (0, 9, 1), (38, 3, 32), W_LEATHER),                  # a leather band at the neck
        ball(2.6, W_BRASS, 9, 9, 15, mesh="ico"),                        # its toggle
        ball(2.6, W_BRASS, -9, 9, 15, mesh="ico"),
        box(1.5, 8, 1.5, W_WRAP, 9, 5, 15), box(1.5, 6, 1.5, W_WRAP, -9, 6, 15),  # cords
        box(30, 1.5, 2, W_CLOTH_D, 0, 24, 6, t=(80, 0, 0)),              # a seam over the top
        box(2, 1.5, 34, W_CLOTH_D, 0, 33, -6),
    ]
    return out


def wanderer_chest():
    out = [
        part("frustum", (0, 7, 0), (62, 14, 48), W_LEATHER),             # coat tails
        box(4, 14, 4, W_DARK, 0, 7, 23),                                 # the split in front
        ball(TR, W_CLOTH, 0, TY, 0, TSX, TSY, TSZ),                      # the padded coat
        part("cyl", (0, 17, 0), (60, 7, 46), W_DARK),                    # belt
        box(9, 8, 4, W_BRASS, 0, 17, 23),                                # buckle
        box(5, 4, 5, W_DARK, 0, 17, 24),
        cyl(12, 8, W_CLOTH_D, 0, 60, -1),                                # collar
        ball(14, W_CLOTH_D, 0, 64, -1, 1.5, 0.45, 1.4),                  # the mantle over the shoulders
        box(58, 6, 4, W_LEATHER, 0, 40, 17, t=(0, 120, 0)),              # a strap across (pack)
        box(16, 18, 10, W_LEATHER, -14, 34, -22),                        # a pack on the back
        box(17, 5, 11, W_DARK, -14, 44, -22),
        ball(5, W_WRAP, -14, 48, -22, 2.8, 1, 1, mesh="cyl", t=(0, 250, 0)),  # bedroll
        box(12, 6, 6, W_DARK, 18, 22, 20, t=(0, 0, -60)),               # a belt pouch
        box(3, 3, 3, W_BRASS, 18, 22, 24),
    ]
    for y in (28, 38, 48):                                              # quilting
        out.append(part("cyl", (0, y, 0), (2 * TR * TSX * 0.98 * math.sqrt(1 - ((y - TY) / TR) ** 2) + 1, 2, 2 * TR * TSZ * math.sqrt(1 - ((y - TY) / TR) ** 2) + 1), W_CLOTH_D))
    out += mirror([
        ball(14, W_CLOTH, -35, 56, 0, 1.0, 0.8, 1.0),                   # padded shoulders
        ball(10, W_CLOTH_D, -38, 49, 0, 1.0, 0.6, 1.0),
        ball(2, W_BRASS, -8, 54, fz(-8, 54) + 1, mesh="ico"),            # coat toggles
        ball(2, W_BRASS, -6, 44, fz(-6, 44) + 1, mesh="ico"),
    ])
    return out


def wanderer_hands():
    one = [
        part("frustum", (-13, 6, 0), (18, 12, 17), W_LEATHER),           # cuff
        part("cyl", (-13, 9, 0), (19, 2, 18), W_DARK),
        part("cyl", (-13, 14, 0), (14, 4, 13), W_WRAP),                  # wraps at the wrist
        ball(9, W_LEATHER, -13, 21, 0, 1.05, 1.0, 0.72),                 # back of the hand
        part("cyl", (-13, 19, 0), (19.5, 3, 14), W_WRAP, t=(0, 50, 0)),  # wraps across the hand
        part("cyl", (-13, 24, 0), (19, 3, 13.5), W_WRAP, t=(0, -40, 0)),
        ball(3.5, W_LEATHER, -4, 21, 3, 1, 1.4, 1),                      # thumb
    ]
    for i, fx in enumerate((-18.5, -15, -11.3, -7.8)):
        h = 9 - abs(i - 1.5) * 1.6
        one.append(part("cyl", (fx, 27 + h / 2, 0), (3.6, h, 4), W_DARK))
        one.append(ball(2, "skin", fx, 27 + h + 0.5, 0))                 # fingertips out of the gloves
    return mirror(one)


def wanderer_feet():
    one = [
        box(15, 3, 27, W_DARK, -11, 1.5, 4),                             # sole
        ball(8.5, W_LEATHER, -11, 8, 7, 0.9, 0.75, 1.55),                # foot
        ball(5, W_LEATHER, -11, 6, 15, 1.3, 0.9, 1.0),                   # toe
        cyl(7, 27, W_LEATHER, -11, 5, 0),                                # shaft
        part("frustum", (-11, 34, 0), (18, 8, 18), W_DARK, t=(500, 0, 0)),  # folded cuff
        part("cyl", (-11, 37.5, 0), (18, 2, 18), W_DARK),
        part("cyl", (-11, 15, 0), (15.5, 2.5, 15.5), W_WRAP),            # lace wraps
        part("cyl", (-11, 22, 0), (15.5, 2.5, 15.5), W_WRAP, t=(0, 40, 0)),
        box(6, 3, 3, W_BRASS, -11, 15, 8),                               # buckle
        ball(3, W_CLOTH_D, -4, 26, 2, 1, 2.2, 1),                        # a patch
    ]
    return mirror(one)


# ---------------------------------------------------------------- azorius
A_WHITE, A_SHADE, A_GOLD, A_BLUE, A_BLUE_D, A_GEM = "#f4f1ea", "#cfd3dc", "gold", "#2f62b8", "#1f3f7a", "#7fd0ff"


def azorius_head():
    out = [
        cyl(16, 6, A_GOLD, 0, 0, 0),                                     # gorget rim
        ball(15, A_WHITE, 0, 16, 0, 1.0, 1.25, 1.05),                    # tall helm
        part("cyl", (0, 9, 0), (33, 3, 34), A_GOLD),                     # gold band
        ball(15.3, A_BLUE_D, 0, 18, 1.5, 1, 0.09, 1.05),                 # T visor
        box(3, 11, 3, A_BLUE_D, 0, 12.5, 15),
        box(2, 22, 5, A_GOLD, 0, 22, 9, t=(-60, 0, 0)),                  # a ridge up the brow
        box(3, 6, 30, A_GOLD, 0, 34, -2),                                # crest ridge over the top
        part("gem", (0, 37, -3), (3, 10, 30), A_BLUE),
        ball(2.6, A_GEM, 0, 29, 15, mesh="gem", g=400),                  # brow gem
        cyl(2, 10, A_GOLD, 0, 24, -15),                                  # the halo's stem
    ]
    out += ring(0, 30, -16, 11, 14, A_GOLD, w=3.2, d=3)                  # the halo
    out += ring(0, 30, -16.5, 7.5, 8, "#ffe48a", w=1.5, d=1.5, start=50, end=450, g=300)  # its rays
    out += mirror([
        ball(5, A_WHITE, -14, 13, 2, 0.6, 1.2, 1.2),                     # cheek guards
        box(2, 10, 9, A_GOLD, -16.5, 13, 2),
        part("cone", (-16.5, 28, -6), (4, 11, 2), A_GOLD, t=(0, 120, 0)),  # small gold wings
    ])
    return out


def azorius_chest():
    out = [
        part("frustum", (0, 7, 0), (60, 14, 46), A_WHITE),               # tassets
        part("cyl", (0, 1, 0), (61, 2, 47), A_GOLD),
        box(22, 30, 3, A_BLUE, 0, 15, 21, t=(-30, 0, 0)),                # tabard
        box(23, 3, 4, A_GOLD, 0, 1.5, 22.5),
        box(3, 24, 4, A_GOLD, 0, 15, 22.5, t=(-30, 0, 0)),
        ball(TR, A_WHITE, 0, TY, 0, TSX, TSY, TSZ),                      # the enamel plate
        part("cyl", (0, 17, 0), (58, 5, 44), A_GOLD),                    # gold belt
        box(2.6, 38, 4, A_GOLD, 0, 40, fz(0, 40) - 1.2),                 # centre ridge
        ball(4, A_GEM, 0, 42, fz(0, 42) + 1, mesh="gem", g=400),         # the blue gem
        ball(6, A_GOLD, 0, 42, fz(0, 42) - 0.5, 1, 0.5, 1, mesh="cyl", t=(250, 0, 0)),
        cyl(13, 10, A_WHITE, 0, 58, -1),                                  # high collar
        part("cyl", (0, 68, -1), (27, 2, 27), A_GOLD),
        ball(22, A_BLUE, 0, 32, -14, 1.05, 1.15, 0.5),                   # a blue cape on the back
        part("cyl", (0, 56, -12), (40, 3, 26), A_GOLD),
    ]
    out += mirror([
        ball(16, A_WHITE, -36, 57, 0, 1.0, 0.8, 1.05),                   # pauldrons
        part("cyl", (-37, 51, 0), (32, 2.5, 34), A_GOLD),
        ball(12, A_WHITE, -40, 47, 0, 1.0, 0.6, 1.0),                    # lower lame
        part("gem", (-41, 64, 0), (5, 14, 28), A_WHITE, t=(0, -40, 0)),  # raised guards
        part("gem", (-41.5, 64, 0), (3, 16, 18), A_GOLD, t=(0, -40, 0)),
        ball(3, A_GEM, -36, 58, 16, mesh="gem", g=400),                  # pauldron gems
        box(2.5, 30, 3, A_GOLD, -17, 40, fz(-17, 40) - 1.2, t=(0, 40, 0)),  # trim lines
        ball(9, A_SHADE, -15, 46, fz(-15, 46) - 7, 1, 0.8, 0.8),         # chest plates
    ])
    return out


def azorius_hands():
    one = [
        part("frustum", (-13, 7, 0), (20, 14, 19), A_WHITE),            # flared cuff
        part("cyl", (-13, 1, 0), (21, 2, 20), A_GOLD),
        part("cyl", (-13, 13.5, 0), (14, 2, 13), A_GOLD),
        part("cyl", (-13, 9, 0), (19, 3, 18), A_BLUE),                  # a blue cloth band
        ball(9, A_WHITE, -13, 21, 0, 1.05, 1.0, 0.72),                   # back plate
        ball(3, A_GEM, -13, 21, 6.5, mesh="gem", g=400),                 # gem on the back of the hand
        ball(3.6, A_GOLD, -13, 21, 5.8, 1, 0.4, 1, mesh="cyl", t=(250, 0, 0)),
        ball(3.5, A_WHITE, -4, 21, 3, 1, 1.4, 1),                        # thumb
    ]
    for i, fx in enumerate((-18.5, -15, -11.3, -7.8)):
        h = 10 - abs(i - 1.5) * 1.6
        one.append(part("cyl", (fx, 27 + h / 2, 0), (3.6, h, 4), A_WHITE))
        one.append(part("cyl", (fx, 28.5, 0), (3.9, 1.4, 4.3), A_GOLD))
        one.append(part("cone", (fx, 27 + h + 1.2, 0), (3.6, 3, 4), A_WHITE))
    return mirror(one)


def azorius_feet():
    one = [
        box(15, 3, 28, A_GOLD, -11, 1.5, 4),                             # gold sole
        ball(8.5, A_WHITE, -11, 8, 7, 0.9, 0.75, 1.6),                   # sabaton
        part("cone", (-11, 6, 18), (10, 10, 7), A_WHITE, t=(250, 0, 0)),  # pointed toe
        box(2, 3, 14, A_GOLD, -11, 12, 9, t=(-60, 0, 0)),
        part("frustum", (-11, 21, 0), (16, 30, 16), A_WHITE, t=(500, 0, 0)),  # tall greave, wider up
        box(3, 28, 3, A_GOLD, -11, 22, 7.5, t=(-30, 0, 0)),              # gold edge
        part("cyl", (-11, 36.5, 0), (18, 2, 18), A_GOLD),
        ball(6, A_WHITE, -11, 40, 4, 1.1, 0.9, 0.8),                     # knee cop
        ball(2, A_GEM, -11, 40, 9, mesh="gem", g=400),
        box(13, 6, 2, A_BLUE, -11, 33, -8.5),                            # blue cloth at the back
    ]
    return mirror(one)


# ---------------------------------------------------------------- izzet
I_BRASS, I_COPPER, I_COPPER_D, I_IRON, I_RED, I_LENS = "#d1a040", "#c06a38", "#7d3f22", "#3d3a40", "#b3302a", "#9fe8ff"


def coil(x, y, z, r, n, step=2.4, c=I_COPPER, core=True):
    """A stacked coil with a glowing core."""
    out = [cyl(r * 0.55, n * step, GLOW, x, y, z)] if core else []
    out[0:0] = []
    if core:
        out[0]["g"] = 900
    for i in range(n):
        out.append(part("cyl", (x, y + step * (i + 0.5), z), (2 * r, step * 0.6, 2 * r), c))
    return out


def izzet_head():
    out = [
        ball(16, I_BRASS, 0, 16, -1, 1.05, 1.0, 1.05),                    # brass cap
        part("cyl", (0, 8, 0), (35, 4, 35), I_COPPER),                   # rim
        box(34, 4, 6, I_IRON, 0, 18, 14),                                # goggle strap front
        part("cyl", (-7, 18, 17), (12, 8, 12), I_COPPER, t=(250, 0, 0)),  # left goggle, big
        part("cyl", (-7, 18, 21.5), (9, 1, 9), I_LENS, t=(250, 0, 0), g=800),
        part("cyl", (7, 18, 16.5), (9, 6, 9), I_BRASS, t=(250, 0, 0)),   # right goggle, small
        part("cyl", (7, 18, 20), (6.5, 1, 6.5), I_LENS, t=(250, 0, 0), g=800),
        part("cyl", (7, 18, 21), (3, 3, 3), I_IRON, t=(250, 0, 0)),      # a loupe on it
        box(4, 3, 4, I_IRON, 0, 18, 18),                                 # bridge
        part("cyl", (17, 17, 0), (10, 5, 10), I_COPPER, t=(0, 250, 0)),  # ear cup
        part("cyl", (19.5, 17, 0), (6, 2, 6), GLOW, t=(0, 250, 0), g=900),
        cyl(1.2, 14, I_IRON, -10, 26, -6),                               # antenna
        ball(2.6, GLOW, -10, 41, -6, g=900),
        box(2, 3, 18, I_IRON, 16, 22, -8, t=(0, 0, 100)),                # a pipe round the back
        part("cyl", (-1, 26, -12), (4, 18, 4), I_COPPER, t=(250, 250, 0)),
        part("cyl", (0, 28, 2), (14, 2, 14), I_COPPER),                  # crown rings
        ball(5, I_BRASS, 0, 31, 0, 1, 0.6, 1),
    ]
    out += coil(-17, 10, 2, 3.8, 5)                                      # a coil over one ear
    out[-6]["p"][1] = 16
    return out


def izzet_chest():
    out = [
        part("frustum", (0, 7, 0), (60, 14, 46), I_COPPER_D),            # leather skirt
        box(16, 15, 4, I_RED, 9, 7, 22),                                  # a red apron flap, off centre
        ball(TR, I_COPPER, 0, TY, 0, TSX, TSY, TSZ),                     # copper plate
        part("cyl", (0, 17, 0), (58, 6, 44), I_IRON),                    # belt
        box(8, 7, 4, I_BRASS, -12, 17, 21),
        ball(12, I_BRASS, -8, 44, fz(-8, 44) - 8, 1, 1.05, 0.75),        # brass plate over one side
        part("cyl", (0, 38, fz(0, 38) + 1), (16, 4, 16), I_BRASS, t=(250, 0, 0)),  # the reactor housing
        part("cyl", (0, 38, fz(0, 38) + 3.5), (10, 2, 10), GLOW, t=(250, 0, 0), g=900),
        ball(3.5, "#e6fbff", 0, 38, fz(0, 38) + 5, g=1000),
        box(2, 18, 2, GLOW, 14, 32, fz(14, 32) + 0.5, t=(0, 60, 0)),      # glowing conduits from the core
        cyl(12, 9, I_IRON, 0, 59, -1),                                    # collar
        part("cyl", (0, 66, -1), (26, 2, 26), I_BRASS),
        # mismatched shoulders: a big brass dome on the left
        ball(17, I_BRASS, -36, 56, 0, 1.0, 0.85, 1.05),
        part("cyl", (-36, 50, 0), (36, 3, 36), I_COPPER),
        ball(3, I_IRON, -36, 70, 0, 1, 1, 1, mesh="cyl"),
        ball(2.5, GLOW, -36, 72, 0, g=900),
        # and on the right, stacked copper rings round a glowing coil
        ball(9, I_COPPER_D, 34, 58, 0, 1.2, 0.8, 1.2),
        # two tanks on the back, piped to the shoulder
        cyl(7, 30, I_BRASS, -11, 26, -24), cyl(6, 24, I_COPPER, 10, 28, -24),
        ball(7, I_BRASS, -11, 56, -24, 1, 0.6, 1), ball(6, I_COPPER, 10, 52, -24, 1, 0.6, 1),
        part("cyl", (-24, 60, -18), (3, 22, 3), I_IRON, t=(0, -220, 0)),
        part("cyl", (6, 50, -32), (2, 8, 2), GLOW, g=900),
    ]
    out += coil(34, 56, 0, 9, 5, 3)
    out += ring(0, 38, fz(0, 38) + 2.5, 8, 8, I_COPPER_D, w=2, d=2)
    out.append(box(58, 5, 3, I_RED, 0, 40, 17, t=(0, -130, 0)))           # a red sash across
    return out


def izzet_hands():
    # left: a big brass gauntlet with copper finger caps
    left = [
        part("frustum", (-14, 7, 0), (22, 14, 21), I_BRASS),
        part("cyl", (-14, 13, 0), (16, 3, 15), GLOW, g=900),             # glowing wrist ring
        ball(10, I_BRASS, -14, 21, 0, 1.05, 1.0, 0.75),
        box(2, 10, 2, GLOW, -17, 21, 7.5, t=(0, 40, 0), mesh="cube"),     # conduits over the hand
        box(2, 10, 2, GLOW, -11, 21, 7.5, t=(0, -40, 0)),
        ball(3, GLOW, -14, 21, 8, g=900),
        ball(4, I_COPPER, -4, 21, 3, 1, 1.4, 1),
        part("cyl", (-14, 3, 10), (6, 6, 6), I_COPPER, t=(250, 0, 0)),     # a dial
    ]
    for i, fx in enumerate((-20, -16, -12, -8)):
        h = 9 - abs(i - 1.5) * 1.6
        left.append(part("cyl", (fx, 28 + h / 2, 0), (3.8, h, 4), I_COPPER))
        left.append(ball(1.9, GLOW, fx, 28 + h + 0.8, 0, g=900))          # glowing tips
    # right: a copper gauntlet with a coil on the wrist and rods for fingers
    right = [
        part("cyl", (13, 6, 0), (17, 12, 16), I_COPPER),
        ball(8.5, I_COPPER, 13, 21, 0, 1.05, 1.0, 0.72),
        box(12, 8, 3, I_IRON, 13, 21, 6),                                 # a plate bolted on
        box(9, 2, 1.6, GLOW, 13, 22.5, 7.8),
        box(9, 2, 1.6, GLOW, 13, 19.5, 7.8),
        ball(3.4, I_BRASS, 4.5, 21, 3, 1, 1.4, 1),
        part("cyl", (13, 20, -7), (3, 22, 3), GLOW, g=900),               # a conduit up the back
    ]
    for i, fx in enumerate((18, 14.6, 11.2, 7.8)):
        h = 9 - abs(i - 1.5) * 1.6
        right.append(part("cyl", (fx, 27 + h / 2, 0), (2.8, h, 3.2), I_IRON))
        right.append(part("cyl", (fx, 28, 0), (3.6, 2, 3.6), I_BRASS))
    right += coil(13, 11, 0, 9.5, 2, 2.6, I_BRASS, core=False)
    return left + right


def izzet_feet():
    # left: brass boot with a coil at the ankle
    left = [
        box(15, 3, 27, I_IRON, -11, 1.5, 4),
        ball(8.5, I_BRASS, -11, 8, 7, 0.9, 0.75, 1.55),
        ball(5, I_COPPER, -11, 7, 15, 1.3, 1.0, 1.0),
        cyl(7, 28, I_COPPER_D, -11, 6, 0),
        part("cyl", (-11, 34, 0), (17, 5, 17), I_BRASS),
        box(10, 22, 3, I_BRASS, -11, 24, 7),                              # shin plate
        box(2, 18, 1.6, GLOW, -11, 24, 8.8),
    ] + coil(-11, 10, 0, 9, 3, 2.4, I_COPPER, core=False) + [
        part("cyl", (-11, 13, 0), (15, 6, 15), GLOW, g=900),
    ]
    # right: copper boot on a piston heel
    right = [
        box(15, 3, 27, I_IRON, 11, 4.5, 4),
        cyl(3, 4, I_BRASS, 11, 0, -5), cyl(1.6, 4, GLOW, 11, 0, 4),     # piston and a glow underneath
        ball(8.5, I_COPPER, 11, 11, 7, 0.9, 0.75, 1.55),
        cyl(7.5, 28, I_COPPER, 11, 9, 0),
        part("cyl", (11, 34, 0), (18, 3, 18), I_IRON),
        part("cyl", (11, 25, 0), (17, 3, 17), I_BRASS),
        part("cyl", (11, 18, 0), (17, 3, 17), I_BRASS),
        part("cyl", (18, 30, 0), (5, 6, 5), I_BRASS, t=(0, 250, 0)),      # a gauge
        part("cyl", (20.5, 30, 0), (3.5, 1, 3.5), GLOW, t=(0, 250, 0), g=900),
        part("cyl", (11, 22, -8), (2, 22, 2), GLOW, g=900),
    ]
    return left + right


# ---------------------------------------------------------------- rakdos
R_BLACK, R_BLACK_L, R_RED, R_RED_D, R_SPIKE, R_IRON, R_BONE = "#2a2228", "#3e3238", "#c02a2a", "#701822", "#c9c2b2", "#8a8590", "#efe6d0"


def rakdos_head():
    out = [
        cyl(15, 5, R_RED_D, 0, 0, 0),                                    # a leather collar
        ball(16, R_BLACK, 0, 17, -1, 1.0, 1.05, 1.05),                   # black hood
        ball(12, R_RED, 0, 15, 8, 1.05, 1.15, 0.75),                     # red demon mask
        ball(3, R_BLACK, -5, 18, 16.5, 1.4, 0.8, 0.6),                   # eye holes
        ball(3, R_BLACK, 5, 18, 16.5, 1.4, 0.8, 0.6),
        ball(1.4, "#ffcf5a", -5, 18, 17.5, g=800), ball(1.4, "#ffcf5a", 5, 18, 17.5, g=800),
        box(11, 3, 3, R_BLACK, 0, 9.5, 16.5),                            # a grin
        box(1, 2, 1, R_BONE, 0, 9.5, 18.2),
        box(9, 1.5, 3, R_RED_D, 0, 23, 15, t=(-80, 0, 0)),               # brow
    ]
    out += [box(1.6, 2.4, 1.2, R_BONE, x, 9.6, 18) for x in (-4, -2, 2, 4)]  # teeth
    out += [part("cone", (0, 32, z), (4, 8, 4), R_SPIKE, t=(-z * 8, 0, 0)) for z in (-10, -3, 4)]  # crest spikes
    out += mirror([
        part("frustum", (-16, 27, 2), (8, 11, 8), R_BONE, t=(0, 150, 0)),  # horns sweeping out then up
        part("frustum", (-21.5, 33.5, 2), (5.4, 10, 5.4), R_BONE, t=(0, 50, 0)),
        part("cone", (-21.7, 41.5, 2), (3.6, 9, 3.6), R_BONE, t=(0, -60, 0)),
        ball(4.5, R_BLACK_L, -13, 25, 2, 1, 0.8, 1),                     # horn bases
        part("cone", (-15, 5, 8), (3, 7, 3), R_SPIKE, t=(0, 380, 0)),     # collar spikes
        box(2, 2, 8, R_SPIKE, -16, 15, 2),                               # studs
        ball(1.5, R_SPIKE, -11, 16, 13, mesh="ico"),
    ])
    out += chain(-8, 6, 13, 8, 6, 13, 7, R_IRON, 1.6)                     # a chain under the jaw
    return out


def rakdos_chest():
    out = [
        part("frustum", (0, 7, 0), (60, 14, 46), R_BLACK_L),
        ball(TR, R_BLACK, 0, TY, 0, TSX, TSY, TSZ),                      # black leather
        part("cyl", (0, 17, 0), (58, 6, 44), R_RED_D),                   # belt
        box(8, 8, 4, R_IRON, 0, 17, 22),
        box(4, 4, 5, R_BLACK, 0, 17, 22.5),
        box(60, 6, 4, R_RED, 0, 40, 18, t=(0, 140, 0)),                  # crossed straps
        box(60, 6, 4, R_RED, 0, 40, 18.5, t=(0, -140, 0)),
        box(7, 7, 4, R_IRON, 0, 40, 21.5, t=(0, 125, 0)),                 # where they cross
        cyl(11, 8, R_RED_D, 0, 60, -1),                                   # collar
        ball(21, R_RED_D, 0, 36, -12, 1.05, 1.05, 0.5),                  # red back panel
    ]
    for i, (x, h) in enumerate(((-24, 14), (-15, 18), (-6, 12), (4, 20), (13, 13), (22, 17))):
        out.append(box(8, h, 2.5, R_RED if i % 2 else R_RED_D, x, 14 - h / 2, 24 - abs(x) * 0.15, t=(0, 0, (i % 3 - 1) * 15)))  # tattered strips
    out += chain(-30, 26, 14, 26, 20, 18, 12, R_IRON)                    # a chain slung across
    out += mirror([
        ball(16, R_BLACK_L, -36, 56, 0, 1.0, 0.8, 1.05),                 # pauldrons
        part("cyl", (-37, 51, 0), (32, 2.5, 34), R_RED),
        part("cone", (-38, 66, 0), (6, 11, 6), R_SPIKE, t=(0, 60, 0)),    # spikes
        part("cone", (-46, 62, 0), (5, 13, 5), R_SPIKE, t=(0, 200, 0)),
        part("cone", (-38, 63, 11), (5, 12, 5), R_SPIKE, t=(100, 100, 0)),
        part("cone", (-38, 63, -11), (5, 12, 5), R_SPIKE, t=(-100, 100, 0)),
        part("cone", (-14, 52, fz(-14, 52) + 2), (3.5, 7, 3.5), R_SPIKE, t=(220, 0, 0)),  # chest studs
        part("cone", (-20, 30, fz(-20, 30) + 2), (3.5, 7, 3.5), R_SPIKE, t=(220, 0, 0)),
    ])
    return out


def rakdos_hands():
    one = [
        part("frustum", (-13, 7, 0), (20, 14, 19), R_RED_D),
        part("cyl", (-13, 2, 0), (20.5, 2.5, 19.5), R_BLACK),
        part("cone", (-22, 9, 0), (4, 9, 4), R_SPIKE, t=(0, 250, 0)),      # cuff spike
        ball(9, R_BLACK, -13, 21, 0, 1.05, 1.0, 0.72),
        ball(3.5, R_BLACK, -4, 21, 3, 1, 1.4, 1),
        box(16, 3, 1.5, R_RED, -13, 17, 6.2),                            # a red strap
    ]
    one += chain(-20, 13, 6, -6, 13, 6, 6, R_IRON, 1.5)                   # a chain round the wrist
    for i, fx in enumerate((-18.5, -15, -11.3, -7.8)):
        h = 9 - abs(i - 1.5) * 1.6
        one.append(part("cyl", (fx, 27 + h / 2, 0), (3.6, h, 4), R_BLACK))
        one.append(part("cone", (fx, 27 + h + 1.8, 0), (2.6, 4, 3), R_SPIKE))  # claws
        one.append(part("cone", (fx, 26, 5), (2.6, 6, 2.6), R_SPIKE, t=(200, 0, 0)))  # knuckle spikes
    return mirror(one)


def rakdos_feet():
    one = [
        box(15, 3, 27, R_BLACK_L, -11, 1.5, 4),
        ball(8.5, R_BLACK, -11, 8, 7, 0.9, 0.75, 1.55),
        part("cone", (-11, 6, 19), (6, 9, 5), R_SPIKE, t=(250, 0, 0)),     # toe spike
        part("cone", (-11, 6, -12), (4, 8, 4), R_SPIKE, t=(-250, 0, 0)),   # heel spike
        cyl(7, 27, R_BLACK, -11, 6, 0),
        part("frustum", (-11, 34, 0), (18, 8, 18), R_RED_D, t=(500, 0, 0)),
        part("cyl", (-11, 13, 0), (15.5, 2.5, 15.5), R_RED),             # straps
        part("cyl", (-11, 20, 0), (15.5, 2.5, 15.5), R_RED),
        part("cyl", (-11, 27, 0), (15.5, 2.5, 15.5), R_RED),
        box(4, 4, 3, R_IRON, -11, 20, 8), box(4, 4, 3, R_IRON, -11, 13, 8),  # buckles
        part("cone", (-11, 40, 6), (5, 9, 5), R_SPIKE, t=(60, 0, 0)),       # knee spike
        ball(5, R_BLACK_L, -11, 36, 6, 1, 1, 0.7),
    ]
    one += chain(-4, 30, 3, -4, 14, 3, 6, R_IRON, 1.4)
    return mirror(one)


# ---------------------------------------------------------------- golgari
G_SHELL, G_SHELL_D, G_SHELL_L, G_BROWN, G_BONE, G_CAP, G_CAP2, G_SPORE = "#3f5e2c", "#26381c", "#5f7d3a", "#5a4028", "#d8ccaa", "#b46a3a", "#8a5c8f", "#c8f07a"


def cap(x, y, z, r, c, stem=G_BONE, tilt=(0, 0, 0)):
    """A mushroom: a stem and a squashed cap."""
    return [cyl(r * 0.3, r * 0.9, stem, x, y, z, t=tilt), ball(r, c, x, y + r * 0.9, z, 1, 0.5, 1, t=tilt)]


def golgari_head():
    out = [
        cyl(15, 5, G_BROWN, 0, 0, 0),
        ball(16, G_SHELL_D, 0, 16, -1, 1.05, 1.0, 1.1),                  # under-shell
        ball(16, G_SHELL, 0, 20, -3, 1.0, 0.85, 1.15),                   # carapace, in plates
        ball(13, G_SHELL_L, 0, 26, -5, 0.9, 0.7, 1.15),
        box(2.5, 6, 32, G_SHELL_D, 0, 29, -4),                           # the seam down the shell
        ball(10, G_BROWN, 0, 13, 9, 1.05, 0.9, 0.75),                    # face under the shell
        box(14, 3, 4, "#151a10", 0, 15, 15),                             # dark eye slit
        ball(1.6, G_SPORE, -4, 15, 17, g=700), ball(1.6, G_SPORE, 4, 15, 17, g=700),
    ]
    out += cap(-11, 30, -4, 5, G_CAP) + cap(-6, 32, -11, 3.5, G_CAP2) + cap(10, 28, -12, 3, G_CAP)
    out += mirror([
        ball(5, "#1b2b16", -13, 22, 9, 1, 1.2, 1),                       # compound eyes
        ball(2, "#86b06a", -14, 24, 12),
        part("cone", (-8, 6, 15), (5.5, 14, 5.5), G_BONE, t=(170, -100, 0)),  # mandibles
        part("cone", (-4.5, 3.5, 21), (3, 8, 3), G_BONE, t=(250, -250, 0)),
        ball(6, G_SHELL_L, -14, 12, 0, 0.6, 1.1, 1.2),                   # cheek plates
        part("cone", (-6, 34, 6), (3, 9, 3), G_BONE, t=(80, 80, 0)),      # little antennae horns
    ])
    return out


def golgari_chest():
    out = [
        part("frustum", (0, 7, 0), (60, 14, 46), G_BROWN),
        ball(TR, G_SHELL_D, 0, TY, 0, TSX, TSY, TSZ),
        part("cyl", (0, 17, 0), (58, 6, 44), G_BROWN),
        cyl(12, 8, G_SHELL_D, 0, 59, -1),
        ball(7, G_BONE, 0, 17, 21, 1, 1, 0.7),                           # a skull at the belt
        ball(1.8, "#151a10", -2.5, 18, 25), ball(1.8, "#151a10", 2.5, 18, 25),
        ball(14, G_SHELL, 0, 34, -24, 1.6, 2.0, 0.5),                    # the back shell
        box(2.5, 50, 3, G_SHELL_D, 0, 36, -32),
    ]
    for y, w, c in ((52, 46, G_SHELL), (42, 52, G_SHELL_L), (31, 48, G_SHELL)):  # overlapping front plates
        out.append(ball(w / 2, c, 0, y, fz(0, y) - 8, 1, 0.3, 0.45))
    for y in (25, 35, 45):                                              # bone ribs at the sides
        out += mirror([part("cyl", (-21, y, fz(-21, y) - 1), (3, 14, 3), G_BONE, t=(0, 230, 0))])
    out += mirror([
        ball(16, G_SHELL, -36, 56, 0, 1.0, 0.78, 1.1),                   # beetle-shell pauldrons
        box(2.5, 5, 30, G_SHELL_D, -36, 67, 0),
        ball(11, G_SHELL_L, -40, 47, 0, 1.0, 0.6, 1.0),
        part("cone", (-46, 50, 8), (4, 12, 4), G_BONE, t=(80, 300, 0)),   # bone hooks
    ])
    out += cap(-36, 66, -6, 6, G_CAP) + cap(-28, 64, -10, 3.5, G_CAP2) + cap(37, 66, 6, 4, G_CAP2)
    out += cap(36, 62, -10, 3, G_CAP) + cap(14, 22, 20, 3.2, G_CAP)
    return out


def golgari_hands():
    one = [
        part("frustum", (-13, 7, 0), (20, 14, 19), G_BROWN),
        part("cyl", (-13, 11, 0), (19, 3, 18), G_SHELL_D),
        ball(9, G_SHELL_D, -13, 21, 0, 1.05, 1.0, 0.72),
        ball(8, G_SHELL, -13, 22, 2.5, 1.0, 1.1, 0.5),                  # shell plate on the back
        box(1.6, 13, 2, G_SHELL_D, -13, 22, 6.5),
        ball(3.5, G_SHELL_D, -4, 21, 3, 1, 1.4, 1),
        part("cone", (-3, 26, 3), (2.6, 5, 2.6), G_BONE, t=(0, -80, 0)),
    ]
    for i, fx in enumerate((-18.5, -15, -11.3, -7.8)):
        h = 8 - abs(i - 1.5) * 1.6
        one.append(part("cyl", (fx, 27 + h / 2, 0), (3.6, h, 4), G_SHELL_L))
        one.append(part("cone", (fx, 27 + h + 2.5, 1), (3, 6, 3), G_BONE, t=(60, 0, 0)))  # bone claws
    one += cap(-19, 13, 2, 3.4, G_CAP)
    out = mirror(one)
    out += cap(15, 12, -6, 2.5, G_CAP2)
    return out


def golgari_feet():
    one = [
        box(15, 3, 27, G_BROWN, -11, 1.5, 4),
        ball(8.5, G_SHELL_D, -11, 8, 7, 0.9, 0.75, 1.55),
        ball(6, G_SHELL, -11, 10, 9, 1, 0.6, 1.4),
        cyl(7, 28, G_SHELL_D, -11, 6, 0),
        part("frustum", (-11, 34, 0), (19, 7, 19), G_BROWN, t=(500, 0, 0)),
        ball(6, G_SHELL_L, -11, 38, 5, 1.1, 0.9, 0.7),                   # knee shell
    ]
    for y in (14, 20, 26):                                              # shell segments on the shin
        one.append(ball(8.2, G_SHELL, -11, y, 2, 1, 0.42, 1))
    for dx in (-4, 0, 4):                                               # bone claws for toes
        one.append(part("cone", (-11 + dx, 3, 19), (3, 7, 3), G_BONE, t=(250, 0, 0)))
    one += cap(-4, 33, -4, 3.5, G_CAP)
    out = mirror(one)
    out += cap(-15, 37, -4, 4, G_CAP2) + cap(17, 9, -8, 2.6, G_CAP2)
    return out


# ---------------------------------------------------------------- selesnya
S_WHITE, S_GREEN, S_GREEN_D, S_LEAF, S_GOLD, S_WOOD = "#f6f3e8", "#3c8d48", "#28663a", "#7cc464", "gold", "#8a5a35"


def leaf(x, y, z, length, c=S_LEAF, t=(0, 0, 0), w=None):
    """A flat leaf (a squashed octahedron), long along y before tilting."""
    return part("gem", (x, y, z), (w or length * 0.5, length, length * 0.16), c, t=t)


def tree(x, y, z, size, c=S_GOLD):
    """The Selesnya tree in gold: trunk, two branches, a round canopy."""
    k = size / 10
    return [
        box(1.6 * k, 6 * k, 1.5, c, x, y + 3 * k, z),
        box(1.2 * k, 4 * k, 1.5, c, x - 2 * k, y + 6 * k, z, t=(0, 120, 0)),
        box(1.2 * k, 4 * k, 1.5, c, x + 2 * k, y + 6 * k, z, t=(0, -120, 0)),
        ball(4 * k, c, x, y + 8.5 * k, z, 1.2, 0.8, 0.25),
    ]


def selesnya_head():
    out = [
        cyl(15, 5, S_GREEN_D, 0, 0, 0),
        ball(15.5, S_WHITE, 0, 15, -1, 1.0, 1.0, 1.05),                  # rounded white helm
        ball(10, "#2a3a2c", 0, 13, 9, 1.0, 0.9, 0.7),                    # open face, shadowed
        ball(11, S_WHITE, 0, 13, 11, 1.05, 0.95, 0.45),
        ball(9, "#2a3a2c", 0, 13, 13, 0.95, 0.85, 0.4),
        part("cyl", (0, 21, -1), (33, 4, 34), S_GOLD),                   # gold circlet
        ball(3, S_LEAF, 0, 23, 17, mesh="gem"),
        part("cyl", (0, 4, 0), (32, 3, 32), S_GOLD),                     # gold band at the neck
    ]
    out += tree(0, 22, 17, 6)
    for i in range(9):                                                  # the leafy crown
        a = 2 * math.pi * (i + 0.5) / 9
        x, z = 15 * math.sin(a), 15 * math.cos(a) - 1
        yaw = round(i * 1000 / 9 + 1000 / 18)
        lc = S_LEAF if i % 2 else S_GREEN
        out.append(leaf(x, 28 + (i % 2) * 2, z, 13 + (i % 2) * 3, lc, t=(60, 0, yaw)))
    for i in range(9):
        a = 2 * math.pi * i / 9
        out.append(leaf(12 * math.sin(a), 25, 12 * math.cos(a) - 1, 9, S_GREEN_D, t=(20, 0, round(i * 1000 / 9))))
    out += mirror([
        leaf(-15, 13, -2, 12, S_LEAF, t=(0, 250, 120)),                   # leaves over the ears
        leaf(-15, 9, -5, 9, S_GREEN, t=(0, 200, 120)),
    ])
    return out


def selesnya_chest():
    out = [
        part("frustum", (0, 7, 0), (60, 14, 46), S_GREEN),               # green skirt
        ball(TR, S_WHITE, 0, TY, 0, TSX, TSY, TSZ),                      # rounded white plate
        part("cyl", (0, 17, 0), (58, 5, 44), S_GOLD),
        cyl(12, 8, S_GREEN, 0, 59, -1),
        part("cyl", (0, 66, -1), (25, 2, 25), S_GOLD),
        ball(13, S_GOLD, 0, 44, fz(0, 44) - 0.5, 1, 0.1, 1, mesh="cyl", t=(250, 0, 0)),  # gold disc
        ball(11.5, S_GREEN, 0, 44, fz(0, 44) + 0.4, 1, 0.1, 1, mesh="cyl", t=(250, 0, 0)),
        ball(22, S_GREEN, 0, 32, -14, 1.05, 1.15, 0.5),                  # a green cloak at the back
    ]
    out += tree(0, 35.5, fz(0, 44) + 2.2, 14)                             # the tree emblem
    for i, x in enumerate((-22, -13, -4, 5, 14, 23)):                   # leaf tassets
        out.append(leaf(x * 1.05, 6, 24 - abs(x) * 0.25, 16, S_LEAF if i % 2 else S_GREEN_D, t=(-40, 0, round(x * 4)), w=9))
    out += mirror([
        ball(15, S_WHITE, -35, 56, 0, 1.0, 0.8, 1.05),                   # round pauldrons
        part("cyl", (-36, 51, 0), (30, 2.5, 32), S_GOLD),
        leaf(-43, 50, 8, 18, S_LEAF, t=(0, 380, 60), w=10),              # leaves falling over the arm
        leaf(-43, 50, -8, 18, S_GREEN, t=(0, 380, -60), w=10),
        leaf(-46, 49, 0, 16, S_GREEN_D, t=(0, 420, 0), w=9),
        box(2.4, 30, 3, S_GOLD, -17, 40, fz(-17, 40) - 1.2, t=(0, 30, 0)),  # gold vine trim
        leaf(-19, 47, fz(-19, 47) + 0.5, 6, S_GOLD, t=(0, 150, 0)),
        leaf(-15, 33, fz(-15, 33) + 0.5, 6, S_GOLD, t=(0, -150, 0)),
    ])
    return out


def selesnya_hands():
    one = [
        part("frustum", (-13, 7, 0), (20, 14, 19), S_GREEN),
        part("cyl", (-13, 13, 0), (15, 2.5, 14), S_GOLD),
        ball(9, S_WHITE, -13, 21, 0, 1.05, 1.0, 0.72),
        leaf(-13, 21, 6.6, 10, S_LEAF, t=(0, 0, 0), w=6),                 # a leaf on the back of the hand
        ball(3.5, S_WHITE, -4, 21, 3, 1, 1.4, 1),
    ]
    for k in range(5):                                                  # leaf cuff
        a = 2 * math.pi * (k - 2) / 10
        one.append(leaf(-13 + 11 * math.sin(a), 8, 11 * math.cos(a), 13, S_LEAF if k % 2 else S_GREEN_D, t=(-100, 0, round((k - 2) * 100)), w=7))
    for i, fx in enumerate((-18.5, -15, -11.3, -7.8)):
        h = 9 - abs(i - 1.5) * 1.6
        one.append(part("cyl", (fx, 27 + h / 2, 0), (3.6, h, 4), S_WHITE))
        one.append(ball(1.9, S_WHITE, fx, 27 + h, 0))
    one.append(part("cyl", (-13, 28, 0), (16, 1.6, 6), S_GOLD))
    return mirror(one)


def selesnya_feet():
    one = [
        box(15, 3, 27, S_WOOD, -11, 1.5, 4),
        ball(8.5, S_GREEN, -11, 8, 7, 0.9, 0.75, 1.55),
        ball(5, S_GREEN, -11, 7, 15, 1.3, 1.0, 1.0),
        cyl(7, 27, S_GREEN, -11, 6, 0),
        ball(6, S_WHITE, -11, 22, 4, 1.15, 2.2, 0.8),                    # white greave
        ball(4.5, S_WHITE, -11, 36, 6, 1, 0.9, 0.7),                     # knee
        ball(1.8, S_GOLD, -11, 36, 9.5, mesh="gem"),
        part("cyl", (-11, 12, 0), (15.5, 2, 15.5), S_GOLD),
        box(2, 16, 2, S_GOLD, -11, 23, 9),                               # gold vine up the greave
        leaf(-14, 25, 8.5, 5, S_GOLD, t=(0, 120, 0)),
        leaf(-8, 20, 8.5, 5, S_GOLD, t=(0, -120, 0)),
    ]
    for k in range(5):                                                  # leaf cuff at the top
        a = 2 * math.pi * (k - 2) / 10
        one.append(leaf(-11 + 9 * math.sin(a), 35, 9 * math.cos(a), 12, S_LEAF if k % 2 else S_GREEN_D, t=(-120, 0, round((k - 2) * 100)), w=7))
    one += [leaf(-11 + 8 * math.sin(a), 34, 8 * math.cos(a), 10, S_GREEN_D, t=(120, 0, 0)) for a in (2.6, 3.14, 3.7)]
    return mirror(one)


# ------------------------------------- T27: the two new cities' own gear
# Rix Maadi's and Svogthos's armourers sell their guild's cut in the city's
# own colours: Rix Maadi's in the embers of its caverns, Svogthos's in bone
# and deep violet, each with a flourish of its own.

def recolour(parts, table):
    out = []
    for q in parts:
        q = dict(q)
        q["c"] = table.get(q["c"], q["c"])
        out.append(q)
    return out


RIX = {R_BLACK: "#3a1a14", R_BLACK_L: "#5a2a1a", R_RED: "#f08a2a", R_RED_D: "#b0401a",
       R_SPIKE: "#e9c46a", R_IRON: "#6a5a50", R_BONE: "#f3d9a0"}
FLAME, EMBER = "#ffb040", "#ff6a20"


def flame(x, y, z, h, n="body"):
    return [part("cone", (x, y + h / 2, z), (h * 0.5, h, h * 0.5), EMBER, n=n, g=700),
            part("cone", (x, y + h * 0.45, z), (h * 0.3, h * 0.7, h * 0.3), FLAME, n=n, g=900)]


def rix_head():
    out = recolour(rakdos_head(), RIX)
    for z in (-10, -3, 4):
        out += flame(0, 34, z, 9)                                        # a crest of flame
    return out


def rix_chest():
    out = recolour(rakdos_chest(), RIX)
    out += [ball(3.5, EMBER, x, y, fz(x, y) + 1.5, g=800) for x, y in ((-10, 46), (10, 46), (0, 30))]  # ember studs
    out += mirror(flame(-38, 64, 0, 10))
    return out


def rix_hands():
    out = recolour(rakdos_hands(), RIX)
    out += mirror([ball(4, FLAME, -13, 21, 7, 1, 1, 0.5, g=900)])           # glowing palms
    return out


def rix_feet():
    out = recolour(rakdos_feet(), RIX)
    out += mirror(flame(-11, 1, -14, 8))                                  # sparks at the heel
    return out


SVOG = {G_SHELL: "#4a3a5a", G_SHELL_D: "#2c2238", G_SHELL_L: "#6a5a7a", G_BROWN: "#3a2e30",
        G_BONE: "#e8dcc0", G_CAP: "#7a9a3a", G_CAP2: "#c8b060", G_SPORE: "#9af0e0"}
SPORE = "#9af0e0"


def svogthos_head():
    out = recolour(golgari_head(), SVOG)
    out += [ball(2.2, SPORE, x, y, z, g=900) for x, y, z in ((-12, 34, 4), (9, 38, -2), (2, 42, 6), (-4, 36, -9))]
    return out


def svogthos_chest():
    out = recolour(golgari_chest(), SVOG)
    out += [box(3, 14, 3, G_BONE, x, 40, fz(x, 40) + 1) for x in (-12, -6, 0, 6, 12)]  # ribs of bone
    return out


def svogthos_hands():
    out = recolour(golgari_hands(), SVOG)
    out += mirror([ball(2, SPORE, -13, 30, 4, g=900), ball(1.6, SPORE, -17, 25, 6, g=900)])
    return out


def svogthos_feet():
    out = recolour(golgari_feet(), SVOG)
    out += mirror([ball(2, SPORE, -11, 30, 8, g=900), ball(1.5, SPORE, -15, 22, 6, g=900)])
    return out


MODELS = {}
for g in ("wanderer", "azorius", "izzet", "rakdos", "golgari", "selesnya", "rix", "svogthos"):
    for slot in ("head", "chest", "hands", "feet"):
        MODELS[f"armour_{g}_{slot}"] = globals()[f"{g}_{slot}"]()
