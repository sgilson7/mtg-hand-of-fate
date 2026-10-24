"""Creatures, set G: the green beasts of Return to Ravnica, a few guild
creatures, Niv-Mizzet (the final boss) and the token creatures. Each after
its card's art: silhouette, colours and signature features first."""
import math

from common import ball, box, cyl, part, mirror, eyes

TURN = 1000 / (2 * math.pi)


def seg(a, b, r, c, n="body", mesh="cyl", rz=None, extra=0):
    """A limb from point a to point b of radius r: a cylinder, cone (apex at b),
    frustum or stretched sphere lying along the line."""
    dx, dy, dz = b[0] - a[0], b[1] - a[1], b[2] - a[2]
    length = math.sqrt(dx * dx + dy * dy + dz * dz) or 1
    pitch = math.acos(max(-1, min(1, dy / length))) * TURN
    yaw = math.atan2(dx, dz) * TURN if (dx or dz) else 0
    mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, (a[2] + b[2]) / 2)
    return part(mesh, mid, (2 * r, length + extra, 2 * (rz if rz else r)), c, n, (pitch, 0, yaw))


def chain(points, radii, c, n="body", mesh="sphere", extra=1.4):
    """Overlapping stretched spheres along a path: necks, tails, tendrils."""
    out = []
    for i in range(len(points) - 1):
        r = radii[i] if isinstance(radii, list) else radii
        out.append(seg(points[i], points[i + 1], r, c, n, mesh, extra=r * extra))
    return out


def scaled(parts, k):
    """The parts grown or shrunk by k about the origin."""
    out = []
    for p in parts:
        q = dict(p)
        q["p"] = [round(v * k) for v in p["p"]]
        q["s"] = [max(1, round(v * k)) for v in p["s"]]
        out.append(q)
    return out


def look(x, y, z, r, white="#fbf6ea", pupil="#1d1626", n="head", spread=None):
    """A pair of cute eyes: whites with pupils, at +-x."""
    return mirror([ball(r, white, x, y, z, n=n), ball(r * 0.55, pupil, x, y, z + r * 0.6, n=n)])


def hoofleg(hip, knee, foot, r, c, hoof="#2b2420", n="body"):
    return [
        seg(hip, knee, r, c, n, "sphere", extra=r),
        seg(knee, foot, r * 0.62, c, n, "cyl"),
        cyl(r * 0.75, r * 0.7, hoof, foot[0], 0, foot[2], n=n),
    ]


# ---------------------------------------------------------------- people

def herald():
    """Centaur's Herald: a red-haired elf scout in olive and black, arms flung
    wide calling the centaurs, a long horn in one hand, tattered skirt."""
    SK, HAIR, OLV, BLK, GLD = "#f0c6a0", "#c8432a", "#6f6a2e", "#2a2622", "#d8a640"
    out = mirror([
        seg((13, 0, 0), (12, 62, 0), 11, "#3a3a2a", mesh="frustum"),
        ball(12, BLK, 13, 7, 4, 1, 0.6, 1.4),
        seg((21, 122, 0), (52, 128, 6), 8, SK, mesh="sphere", extra=8),
        seg((52, 128, 6), (86, 136, 10), 7, BLK, mesh="sphere", extra=8),
        ball(7, BLK, 90, 137, 11),
        part("cone", (30, 162, -2), (8, 22, 6), SK, "head", (0, 0, -300)),
    ])
    for i, (x, z) in enumerate([(-22, 8), (-8, 18), (8, 18), (22, 8), (-20, -10), (0, -16), (20, -10), (0, 14)]):
        out.append(seg((x * 0.7, 92, z * 0.7), (x * 1.25, 34 + (i % 3) * 8, z * 1.3), 7, OLV if i % 2 else "#4c5a26", mesh="cone"))
    out += [
        ball(22, OLV, 0, 88, 0, 1, 0.55, 0.9),
        ball(20, GLD, 0, 96, 0, 1.05, 0.2, 0.95),
        ball(17, SK, 0, 106, 0, 1, 0.7, 0.85),
        ball(21, "#5a5226", 0, 122, 0, 1, 0.7, 0.82),
        ball(10, GLD, 0, 124, 15, 0.9, 0.6, 0.4),
        cyl(6, 10, SK, 0, 134, 0),
        ball(27, SK, 0, 162, 2, n="head"),
        ball(29, HAIR, 0, 172, -6, 1.02, 0.82, 1.0, n="head"),
        ball(20, HAIR, 0, 132, -24, 1.1, 1.5, 0.7, n="head"),
        ball(12, HAIR, -22, 150, 4, 0.7, 1.2, 0.8, n="head"),
        ball(12, HAIR, 22, 150, 4, 0.7, 1.2, 0.8, n="head"),
        ball(3.5, "#a8443a", 0, 149, 26, 1.3, 0.5, 0.5, n="head"),
        seg((-92, 137, 12), (-118, 128, 40), 3, GLD, mesh="cone"),
        ball(6, GLD, -92, 137, 12, 0.8, 0.8, 0.8),
        part("cone", (-120, 127, 42), (12, 8, 12), GLD, t=(250, 0, 750)),
    ]
    out += look(10, 162, 22, 5)
    return out


def dryad():
    """Dryad Militant: tan skin, a great crown of ivy for hair, a long white
    dress, gold-and-black shoulder armour and bracers, a spear."""
    SK, LEAF, LEAF2, WHT, GLD, BLK = "#c48a58", "#3f8a34", "#69b043", "#efeadf", "#cf9e3a", "#2c2a28"
    out = [
        part("frustum", (0, 52, 0), (56, 104, 50), WHT, t=(500, 0, 0)),
        ball(30, WHT, 0, 6, 2, 1, 0.3, 0.95),
        ball(20, WHT, 0, 104, 0, 1, 0.6, 0.85),
        ball(19, GLD, 0, 100, 4, 1.06, 0.22, 0.95),
        ball(9, BLK, 0, 100, 19, 1, 1, 0.4),
        ball(6, GLD, 0, 100, 22, 1, 1, 0.4),
        ball(19, SK, 0, 124, 0, 1, 0.8, 0.8),
        ball(20, GLD, 0, 134, -2, 1.25, 0.45, 0.95),
        cyl(6, 10, SK, 0, 140, 0),
        ball(26, SK, 0, 166, 2, n="head"),
        ball(5, "#9a4a3a", 0, 154, 26, 1.1, 0.5, 0.5, n="head"),
        seg((44, 4, 34), (40, 214, 14), 3.5, "#7a5a32"),
        seg((40, 210, 14), (39, 244, 11), 7, GLD, mesh="cone", rz=3),
    ]
    out += mirror([
        ball(16, GLD, 22, 138, 0, 1, 0.7, 1),
        part("cone", (30, 146, 0), (12, 22, 12), BLK, t=(0, -150, 0)),
        seg((24, 130, 0), (32, 100, 12), 7, SK, mesh="sphere", extra=7),
        seg((32, 100, 12), (40, 80, 22), 7.5, GLD, mesh="cyl"),
        ball(7, SK, 41, 76, 24),
        ball(20, LEAF, 18, 180, -6, n="head"),
        ball(16, LEAF2, 26, 166, -14, n="head"),
        ball(17, LEAF, 12, 196, 4, n="head"),
        ball(13, LEAF2, 22, 152, -20, 1, 1.4, 1, n="head"),
    ])
    out += [
        ball(24, LEAF2, 0, 196, -8, n="head"),
        ball(22, LEAF, 0, 170, -22, 1.1, 1.2, 1, n="head"),
        ball(16, LEAF, 0, 140, -24, 1.1, 1.3, 0.8, n="head"),
        part("cone", (0, 222, -4), (16, 22, 10), LEAF2, "head"),
    ]
    out += look(9, 166, 21, 4.6)
    return out


