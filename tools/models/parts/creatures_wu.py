"""White, blue and Azorius creatures of Return to Ravnica, after each card's art.

Shared builders (a chunky person, a feathered wing, a bat wing, a lion body,
a bird body) and one function per card. Sizes in cm; models face +z.
"""
import math

from common import ball, box, cyl, part, mirror, eyes

EYE = "#1d1626"
SKIN = "#f2c9a6"
TAN_SKIN = "#d9a27c"
SILVER = "#cdd2dc"
STEEL = "#8e96aa"
AZ_BLUE = "#2f5fb0"
AZ_WHITE = "#f3ecd2"
GOLD = "#f2c14e"


# ---------------------------------------------------------------- helpers

def _turn(rad):
    return rad / (2 * math.pi) * 1000


def limb(a, b, r, c, n="body", mesh="sphere", r2=None):
    """A part stretched from point a to point b, r thick (r2 deep)."""
    dx, dy, dz = b[0] - a[0], b[1] - a[1], b[2] - a[2]
    L = math.sqrt(dx * dx + dy * dy + dz * dz) or 1
    roll = math.asin(max(-1, min(1, -dx / L)))
    pitch = math.atan2(dz, dy)
    mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, (a[2] + b[2]) / 2)
    extra = 2 * r if mesh == "sphere" else 0
    return part(mesh, mid, (2 * r, L + extra * 0.5, 2 * (r2 or r)), c, n, (_turn(pitch), _turn(roll), 0))


def move(parts, dx=0, dy=0, dz=0, k=1.0):
    """Scale parts by k about the origin, then shift them."""
    out = []
    for p in parts:
        q = dict(p)
        q["p"] = [round(p["p"][0] * k + dx), round(p["p"][1] * k + dy), round(p["p"][2] * k + dz)]
        q["s"] = [max(1, round(v * k)) for v in p["s"]]
        out.append(q)
    return out


def node(parts, n):
    out = []
    for p in parts:
        q = dict(p)
        if n == "body":
            q.pop("n", None)
        else:
            q["n"] = n
        out.append(q)
    return out


ARM = {
    # elbow, hand (for the +x side; mirrored for -x)
    "down": ((40, 86, 3), (42, 66, 8)),
    "fwd": ((40, 94, 16), (30, 96, 36)),
    "up": ((42, 126, 4), (44, 150, 10)),
    "out": ((54, 102, 8), (72, 108, 16)),
    "hip": ((44, 88, -2), (34, 70, 12)),
    "raise": ((48, 118, 14), (52, 136, 30)),
    "reach": ((44, 100, 22), (44, 104, 44)),
}


def arm(side, pose, sleeve, hand, r=10, glove=None):
    sx = 1 if side == "l" else -1
    e, h = ARM[pose]
    sh = (32 * sx, 108, 0)
    e = (e[0] * sx, e[1], e[2])
    h = (h[0] * sx, h[1], h[2])
    return [limb(sh, e, r, sleeve), limb(e, h, r * 0.9, glove or sleeve), ball(r * 0.95, hand, *h)], h


def person(shirt, pants, boots, skin=SKIN, sleeve=None, hand=None, pose=("down", "down"),
           hips=None, seated=False, glove=None):
    """A chunky person ~170 cm to the crown, facing +z. Returns parts and hand points."""
    sleeve = sleeve or shirt
    hand = hand or skin
    out = []
    if seated:
        for sx in (1, -1):
            out += [limb((13 * sx, 56, 0), (24 * sx, 46, 24), 12, pants),
                    limb((24 * sx, 46, 24), (26 * sx, 18, 18), 10, pants),
                    ball(11, boots, 26 * sx, 12, 24, 1, 0.7, 1.4)]
    else:
        for sx in (1, -1):
            out += [limb((13 * sx, 14, 0), (14 * sx, 54, 0), 12, pants),
                    ball(12, boots, 13 * sx, 8, 5, 1, 0.7, 1.45)]
    out += [
        ball(28, hips or pants, 0, 60, 0, 1.0, 0.55, 0.85),
        ball(31, shirt, 0, 86, 0, 1, 1.0, 0.8),
        ball(27, skin, 0, 140, 3, n="head"),
    ] + eyes(142, 28, 10, 4.5)
    al, hl = arm("l", pose[0], sleeve, hand, glove=glove)
    ar, hr = arm("r", pose[1], sleeve, hand, glove=glove)
    return out + al + ar, {"l": hl, "r": hr}


def feather_wing(x, y, z, span, c, tip=None, up=70, sweep=40, chord=None, n=4, edge=None):
    """A wing on wing_l reaching +x from (x, y, z): a fan of long feathers."""
    tip = tip or c
    chord = chord or span * 0.5
    out = []
    a = up / 1000 * 2 * math.pi
    cx, cy = x + math.cos(a) * span / 2, y + math.sin(a) * span / 2
    out.append(part("sphere", (cx - span * 0.1, cy + 2, z + chord * 0.05), (span * 0.8, 14, chord * 0.75), c, "wing_l",
                    (0, up, sweep * 0.2)))
    for i in range(n):
        f = i / max(1, n - 1)
        L = span * (1.0 - 0.3 * f)
        out.append(part("sphere", (cx + span * (0.04 - 0.12 * f), cy - 2 - 3 * f - span * 0.05 * f,
                                   z - chord * (0.15 + 0.35 * f)),
                        (L, 8, chord * 0.34), tip if i < n - 1 or edge is None else edge, "wing_l",
                        (0, up - 10 * f, sweep * (0.1 + 0.5 * f))))
    return out


