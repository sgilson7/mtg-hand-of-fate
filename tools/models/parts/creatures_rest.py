"""The rest of Return to Ravnica's creatures, both planeswalkers and the new
tokens, after each card's art: silhouette and colours first, then one or two
features the card is known by. Built on the shared builders of the first
region's part files (a person, wings, a cat body, a rhino). Sizes in cm;
models face +z.
"""
import math
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

from common import ball, box, cyl, part, mirror, eyes  # noqa: E402
from creatures_wu import person, limb, feather_wing, bat_wing, lion_body, eagle_head, talons, move  # noqa: E402
from creatures_g import rhino, centaur  # noqa: E402

INK = "#1d1626"
SKIN = "#f2c9a6"
DARK_SKIN = "#9a6a4a"
GREY_SKIN = "#9aa0a8"
GREEN_SKIN = "#6f8f4a"
AZ_BLUE, AZ_WHITE, GOLD = "#2f5fb0", "#f3ecd2", "#f2c14e"
IZ_RED, IZ_BLUE, COPPER = "#c8462a", "#2f6fc8", "#c8783a"
RK_RED, RK_BLACK = "#a8202a", "#2a1a22"
GO_GREEN, GO_BLACK, BONE = "#4f6a2a", "#2e2a22", "#e8dcc0"
SE_GREEN, SE_WHITE = "#5a9a3a", "#f1ecd6"


# ---------------------------------------------------------------- builders

def hat(c, y=166, r=24, tall=20, brim=None):
    out = [part("cone", (0, y + tall / 2, 0), (r * 1.4, tall, r * 1.4), c, "head")]
    if brim:
        out.append(cyl(r * 1.25, 3, brim, 0, y - 2, 0, n="head"))
    return out


def hood(c, inner="#2c2019"):
    return [ball(30, c, 0, 146, -2, 1.05, 1.0, 1.05, n="head"), ball(20, inner, 0, 140, 16, 1, 1, 0.5, n="head")]


def staff(x, z, top, c="#6a4a2a", h=180, g=700):
    return [cyl(3, h, c, x, 0, z), ball(9, top, x, h + 6, z, g=g)]


def blade(x, y, z, length, c="#cdd2dc", hilt=GOLD, up=True):
    a = 0 if up else 500
    return [part("cone", (x, y + length / 2, z), (6, length, 2), c, t=(0, a, 0)), box(18, 4, 5, hilt, x, y, z)]


def robe(c, trim=None, y=30, r=36):
    out = [part("frustum", (0, y, 0), (r * 2, 60, r * 1.6), c)]
    if trim:
        out.append(cyl(r * 1.02, 4, trim, 0, 0, 0))
    return out


def mage(shirt, pants, boots, robe_c, trim, top_c, skin=SKIN, headwear=None, pose=("down", "fwd")):
    out, hands = person(shirt, pants, boots, skin=skin, pose=pose)
    out += robe(robe_c, trim)
    if headwear:
        out += headwear
    out += staff(hands["r"][0], hands["r"][2], top_c)
    return out


def quadruped(hide, belly, y=70, length=150, w=60, head_r=26, leg=None, tail=True, horns=None, snout=None, k=1.0):
    """A four-legged beast around height y, head at +z."""
    leg = leg or hide
    out = [ball(w / 2, hide, 0, y, 0, 1, 0.8, length / w * 0.6),
           ball(w * 0.38, belly, 0, y - w * 0.2, 0, 0.9, 0.5, 1.5)]
    for sx in (1, -1):
        for z in (length * 0.32, -length * 0.32):
            out += [limb((w * 0.3 * sx, y - 8, z), (w * 0.32 * sx, 8, z + 4), 11, leg),
                    ball(10, "#3a2a20", w * 0.32 * sx, 6, z + 6, 1, 0.6, 1.2)]
    hz = length * 0.55
    out += [ball(head_r, hide, 0, y + 10, hz, 1, 0.9, 1.1, n="head")]
    out += eyes(y + 20, hz + head_r * 0.8, head_r * 0.45, 4, n="head")
    if snout:
        out.append(ball(head_r * 0.6, snout, 0, y, hz + head_r * 0.9, 1, 0.8, 1.2, n="head"))
    if horns:
        for sx in (1, -1):
            out.append(part("cone", (head_r * 0.6 * sx, y + 32, hz), (8, 30, 8), horns, "head", (0, -120 * sx, 0)))
    if tail:
        out += [limb((0, y, -length * 0.45), (0, y - 20, -length * 0.8), 6, hide, n="tail")]
    return move(out, k=k) if k != 1.0 else out


def worm(c, belly, segs=8, r=30, length=300, rise=80, mouth="#5a1a1a", spines=None):
    out = []
    for i in range(segs):
        f = i / (segs - 1)
        z = length * (0.5 - f)
        y = r + rise * (1 - f) ** 2  # the head rears up, the tail lies on the ground
        rr = r * (1.0 - 0.35 * f)
        n = "head" if i == 0 else ("tail" if i > segs - 3 else "body")
        out.append(ball(rr, c if i % 2 == 0 else belly, 0, y, z, 1, 1, 0.9, n=n))
        if spines and i % 2 == 0:
            out.append(part("cone", (0, y + rr, z), (rr * 0.5, rr * 0.8, rr * 0.5), spines, n))
    out.append(ball(r * 0.6, mouth, 0, r + rise, length * 0.5 + r * 0.8, 1, 1, 0.5, n="head"))
    return out


def spider(c, mark, r=40, y=60, legs=4, leg_c=None):
    leg_c = leg_c or c
    out = [ball(r, c, 0, y + 10, -r * 0.6, 1, 0.85, 1.1), ball(r * 0.6, c, 0, y, r * 0.7, n="head"),
           ball(r * 0.25, mark, 0, y + r * 0.9, -r * 0.6, g=400)]
    out += eyes(y + 8, r * 1.2, r * 0.25, 4, c="#e83a3a", n="head")
    for sx in (1, -1):
        for i in range(legs):
            z = r * (0.6 - 0.45 * i)
            knee = (r * 1.3 * sx, y + r * 0.9, z)
            out += [limb((r * 0.5 * sx, y, z * 0.5), knee, 5, leg_c), limb(knee, (r * 2.0 * sx, 4, z * 1.2), 4, leg_c)]
    return out