def goblin_mage():
    """Goblin Electromancer: a hunched grey-violet goblin in a maroon coat,
    long nose, great ears, a crackling blue orb of lightning in his claws."""
    SK, COAT, DARK, BLUE = "#8a7a8c", "#7a2a2c", "#3a2a30", "#a8e2ff"
    out = [
        ball(30, COAT, 0, 46, -4, 1, 0.95, 0.9),
        part("frustum", (0, 22, -4), (60, 44, 52), COAT, t=(500, 0, 0)),
        ball(18, "#c9a24a", 0, 52, 22, 0.6, 0.6, 0.3),
        ball(27, SK, 0, 82, 16, 1, 0.92, 1, n="head"),
        part("cone", (0, 78, 50), (12, 34, 12), SK, "head", (230, 0, 0)),
        ball(14, "#c8c8d0", 0, 96, -8, 1.5, 0.6, 1, n="head"),
        ball(6, "#5a2030", 0, 66, 34, 1.4, 0.4, 0.5, n="head"),
        ball(17, BLUE, 0, 44, 52, g=950),
        ball(9, "#ffffff", 0, 44, 52, g=1000),
    ]
    for a in range(5):
        ang = a * 2 * math.pi / 5
        out.append(seg((0, 44, 52), (28 * math.cos(ang), 44 + 26 * math.sin(ang), 52 + 10 * math.sin(2 * ang)), 1.5, BLUE, mesh="cone"))
        out[-1]["g"] = 1000
    out += mirror([
        part("cone", (40, 90, 10), (14, 42, 8), SK, "head", (0, 220, 0)),
        part("cone", (40, 90, 11), (8, 30, 4), "#c88a9a", "head", (0, 220, 0)),
        ball(3, "#d8c060", 30, 80, 14, n="head"),
        seg((24, 60, 4), (22, 46, 40), 9, COAT, mesh="sphere", extra=8),
        ball(9, SK, 18, 46, 42),
        ball(10, DARK, 14, 6, 6, 1, 0.6, 1.5),
    ])
    out += look(11, 88, 38, 5.5, white="#ffe27a")
    return out


def goblin_token():
    """A Goblin Rally goblin: orange skin, big ears, a grin, a hammer."""
    SK, CLO = "#d9783a", "#6a4a2a"
    out = [
        ball(24, SK, 0, 40, 0, 1, 0.95, 0.9),
        ball(22, CLO, 0, 24, 0, 1.05, 0.55, 0.95),
        ball(26, SK, 0, 74, 8, n="head"),
        part("cone", (0, 72, 36), (10, 18, 10), SK, "head", (230, 0, 0)),
        ball(7, "#6a1f18", 0, 60, 28, 1.6, 0.4, 0.5, n="head"),
        box(8, 4, 2, "#fbf6ea", -4, 60, 31, n="head"),
        cyl(3, 80, "#7a5030", -32, 20, 14, t=(0, 0, 0)),
        box(30, 20, 20, "#7a7268", -32, 104, 14),
        ball(8, "#c9a24a", 0, 26, 21, 1, 1, 0.4),
        part("cone", (0, 100, 4), (10, 16, 10), "#3a2a20", "head", (-60, 0, 0)),
    ]
    out += mirror([
        part("cone", (34, 82, 4), (14, 36, 8), SK, "head", (0, 230, 0)),
        seg((20, 52, 0), (32, 36, 14), 7, SK, mesh="sphere", extra=7),
        ball(10, SK, 12, 6, 6, 1, 0.6, 1.4),
    ])
    out += look(10, 80, 28, 5, white="#ffe27a")
    return out


def soldier():
    """Precinct Captain's soldier: Boros white plate with a red sash, a
    sword and a round shield."""
    SK, ARM, RED, GLD, LEA = "#e9bd96", "#e9e3d4", "#c8392c", "#e0b043", "#6a4a32"
    out = mirror([
        seg((14, 0, 0), (13, 64, 0), 12, "#8a8478", mesh="frustum"),
        ball(13, LEA, 14, 7, 5, 1, 0.6, 1.4),
        ball(17, ARM, 30, 122, 0, 1, 0.8, 1),
        seg((32, 116, 2), (40, 82, 14), 8, LEA, mesh="sphere", extra=8),
        ball(9, SK, 41, 78, 16),
    ])
    out += [
        ball(30, ARM, 0, 106, 0, 1, 1.0, 0.85),
        ball(28, RED, 0, 72, 0, 1.0, 0.5, 0.9),
        ball(29, GLD, 0, 84, 0, 1.0, 0.16, 0.9),
        part("frustum", (0, 116, 4), (12, 50, 6), RED, t=(0, 160, 0)),
        ball(28, SK, 0, 158, 2, n="head"),
        ball(30, ARM, 0, 168, -2, 1.03, 0.72, 1.03, n="head"),
        part("cone", (0, 192, -2), (12, 18, 30), RED, "head"),
        box(5, 14, 4, ARM, 0, 157, 28, n="head"),
        cyl(3, 8, GLD, 42, 72, 18, t=(250, 0, 0)),
        box(6, 74, 2, "#d8dde4", 42, 72, 52, t=(250, 0, 0)),
        part("cyl", (-46, 96, 14), (52, 8, 52), RED, t=(0, 250, 0)),
        part("cyl", (-50, 96, 14), (20, 6, 20), GLD, t=(0, 250, 0)),
    ]
    out += look(10, 158, 25, 4.6)
    return out


def knight():
    """Selesnya Charm's knight: grey plate, long dark hair streaming, a great
    round green shield bearing the white-green tree."""
    ARM, ARM2, SK, HAIR, GRN, GRN2 = "#9aa2ac", "#6e7680", "#e2b892", "#2a2e3e", "#7fc44a", "#2e6a2a"
    out = mirror([
        seg((15, 0, 0), (14, 66, 0), 13, ARM2, mesh="frustum"),
        ball(14, ARM2, 15, 8, 6, 1, 0.6, 1.4),
        ball(19, ARM, 31, 124, 0, 1, 0.75, 1.05),
        seg((33, 118, 0), (40, 84, 12), 9, ARM2, mesh="sphere", extra=8),
        ball(10, ARM2, 41, 80, 14),
    ])
    out += [
        ball(32, ARM, 0, 106, 0, 1, 1.0, 0.85),
        ball(30, ARM2, 0, 72, 0, 1.0, 0.5, 0.9),
        ball(27, SK, 0, 160, 2, n="head"),
        ball(28, HAIR, 0, 170, -6, 1.02, 0.7, 1.0, n="head"),
        ball(22, HAIR, 0, 140, -30, 1, 1.4, 0.6, n="head"),
        ball(20, HAIR, 14, 128, -40, 0.8, 1.2, 0.6, n="head"),
        part("cyl", (-50, 104, 26), (90, 10, 90), GRN2, t=(250, 0, -80)),
        part("cyl", (-54, 104, 30), (76, 10, 76), GRN, t=(250, 0, -80)),
        ball(15, GRN2, -56, 112, 34, 1, 1, 0.5, t=(0, 0, -80)),
        cyl(3, 28, GRN2, -57, 82, 34),
        ball(10, ARM, -58, 104, 38, 1, 1, 0.6),
        cyl(3, 120, "#d8dde4", 42, 80, 14, t=(60, 0, 0)),
        box(16, 4, 4, "#b89040", 42, 80, 14),
    ]
    out += look(10, 160, 24, 4.6)
    return out