def bat_wing(x, y, z, span, c, bone, up=80, sweep=60):
    """A drake's wing on wing_l: a leathery sail with three finger spurs and a ragged edge."""
    a = up / 1000 * 2 * math.pi
    cx, cy = x + math.cos(a) * span / 2, y + math.sin(a) * span / 2
    out = [part("sphere", (cx, cy, z - span * 0.15), (span * 0.95, 10, span * 0.55), c, "wing_l", (0, up, sweep * 0.4)),
           part("sphere", (cx, cy + 5, z + span * 0.08), (span * 1.0, 12, 14), bone, "wing_l", (0, up, 0))]
    # the ragged trailing edge
    for i in range(4):
        f = i / 3
        out.append(part("sphere", (x + math.cos(a) * span * (0.2 + 0.75 * f), y + math.sin(a) * span * (0.2 + 0.75 * f) - 3,
                                   z - span * (0.36 + 0.1 * f)), (span * 0.16, 8, span * 0.16), c, "wing_l", (0, up, 125)))
    return out


def lion_body(fur, belly, y, length=150, w=70, paw=None, tail_tip=None):
    """A four-legged cat body around height y, front at +z (no head)."""
    paw = paw or fur
    out = [ball(w / 2, fur, 0, y, 0, 1, 0.85, length / w * 0.62),
           ball(w * 0.48, fur, 0, y + 6, length * 0.28, 1, 0.95, 1),
           ball(w * 0.38, belly, 0, y - w * 0.18, length * 0.05, 0.9, 0.5, 1.6)]
    for sx in (1, -1):
        out += [limb((w * 0.3 * sx, y - 5, -length * 0.3), (w * 0.33 * sx, y - 40, -length * 0.42), 13, fur),
                limb((w * 0.33 * sx, y - 40, -length * 0.42), (w * 0.3 * sx, y - 70, -length * 0.5), 9, fur),
                ball(10, paw, w * 0.3 * sx, y - 74, -length * 0.48, 1, 0.7, 1.3)]
    out += [limb((0, y + 5, -length * 0.4), (0, y - 20, -length * 0.8), 6, fur, n="tail"),
            limb((0, y - 20, -length * 0.8), (0, y - 10, -length * 1.05), 5, fur, n="tail"),
            ball(9, tail_tip or fur, 0, y - 8, -length * 1.1, 1, 1, 1.3, n="tail")]
    return out


def eagle_head(feather, beak, x, y, z, r=24, n="head", eye=EYE):
    return [ball(r, feather, x, y, z, 1, 0.95, 1.1, n=n),
            ball(r * 0.8, feather, x, y - r * 0.2, z - r * 0.5, 1.1, 1, 1, n=n),
            part("cone", (x, y - r * 0.15, z + r * 1.05), (r * 0.6, r * 1.1, r * 0.6), beak, n, (240, 0, 0)),
            ball(r * 0.33, beak, x, y + r * 0.05, z + r * 0.75, 1.1, 0.9, 1, n=n),
            ball(r * 0.17, eye, x + r * 0.58, y + r * 0.22, z + r * 0.55, n=n),
            ball(r * 0.17, eye, x - r * 0.58, y + r * 0.22, z + r * 0.55, n=n)]


def talons(x, y, z, c, leg, length=40, r=8):
    out = []
    for sx in (1, -1):
        out += [limb((x * sx, y, z), (x * sx, y - length, z + length * 0.4), r, leg),
                ball(r * 1.2, c, x * sx, y - length - 3, z + length * 0.45, 1.1, 0.7, 1.3)]
    return out


# ---------------------------------------------------------------- cards

def trained_caracal():
    fur, dark, white = "#d9772e", "#3a2016", "#f4e6d0"
    out = [
        ball(20, fur, 0, 30, -4, 0.85, 0.75, 1.6),
        ball(16, fur, 0, 33, 16, 1, 1, 1),
        ball(12, white, 0, 24, 6, 0.9, 0.5, 1.8),
    ]
    for sx in (1, -1):
        for zz, lift in ((20, 0), (-26, 0)):
            out += [limb((9 * sx, 26, zz), (9 * sx, 6, zz + 2), 5.5, fur),
                    ball(5.5, white, 9 * sx, 4, zz + 4, 1, 0.7, 1.3)]
        out.append(cyl(5.8, 4, GOLD, 9 * sx, 10, 22))
    # stripes along the back and flanks
    for i, zz in enumerate((-26, -14, -2, 10)):
        out.append(ball(4, dark, 0, 44 - i * 0.5, zz, 2.6, 0.5, 0.6))
        for sx in (1, -1):
            out.append(ball(3.5, dark, 14 * sx, 34, zz + 3, 0.5, 2.0, 0.6))
    # head with the gold mask
    out += [
        ball(13, fur, 0, 46, 34, 1.05, 0.95, 1, n="head"),
        ball(13.6, GOLD, 0, 50, 34, 1.02, 0.7, 0.95, n="head"),
        ball(7, white, 0, 41, 45, 1.1, 0.8, 0.9, n="head"),
        ball(2.5, "#5a2a2a", 0, 44, 51, n="head"),
        part("gem", (0, 56, 46), (6, 7, 4), "#9a3ab0", "head"),
    ] + eyes(48, 45, 6, 2.6)
    for sx in (1, -1):
        out += [part("cone", (8 * sx, 63, 31), (10, 20, 6), fur, "head", (0, -30 * sx, 0)),
                part("cone", (8.3 * sx, 63, 33.5), (6, 14, 2), "#f4d8c0", "head", (0, -30 * sx, 0)),
                part("cone", (10.3 * sx, 75, 31), (4, 9, 4), dark, "head", (0, -30 * sx, 0))]
    out += [limb((0, 36, -30), (0, 22, -48), 3.5, fur, n="tail"),
            limb((0, 22, -48), (0, 26, -64), 3.2, fur, n="tail"),
            ball(3.8, dark, 0, 22, -48, n="tail"), ball(3.6, dark, 0, 28, -66, n="tail")]
    return out