def drake(hide, belly, wing, bone, y=110, length=170, head_r=22, horns=None, fire=None, k=1.0):
    """A dragon or drake: body, neck, head, two leathery wings, a tail."""
    out = [ball(40, hide, 0, y, 0, 1, 0.85, 1.4), ball(30, belly, 0, y - 18, 10, 0.9, 0.6, 1.3)]
    out += [limb((0, y + 10, 40), (0, y + 60, 80), 16, hide), ball(head_r, hide, 0, y + 70, 100, 1, 0.85, 1.3, n="head")]
    out += eyes(y + 78, 118, head_r * 0.5, 4, c="#f2c14e", n="head")
    out.append(part("cone", (0, y + 66, 128), (head_r * 0.9, head_r * 1.2, head_r * 0.7), hide, "head", (250, 0, 0)))
    if horns:
        for sx in (1, -1):
            out.append(part("cone", (12 * sx, y + 92, 88), (7, 30, 7), horns, "head", (-120, -80 * sx, 0)))
    if fire:
        out.append(ball(10, fire, 0, y + 62, 146, g=900))
    for sx in (1, -1):
        out += [limb((22 * sx, y - 10, 30), (30 * sx, 10, 40), 11, hide), limb((22 * sx, y - 10, -30), (30 * sx, 10, -40), 12, hide)]
    out += mirror(bat_wing(30, y + 20, 0, 150, wing, bone))
    out += [limb((0, y, -50), (0, y - 30, -length * 0.6), 12, hide, n="tail"), limb((0, y - 30, -length * 0.6), (0, y - 40, -length), 7, hide, n="tail")]
    return move(out, k=k) if k != 1.0 else out


def bird_body(feather, belly, beak, wing, wing_tip, y=100, r=30, k=1.0):
    out = [ball(r, feather, 0, y, 0, 1, 0.9, 1.3), ball(r * 0.7, belly, 0, y - 8, r * 0.5, 0.9, 0.8, 1)]
    out += eagle_head(feather, beak, 0, y + r * 1.1, r * 1.0, r=r * 0.6)
    out += mirror(feather_wing(r * 0.6, y + 10, 0, r * 4, wing, wing_tip))
    out += talons(r * 0.4, y - r * 0.7, 0, "#3a2a1a", beak, length=r * 0.9, r=5)
    out.append(part("cone", (0, y, -r * 1.6), (r * 1.2, r * 1.4, r * 0.4), wing, "tail", (-250, 0, 0)))
    return move(out, k=k) if k != 1.0 else out


def blob(c, highlight, r=50, y=None, eyes_c=INK, glow=0):
    y = y or r
    out = [ball(r, c, 0, y, 0, 1.1, 0.85, 1.1, g=glow), ball(r * 0.4, highlight, -r * 0.4, y + r * 0.5, r * 0.4, g=glow)]
    out += eyes(y + r * 0.3, r * 0.95, r * 0.3, r * 0.12, c=eyes_c)
    return out


def giant(shirt, pants, boots, skin, k=1.6, pose=("down", "down")):
    out, hands = person(shirt, pants, boots, skin=skin, pose=pose)
    return move(out, k=k), {s: tuple(v * k for v in h) for s, h in hands.items()}


def demon(hide, wing, bone, horn, k=1.7, glow="#ff5a2a"):
    out, _ = person(hide, hide, "#1a1214", skin=hide, pose=("out", "out"))
    out = move(out, k=k)
    out += mirror(bat_wing(30 * k, 110 * k, -10, 110 * k, wing, bone))
    for sx in (1, -1):
        out.append(part("cone", (16 * k * sx, 172 * k, 0), (10 * k, 34 * k, 10 * k), horn, "head", (-60, -150 * sx, 0)))
    out += [ball(5 * k, glow, -9 * k, 144 * k, 26 * k, g=900, n="head"), ball(5 * k, glow, 9 * k, 144 * k, 26 * k, g=900, n="head")]
    return out