def centaur(body, torso, cloth, hair, healer):
    """A centaur: horse body, human torso rising from the horse's chest. The
    Healer has a white crested helmet, a curling staff and a green flask held
    out; the token a spear and a green cloak."""
    Z = 52
    out = [
        ball(46, body, 0, 128, -30, 0.9, 0.78, 1.5),
        ball(36, body, 0, 138, 26, 0.9, 0.92, 1),
        ball(26, torso, 0, 184, Z, 1, 1, 0.8),
        ball(25, cloth, 0, 160, Z - 4, 1.08, 0.6, 0.9),
        ball(26, torso, 0, 230, Z + 4, n="head"),
    ]
    for x in (21, -21):
        out += hoofleg((x, 120, 40), (x, 64, 52), (x, 0, 46), 13, body)
        out += hoofleg((x, 122, -74), (x, 60, -90), (x, 0, -74), 14, body)
    out += chain([(0, 140, -92), (0, 120, -126), (0, 70, -142)], [11, 8], hair, "tail")
    out += mirror([ball(14, "#d8d2c4" if healer else cloth, 24, 200, Z - 2, 1, 0.7, 1)])
    out += look(9, 230, Z + 22, 4.4)
    if healer:
        out += [
            ball(28, "#e8e0cc", 0, 242, Z, 1.03, 0.75, 1.03, n="head"),
            part("cone", (0, 270, Z - 6), (8, 26, 34), "#e8e0cc", "head"),
            box(4, 16, 4, "#e8e0cc", 0, 232, Z + 26, n="head"),
            ball(16, "#c9a24a", 36, 124, -30, 0.4, 1, 1),
            ball(8, "#3a2a22", 39, 124, -30, 0.3, 1, 1),
            seg((24, 196, Z), (38, 168, Z + 16), 7.5, torso, mesh="sphere", extra=7),
            seg((40, 40, Z + 24), (40, 290, Z + 14), 3.5, "#a8a090"),
            ball(12, "#a8a090", 40, 300, Z + 14, 1, 0.5, 1.6),
            ball(8, "#a8a090", 40, 312, Z + 2),
            seg((-24, 198, Z), (-44, 200, Z + 48), 7, torso, mesh="sphere", extra=7),
            ball(10, "#4fbf5a", -46, 204, Z + 62, g=700),
            cyl(3, 10, "#4fbf5a", -46, 212, Z + 62),
        ]
    else:
        out += mirror([seg((24, 196, Z), (38, 168, Z + 16), 7.5, torso, mesh="sphere", extra=7)])
        out += [
            ball(28, hair, 0, 242, Z - 4, 1.02, 0.7, 1.0, n="head"),
            ball(18, hair, 0, 212, Z - 18, 1, 1.4, 0.6, n="head"),
            seg((40, 30, Z + 26), (38, 280, Z + 10), 3.5, "#7a5a32"),
            seg((38, 278, Z + 10), (38, 312, Z + 8), 7, "#c8c8c0", mesh="cone"),
            ball(30, "#4f8a34", 0, 198, Z - 14, 1.15, 0.5, 1.0),
            ball(28, "#4f8a34", 0, 170, Z - 26, 1.1, 1.0, 0.5),
        ]
    return out


def weird():
    """Frostburn Weird: a floating elemental, its left half crystal ice and
    its right half fire, a crown of shards and flames."""
    ICE, ICE2, FIRE, FIRE2, CORE = "#9fd3ff", "#5a8ee0", "#ff7a2a", "#ffc04a", "#eef8ff"
    out = [
        part("cone", (0, 40, 0), (50, 80, 50), "#6aa8f0", t=(500, 0, 0)),
        ball(14, CORE, 0, 122, 18, g=900),
    ]
    # ice half (+x): faceted shards
    out += [
        ball(34, ICE2, 18, 116, 0, 0.9, 1.1, 0.9, mesh="ico"),
        ball(22, ICE, 40, 148, 0, mesh="ico"),
        seg((40, 146, 0), (62, 100, 14), 12, ICE, mesh="sphere", extra=10),
        part("gem", (66, 88, 18), (20, 32, 20), ICE),
        part("gem", (30, 170, -6), (12, 40, 12), ICE, t=(0, -200, 0)),
        part("gem", (50, 168, -2), (12, 36, 12), ICE2, t=(0, -330, 0)),
        part("gem", (20, 108, 26), (14, 30, 10), CORE),
        ball(26, ICE, 9, 184, 2, 0.6, 1, 0.95, n="head", mesh="ico"),
        part("gem", (12, 222, 0), (12, 40, 12), ICE, "head", (0, -80, 0)),
        part("gem", (26, 212, 0), (10, 30, 10), ICE2, "head", (0, -220, 0)),
        part("gem", (0, 228, 0), (10, 34, 10), CORE, "head"),
        ball(5, "#1d3a7a", 8, 186, 24, n="head", g=600),
    ]
    # fire half (-x): glowing flames
    out += [
        ball(32, FIRE, -18, 116, 0, 0.9, 1.1, 0.9, g=500),
        ball(22, FIRE, -40, 148, 0, g=500),
        seg((-40, 146, 0), (-62, 100, 14), 12, FIRE, mesh="sphere", extra=10),
        ball(11, FIRE2, -66, 92, 18, g=800),
        part("cone", (-36, 176, -4), (20, 40, 20), FIRE2, t=(0, 120, 0), g=800),
        part("cone", (-52, 168, -2), (14, 32, 14), FIRE, t=(0, 250, 0), g=600),
        part("cone", (-26, 96, -20), (18, 34, 18), FIRE2, t=(0, 200, 0), g=700),
        ball(26, FIRE, -9, 184, 2, 0.6, 1, 0.95, n="head", g=500),
        part("cone", (-12, 220, 0), (16, 40, 16), FIRE2, "head", (0, 80, 0), g=800),
        part("cone", (-28, 210, 0), (12, 30, 12), FIRE, "head", (0, 220, 0), g=700),
        ball(5, "#fff2a0", -8, 186, 24, n="head", g=1000),
    ]
    return out