def azorius_arrester():
    out, h = person(SILVER, "#41558a", STEEL, pose=("down", "fwd"), sleeve=STEEL, hand=STEEL, hips=STEEL)
    out += [
        ball(29, SILVER, 0, 142, 1, 1.02, 1.08, 1.02, n="head"),
        box(30, 6, 6, "#1d1626", 0, 140, 28, n="head"),
        box(5, 22, 6, "#1d1626", 0, 130, 27, n="head"),
        part("cone", (0, 168, 0), (12, 16, 30), STEEL, "head"),
        ball(18, SILVER, 34, 112, 0, 1, 0.75, 1), ball(18, SILVER, -34, 112, 0, 1, 0.75, 1),
        ball(15, GOLD, 0, 66, 18, 1.5, 0.35, 0.4),
        # the great blue cloak sweeping behind and to one side
        ball(40, AZ_BLUE, -6, 80, -26, 1.2, 1.55, 0.4, t=(-40, 30, 0)),
        ball(30, AZ_BLUE, 32, 60, -20, 0.6, 1.4, 0.6, t=(0, 60, 0)),
        ball(26, "#23468a", -34, 50, -10, 0.6, 1.5, 0.7, t=(0, -60, 0)),
        ball(26, AZ_BLUE, 0, 116, -14, 1.5, 0.45, 0.8),
        # parchment warrant bands
        box(70, 7, 4, AZ_WHITE, 0, 96, 24, t=(0, 110, 0)),
        box(50, 6, 4, AZ_WHITE, 0, 74, 25, t=(0, -60, 0)),
        ball(13, STEEL, h["r"][0], h["r"][1], h["r"][2]),
        cyl(3, 22, "#3a2a20", h["r"][0], h["r"][1] - 4, h["r"][2] + 4, t=(250, 0, 0)),
    ]
    return out


def concordia_pegasus():
    grey, dapple, black = "#e4e2dc", "#b9b8b6", "#2a2630"
    out = [
        ball(42, grey, 0, 150, -4, 0.85, 0.8, 1.45, t=(-70, 0, 0)),
        ball(33, grey, 0, 168, 34, 0.95, 1, 0.95),
        ball(10, dapple, 22, 150, -20), ball(9, dapple, -24, 156, 0), ball(8, dapple, 18, 140, 20),
        limb((0, 175, 42), (0, 214, 66), 17, grey),
    ]
    # rearing forelegs, dangling hind legs with black stockings
    for sx in (1, -1):
        out += [limb((15 * sx, 158, 46), (15 * sx, 140, 70), 9, grey),
                limb((15 * sx, 140, 70), (15 * sx, 116, 60), 7, black),
                ball(7, black, 15 * sx, 112, 62, 1, 0.8, 1.2),
                limb((18 * sx, 130, -38), (18 * sx, 102, -30), 11, grey),
                limb((18 * sx, 102, -30), (18 * sx, 72, -50), 7, black),
                ball(8, black, 18 * sx, 66, -52, 1, 0.8, 1.2)]
    out += [
        ball(15, grey, 0, 222, 72, 0.9, 1, 1.1, n="head"),
        ball(11, grey, 0, 210, 94, 0.85, 0.85, 1.4, n="head", t=(90, 0, 0)),
        ball(3, black, 4, 208, 110, n="head"), ball(3, black, -4, 208, 110, n="head"),
        part("cone", (6, 240, 66), (6, 14, 5), grey, "head", (-40, -40, 0)),
        part("cone", (-6, 240, 66), (6, 14, 5), grey, "head", (-40, 40, 0)),
        ball(10, black, 0, 232, 60, 0.6, 0.8, 1.2, n="head"),
    ] + eyes(226, 80, 12, 3.4)
    # black mane down the neck
    for i, (yy, zz) in enumerate(((218, 52), (204, 44), (190, 36))):
        out.append(ball(11, black, 0, yy, zz, 0.6, 1, 1.1, t=(-120, 0, 0)))
    out += [limb((0, 150, -54), (0, 128, -96), 9, black, n="tail"),
            limb((0, 128, -96), (0, 104, -112), 8, black, n="tail")]
    out += mirror(feather_wing(18, 176, 4, 165, "#3a3440", "#26222c", up=90, sweep=50))
    return out


def precinct_captain():
    leather, shirt = "#7a4a2a", "#e8e2d4"
    out, h = person(leather, "#5a3a24", "#3a2a1e", sleeve=shirt, pose=("down", "down"), glove="#5a3a24")
    out += [
        ball(31.5, "#9a6236", 0, 86, 2, 0.9, 0.95, 0.8),
        box(16, 60, 6, shirt, 0, 90, 24),
        ball(19, "#7a6c94", 34, 110, 0, 1.1, 0.8, 1.1),
        ball(13, SILVER, 34, 114, 2, 1.1, 0.5, 1.1),
        ball(16, "#9a6236", -34, 110, 0, 1, 0.7, 1),
        ball(29, "#4a3a2e", 0, 64, 0, 1.0, 0.25, 0.88),
        part("gem", (0, 64, 25), (10, 10, 4), SILVER),
        # short-cropped grey-brown hair and a heavy brow
        ball(27.5, "#6a5a4a", 0, 147, -1, 1, 0.75, 1, n="head"),
        box(30, 4, 6, "#4a3a2e", 0, 149, 27, n="head"),
        ball(5, TAN_SKIN, 0, 136, 30, n="head"),
        # sword in the right hand pointing down
        cyl(3, 70, SILVER, h["r"][0], h["r"][1] - 72, h["r"][2] + 6),
        box(18, 4, 5, "#3a2a1e", h["r"][0], h["r"][1] - 2, h["r"][2] + 4),
    ]
    return out