def elemental(c, core, r=40, glow=800, sparks=6):
    out = [ball(r, c, 0, r * 2, 0, 1, 1.2, 1, g=glow // 2), ball(r * 0.5, core, 0, r * 2, r * 0.4, g=glow)]
    for i in range(sparks):
        a = 2 * math.pi * i / sparks
        out.append(ball(r * 0.25, core, r * 1.1 * math.cos(a), r * 2 + r * 0.8 * math.sin(a), r * 0.3, g=glow))
    for sx in (1, -1):
        out += [limb((r * 0.8 * sx, r * 2, 0), (r * 1.4 * sx, r * 1.2, r * 0.4), r * 0.25, c, n="body"), limb((r * 0.4 * sx, r * 1.2, 0), (r * 0.5 * sx, 4, 0), r * 0.3, c)]
    return out


def troll(skin, hair, k=1.4, extra=None):
    out, hands = person(skin, "#4a3a2a", "#2a2018", skin=skin, pose=("down", "fwd"))
    out = move(out, k=k)
    out += [ball(14 * k, hair, 0, 162 * k, -6 * k, 1.4, 0.7, 1.2, n="head"), ball(9 * k, skin, 0, 136 * k, 30 * k, 1, 0.8, 1.2, n="head")]
    return out + (extra or [])


# ---------------------------------------------------------------- white

def armory_guard():
    out, h = person(AZ_WHITE, "#3a3f55", "#2a2a33", pose=("down", "fwd"))
    out += [ball(30, "#9aa6c0", 0, 150, 0, 1.05, 0.9, 1.05, n="head"), box(60, 12, 30, "#9aa6c0", 0, 112, 0)]
    out += [cyl(3, 210, "#6a5a4a", h["r"][0], 0, h["r"][2]), part("cone", (h["r"][0], 222, h["r"][2]), (16, 30, 4), "#cdd2dc")]
    return out


def bazaar_krovod():
    out = quadruped("#8a7a6a", "#c8b49a", y=90, length=220, w=90, head_r=30, horns="#e8dcc0", snout="#6a5a4a")
    out += [box(100, 50, 90, "#7a4a2a", 0, 150, -20), box(104, 8, 94, GOLD, 0, 178, -20)]
    return out


def keening_apparition():
    out = [part("cone", (0, 70, 0), (70, 140, 50), "#d9e4f0", g=300), ball(24, "#e8f0f8", 0, 150, 0, n="head", g=400),
           ball(7, INK, -8, 154, 20, n="head"), ball(7, INK, 8, 154, 20, n="head"), ball(9, INK, 0, 138, 20, 1, 1.3, 0.5, n="head")]
    out += [limb((24, 120, 0), (60, 140, 20), 7, "#d9e4f0", n="wing_l"), limb((-24, 120, 0), (-60, 140, 20), 7, "#d9e4f0", n="wing_r")]
    return out


def palisade_giant():
    out, h = giant("#c8b48a", "#6a5a4a", "#3a2a20", "#d9a27c", k=1.9, pose=("out", "out"))
    out += [box(140, 90, 12, "#8a7a5a", h["l"][0] - 20, 140, h["l"][2] + 20), box(150, 6, 16, "#5a4a3a", h["l"][0] - 20, 188, h["l"][2] + 20)]
    return out


def selesnya_sentry():
    # a loxodon: grey hide, gold face plate, axe raised, a shield of coiled roots
    out, h = giant("#8a8a90", "#5a7a3a", "#3a3a3a", "#8a8a90", k=1.35, pose=("fwd", "raise"))
    out += [ball(26, "#8a8a90", 0, 196, 28, 1.1, 1, 1, n="head"), limb((0, 186, 52), (0, 140, 70), 8, "#8a8a90", n="head")]
    for sx in (1, -1):
        out += [ball(22, "#8a8a90", 34 * sx, 198, 10, 0.4, 1.1, 1, n="head"), part("cone", (12 * sx, 178, 58), (5, 26, 5), BONE, "head", (250, 0, 0))]
    out += [part("cone", (0, 220, 30), (20, 40, 10), GOLD, "head"), box(56, 12, 34, GOLD, 0, 150, 16)]
    out += [cyl(36, 8, "#6a4a2a", h["l"][0], h["l"][1] - 30, h["l"][2] + 8, t=(250, 0, 0))]
    out += [cyl(3, 120, "#6a4a2a", h["r"][0], h["r"][1] - 70, h["r"][2]), box(30, 30, 4, "#d88a5a", h["r"][0] + 14, h["r"][1] + 44, h["r"][2])]
    return out


# ---------------------------------------------------------------- blue

def aquus_steed():
    # yellow hide, red saddle, a crested head on a long neck
    out = quadruped("#e8c030", "#f0d870", y=80, length=170, w=56, head_r=20, snout="#b08a1a", leg="#b08a1a")
    out += [box(60, 14, 50, "#b83a1a", 0, 112, 0), part("cone", (0, 92, -130), (40, 30, 8), "#e8783a", "tail", (-250, 0, 0))]
    for z in (96, 108):
        out.append(part("cone", (0, 118, z), (8, 30, 18), "#e8783a", "head"))
    return out


def crosstown_courier():
    # a blue-skinned vedalken in a white cape, running
    out, h = person("#3a3a48", "#2a2a3a", "#1a1a22", skin="#5aa8b0", pose=("out", "down"))
    out += [part("frustum", (0, 100, -14), (70, 60, 30), "#f0ece0"), part("cone", (0, 118, 26), (14, 16, 4), "#b02a2a", t=(500, 0, 0))]
    out += [box(30, 26, 14, "#6a4028", -34, 70, -10)]
    return out


def doorkeeper():
    # a one-eyed homunculus crouched on a green door lintel, spear in hand
    out = [box(16, 120, 16, "#5a7a5a", -50, 60, 0), box(16, 120, 16, "#5a7a5a", 50, 60, 0), box(120, 14, 22, "#5a7a5a", 0, 124, 0)]
    out += [ball(34, "#6a4a22", 0, 160, 0, 1.1, 0.85, 1), ball(26, "#a8783a", 0, 166, 14, n="head"),
            ball(16, "#f0ece0", 0, 170, 34, n="head"), ball(8, "#4a7ab8", 0, 170, 46, n="head"), ball(3, INK, 0, 170, 52, n="head")]
    for sx in (1, -1):
        out += [limb((26 * sx, 150, 10), (40 * sx, 118, 26), 6, "#a8783a"), ball(6, "#a8783a", 40 * sx, 116, 28)]
    out += [limb((-40, 116, 28), (40, 90, 60), 2, "#6a4a22"), part("cone", (44, 88, 64), (6, 16, 3), "#8a8a8a", t=(0, 750, 0))]
    out += [cyl(9, 12, "#c89a3a", 30, 100, 10)]
    return out


def jace():
    # teal cloak and coat with pale runes, bare head, a light in his hand
    out, h = person("#1e6a62", "#14403c", "#14141e", pose=("raise", "down"))
    out += [part("frustum", (0, 44, -6), (76, 88, 60), "#1e6a62"), box(8, 80, 2, "#a8e8e0", 0, 60, 26)]
    out += [ball(26, "#5a3a22", 0, 150, -6, 1.05, 0.7, 1.0, n="head"), ball(8, "#9be7ff", h["l"][0], h["l"][1] + 14, h["l"][2], g=900)]
    out += [part("cone", (0, 120, -20), (90, 70, 20), "#14403c", "body", (0, 0, 0))]
    for i in range(3):
        a = 2 * math.pi * i / 3
        out.append(box(16, 4, 12, "#e8f0ff", 50 * math.cos(a), 190 + 8 * i, 50 * math.sin(a)))
    return out


def sphinx_of_the_chimes():
    # a tawny lion couched under dark folded wings, a bearded man's face
    out = lion_body("#a8884a", "#c8a868", 70, length=200, w=80, paw="#6a5030", tail_tip="#6a5030")
    out += [ball(36, "#c8a868", 0, 130, 100, n="head"), ball(24, "#c8a080", 0, 132, 124, n="head"),
            ball(16, "#c8a868", 0, 116, 130, 1, 1.2, 0.8, n="head")]
    out += eyes(138, 144, 9, 3.5, n="head")
    out += mirror(feather_wing(36, 110, -10, 150, "#3a3a3a", "#6a6a6a", up=40, sweep=80))
    return out


def tower_drake():
    return drake("#4a7ac0", "#c8d8f0", "#3a5aa0", "#9ab0e0", y=90, length=150, head_r=18, k=0.8)


# ---------------------------------------------------------------- black

def desecration_demon():
    return demon("#4a2a30", "#2a1a1e", "#6a4a4a", "#c8b49a", k=1.9)


def drainpipe_vermin():
    out = quadruped("#5a4a3a", "#8a7a6a", y=30, length=80, w=34, head_r=14, snout="#c88a8a")
    out += [ball(6, "#c88a8a", -8, 54, 58, n="head"), ball(6, "#c88a8a", 8, 54, 58, n="head"), limb((0, 30, -36), (0, 10, -90), 3, "#c88a8a", n="tail")]
    return out


def necropolis_regent():
    # a vampire queen floating in a great purple gown, red gloves, dark ragged wings
    out = [part("cone", (0, 70, 0), (130, 140, 110), "#5a2a6a"), ball(30, "#1a1a1a", 0, 150, 0, 1, 1.2, 0.8),
           ball(24, "#f0e8e8", 0, 196, 4, n="head"), ball(24, "#1a1014", 0, 210, -8, 1, 1.2, 0.9, n="head")]
    out += eyes(198, 26, 8, 3.5, c="#c83a5a", n="head")
    for sx in (1, -1):
        out += [limb((26 * sx, 166, 0), (64 * sx, 150, 20), 7, "#f0e8e8"), ball(8, "#a81a2a", 66 * sx, 150, 22)]
    out += mirror(bat_wing(30, 170, -20, 150, "#1a1a1a", "#3a3a3a"))
    return move(out, k=1.3)


def ogre_jailbreaker():
    out, h = giant("#7a6a5a", "#4a3a2a", "#2a2018", "#8a9a6a", k=1.6, pose=("out", "down"))
    for i in range(6):
        out.append(ball(6, "#8a8a94", h["l"][0] + i * 10, h["l"][1] - i * 12, h["l"][2] + 10))
    out.append(cyl(10, 12, "#6a6a74", h["r"][0], h["r"][1] - 6, h["r"][2]))
    return out


def pack_rat():
    out = quadruped("#4a3a32", "#8a7a6a", y=40, length=90, w=40, head_r=18, snout="#c88a8a")
    out += [ball(7, "#c88a8a", -10, 70, 62, n="head"), ball(7, "#c88a8a", 10, 70, 62, n="head"), ball(3, "#e83a3a", -5, 62, 70, n="head", g=600), ball(3, "#e83a3a", 5, 62, 70, n="head", g=600)]
    out += [limb((0, 40, -40), (0, 10, -100), 3, "#c88a8a", n="tail")]
    return out


def tavern_swindler():
    # a white-haired woman in a black cloak, a fan of cards in her hand
    out, h = person("#4a2a4a", "#1a1a22", "#1a1214", skin="#f0dcd8", pose=("fwd", "down"))
    out += [part("frustum", (0, 70, -8), (84, 140, 64), "#1a1a22"), ball(29, "#ece8f0", 0, 148, -6, 1.05, 0.9, 1.0, n="head")]
    out += [limb((0, 148, -20), (0, 100, -24), 18, "#ece8f0", n="head")]
    for i in range(4):
        out.append(box(10, 14, 1, "#f0e0c0", h["l"][0] - 9 + 6 * i, h["l"][1] + 8, h["l"][2] + 4, t=(0, 0, 60 * i - 90)))
    return out


def terrus_wurm():
    return worm("#5a4a3a", "#8a6a4a", segs=8, r=34, length=320, rise=220, spines="#c8b49a")


# ---------------------------------------------------------------- red

def batterhorn():
    out = quadruped("#8a5a3a", "#c88a5a", y=90, length=190, w=90, head_r=32, snout="#6a3a2a")
    out += [part("cone", (0, 110, 170), (30, 70, 30), "#e8dcc0", "head", (250, 0, 0))]
    for sx in (1, -1):
        out.append(part("cone", (22 * sx, 130, 140), (14, 40, 14), "#e8dcc0", "head", (0, -100 * sx, 0)))
    return out


def chaos_imps():
    out = []
    for i, (x, y, z) in enumerate([(0, 120, 0), (-50, 90, -20), (50, 100, -10), (-20, 160, -30), (30, 150, 20)]):
        out += [ball(16, "#c83a2a", x, y, z), ball(6, GOLD, x - 5, y + 6, z + 14, g=600), ball(6, GOLD, x + 5, y + 6, z + 14, g=600)]
        out += [part("cone", (x - 8, y + 16, z), (5, 14, 5), "#2a1a1a"), part("cone", (x + 8, y + 16, z), (5, 14, 5), "#2a1a1a")]
        out += [limb((x + 10, y, z - 8), (x + 34, y + 14, z - 20), 4, "#6a1a1a", n="wing_l"), limb((x - 10, y, z - 8), (x - 34, y + 14, z - 20), 4, "#6a1a1a", n="wing_r")]
    return out


def minotaur_aggressor():
    out, h = giant("#6a4a3a", "#3a2a20", "#2a1a14", "#7a5a40", k=1.6, pose=("raise", "fwd"))
    for sx in (1, -1):
        out.append(part("cone", (24 * sx, 262, 10), (12, 44, 12), "#e8dcc0", "head", (0, -200 * sx, 0)))
    out += [ball(16, "#5a3a2a", 0, 220, 48, 1, 0.8, 1.2, n="head"), ball(6, "#ff7a3c", 0, 250, 0, g=800, n="head")]
    out += [cyl(4, 120, "#5a4a3a", h["l"][0], h["l"][1] - 40, h["l"][2]), part("cone", (h["l"][0], h["l"][1] + 88, h["l"][2]), (40, 30, 8), "#8a8a94", t=(0, 250, 0))]
    return out


def utvara_hellkite():
    return drake("#8a6a3a", "#c8a060", "#5a4a32", "#e8c08a", y=130, length=230, head_r=26, horns="#2a1a1a", fire="#ff9a3c", k=1.1)


def viashino_racketeer():
    # dark-scaled, crouched, jaws open, a grey shoulder wrap
    out, h = person("#2a2a30", "#4a4a52", "#1a1a1e", skin="#2a2a30", pose=("fwd", "down"))
    out += [ball(16, "#2a2a30", 0, 136, 34, 1, 0.8, 1.6, n="head"), part("cone", (0, 128, 62), (14, 10, 20), "#a83a3a", "head", (250, 0, 0)),
            limb((0, 60, -20), (0, 20, -90), 9, "#2a2a30", n="tail"), box(50, 20, 40, "#8a8a96", 0, 106, 0)]
    for i in range(4):
        out.append(part("cone", (0, 160 - i * 6, 10 - i * 12), (6, 18, 6), "#5a5a62", "head", (-150, 0, 0)))
    return out


# ---------------------------------------------------------------- green

def axebane_guardian():
    # a long-haired druid in a brown robe, arms raised, thorny branches round him
    out, h = person("#6a3a22", "#4a2a18", "#2a1a10", skin="#c8906a", pose=("raise", "raise"))
    out += robe("#6a3a22", "#9a5a32") + [ball(28, "#3a2416", 0, 146, -8, 1.05, 1.0, 1.0, n="head"), limb((0, 140, -18), (0, 100, -20), 14, "#3a2416", n="head")]
    for a in range(5):
        t = a * 2 * math.pi / 5
        out += [limb((60 * math.cos(t), 4, 60 * math.sin(t)), (70 * math.cos(t + 0.6), 120, 70 * math.sin(t + 0.6)), 4, "#2a2a1a")]
    return out


def gatecreeper_vine():
    # a walking arch of roots and moss with one green eye at the crown
    out = []
    for sx in (1, -1):
        out += [limb((50 * sx, 0, 0), (46 * sx, 80, 0), 14, "#6a5a2a"), limb((46 * sx, 80, 0), (30 * sx, 140, 0), 14, "#6a5a2a"),
                limb((40 * sx, 110, 0), (80 * sx, 70, 20), 6, "#6a5a2a"), ball(9, "#9ac84a", 80 * sx, 66, 22)]
    out += [limb((-30, 140, 0), (30, 140, 0), 16, "#6a5a2a", n="head"), ball(28, "#5a8a2a", 0, 156, 0, 2.0, 0.6, 1.0, n="head"), ball(8, "#6af06a", 0, 150, 18, g=900, n="head")]
    for y in (30, 60, 100):
        out += [ball(8, "#9ac84a", 48, y, 10), ball(8, "#9ac84a", -48, y, 10)]
    return out


def gobbling_ooze():
    out = blob("#6a9a3a", "#a8d86a", r=55)
    for i in range(4):
        out.append(ball(8, "#e8dcc0", -30 + 20 * i, 40, 50))
    return out


def golgari_decoy():
    # a hooded green-skinned elf in mossy leather, leaping
    out, h = person("#4a6a3a", "#4a3a5a", "#2a1e22", skin="#5a8a4a", pose=("out", "out"))
    out += hood("#3a2a3a", "#5a8a4a") + [part("cone", (24, 150, 4), (6, 22, 6), "#5a8a4a", "head", (0, -200, 0))]
    out += [ball(5, "#f8c860", -8, 142, 26, n="head"), ball(5, "#f8c860", 8, 142, 26, n="head")]
    return out


def oak_street_innkeeper():
    # an elf woman in a long green gown and cloak, gold belt
    out, h = person("#3a7a3a", "#3a7a3a", "#2a2018", skin="#a8704a", pose=("down", "out"))
    out += [part("frustum", (0, 46, 0), (80, 92, 64), "#3a7a3a"), part("frustum", (0, 70, -12), (90, 120, 40), "#6aa84a"),
            cyl(26, 6, "#d8a83a", 0, 58, 0), ball(26, "#2a1e16", 0, 150, -10, 1.05, 0.8, 1.0, n="head")]
    for sx in (1, -1):
        out.append(part("cone", (26 * sx, 146, 2), (5, 20, 5), "#a8704a", "head", (0, -200 * sx, 0)))
    return out


def stonefare_crocodile():
    out = quadruped("#5a6a3a", "#c8c08a", y=34, length=220, w=60, head_r=20, snout="#4a5a2a", k=1.0)
    for z in (-60, -20, 20):
        out.append(part("cone", (0, 66, z), (18, 18, 30), "#8a8a7a"))
    out.append(box(30, 14, 70, "#4a5a2a", 0, 40, 180))
    return out


def wild_beastmaster():
    # a red-haired woman in a dark cloak, fist raised, with a bear at her side
    out, h = person("#8a8a7a", "#3a3a22", "#2a2018", skin="#f0c8a8", pose=("raise", "down"))
    out += [part("frustum", (0, 70, -12), (90, 130, 40), "#3a3a22"), ball(32, "#c84a1a", 0, 150, -10, 1.15, 1.0, 1.1, n="head"),
            limb((0, 150, -24), (0, 100, -30), 18, "#c84a1a", n="head")]
    out += move(quadruped("#4a3a2a", "#6a5a4a", y=50, length=110, w=56, head_r=20, snout="#3a2a1a"), dx=80, dz=20)
    return out


def worldspine_wurm():
    return worm("#4a5a3a", "#8a8a5a", segs=10, r=55, length=600, rise=700, spines="#c8c08a", mouth="#3a1a1a")


# ---------------------------------------------------------------- Azorius

def archon_of_the_triumvirate():
    # a hooded rider on a dark shaggy winged beast with curled horns
    out = quadruped("#4a4a52", "#7a7a82", y=90, length=190, w=90, head_r=30, horns="#c8b490", snout="#2a2a30")
    out += mirror(bat_wing(40, 130, 0, 180, "#2a2a30", "#5a5a62"))
    rider, h = person("#2a2a3a", "#2a2a3a", "#1a1a2a", pose=("raise", "down"), seated=True)
    out += move(rider + hood("#2a2a3a", "#1a1a26"), dy=80)
    out += [limb((h["l"][0], 80 + h["l"][1], h["l"][2]), (h["l"][0] - 30, 80 + h["l"][1] + 90, h["l"][2] + 20), 3, "#8a8a9a")]
    return out


def hussar_patrol():
    out = quadruped("#2a2a32", "#4a4a52", y=100, length=180, w=60, head_r=22)
    out += [box(70, 30, 90, "#8a9aa0", 0, 104, 30)]
    rider, h = person("#8a9aa0", "#3a5a7a", "#1a1a2a", pose=("down", "fwd"), seated=True)
    out += move(rider, dy=90) + [part("frustum", (0, 190, -30), (60, 40, 20), "#3a7a9a")]
    out += [cyl(3, 160, "#6a5a4a", h["r"][0], 90 + h["r"][1] - 60, h["r"][2]), part("cone", (h["r"][0], 90 + h["r"][1] + 104, h["r"][2]), (10, 24, 10), "#cdd2dc")]
    return out


def isperia():
    out = lion_body("#3a5aa0", "#c8d8f0", 110, length=260, w=110, paw="#2a4a8a", tail_tip=GOLD)
    out += [ball(34, "#e8d8b0", 0, 190, 140, n="head"), part("frustum", (0, 178, 128), (90, 60, 50), "#2a4a8a", "head"), cyl(30, 6, GOLD, 0, 222, 140, n="head")]
    out += eyes(198, 170, 12, 5, n="head")
    out += mirror(feather_wing(50, 160, 0, 240, "#3a5aa0", AZ_WHITE))
    return out


def new_prahv_guildmage():
    # black plate and a heavy cloak, a blue gem on the chest, a staff
    out, h = person("#3a3a46", "#2a2a36", "#1a1a22", skin="#6a4a32", pose=("fwd", "down"))
    out += [part("frustum", (0, 80, -16), (100, 150, 50), "#1e1e26"), ball(12, "#3a6ac8", 0, 90, 26, 1, 1.2, 0.5, g=500), cyl(14, 4, "#a8884a", 0, 88, 24, t=(250, 0, 0))]
    out += [cyl(3, 180, "#2a2a2a", h["r"][0], 0, h["r"][2])]
    return out


def skymark_roc():
    return bird_body("#a88a5a", "#c8a878", "#e8c05a", "#3a5ab0", "#8a9ad0", y=110, r=36, k=1.1)


def azors_elocutors():
    out = []
    for x in (-40, 40):
        body, h = person(AZ_WHITE, "#2a3a6a", "#1a1a2a", pose=("fwd", "down"))
        body += [part("cone", (0, 176, 0), (40, 34, 40), AZ_BLUE, "head")]
        out += move(body, dx=x)
    out += [box(60, 40, 6, "#f3ecd2", 0, 100, 40), ball(6, "#9be7ff", 0, 124, 44, g=800)]
    return out


def judges_familiar():
    out = [ball(26, "#c8c0b0", 0, 40, 0, 1, 1.1, 0.9), ball(20, "#d8d0c0", 0, 74, 6, n="head")]
    out += [ball(9, GOLD, -9, 78, 22, n="head"), ball(9, GOLD, 9, 78, 22, n="head"), ball(4, INK, -9, 78, 28, n="head"), ball(4, INK, 9, 78, 28, n="head")]
    out += mirror(feather_wing(16, 50, 0, 60, "#a8a090", AZ_WHITE))
    return out


# ---------------------------------------------------------------- Izzet

def hypersonic_dragon():
    return drake(IZ_RED, "#f2a05a", IZ_BLUE, "#9ad8ff", y=110, length=190, head_r=22, horns=COPPER, fire="#9be7ff")


def izzet_staticaster():
    out, h = person(IZ_BLUE, IZ_RED, "#2a1a14", pose=("fwd", "down"))
    out += [cyl(18, 60, COPPER, 0, 70, -36), ball(14, "#9be7ff", 0, 140, -36, g=900)]
    out += [limb((0, 130, -36), h["l"], 3, "#9be7ff"), ball(6, "#e8ffff", *h["l"], g=1000)]
    out += [box(46, 10, 20, COPPER, 0, 156, 18, n="head")]
    return out


def mercurial_chemister():
    out = mage(IZ_RED, "#3a2a2a", "#1a1214", IZ_BLUE, COPPER, "#ff9a3c", headwear=[box(40, 12, 20, COPPER, 0, 150, 22, n="head")], pose=("fwd", "fwd"))
    out += [ball(9, "#9be7ff", 30, 96, 36, g=800)]
    return out


def nivix_guildmage():
    return mage(IZ_BLUE, IZ_RED, "#1a1214", IZ_RED, COPPER, "#9be7ff", headwear=hat(IZ_BLUE, y=164, r=22, tall=26))


def blistercoil_weird():
    # a hulking figure of dark water with lava burning through its chest
    out, h = giant("#2a3a6a", "#2a3a6a", "#2a3a6a", "#2a3a6a", k=1.2, pose=("out", "fwd"))
    out += [ball(22, "#e83a1a", 0, 112, 26, 1, 1.3, 0.5, g=700), ball(10, "#ffb040", 0, 112, 34, g=900)]
    for x, y in ((-20, 190), (24, 200), (-40, 160), (40, 170)):
        out.append(ball(6, "#6a8ad8", x, y, 10))
    return out


def nivmagus_elemental():
    return elemental("#5a7ad8", "#ff9a3c", r=30)


# ---------------------------------------------------------------- Rakdos

def hellhole_flailer():
    out = demon("#6a2a2a", "#3a1414", "#8a5a4a", "#2a1a1a", k=1.3)
    for sx in (1, -1):
        for i in range(5):
            out.append(ball(5, "#8a8a94", 70 * sx + i * 12 * sx, 150 - i * 8, 10 + i * 4))
        out.append(part("cone", (130 * sx, 112, 30), (14, 24, 14), "#cdd2dc", t=(0, 250, 0)))
    return out


def rakdos_ragemutt():
    out = quadruped("#6a2a2a", "#a85a4a", y=60, length=150, w=60, head_r=24, snout="#3a1414")
    for z in (-40, 0, 40):
        out.append(part("cone", (0, 96, z), (10, 26, 10), "#e8dcc0"))
    out += [ball(5, "#ff3a3a", -9, 84, 100, g=900, n="head"), ball(5, "#ff3a3a", 9, 84, 100, g=900, n="head")]
    return out


def rakdos_ringleader():
    out, h = person(RK_RED, RK_BLACK, "#1a1214", pose=("raise", "down"))
    out += hat(RK_BLACK, y=166, r=22, tall=40, brim=RK_BLACK) + [cyl(23, 6, RK_RED, 0, 176, 0, n="head")]
    for i in range(8):
        a = i / 7
        out.append(ball(3, "#5a3a2a", h["l"][0] + 30 * a, h["l"][1] + 30 * a, h["l"][2] + 40 * a))
    return out


def rakdos_lord():
    out = demon("#8a1a1a", "#4a0e0e", "#c87a5a", "#2a1a1a", k=2.4, glow="#ffaa3a")
    for sx in (1, -1):
        out.append(part("cone", (40 * sx, 430, -20), (24, 80, 24), "#2a1a1a", "head", (-80, -250 * sx, 0)))
    out += [ball(30, "#ff7a2a", 0, 260, 30, g=700)]
    return out


def rix_maadi_guildmage():
    out = mage(RK_BLACK, RK_RED, "#1a1214", RK_RED, "#2a1a1a", "#ff3a3a", skin="#e8d0d0")
    out += [ball(30, "#c8c8d0", 0, 156, -14, 1.1, 0.7, 1.0, n="head")]
    return out


def spawn_of_rix_maadi():
    out = quadruped("#8a2a3a", "#c85a5a", y=70, length=160, w=70, head_r=30, snout="#3a1414", horns="#e8dcc0")
    for i in range(6):
        out.append(part("cone", (0, 104, 60 - 24 * i), (12, 30, 12), "#e8dcc0"))
    out += [ball(10, "#ffaa3a", 0, 96, 130, g=800, n="head")]
    return out


def cryptborn_horror():
    out = [ball(50, "#4a3a4a", 0, 70, 0, 1.2, 0.9, 1.0)]
    for i in range(6):
        a = 2 * math.pi * i / 6
        out += [limb((40 * math.cos(a), 60, 40 * math.sin(a)), (90 * math.cos(a), 4, 90 * math.sin(a)), 9, "#6a4a5a")]
    out += [ball(20, "#c8b49a", 0, 120, 40, n="head"), ball(6, "#ff3a3a", -8, 124, 58, g=900, n="head"), ball(6, "#ff3a3a", 8, 124, 58, g=900, n="head")]
    out += [ball(14, "#ff5a5a", 0, 70, 54, g=500)]
    return out


# ---------------------------------------------------------------- Golgari

def corpsejack_menace():
    # a dark knot of fungal tendrils round a red maw
    out = [ball(60, "#2a2a28", 0, 80, 0, 1.1, 1.2, 1.0), ball(24, "#6a2a1e", 0, 90, 54, 0.8, 1.2, 0.4, n="head"), ball(12, "#a84a2a", 0, 90, 62, 0.8, 1.2, 0.4, n="head")]
    for i in range(10):
        a = 2 * math.pi * i / 10
        tip = (110 * math.cos(a), 80 + 90 * math.sin(a), 20 * math.sin(3 * a))
        mid = (70 * math.cos(a + 0.3), 80 + 60 * math.sin(a + 0.3), 10)
        out += [limb((40 * math.cos(a), 80 + 40 * math.sin(a), 0), mid, 9, "#2a2a28"), limb(mid, tip, 5, "#3a3a34")]
    for sx in (1, -1):
        out.append(limb((30 * sx, 40, 0), (50 * sx, 0, 10), 12, "#2a2a28"))
    return out


def jarad():
    out = mage(GO_BLACK, "#3a3a2a", "#1a1a14", GO_GREEN, BONE, "#7fd36a", skin="#a8b08a", headwear=[part("cone", (x, 176, 0), (8, 30, 8), BONE, "head") for x in (-18, -6, 6, 18)])
    out += [ball(8, "#7fd36a", -8, 144, 24, g=900, n="head"), ball(8, "#7fd36a", 8, 144, 24, g=900, n="head")]
    return out


def korozda_guildmage():
    out = mage("#4a5a2a", "#2a2a1a", "#1a1a14", GO_BLACK, "#7fd36a", "#c87ad8", skin="#c8c0a8", headwear=hood("#3a4a2a"))
    out += [ball(12, "#8a9a5a", 50, 20, 40), ball(10, "#8a9a5a", -50, 20, 40)]
    return out


def lotleth_troll():
    # pale bulbous flesh, sinewy red arms hooked under it
    out = troll("#d8c8a0", "#a88a6a", k=1.3)
    out += [ball(46, "#d8c8a0", 0, 200, -10, 1.4, 1.0, 1.2, n="head")]
    for sx in (1, -1):
        out.append(limb((40 * sx, 150, 10), (60 * sx, 40, 50), 12, "#c86a5a"))
    return out


def trestle_troll():
    # a mossy green troll with a crest of white feathers
    out = troll("#4a5a2a", "#3a4a1e", k=1.3)
    for i, x in enumerate((-16, -6, 6, 16)):
        out.append(part("cone", (x, 230, -10), (6, 36, 3), "#f0f0e8", "head", (-60, 0, 0)))
    for y in (120, 150, 180):
        out += [ball(8, "#8a9a5a", -20, y, 26), ball(8, "#8a9a5a", 20, y, 26)]
    return out


def vraska():
    out, h = person("#3a4a3a", "#2a2a22", "#1a1a14", skin="#8aa07a", pose=("fwd", "down"))
    for i in range(9):
        a = math.pi * (0.1 + 0.8 * i / 8)
        out.append(limb((0, 160, -6), (34 * math.cos(a), 190 + 10 * math.sin(a * 2), -20), 5, "#5a7a4a", n="head"))
    out += [ball(5, GOLD, -9, 144, 24, g=900, n="head"), ball(5, GOLD, 9, 144, 24, g=900, n="head")]
    out += blade(h["l"][0], h["l"][1], h["l"][2] + 6, 70, c="#8a9aa0", hilt=GO_BLACK)
    out += [part("frustum", (0, 40, -6), (70, 80, 56), "#2a3a2a")]
    return out


def deathrite_shaman():
    out = mage("#3a3a2a", "#2a2a1a", "#1a1a14", "#4a4a3a", BONE, "#c8e86a")
    out += [ball(26, BONE, 0, 144, 6, 1, 1.1, 0.9, n="head"), ball(5, INK, -8, 148, 28, n="head"), ball(5, INK, 8, 148, 28, n="head")]
    return out


def golgari_longlegs():
    # a small body high up on four absurdly tall stilt legs
    out = [ball(18, "#8a8a7a", 0, 330, 0, 0.8, 1.4, 0.8), ball(12, "#8a8a7a", 0, 362, 6, n="head")]
    for i in range(7):
        a = math.pi * (0.2 + 0.1 * i)
        out.append(limb((0, 370, 0), (20 * math.cos(a), 370 + 20 * math.sin(a), 0), 1.5, "#d8d8c8", n="head"))
    for sx, sz in ((1, 1), (1, -1), (-1, 1), (-1, -1)):
        knee = (60 * sx, 360, 40 * sz)
        out += [limb((0, 320, 0), knee, 4, "#4a4a3e"), limb(knee, (90 * sx, 0, 70 * sz), 4, "#4a4a3e")]
    return out


def slitherhead():
    # a skull set in a coil of tentacles
    out = [ball(26, "#4a4a22", 0, 20, 0, 1.4, 0.6, 1.2), ball(20, "#c8b880", 0, 54, 10, 1, 1.1, 1, n="head")]
    out += [ball(5, "#4a4220", -8, 58, 28, n="head"), ball(5, "#4a4220", 8, 58, 28, n="head"), box(16, 6, 4, "#c8b880", 0, 38, 26, n="head")]
    for i in range(5):
        a = 2 * math.pi * i / 5
        out.append(limb((20 * math.cos(a), 16, 20 * math.sin(a)), (50 * math.cos(a + 0.5), 4, 50 * math.sin(a + 0.5)), 5, "#4a4a22"))
    return out


# ---------------------------------------------------------------- Selesnya

def armada_wurm():
    out = worm("#8a6a3a", "#c8a060", segs=9, r=50, length=480, rise=360)
    out += [part("cone", (0, 50 + 360 + 40, 240), (110, 110, 90), "#e8ecf0", "head")]
    return out


def loxodon_smiter():
    out, h = giant("#8a8a94", "#4a5a3a", "#3a3a3a", "#9aa0a8", k=1.5, pose=("raise", "down"))
    out += [ball(30, "#9aa0a8", 0, 214, 30, 1.2, 1, 1, n="head"), limb((0, 204, 66), (0, 150, 90), 10, "#9aa0a8", n="head")]
    for sx in (1, -1):
        out += [ball(26, "#9aa0a8", 40 * sx, 218, 10, 0.5, 1.1, 1, n="head"), part("cone", (16 * sx, 196, 70), (6, 30, 6), BONE, "head", (250, 0, 0))]
    out += [cyl(5, 90, "#6a5a4a", h["l"][0], h["l"][1] - 20, h["l"][2]), box(40, 30, 30, "#8a8a94", h["l"][0], h["l"][1] + 70, h["l"][2])]
    return out


def risen_sanctuary():
    out = quadruped("#7a8a6a", "#a8b08a", y=160, length=300, w=160, head_r=50, k=1.0)
    out += [box(140, 90, 160, SE_WHITE, 0, 260, -20), part("cone", (0, 340, -20), (150, 80, 170), SE_GREEN), ball(20, GOLD, 0, 380, -20, g=600)]
    return out


def trostani():
    out = [cyl(30, 120, "#6a4a2a", 0, 0, 0)]
    for i, x in enumerate((-40, 0, 40)):
        out += [ball(22, "#d8e8b0", x, 150 + (10 if i == 1 else 0), 10, n="head"), ball(26, SE_GREEN, x, 176 + (10 if i == 1 else 0), 0, 1.2, 0.8, 1.2, n="head")]
        out += eyes(154 + (10 if i == 1 else 0), 28, 7, 3)
    out += [ball(10, GOLD, 0, 210, 10, g=700)]
    for sx in (1, -1):
        out.append(limb((20 * sx, 120, 0), (90 * sx, 170, 10), 9, "#6a4a2a"))
    return out


def vitu_ghazi_guildmage():
    return mage(SE_WHITE, SE_GREEN, "#3a2a20", SE_GREEN, GOLD, "#c8f0a0", headwear=[cyl(28, 6, "#7fd36a", 0, 164, 0, n="head")])


def wayfaring_temple():
    # a towering stone construct walking on two legs, its body a ribbed shrine
    out = []
    for sx in (1, -1):
        out += [box(40, 100, 40, "#6a6a4a", 36 * sx, 50, 0), box(46, 14, 56, "#3a3a2a", 36 * sx, 6, 10)]
        out += [limb((70 * sx, 220, 0), (110 * sx, 140, 20), 18, "#6a6a4a"), ball(16, "#3a3a2a", 112 * sx, 130, 22)]
    out += [ball(80, "#6a6a4a", 0, 190, 0, 1, 1.1, 0.8)]
    for y in (140, 170, 200, 230):
        out.append(cyl(70 - abs(y - 185) * 0.5, 6, "#a8a080", 0, y, 0))
    out += [part("cone", (0, 300, 0), (40, 60, 40), "#6a6a4a", "head"), box(40, 50, 6, "#3a3a2a", 0, 170, 60)]
    return out


# ---------------------------------------------------------------- colourless

def volatile_rig():
    out = [box(120, 70, 150, "#6a6a74", 0, 70, 0), cyl(26, 70, COPPER, -36, 100, -30), cyl(26, 70, COPPER, 36, 100, -30)]
    for sx in (1, -1):
        for z in (50, -50):
            out += [cyl(24, 14, "#2a2a2a", 64 * sx, 20, z, t=(0, 250, 0))]
    out += [ball(12, "#ff9a3c", 0, 120, 70, g=900, n="head"), box(80, 10, 10, "#8a8a94", 0, 40, 80)]
    return out


# ---------------------------------------------------------------- tokens

def dragon_token():
    return drake("#c8302a", "#f2a05a", "#8a1a1a", "#e8c08a", y=120, length=210, head_r=24, horns="#3a1a1a", fire="#ff9a3c")


def wurm_token():
    return worm("#5a8a3a", "#a8c86a", segs=8, r=42, length=380, rise=260, spines="#c8d88a")


def saproling_token():
    out = [ball(16, "#7a9a4a", 0, 18, 0), ball(26, "#c87a4a", 0, 46, 0, 1.2, 0.6, 1.2, n="head")]
    out += eyes(30, 14, 6, 3)
    return out


def assassin_token():
    out, h = person("#2a2a2a", "#1a1a1a", "#111111", pose=("fwd", "down"))
    out += hood("#1e1e22", "#0a0a0a") + [ball(4, "#7fd36a", -7, 142, 26, g=900, n="head"), ball(4, "#7fd36a", 7, 142, 26, g=900, n="head")]
    out += blade(h["r"][0], h["r"][1], h["r"][2] + 6, 50, c="#8a9aa0", hilt="#2a2a2a")
    return out


def elemental_token():
    out = [box(110, 150, 80, "#6a8a4a", 0, 120, 0), ball(50, SE_WHITE, 0, 230, 0, n="head"), ball(16, "#c8f0a0", 0, 236, 44, g=800, n="head")]
    for sx in (1, -1):
        out += [limb((60 * sx, 170, 0), (90 * sx, 60, 20), 20, "#6a8a4a"), limb((30 * sx, 50, 0), (34 * sx, 0, 0), 22, "#6a5a3a")]
    return out


def ooze_token():
    return blob("#5aa83a", "#a8e86a", r=48, glow=200)


MODELS = {
    "c2": armory_guard(),
    "c7": bazaar_krovod(),
    "c12": keening_apparition(),
    "c15": palisade_giant(),
    "c21": selesnya_sentry(),
    "c29": aquus_steed(),
    "c34": crosstown_courier(),
    "c37": doorkeeper(),
    "c44": jace(),
    "c52": sphinx_of_the_chimes(),
    "c55": tower_drake(),
    "c63": desecration_demon(),
    "c66": drainpipe_vermin(),
    "c71": necropolis_regent(),
    "c72": ogre_jailbreaker(),
    "c73": pack_rat(),
    "c79": tavern_swindler(),
    "c80": terrus_wurm(),
    "c87": batterhorn(),
    "c90": chaos_imps(),
    "c100": minotaur_aggressor(),
    "c110": utvara_hellkite(),
    "c112": viashino_racketeer(),
    "c115": axebane_guardian(),
    "c124": gatecreeper_vine(),
    "c126": gobbling_ooze(),
    "c127": golgari_decoy(),
    "c131": oak_street_innkeeper(),
    "c136": stonefare_crocodile(),
    "c139": wild_beastmaster(),
    "c140": worldspine_wurm(),
    "c142": archon_of_the_triumvirate(),
    "c169": hussar_patrol(),
    "c171": isperia(),
    "c181": new_prahv_guildmage(),
    "c196": skymark_roc(),
    "c210": azors_elocutors(),
    "c218": judges_familiar(),
    "c170": hypersonic_dragon(),
    "c173": izzet_staticaster(),
    "c180": mercurial_chemister(),
    "c182": nivix_guildmage(),
    "c211": blistercoil_weird(),
    "c219": nivmagus_elemental(),
    "c167": hellhole_flailer(),
    "c185": rakdos_ragemutt(),
    "c186": rakdos_ringleader(),
    "c187": rakdos_lord(),
    "c192": rix_maadi_guildmage(),
    "c199": spawn_of_rix_maadi(),
    "c212": cryptborn_horror(),
    "c152": corpsejack_menace(),
    "c174": jarad(),
    "c176": korozda_guildmage(),
    "c177": lotleth_troll(),
    "c205": trestle_troll(),
    "c208": vraska(),
    "c213": deathrite_shaman(),
    "c216": golgari_longlegs(),
    "c222": slitherhead(),
    "c143": armada_wurm(),
    "c178": loxodon_smiter(),
    "c190": risen_sanctuary(),
    "c206": trostani(),
    "c207": vitu_ghazi_guildmage(),
    "c209": wayfaring_temple(),
    "c236": volatile_rig(),
    "token_dragon": dragon_token(),
    "token_wurm": wurm_token(),
    "token_saproling": saproling_token(),
    "token_assassin": assassin_token(),
    "token_elemental": elemental_token(),
    "token_ooze": ooze_token(),
}