def sweeper():
    """Street Sweeper: a tall wooden drum on piston legs, a mushroom roof,
    a blue glowing core in front and a ring of brooms that scrub the street."""
    WOOD, WOOD2, MET, BRUSH, CORE = "#8a5232", "#6a3a22", "#6e6a72", "#6a4a28", "#7fe0ff"
    out = [
        cyl(80, 110, WOOD, 0, 120, 0),
        cyl(84, 10, WOOD2, 0, 150, 0),
        cyl(84, 10, WOOD2, 0, 205, 0),
        part("sphere", (0, 248, 0), (230, 50, 230), "#a06a42"),
        cyl(16, 30, WOOD2, 0, 262, 0),
        ball(12, MET, 0, 296, 0),
        box(46, 74, 30, "#8a8e96", 0, 150, 82),
        box(30, 56, 20, CORE, 0, 152, 92),
        ball(24, CORE, 0, 150, 96, 1.2, 1.4, 0.5, g=700),
        part("cone", (0, 202, 84), (20, 26, 20), "#c8392c"),
        ball(10, MET, 0, 112, 84),
        cyl(46, 22, MET, 0, 98, 0),
    ]
    for i in range(6):
        a = (i + 0.5) * 2 * math.pi / 6
        cx, cz = math.sin(a), math.cos(a)
        hip = (70 * cx, 118, 70 * cz)
        knee = (140 * cx, 140, 140 * cz)
        foot = (150 * cx, 34, 150 * cz)
        out += [
            seg(hip, knee, 11, MET),
            ball(14, MET, *knee),
            seg(knee, foot, 13, WOOD2, mesh="frustum"),
            part("cone", (150 * cx, 18, 150 * cz), (56, 36, 56), BRUSH),
        ]
    out += mirror([
        seg((40, 150, 70), (90, 120, 110), 7, MET),
        ball(9, MET, 90, 120, 110),
        seg((90, 120, 110), (100, 40, 120), 7, MET),
        part("cone", (100, 24, 120), (40, 30, 40), "#8a6a3a"),
    ])
    return out


# ---------------------------------------------------------------- insects and such

def drudge_beetle():
    """Drudge Beetle: a dark green domed beetle crusted with scales, long
    yellow-and-black striped legs, red hooked mandibles, bristles on its back."""
    SH, SH2, SC, LEG, LEG2, RED = "#3e5a28", "#2c4220", "#6d8c3a", "#c8b040", "#2a2a1a", "#b0302a"
    out = [
        ball(80, SH, 0, 130, -20, 1, 0.72, 1.25),
        ball(70, SH2, 0, 112, -20, 1.05, 0.5, 1.2),
        ball(34, SH2, 0, 110, 84, 1.1, 0.8, 1),
        ball(28, "#3a2a20", 0, 100, 108, n="head"),
    ]
    for (x, y, z) in [(0, 178, -10), (36, 170, -40), (-36, 170, -40), (40, 166, 20), (-40, 166, 20),
                      (0, 168, 40), (0, 172, -60), (60, 150, -70), (-60, 150, -70), (66, 150, 10), (-66, 150, 10)]:
        out.append(ball(16, SC, x, y, z, 1.1, 0.5, 1.1))
    for (x, z) in [(20, -30), (-24, 0), (10, 30), (-12, -70), (40, -10)]:
        out.append(seg((x, 170, z), (x * 1.6, 214, z - 20), 2.5, "#8a9a5a", mesh="cone"))
    out += eyes(110, 132, 14, 6, "#d8402a")
    out += mirror([
        seg((14, 92, 128), (34, 84, 156), 6, RED, mesh="sphere", extra=6),
        seg((34, 84, 156), (14, 76, 178), 4, RED, mesh="cone", n="head"),
    ])
    for z, spread, fz in [(60, 150, 140), (0, 170, 10), (-70, 150, -150)]:
        out += mirror([
            seg((50, 110, z), (spread, 168, fz * 0.8), 8, LEG2),
            ball(9, LEG, spread, 168, fz * 0.8),
            seg((spread, 168, fz * 0.8), (spread + 30, 70, fz), 7, LEG),
            seg((spread + 30, 70, fz), (spread + 40, 0, fz * 1.05), 5, LEG2, mesh="cone"),
        ])
    return out


def goliath():
    """Deadbridge Goliath: a vast black-green rhinoceros beetle, a mossy back
    sprouting pale mushrooms, great curved red-tipped horns on a low head."""
    SH, SH2, MOSS, CAP, STEM, HORN = "#27302a", "#3a4636", "#6f7a36", "#b8b888", "#d8d4b8", "#9a2a24"
    out = [
        ball(150, SH, 0, 210, -40, 1, 0.68, 1.3),
        ball(140, MOSS, 0, 240, -50, 0.92, 0.44, 1.16),
        ball(90, SH2, 0, 170, 140, 1.05, 0.85, 0.9),
        ball(64, SH, 0, 130, 230, 1, 0.9, 1, n="head"),
        seg((0, 150, 270), (0, 250, 350), 24, "#3a2624", "head", mesh="frustum"),
        seg((0, 246, 348), (0, 360, 330), 16, HORN, "head", mesh="cone"),
        seg((0, 104, 286), (0, 110, 380), 14, HORN, "head", mesh="cone"),
    ]
    out += mirror([
        seg((44, 150, 250), (120, 216, 300), 16, "#3a2624", "head", mesh="frustum"),
        seg((118, 214, 298), (150, 310, 270), 11, HORN, "head", mesh="cone"),
        seg((40, 110, 260), (90, 60, 320), 10, "#4a2a26", "head", mesh="cone"),
        ball(10, "#e05a2a", 34, 144, 280, n="head", g=500),
        seg((110, 160, 150), (190, 120, 210), 38, SH2, mesh="sphere", extra=20),
        ball(36, SH2, 190, 120, 210),
        seg((190, 120, 210), (190, 0, 230), 24, SH, mesh="frustum"),
        seg((130, 160, 0), (220, 130, -10), 40, SH2, mesh="sphere", extra=20),
        ball(36, SH2, 220, 130, -10),
        seg((220, 130, -10), (220, 0, 0), 26, SH, mesh="frustum"),
        seg((120, 150, -160), (200, 120, -220), 38, SH2, mesh="sphere", extra=20),
        ball(36, SH2, 200, 120, -220),
        seg((200, 120, -220), (200, 0, -240), 24, SH, mesh="frustum"),
    ])
    for (x, z, h, r) in [(0, -40, 120, 46), (-60, 0, 80, 34), (70, -90, 96, 38), (40, 60, 64, 26),
                         (-70, -130, 70, 30), (-30, 80, 50, 22)]:
        base = 270 if abs(x) < 50 else 255
        out.append(cyl(7, h, STEM, x, base, z))
        out.append(ball(r, CAP, x, base + h, z, 1, 0.32, 1))
    return out