def fencing_ace():
    bronze, dark = "#b07a3c", "#6a4220"
    out, h = person(bronze, dark, "#3a2416", sleeve=bronze, pose=("up", "out"), glove=dark, hips=bronze)
    out += [
        ball(15, bronze, 34, 110, 0, 1.1, 0.7, 1), ball(15, bronze, -34, 110, 0, 1.1, 0.7, 1),
        ball(20, "#c99a54", 0, 92, 18, 1.2, 1, 0.5),
        ball(28.5, "#1d1626", 0, 147, -2, 1, 0.8, 1, n="head"),
        ball(9, "#1d1626", 0, 130, 26, 1.1, 0.6, 0.5, n="head"),
        ball(30, dark, 0, 50, -6, 1.05, 0.45, 0.8),
        # rapier held high, pointing forward
        ball(6, GOLD, *h["r"]),
        limb(h["r"], (h["r"][0] - 10, h["r"][1] + 24, h["r"][2] + 100), 2.6, "#e8ecf4", mesh="cyl"),
        ball(8, GOLD, h["r"][0] - 1, h["r"][1] + 4, h["r"][2] + 3, 1, 1, 0.5),
        ball(4, GOLD, h["r"][0], h["r"][1] - 2, h["r"][2] + 2),
    ]
    return out


def seller_of_songbirds():
    red, fur, spot = "#b8322a", "#c89a50", "#5a3220"
    out, h = person(red, red, "#5a2a20", sleeve=red, pose=("raise", "down"), hand=SKIN)
    out += [
        part("frustum", (0, 40, 0), (66, 80, 56), red),
        ball(24, fur, 30, 92, -2, 0.7, 1.6, 0.9), ball(24, fur, -30, 92, -2, 0.7, 1.6, 0.9),
        ball(30, fur, 0, 90, -14, 1.2, 1.2, 0.5),
        part("frustum", (0, 36, -10), (72, 72, 46), fur),
        ball(5, spot, 34, 96, 12), ball(5, spot, -32, 86, 16), ball(5, spot, 26, 70, 18),
        ball(5, spot, -36, 108, 6), ball(5, spot, 0, 50, -34), ball(6, spot, 24, 30, -28),
        ball(5, spot, -28, 40, -26), ball(4, GOLD, 0, 116, 24),
        # dark hair, blue feather crown and little white wings
        ball(28, "#2a2a3a", 0, 146, -4, 1.05, 0.9, 1, n="head"),
        ball(13, "#2a2a3a", 0, 124, -16, 1.4, 1, 0.8, n="head"),
    ] + mirror([
        part("cone", (10, 182, -4), (12, 40, 10), "#2a4aa8", "head", (0, -60, 0)),
        part("cone", (0 + 4, 186, -6), (12, 46, 10), "#3a6ad0", "head", (0, -20, 0)),
        part("cone", (16, 172, -2), (10, 26, 8), "#c8a030", "head", (0, -120, 0)),
        part("sphere", (34, 168, -6), (30, 10, 14), "#f4efe4", "head", (0, 120, 60)),
    ])
    # robins: one on the raised hand, one on the shoulder
    for (x, y, z) in (h["l"], (-32, 118, 6)):
        out += [ball(8, "#e0552c", x, y + 12, z + 2, 1, 1, 1.2),
                ball(7, "#6a4a32", x, y + 16, z - 4, 1, 0.8, 1.3),
                ball(5.5, "#e0552c", x, y + 22, z + 7),
                part("cone", (x, y + 21, z + 13), (3, 5, 3), "#3a2a20", "body", (250, 0, 0)),
                ball(1.6, EYE, x + 3.5, y + 23, z + 10), ball(1.6, EYE, x - 3.5, y + 23, z + 10),
                part("cube", (x, y + 10, z - 12), (6, 2, 10), "#6a4a32", "body", (-80, 0, 0))]
    return out


def griffin(fur, belly, head, beak, wing, wing_tip, y=120, k=1.0, saddle=None, tail_tip=None):
    out = lion_body(fur, belly, y, length=150, w=72, tail_tip=tail_tip)
    out += eagle_head(head, beak, 0, y + 50, 82, r=30)
    out += [limb((0, y + 10, 46), (0, y + 40, 70), 22, head)]
    out += talons(18, y - 10, 50, beak, head, length=40, r=8)
    if saddle:
        out += [ball(38, saddle, 0, y + 20, 4, 1.05, 0.35, 1.1)]
    out += mirror(feather_wing(28, y + 22, 20, 190 * k, wing, wing_tip, up=80, sweep=55))
    return out


def sunspire_griffin():
    out = griffin("#5a3a26", "#7a5434", "#3e2a1e", "#e0b040", "#3a2618", "#24160e", saddle="#6a7a94")
    out += [box(30, 4, 60, "#4a5670", 0, 152, -10), ball(5, GOLD, 0, 156, 20)]
    return out


def azorius_justiciar():
    grey, robe = "#b8bcc6", "#9aa0ac"
    out, h = person(SILVER, robe, STEEL, pose=("down", "fwd"), sleeve=STEEL, hand=SKIN)
    out += [
        part("frustum", (0, 36, 0), (70, 72, 56), robe),
        ball(42, "#c8ccd4", 0, 84, -22, 1.15, 1.5, 0.4, t=(-30, 0, 0)),
        ball(24, grey, 32, 108, 2, 1, 0.9, 1.1), ball(22, grey, -32, 108, 2, 1, 0.85, 1.05),
        ball(5, "#3a7ad8", 30, 116, 24, g=500), ball(5, "#3a7ad8", -30, 114, 22, g=500),
        ball(5, "#3a7ad8", 0, 62, 26, g=500),
        box(8, 50, 4, "#6a5440", 8, 90, 24, t=(0, 80, 0)),
        # helm and long blonde hair
        ball(28.5, SILVER, 0, 148, 1, 1.02, 0.85, 1.02, n="head"),
        part("cone", (0, 172, 2), (10, 18, 22), STEEL, "head"),
        box(28, 5, 4, SILVER, 0, 150, 27, n="head"),
        ball(22, "#e8c060", 0, 118, -18, 1.3, 1.6, 0.6, n="head"),
        ball(12, "#e8c060", 24, 126, -4, 0.6, 1.6, 0.7, n="head"),
        ball(12, "#e8c060", -24, 126, -4, 0.6, 1.6, 0.7, n="head"),
        # the long staff
        cyl(4, 200, "#8a5a35", h["r"][0], 0, h["r"][2] + 4),
        ball(8, "#a8703f", h["r"][0], 206, h["r"][2] + 4),
        cyl(6, 12, "#6a4426", h["r"][0], 170, h["r"][2] + 4),
    ]
    return out