def scorpion():
    """Sluiceway Scorpion: a dark sewer scorpion, segmented tail arched high
    over its back with a red bulb and amber sting, pincers forward."""
    C1, C2, LEG, RED, TIP = "#3e3e2a", "#5a5a3a", "#2e2a22", "#8a2a22", "#e08a2a"
    out = [
        ball(56, C1, 0, 56, 0, 1, 0.55, 1.3),
        ball(46, C2, 0, 66, -10, 0.9, 0.4, 1.2),
        ball(32, C1, 0, 52, 72, 1, 0.6, 0.9, n="head"),
    ]
    out += look(12, 66, 96, 5, white="#e0d060", n="head")
    pts = [(0, 60, -60), (0, 80, -110), (0, 120, -136), (0, 168, -130), (0, 204, -100), (0, 214, -56)]
    out += chain(pts, [22, 20, 18, 16, 14], C2, "tail")
    for p in pts[1:5]:
        out.append(ball(8, C1, p[0], p[1] + 12, p[2], n="tail"))
    out += [
        ball(20, RED, 0, 206, -30, n="tail"),
        seg((0, 200, -20), (0, 172, 0), 7, TIP, "tail", mesh="cone"),
    ]
    out += mirror([
        seg((26, 50, 84), (60, 52, 126), 10, C2, mesh="sphere", extra=8),
        ball(17, C1, 66, 52, 140, 1, 0.7, 1.2),
        seg((60, 52, 150), (54, 52, 186), 7, C1, mesh="cone"),
        seg((74, 52, 150), (80, 52, 180), 6, C2, mesh="cone"),
    ])
    for z in (40, 10, -20, -50):
        out += mirror([
            seg((40, 56, z), (90, 84, z * 1.3), 6, LEG),
            seg((90, 84, z * 1.3), (110, 0, z * 1.5), 5, LEG, mesh="cone"),
        ])
    return out


def archweaver():
    """Archweaver: a huge hairy spider, dark brown legs with red-orange
    banded tips and fangs, a yellow-green patterned abdomen."""
    LEG, LEG2, AB, AB2, ORG = "#3a2c24", "#24201c", "#a6a23a", "#5a6a2a", "#e0502a"
    out = [
        ball(84, AB, 0, 150, -110, 1, 0.85, 1.15),
        ball(40, AB2, 0, 200, -110, 1.2, 0.4, 1.5),
        ball(20, ORG, 0, 214, -60, 1, 0.4, 1),
        ball(20, ORG, 0, 214, -160, 1, 0.4, 1),
        ball(54, LEG, 0, 130, 10, 1, 0.7, 1.1),
        ball(34, LEG2, 0, 126, 70, n="head"),
    ]
    out += mirror([
        ball(18, AB2, 50, 190, -110, 1, 0.5, 1),
        ball(14, ORG, 44, 196, -80, 1, 0.4, 1),
        seg((12, 104, 94), (14, 70, 112), 7, ORG, "head", mesh="cone"),
        seg((24, 120, 92), (40, 120, 140), 6, LEG, "head", mesh="sphere", extra=4),
        ball(6, ORG, 40, 118, 144, n="head"),
    ])
    out += eyes(146, 96, 10, 6, "#e0d040") + eyes(136, 100, 22, 4, "#e0d040")
    for i, a in enumerate([30, 65, 105, 145]):
        r = math.radians(a)
        dx, dz = math.sin(r), math.cos(r)
        hip = (40 * dx, 130, 10 + 40 * dz)
        knee = (150 * dx, 250, 10 + 150 * dz)
        foot = (250 * dx, 0, 10 + 250 * dz)
        out += mirror([
            seg(hip, knee, 13, LEG, mesh="sphere", extra=10),
            ball(13, LEG2, *knee),
            seg(knee, (foot[0] * 0.8 + knee[0] * 0.2, 60, foot[2] * 0.8 + knee[2] * 0.2), 10, LEG),
            seg((foot[0] * 0.8 + knee[0] * 0.2, 60, foot[2] * 0.8 + knee[2] * 0.2), foot, 8, ORG if i == 0 else LEG2, mesh="cone"),
        ])
    return out


def mangler():
    """Dreg Mangler: a lurching heap of olive vine and rot, a gourd-skull head
    with burning orange eyes, a red ribbed maw of fangs, thorny back."""
    VINE, VINE2, SKULL, RED, TOOTH, EYE = "#7a7a32", "#4a4a22", "#a8a440", "#b0302a", "#f0e8d0", "#ff9a2a"
    out = [
        ball(70, VINE, 0, 110, -40, 0.9, 0.8, 1.4),
        ball(56, RED, 0, 112, 50, 0.85, 0.9, 0.8),
        ball(44, SKULL, 0, 170, 100, 1, 0.85, 0.95, n="head"),
        ball(36, RED, 0, 132, 124, 1, 0.6, 0.9, n="head"),
    ]
    out += mirror([
        ball(12, EYE, 18, 178, 134, g=1000, n="head"),
        ball(17, "#3a2a10", 18, 178, 128, n="head"),
    ])
    for i in range(6):
        x = -30 + i * 12
        out.append(part("cone", (x, 140, 154), (8, 22, 8), TOOTH, "head", (500, 0, 0)))
        out.append(part("cone", (x, 114, 150), (8, 20, 8), TOOTH, "head"))
    for i in range(4):
        z = 76 - i * 20
        out.append(part("cyl", (0, 110, z), (100, 12, 110 - i * 8), "#e0c8a0", t=(0, 250, 0)))
    for (z, h) in [(-10, 80), (-60, 70), (-100, 54), (30, 60)]:
        out.append(part("cone", (0, 190, z), (14, h, 14), RED, t=(-60, 0, 0)))
    for i, (z, x) in enumerate([(70, 50), (0, 60), (-80, 56), (-140, 30)]):
        out += mirror([
            seg((x * 0.6, 110, z), (x * 1.6, 40, z + 30), 12, VINE2, mesh="sphere", extra=10),
            seg((x * 1.6, 40, z + 30), (x * 1.8, 0, z + 50), 8, VINE, mesh="cone"),
        ])
    out += chain([(-30, 110, 120), (-90, 90, 150), (-120, 50, 120), (-110, 30, 160)], [7, 6, 4], VINE2)
    out += chain([(0, 90, -130), (20, 60, -200), (60, 30, -230)], [12, 8], VINE, "tail")
    return out


# ---------------------------------------------------------------- beasts