def phantom_general():
    fire, deep, pale = "#f07a2a", "#c84a1a", "#ffc27a"
    out, h = person(fire, fire, fire, skin=pale, pose=("down", "up"), sleeve=fire, hand=pale)
    out = [p for p in out if p["p"][1] > 40 or p["m"] != "sphere" or p["s"][0] > 40]
    out = [dict(p, g=450) for p in out]
    out += [
        part("cone", (0, 30, -4), (50, 70, 40), deep, "body", (500, 0, 0), g=400),
        ball(28.5, "#f6a04a", 0, 150, 1, 1.02, 0.8, 1.02, n="head", g=300),
        part("cone", (0, 182, -6), (10, 36, 40), deep, "head", (-60, 0, 0), g=400),
        ball(18, "#f6a04a", 34, 110, 0, 1, 0.75, 1, g=300), ball(18, "#f6a04a", -34, 110, 0, 1, 0.75, 1, g=300),
        # flame cape streaming behind
        part("cone", (0, 110, -40), (60, 90, 16), deep, "body", (-320, 0, 0), g=500),
        part("cone", (26, 90, -48), (30, 80, 12), fire, "body", (-300, -40, 0), g=500),
        part("cone", (-26, 90, -48), (30, 80, 12), fire, "body", (-300, 40, 0), g=500),
        part("cone", (0, 70, -60), (36, 70, 10), "#ffb050", "body", (-280, 0, 0), g=600),
        part("cone", (20, 176, -12), (14, 30, 10), "#ffb050", "head", (-80, -80, 0), g=600),
        part("cone", (-20, 176, -12), (14, 30, 10), "#ffb050", "head", (-80, 80, 0), g=600),
        part("cone", (40, 126, -14), (14, 34, 10), fire, "body", (-60, -120, 0), g=600),
        part("cone", (-40, 126, -14), (14, 34, 10), fire, "body", (-60, 120, 0), g=600),
        # a ghostly blade raised
        limb(h["r"], (h["r"][0] - 6, h["r"][1] + 70, h["r"][2] + 20), 3.5, "#ffe0a0", mesh="cyl"),
        box(22, 4, 6, deep, h["r"][0], h["r"][1] + 2, h["r"][2]),
    ]
    out = move(out, dy=24)
    for p in out:
        if p["c"] in (fire, deep, pale) and "g" not in p:
            p["g"] = 450
    return out


def angel_of_serenity():
    armour, hair, wing = "#c8ccd6", "#f0d080", "#f2e8cc"
    out, h = person(armour, "#9aa2b2", STEEL, pose=("raise", "down"), sleeve=STEEL, hand=SKIN, hips=STEEL)
    out += [
        ball(17, armour, 34, 110, 0, 1.1, 0.8, 1), ball(17, armour, -34, 110, 0, 1.1, 0.8, 1),
        ball(22, "#e6e8ee", 0, 92, 16, 1.2, 1, 0.5),
        part("frustum", (0, 40, 0), (60, 50, 46), "#8a92a4", "body", (500, 0, 0)),
        ball(30, hair, 0, 148, -4, 1.1, 0.95, 1, n="head"),
        ball(18, hair, 0, 120, -18, 1.4, 1.2, 0.8, n="head"),
        # sword down, light in the raised hand
        limb(h["r"], (h["r"][0] - 4, h["r"][1] - 80, h["r"][2] + 30), 4, SILVER, mesh="cyl"),
        box(22, 4, 6, GOLD, h["r"][0], h["r"][1] - 4, h["r"][2] + 2),
        ball(10, "#ff6a8a", h["l"][0], h["l"][1] + 6, h["l"][2] + 4, g=900),
        part("gem", (h["l"][0], h["l"][1] + 26, h["l"][2] + 6), (8, 24, 8), "#ffb0c0", g=900),
    ]
    # halo: a ring of light behind the head
    out.append(part("cyl", (0, 150, -26), (88, 3, 88), "#ffc8d4", "head", (250, 0, 0), g=700))
    out.append(part("cyl", (0, 150, -24), (66, 4, 66), "#fff0e0", "head", (250, 0, 0), g=800))
    out += mirror(feather_wing(24, 112, -22, 170, wing, "#e4d6b0", up=110, sweep=60, n=5))
    return move(out, dy=40, k=1.55)


def faerie_impostor():
    skin, hair, armour, pants = "#8a5a4a", "#7a3a8c", SILVER, "#c04a6a"
    out, h = person(armour, pants, "#4a2a3a", skin=skin, sleeve=skin, pose=("out", "down"))
    out += [
        ball(31, hair, 0, 148, -6, 1.1, 0.95, 1.05, n="head"),
        ball(18, hair, 26, 132, -10, 0.6, 1.4, 1, n="head"),
        ball(18, hair, -26, 132, -10, 0.6, 1.4, 1, n="head"),
        part("cone", (30, 150, 6), (8, 22, 6), skin, "head", (0, -200, 0)),
        part("cone", (-30, 150, 6), (8, 22, 6), skin, "head", (0, 200, 0)),
        box(70, 5, 4, "#f07090", 0, 60, 20, t=(0, 60, 0)),
        part("cone", (40, 52, -10), (8, 50, 4), "#f07090", "body", (0, 300, 0)),
        part("cone", (-40, 48, -4), (8, 40, 4), "#f07090", "body", (0, -320, 0)),
    ]
    wings = []
    for up, L, zz, c in ((140, 90, -20, "#e4dcf8"), (40, 70, -24, "#c8c0f0")):
        a = up / 1000 * 2 * math.pi
        wings.append(part("sphere", (18 + math.cos(a) * L / 2, 110 + math.sin(a) * L / 2, zz), (L, 4, 34), c,
                          "wing_l", (0, up, 0), g=300))
    out += mirror(wings)
    return move(out, dy=16, k=0.32)


def stealer_of_secrets():
    blue, leather, strap = "#2c4fa0", "#3a5a8e", "#9a6232"
    out, h = person(leather, "#2c3e6a", "#2a2a3a", skin=TAN_SKIN, sleeve=leather, pose=("reach", "down"),
                    glove="#2a2a3a")
    out += [
        ball(31, blue, 0, 148, -4, 1.08, 1.05, 1.05, n="head"),
        ball(20, blue, 0, 120, -16, 1.6, 0.9, 1, n="head"),
        ball(42, blue, 0, 86, -22, 1.05, 1.6, 0.4, t=(-30, 0, 0)),
        ball(24, blue, 40, 70, -10, 0.5, 1.5, 0.7, t=(0, 40, 0)),
        ball(24, blue, -40, 70, -10, 0.5, 1.5, 0.7, t=(0, -40, 0)),
        ball(14, "#b07a40", 0, 86, 22, 1.2, 1.4, 0.3),
        box(44, 4, 5, strap, 0, 98, 25), box(44, 4, 5, strap, 0, 76, 26),
        box(66, 7, 5, strap, 0, 58, 22, t=(0, 40, 0)),
        ball(14, "#2a2a3a", 0, 140, 24, 1.4, 1.2, 0.4, n="head"),
    ] + eyes(142, 28, 10, 4.5)
    # a glowing rune arc in the lowered hand
    x, y, z = h["r"]
    for i in range(5):
        a = (i / 4 - 0.5) * 2.2
        out.append(part("gem", (x + math.sin(a) * 30, y + 10 + math.cos(a) * 14, z + 16 + math.cos(a) * 12),
                        (9, 12, 9), "#6ad8ff", g=900))
    return out


def hover_barrier():
    deep, water, foam = "#1f8a9c", "#3ec8d0", "#a8f4f0"
    out = [
        box(200, 230, 26, water, 0, 160, 0, mesh="cube"),
        box(184, 214, 30, "#2aa8b8", 0, 160, 0, mesh="cube"),
        ball(60, deep, -30, 190, 14, 1, 1, 0.12, g=300),
        ball(40, foam, 20, 140, 16, 1, 1, 0.1, g=500),
        ball(26, water, -40, 110, 17, 1, 1, 0.1, g=400),
        ball(30, foam, 50, 220, 16, 1, 0.6, 0.1, g=500),
        ball(18, deep, 40, 100, 18, 1, 1, 0.1, g=300),
    ]
    out = [dict(p, g=p.get("g", 250)) for p in out]
    # foaming lower edge and drifting spray
    for i in range(7):
        x = -90 + i * 30
        out.append(ball(16, foam if i % 2 else water, x, 46 + (i % 3) * 4, 0, 1, 1.2, 1.4, mesh="ico", g=500))
    for x, y in ((-112, 90), (110, 150), (-108, 230), (112, 260), (-60, 284), (40, 290)):
        out.append(ball(10, foam, x, y, 0, mesh="ico", g=600))
    return out


def runewing():
    white, brown, rune = "#f0ece2", "#6a4830", "#1e1410"
    out = [
        ball(16, white, 0, 80, 0, 0.9, 0.8, 1.6),
        ball(12, brown, 0, 88, -6, 0.9, 0.5, 1.6),
        part("cone", (0, 82, -34), (24, 26, 8), brown, "tail", (-250, 0, 0)),
        ball(11, white, 0, 92, 24, n="head"),
        ball(4, brown, 8, 94, 26, 0.6, 0.6, 1.6, n="head"),
        ball(4, brown, -8, 94, 26, 0.6, 0.6, 1.6, n="head"),
        part("cone", (0, 90, 37), (6, 9, 6), "#2a2a2a", "head", (260, 0, 0)),
        ball(1.8, "#f2c14e", 7, 95, 31, n="head"), ball(1.8, "#f2c14e", -7, 95, 31, n="head"),
    ] + talons(6, 72, 4, "#e0c040", white, length=14, r=3)
    w = feather_wing(12, 88, 0, 110, "#e4d8c0", brown, up=40, sweep=50, chord=44, n=4)
    # dark runes along the wing's top
    for i, f in enumerate((0.3, 0.5, 0.7, 0.88)):
        a = 40 / 1000 * 2 * math.pi
        w.append(part("cube", (12 + math.cos(a) * 110 * f, 90 + math.sin(a) * 110 * f + 9, -6 + (i % 2) * 8),
                      (9, 4, 12), rune, "wing_l", (0, 40, 120 * (i % 2))))
        w.append(ball(4, rune, 12 + math.cos(a) * 110 * f + 7, 90 + math.sin(a) * 110 * f + 9, 4, 1, 0.5, 1,
                      n="wing_l"))
    return out + mirror(w)