def brushstrider():
    """Brushstrider: a tall gangling beast like a hare-deer on stilt legs,
    tan with cream speckles, long ears swept back, shaggy brushes of fur."""
    FUR, FUR2, CREAM, HOOF = "#b8875a", "#8a6440", "#f0dcb8", "#3a2a20"
    out = [
        ball(46, FUR, 0, 196, -10, 0.85, 0.8, 1.35),
        ball(32, FUR2, 0, 186, -70, 0.9, 0.9, 0.9),
    ]
    out += chain([(0, 210, 40), (0, 250, 70), (0, 290, 80)], [24, 20], FUR)
    out += [
        ball(26, FUR, 0, 300, 90, 0.9, 0.95, 1.1, n="head"),
        ball(16, CREAM, 0, 290, 122, 0.8, 0.8, 1.1, n="head"),
        ball(5, "#3a2a20", 0, 292, 138, n="head"),
        ball(18, CREAM, 0, 160, -96, 1, 1.2, 1),
    ]
    out += mirror([
        part("cone", (14, 330, 70), (14, 56, 8), FUR, "head", (-120, -80, 0)),
        part("cone", (14, 330, 71), (8, 42, 4), "#e8b8a0", "head", (-120, -80, 0)),
        ball(4, "#1d1626", 18, 306, 112, n="head"),
        ball(7, CREAM, 26, 210, 10),
        ball(6, CREAM, 30, 200, -30),
        ball(6, CREAM, 28, 216, -50),
        seg((22, 180, 30), (30, 100, 60), 10, FUR, mesh="sphere", extra=8),
        seg((30, 100, 60), (30, 0, 54), 5, FUR2),
        ball(9, CREAM, 30, 96, 60, 1.3, 1.6, 1.3),
        cyl(7, 8, HOOF, 30, 0, 56),
        seg((22, 180, -60), (34, 110, -100), 14, FUR, mesh="sphere", extra=8),
        seg((34, 110, -100), (34, 0, -80), 5, FUR2),
        ball(10, CREAM, 34, 106, -100, 1.3, 1.6, 1.3),
        cyl(7, 8, HOOF, 34, 0, -80),
    ])
    out += look(15, 302, 104, 6, n="head")
    return out


def indrik():
    """Towering Indrik: a giraffe-tall beast, tan hide, a long neck, a horse
    head with tall pointed ears and a gaping red mouth, a pale mane and a
    harness from which red tendrils hang about its chest."""
    HIDE, HIDE2, MANE, RED, HOOF = "#a88e74", "#7a6450", "#e4dccc", "#b0302a", "#3a2e28"
    out = [
        ball(56, HIDE, 0, 210, -20, 0.85, 0.8, 1.35),
        ball(44, HIDE, 0, 226, 40, 0.9, 1, 0.9),
    ]
    neck = [(0, 240, 60), (0, 300, 90), (0, 360, 100), (0, 410, 96)]
    out += chain(neck, [30, 26, 22], HIDE)
    out += chain([(0, 250, 40), (0, 320, 70), (0, 400, 76)], [12, 12], MANE)
    out += [
        ball(30, HIDE, 0, 428, 108, 0.85, 0.9, 1.0, n="head"),
        seg((0, 424, 120), (0, 404, 170), 18, HIDE, "head", mesh="sphere", extra=10),
        ball(15, RED, 0, 396, 160, 0.9, 0.5, 1.2, n="head"),
        ball(14, HIDE2, 0, 386, 150, 0.9, 0.4, 1.1, n="head"),
        part("cyl", (0, 222, 46), (112, 14, 110), "#6a3a2a"),
        ball(14, "#c9a24a", 0, 222, 100, 1, 1, 0.4),
        chain([(0, 180, -110), (0, 140, -134), (0, 100, -132)], [8, 6], HIDE2, "tail")[0],
        chain([(0, 180, -110), (0, 140, -134), (0, 100, -132)], [8, 6], HIDE2, "tail")[1],
    ]
    out += mirror([
        part("cone", (16, 470, 96), (14, 50, 8), HIDE, "head", (-60, -100, 0)),
        part("cone", (16, 470, 97), (8, 38, 4), "#e8c0b0", "head", (-60, -100, 0)),
        ball(6, "#fbf6ea", 22, 434, 124, n="head"),
        ball(3.5, "#1d1626", 24, 434, 130, n="head"),
        seg((34, 200, 50), (40, 110, 66), 16, HIDE, mesh="sphere", extra=10),
        seg((40, 110, 66), (40, 0, 60), 10, HIDE2),
        cyl(12, 12, HOOF, 40, 0, 60),
        seg((32, 200, -80), (40, 110, -100), 18, HIDE, mesh="sphere", extra=10),
        seg((40, 110, -100), (40, 0, -86), 10, HIDE2),
        cyl(12, 12, HOOF, 40, 0, -86),
    ])
    for i in range(7):
        a = math.radians(-70 + i * 23)
        x, z = 56 * math.sin(a), 46 + 54 * math.cos(a)
        out.append(seg((x, 218, z), (x * 1.25, 130 + (i % 2) * 20, z + 16), 4, RED, mesh="cone"))
    return out


def monitor():
    """Korozda Monitor: a long low swamp lizard, olive scales, a back grown
    thick with mushrooms, yellow eyes and a long forked red tongue."""
    SC, SC2, BELLY, CAP, STEM, RED = "#6a6c38", "#4a4c2a", "#9a9a62", "#c8bc80", "#e0d8b8", "#b0302a"
    out = [
        ball(70, SC, 0, 76, -20, 1, 0.66, 1.6),
        ball(54, BELLY, 0, 60, 0, 0.95, 0.45, 1.4),
        ball(44, SC, 0, 76, 100, 1, 0.7, 1.1),
        ball(42, SC, 0, 70, 160, 1.05, 0.62, 1.1, n="head"),
        seg((0, 64, 180), (0, 56, 260), 34, SC2, "head", mesh="sphere", extra=10),
        ball(30, BELLY, 0, 46, 228, 0.95, 0.35, 1.2, n="head"),
    ]
    out += chain([(0, 68, -110), (0, 50, -200), (30, 30, -290), (80, 18, -350)], [36, 24, 14], SC, "tail")
    out += chain([(0, 36, 280), (0, 28, 320), (-6, 18, 350)], [3, 2.5], RED, "head")
    out += chain([(0, 36, 280), (0, 28, 320), (8, 16, 352)], [3, 2.5], RED, "head")
    out += mirror([
        ball(13, "#e8c84a", 28, 96, 190, n="head", g=300),
        ball(6, "#1d1626", 31, 98, 200, n="head"),
        ball(12, SC2, 26, 100, 186, 1.2, 0.5, 1.2, n="head"),
        seg((56, 70, 70), (84, 40, 100), 16, SC, mesh="sphere", extra=10),
        seg((84, 40, 100), (90, 0, 120), 12, SC2, mesh="cone"),
        seg((56, 70, -90), (90, 40, -110), 18, SC, mesh="sphere", extra=10),
        seg((90, 40, -110), (96, 0, -90), 13, SC2, mesh="cone"),
    ])
    for (x, z, h, r) in [(0, -10, 40, 26), (-24, 40, 30, 20), (26, 20, 28, 18), (10, -70, 34, 22),
                         (-28, -50, 24, 16), (0, 90, 22, 16), (30, -110, 20, 14), (-10, -140, 18, 13)]:
        y = 116 if abs(z) < 100 else 100
        out.append(cyl(5, h, STEM, x, y - 10, z))
        out.append(ball(r, CAP, x, y + h - 10, z, 1, 0.38, 1))
    return out