def soulsworn_spirit():
    robe, deep, skin = "#c8d4a0", "#8ab0a8", "#a8cce0"
    out = [
        part("frustum", (0, 50, 0), (70, 100, 60), robe),
        part("frustum", (0, 20, 2), (80, 40, 68), deep),
        ball(26, robe, 0, 118, 0, 1, 1.3, 0.9),
        ball(22, robe, 0, 160, 6, 0.9, 1.4, 1, n="head"),
        ball(13, robe, 0, 192, 22, 0.9, 0.9, 1.6, n="head", t=(80, 0, 0)),
        ball(8, deep, 0, 184, 40, 1, 1, 1.2, n="head"),
        ball(14, "#24343a", 0, 156, 22, 0.8, 1.3, 0.4, n="head"),
        ball(3, "#d8ffe0", 5, 160, 27, n="head", g=900), ball(3, "#d8ffe0", -5, 160, 27, n="head", g=900),
        box(50, 6, 50, "#e8ecd0", 0, 132, 2, t=(0, 0, 0)),
    ]
    for sx in (1, -1):
        sh = (24 * sx, 136, 6)
        el = (44 * sx, 120, 50)
        wr = (40 * sx, 136, 104)
        out += [limb(sh, el, 5, skin), limb(el, wr, 4.5, skin), ball(6, skin, *wr)]
        for j, dx in enumerate((-8, 0, 8)):
            tip = (wr[0] + dx * sx, wr[1] + 8 - abs(dx) * 0.6, wr[2] + 30)
            out += [limb(wr, tip, 2.2, skin),
                    part("cone", (tip[0], tip[1] - 3, tip[2] + 4), (4, 9, 4), skin, "body", (330, 0, 0))]
    return out


def voidwielder():
    coat, dark = "#33424e", "#222a34"
    out, h = person(coat, dark, "#1e1e24", sleeve=coat, pose=("up", "out"), glove=dark)
    out += [
        part("frustum", (0, 52, -2), (72, 70, 58), coat),
        ball(32, coat, 0, 110, -4, 1.3, 0.4, 0.9),
        box(5, 50, 4, "#1a1a22", 0, 90, 25),
        ball(27.5, "#5a3a28", 0, 148, -2, 1, 0.8, 1, n="head"),
        ball(14, "#5a3a28", 0, 128, 18, 1.2, 1.0, 0.9, n="head"),
        ball(5, TAN_SKIN, 0, 140, 30, n="head"),
        ball(12, "#8fe8e8", h["r"][0] + 4, h["r"][1] + 18, h["r"][2] + 6, g=900),
        ball(6, "#1a3a8a", h["r"][0] + 4, h["r"][1] + 18, h["r"][2] + 14, g=500),
    ]
    for i in range(5):
        a = i / 5 * 2 * math.pi
        out.append(part("gem", (h["r"][0] + 4 + math.cos(a) * 22, h["r"][1] + 18 + math.sin(a) * 22, h["r"][2] + 6),
                        (6, 6, 6), "#bff4f4", g=800))
    return out


def isperias_skywatch():
    feather, dark, beak = "#7a4a2a", "#4a2c18", "#3a3a40"
    out = [
        ball(46, feather, 0, 130, 0, 1, 0.8, 1.5),
        ball(40, feather, 0, 140, 46, 1.05, 1, 1),
        ball(30, "#a86a3a", 0, 118, 44, 0.9, 0.9, 0.9),
        part("cone", (0, 130, -86), (60, 50, 14), dark, "tail", (-250, 0, 0)),
    ]
    out += eagle_head(feather, "#6a6a74", 0, 162, 108, r=34)
    out += [ball(5, "#7ab8ff", 16, 167, 120, n="head", g=600), ball(5, "#7ab8ff", -16, 167, 120, n="head", g=600)]
    out += talons(22, 104, 18, beak, dark, length=36, r=9)
    out += [ball(42, "#b8a080", 0, 168, 4, 1, 0.3, 1.0),
            ball(10, "#8fc0ff", 0, 138, 82, g=700)]
    out += mirror(feather_wing(36, 152, 10, 220, feather, dark, up=60, sweep=50, n=4))
    # the vedalken knight in the saddle
    r, h = person(SILVER, STEEL, STEEL, skin="#8fb0d8", sleeve=STEEL, pose=("fwd", "down"), seated=True,
                  hand=STEEL)
    r += [ball(28, SILVER, 0, 146, -2, 1, 0.8, 1, n="head"),
          ball(40, "#5a5ab0", 0, 100, -34, 1.3, 1.6, 0.4, t=(-120, 0, 0)),
          ball(14, SILVER, 32, 110, 0), ball(14, SILVER, -32, 110, 0)]
    r += [limb((h["l"][0], h["l"][1], h["l"][2] - 40), (h["l"][0] - 4, h["l"][1] + 30, h["l"][2] + 150), 3, "#d8d8e0",
               mesh="cyl"),
          part("cone", (h["l"][0] - 5, h["l"][1] + 38, h["l"][2] + 178), (10, 34, 6), GOLD, "body", (180, 0, 0))]
    out += move(r, dy=118, dz=-6, k=0.85)
    return out


def skyline_predator():
    scale, belly, wing, bone = "#4a4aa0", "#9a8ac8", "#2c2c6a", "#6a6ab8"
    out = [
        ball(36, scale, 0, 140, 0, 0.9, 0.8, 1.4, t=(-60, 0, 0)),
        ball(26, belly, 0, 128, 14, 0.75, 0.9, 1.1),
        limb((0, 150, 40), (0, 210, 70), 16, scale),
        limb((0, 146, 44), (0, 204, 76), 12, belly),
    ]
    for i in range(4):
        out.append(box(30 - i * 3, 4, 6, "#c0b4e0", 0, 160 + i * 14, 62 + i * 4, t=(-80, 0, 0)))
    out += [
        ball(17, scale, 0, 226, 80, 0.9, 0.85, 1.3, n="head"),
        ball(12, scale, 0, 220, 104, 0.8, 0.6, 1.4, n="head"),
        part("cone", (8, 238, 62), (6, 26, 6), "#c8c8f0", "head", (-300, -30, 0)),
        part("cone", (-8, 238, 62), (6, 26, 6), "#c8c8f0", "head", (-300, 30, 0)),
        part("cone", (0, 236, 76), (8, 16, 8), "#8a8ad0", "head", (-200, 0, 0)),
        ball(3.2, "#ffd060", 11, 230, 92, n="head", g=800), ball(3.2, "#ffd060", -11, 230, 92, n="head", g=800),
    ]
    for sx in (1, -1):
        out += [limb((18 * sx, 126, -20), (20 * sx, 96, 0), 10, scale),
                limb((20 * sx, 96, 0), (20 * sx, 74, -18), 6, scale),
                part("cone", (20 * sx, 66, -14), (8, 14, 8), "#c8c8f0", "body", (500, 0, 0))]
    out += [limb((0, 130, -44), (0, 100, -100), 10, scale, n="tail"),
            limb((0, 100, -100), (0, 92, -160), 6, scale, n="tail"),
            part("cone", (0, 92, -178), (12, 22, 4), bone, "tail", (-250, 0, 0))]
    out += mirror(bat_wing(24, 156, 6, 210, wing, bone, up=90, sweep=70))
    return out


def vassal_soul():
    shroud, veil, shade, band = "#ece6f4", "#f8f6fc", "#c4bcd8", "#d8d0e4"
    out = [
        # the tall wrapped head, its crown bowed forward
        ball(30, shroud, 0, 200, -2, 0.9, 1.6, 0.95, n="head", t=(30, 0, 0)),
        ball(20, shroud, 0, 246, 14, 0.9, 1.2, 0.95, n="head", t=(110, 0, 0)),
        ball(27.5, band, 0, 172, 4, 1.0, 0.25, 1.0, n="head"),
        ball(26.5, band, 0, 192, 6, 1.0, 0.2, 1.0, n="head"),
        ball(11, "#3a3450", 0, 204, 24, 0.95, 1.3, 0.45, n="head"),
        ball(2.8, "#c8f0ff", 4, 207, 28, n="head", g=900), ball(2.8, "#c8f0ff", -4, 207, 28, n="head", g=900),
        # a narrow body under the shroud and the long veil streaming behind
        ball(24, shade, 0, 140, -2, 0.9, 1.3, 0.8),
        limb((0, 176, -8), (0, 100, -40), 30, veil, r2=9),
        limb((0, 110, -36), (0, 40, -84), 38, veil, r2=8),
        limb((0, 50, -76), (0, 8, -130), 46, veil, r2=6),
        ball(40, "#eeeaf6", 0, 6, -140, 1.3, 0.15, 0.8),
    ]
    for sx in (1, -1):
        out += [limb((22 * sx, 156, 4), (30 * sx, 116, 16), 4, shade),
                ball(5, "#9a94ac", 30 * sx, 112, 18)]
        for dx in (-3, 3):
            out.append(limb((30 * sx, 110, 18), ((30 + dx) * sx, 92, 24), 1.8, "#9a94ac"))
    return move(out, dy=10)


def lyev_skyknight():
    out = griffin("#c8964a", "#e8c890", "#d0a050", "#e8d070", "#3a2a1e", "#24180e", y=110, tail_tip="#7a5028")
    out += [ball(40, "#3a62c8", 0, 132, 2, 1.0, 0.25, 0.9), ball(6, GOLD, 0, 136, 30)]
    r, h = person(SILVER, STEEL, STEEL, sleeve=SILVER, pose=("up", "fwd"), seated=True, hand=STEEL)
    r += [ball(28, "#f0d080", 0, 146, -4, 1.02, 0.85, 1, n="head"),
          ball(14, SILVER, 32, 110, 0), ball(14, SILVER, -32, 110, 0),
          # streaming blue sashes
          part("cube", (20, 120, -46), (10, 4, 70), "#3a7ae0", "body", (-60, 0, -40)),
          part("cube", (-14, 104, -50), (10, 4, 76), "#2a5ac0", "body", (-40, 0, 50)),
          part("cube", (0, 150, -40), (12, 4, 60), "#3a7ae0", "head", (-80, 0, 0))]
    r += [limb((h["r"][0], h["r"][1] - 10, h["r"][2] - 80), (h["r"][0], h["r"][1] + 20, h["r"][2] + 170), 2.5,
               "#d8d8e0", mesh="cyl"),
          part("cone", (h["r"][0], h["r"][1] + 22, h["r"][2] + 180), (8, 26, 6), SILVER, "body", (210, 0, 0)),
          part("cube", (h["r"][0] + 8, h["r"][1] + 14, h["r"][2] + 140), (16, 3, 30), "#3a7ae0", "body",
               (0, 0, 60))]
    out += move(r, dy=104, dz=0, k=0.7)
    return out


MODELS = {
    "c27": trained_caracal(),
    "c5": azorius_arrester(),
    "c8": concordia_pegasus(),
    "c17": precinct_captain(),
    "c11": fencing_ace(),
    "c22": seller_of_songbirds(),
    "c25": sunspire_griffin(),
    "c6": azorius_justiciar(),
    "c16": phantom_general(),
    "c1": angel_of_serenity(),
    "c39": faerie_impostor(),
    "c53": stealer_of_secrets(),
    "c40": hover_barrier(),
    "c48": runewing(),
    "c51": soulsworn_spirit(),
    "c56": voidwielder(),
    "c43": isperias_skywatch(),
    "c50": skyline_predator(),
    "c224": vassal_soul(),
    "c179": lyev_skyknight(),
}