def rhino(SKIN, SKIN2, back, horn, plates):
    """A rhino. Rubbleback: tan-olive hide with a heap of rubble along its
    back. The Horncaller's token: pale armour plates over hide."""
    out = [
        ball(80, SKIN, 0, 124, -20, 0.95, 0.82, 1.35),
        ball(66, SKIN, 0, 130, 60, 1, 0.92, 0.9),
        ball(44, SKIN, 0, 108, 128, 0.9, 0.85, 1.1, n="head"),
        ball(30, SKIN2, 0, 90, 168, 0.9, 0.8, 1, n="head"),
        seg((0, 112, 180), (0, 210, 230), 16, horn, "head", mesh="cone"),
        seg((0, 132, 150), (0, 172, 166), 9, horn, "head", mesh="cone"),
    ]
    out += mirror([
        part("cone", (34, 150, 110), (14, 24, 8), SKIN, "head", (-60, -150, 0)),
        ball(6, "#fbf6ea", 34, 118, 150, n="head"),
        ball(3.5, "#1d1626", 36, 119, 155, n="head"),
    ])
    for x in (40, -40):
        out += hoofleg((x, 110, 70), (x, 56, 80), (x, 0, 76), 22, SKIN2, "#3a3228")
        out += hoofleg((x, 110, -80), (x, 56, -90), (x, 0, -80), 24, SKIN2, "#3a3228")
    out += chain([(0, 150, -120), (0, 120, -140)], [6], SKIN2, "tail")
    if plates:
        out += [
            ball(84, back, 0, 150, -24, 0.95, 0.55, 1.2),
            ball(70, back, 0, 160, 60, 1.0, 0.6, 0.75),
            ball(40, back, 0, 136, 130, 0.9, 0.5, 0.9, n="head"),
        ]
        for z in (-80, -20, 40, 90):
            out.append(part("cone", (0, 200 + (10 if z == 40 else 0), z), (20, 30, 30), "#8a8a7a"))
        out += mirror([ball(30, back, 62, 116, 74, 0.5, 1, 1), ball(30, back, 64, 112, -60, 0.5, 1, 1.2)])
    else:
        for (x, y, z, r) in [(0, 196, -30, 30), (30, 186, 10, 24), (-28, 190, -60, 26), (0, 200, 40, 26),
                             (36, 180, -80, 20), (-34, 182, 30, 22), (10, 176, -110, 22), (-8, 206, -6, 20)]:
            out.append(ball(r, back if (x + z) % 3 else "#a09684", x, y, z, mesh="rock"))
        out += [ball(14, "#6a8a3a", 20, 204, -40, 1, 0.5, 1), ball(12, "#6a8a3a", -20, 202, 20, 1, 0.5, 1)]
    return out


def stag():
    """Axebane Stag: a great dark brown elk, its bark-patterned hide and huge
    palmate antlers like axe blades sweeping up and back."""
    HIDE, HIDE2, ANT, ANT2, HOOF = "#6a4a2c", "#4a3220", "#9a7a48", "#7a5a32", "#2a2018"
    out = [
        ball(70, HIDE, 0, 220, -20, 0.85, 0.8, 1.4),
        ball(56, HIDE, 0, 236, 60, 0.9, 1.0, 0.9),
        ball(40, "#c8b488", 0, 216, -120, 0.8, 0.6, 0.5),
    ]
    out += chain([(0, 250, 80), (0, 300, 118), (0, 330, 130)], [36, 30], HIDE)
    out += [
        ball(34, HIDE, 0, 340, 140, 0.8, 0.85, 1.0, n="head"),
        seg((0, 336, 156), (0, 316, 212), 20, HIDE2, "head", mesh="sphere", extra=12),
        ball(8, "#1d1626", 0, 314, 232, n="head"),
    ]
    out += mirror([
        ball(6, "#fbf6ea", 22, 348, 168, n="head"),
        ball(3.5, "#1d1626", 24, 349, 172, n="head"),
        part("cone", (34, 352, 128), (14, 30, 6), HIDE, "head", (0, -250, 0)),
        seg((14, 366, 136), (56, 420, 124), 9, ANT2, "head"),
        seg((56, 420, 124), (100, 450, 96), 8, ANT2, "head"),
        part("sphere", (130, 472, 78), (130, 26, 96), ANT, "head", (0, 110, 0)),
        part("sphere", (84, 448, 130), (60, 20, 50), ANT, "head", (-120, 60, 0)),
        seg((100, 470, 40), (96, 524, 20), 8, ANT, "head", mesh="cone"),
        seg((136, 480, 44), (146, 540, 30), 8, ANT, "head", mesh="cone"),
        seg((170, 486, 64), (196, 540, 54), 8, ANT, "head", mesh="cone"),
        seg((186, 484, 100), (224, 524, 104), 8, ANT, "head", mesh="cone"),
        seg((160, 480, 118), (180, 520, 140), 7, ANT, "head", mesh="cone"),
        seg((90, 452, 150), (100, 474, 186), 7, ANT, "head", mesh="cone"),
    ])
    for x in (36, -36):
        out += hoofleg((x, 210, 70), (x, 110, 86), (x, 0, 80), 20, HIDE, HOOF)
        out += hoofleg((x, 210, -90), (x, 110, -110), (x, 0, -96), 22, HIDE, HOOF)
    for (z, y) in [(-60, 266), (0, 280), (40, 270)]:
        out += mirror([part("cube", (50, y - 40, z), (4, 30, 40), HIDE2, t=(0, 40, 0))])
    out += chain([(0, 230, -126), (0, 210, -144)], [14], HIDE, "tail")
    return out


# ---------------------------------------------------------------- fliers

def bird():
    """The white Bird token (Seller of Songbirds, Eyes in the Skies): a
    round white bird with spread wings, gold beak and legs."""
    W, W2, GOLD = "#f6f2e6", "#c8c4b8", "#f2b43a"
    out = [
        ball(26, W, 0, 120, 0, 0.9, 0.85, 1.2),
        ball(20, W, 0, 140, 26, n="head"),
        part("cone", (0, 136, 50), (12, 18, 10), GOLD, "head", (250, 0, 0)),
        ball(14, W2, 0, 112, -34, 1.2, 0.4, 1.2, n="tail"),
        part("cone", (0, 114, -48), (36, 30, 10), W, "tail", (-250, 0, 0)),
        ball(18, "#fffdf6", 0, 114, 14, 0.8, 0.8, 0.6),
        part("cone", (0, 162, 20), (8, 18, 14), W2, "head", (-80, 0, 0)),
        ball(6, GOLD, 0, 136, 44, 1.2, 0.5, 0.6, n="head"),
    ]
    out += look(10, 146, 40, 4.5)
    out += mirror([
        part("sphere", (36, 128, -4), (54, 14, 44), W, "wing_l", (0, 50, 0)),
        part("sphere", (74, 146, -10), (54, 11, 38), W, "wing_l", (0, 90, 0)),
        part("sphere", (108, 166, -16), (40, 9, 28), W2, "wing_l", (0, 120, 0)),
        seg((10, 104, 0), (10, 84, 4), 3, GOLD),
    ])
    return out


def niv():
    """Niv-Mizzet, Dracogenius: a great red dragon rearing on spread wings,
    long serpentine neck, a fan of blue frill with red spines round his head,
    long whiskers, clawed hands, a long tail; the Izzet's own genius."""
    RED, RED2, DEEP, BELLY, BLUE, BLUE2, HORN, CLAW, WHISK = (
        "#c23a28", "#a02a20", "#7a1e1a", "#e89a58", "#3f6fd8", "#2a4aa8", "#efe0c0", "#f4ead6", "#f0a040")
    out = [
        # body: upright chest, belly and haunches, raised a little off the ground
        ball(110, RED, 0, 230, -40, 0.95, 1.05, 0.9),
        ball(90, RED, 0, 330, 10, 0.95, 0.95, 0.85),
        ball(78, BELLY, 0, 250, 20, 0.85, 1.1, 0.6),
        ball(60, BELLY, 0, 340, 50, 0.8, 0.9, 0.5),
        ball(96, RED2, 0, 150, -70, 1.1, 0.75, 1.0),
    ]
    for y in (210, 270, 330):
        out.append(ball(48 - (y - 200) / 12, "#f0b070", 0, y, 54 - abs(y - 280) / 6, 1, 0.22, 0.5))
    neck = [(0, 380, 40), (0, 430, 66), (0, 476, 60), (0, 516, 46), (0, 552, 60), (0, 576, 92)]
    out += chain(neck, [56, 48, 42, 38, 34], RED)
    out += chain([(0, 380, 76), (0, 440, 100), (0, 500, 84)], [30, 24], BELLY)
    for p in neck[1:4]:
        out.append(part("cone", (p[0], p[1] + 20, p[2] - 46), (14, 34, 14), RED2, t=(-200, 0, 0)))
    for (y, z) in [(330, -80), (270, -128), (200, -150), (140, -150)]:
        out.append(part("cone", (0, y, z), (16, 40, 16), RED2, t=(-300, 0, 0)))
    # head
    H = "head"
    out += [
        ball(40, RED, 0, 596, 104, 0.95, 0.85, 1.05, n=H),
        seg((0, 590, 120), (0, 570, 196), 26, RED, H, mesh="sphere", extra=16),
        ball(20, BELLY, 0, 560, 170, 0.9, 0.5, 1.6, n=H),
        ball(5, DEEP, -9, 580, 206, n=H),
        ball(5, DEEP, 9, 580, 206, n=H),
        ball(30, BLUE, 0, 610, 66, 1, 1, 0.35, n=H),
    ]
    # the frill: blue fins with red spines, a fan behind and around the head
    for i in range(11):
        a = math.radians(-115 + i * 23)
        dx, dy = math.sin(a), math.cos(a)
        base = (dx * 24, 604 + dy * 22, 74)
        tip = (dx * 112, 604 + dy * 92 - 6, 44)
        out.append(seg(base, tip, 26, BLUE if i % 2 else BLUE2, H, mesh="sphere", rz=6, extra=10))
        out.append(seg(base, (tip[0] * 1.08, 604 + (tip[1] - 604) * 1.08, 48), 6, RED, H, mesh="cone"))
    out += mirror([
        ball(10, "#ffd84a", 26, 604, 140, n=H, g=900),
        ball(5, "#1d1626", 29, 605, 148, n=H),
        ball(12, RED2, 26, 616, 136, 1.3, 0.5, 1.2, n=H),
        seg((22, 620, 100), (44, 680, 50), 9, HORN, H, mesh="cone"),
        seg((10, 576, 196), (60, 560, 230), 4.5, WHISK, H),
        seg((60, 560, 230), (120, 490, 226), 4, WHISK, H, mesh="cone"),
        seg((28, 560, 160), (60, 520, 170), 8, RED2, H, mesh="cone"),
    ])
    # arms: clawed hands held before the chest
    out += mirror([
        ball(36, RED, 70, 330, 30),
        seg((76, 320, 40), (96, 240, 90), 24, RED, mesh="sphere", extra=16),
        seg((96, 240, 90), (82, 190, 160), 18, RED2, mesh="sphere", extra=12),
        ball(20, RED2, 80, 186, 170),
        seg((70, 180, 180), (64, 150, 210), 5, CLAW, mesh="cone"),
        seg((82, 180, 184), (84, 148, 214), 5, CLAW, mesh="cone"),
        seg((94, 182, 180), (104, 152, 206), 5, CLAW, mesh="cone"),
    ])
    # hind legs, thighs and clawed feet down to the ground
    out += mirror([
        ball(64, RED2, 96, 150, -30, 0.8, 1.1, 1.2),
        seg((100, 110, -10), (104, 40, 50), 26, RED, mesh="sphere", extra=14),
        ball(28, RED2, 104, 22, 70, 1.1, 0.6, 1.5),
        seg((92, 12, 100), (88, 0, 140), 6, CLAW, mesh="cone"),
        seg((106, 12, 104), (110, 0, 144), 6, CLAW, mesh="cone"),
        seg((118, 12, 96), (132, 0, 128), 6, CLAW, mesh="cone"),
    ])
    # wings: arm bone, finger bones and a red membrane, swept up and back
    W = "wing_l"
    out += mirror([
        seg((70, 380, -60), (230, 540, -120), 18, RED, W, mesh="sphere", extra=10),
        seg((230, 540, -120), (420, 620, -170), 13, RED, W, mesh="sphere", extra=8),
        seg((420, 620, -170), (450, 668, -176), 9, HORN, W, mesh="cone"),
        seg((420, 620, -170), (520, 360, -200), 7, RED2, W),
        seg((420, 620, -170), (380, 270, -176), 7, RED2, W),
        seg((230, 540, -120), (250, 270, -136), 7, RED2, W),
        part("sphere", (400, 450, -184), (230, 340, 12), DEEP, W, (0, 60, 60)),
        part("sphere", (280, 420, -152), (180, 280, 12), RED2, W, (0, -20, 60)),
        part("sphere", (150, 420, -104), (140, 230, 12), DEEP, W, (0, -110, 60)),
    ])
    # tail: long and curling round on the ground
    tail = [(0, 120, -140), (0, 70, -250), (20, 34, -350), (90, 22, -430), (190, 18, -450), (280, 18, -410), (330, 22, -340)]
    out += chain(tail, [62, 50, 40, 30, 22, 16], RED, "tail")
    for p in tail[1:6]:
        out.append(part("cone", (p[0], p[1] + 30, p[2]), (12, 28, 12), RED2, "tail"))
    out.append(seg((330, 22, -340), (350, 30, -280), 22, BLUE, "tail", mesh="cone", rz=6))
    return out


MODELS = {
    "c118": herald(),
    "c122": drudge_beetle(),
    "c117": brushstrider(),
    "c129": monitor(),
    "c137": indrik(),
    "c120": goliath(),
    "c132": rhino("#8a8448", "#6e6a3a", "#9a9070", "#d8cca0", False),
    "c116": stag(),
    "c114": archweaver(),
    "c183": scaled(niv(), 0.93),
    "c215": weird(),
    "c163": goblin_mage(),
    "c158": mangler(),
    "c198": scorpion(),
    "c214": dryad(),
    "c148": centaur("#3a2a22", "#8a5a3a", "#4a4a3a", "#2a2018", True),
    "c234": sweeper(),
    "token_bird": bird(),
    "token_soldier": soldier(),
    "token_goblin": goblin_token(),
    "token_centaur": centaur("#9a6a3a", "#e0b088", "#4f8a34", "#5a3a22", False),
    "token_rhino": rhino("#8a7a5a", "#6a5a44", "#cfcab4", "#efe6cc", True),
    "token_knight": knight(),
}
