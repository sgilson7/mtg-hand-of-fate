"""Enchantments and artifacts on the fight's table (T12).

An enchantment is a sigil: a stone disc with a ring of glowing beads in the
card's colours, and the card's emblem floating above it, after its art. An
artifact is the object itself. A model is "c<collector number>", the same
name a creature card's model has. Each stands on y = 0 and faces +z; the
game scales a sigil to about 90 cm and an object to its own size.
"""
import math

from common import ball, box, cyl, part, mirror

STONE, STONE_D, BRASS, STEEL, DARK = "#8d8478", "#5f574e", "#c9a45a", "#b9c2cc", "#2b2530"
COL = {"W": "#fff1b8", "U": "#6fb0ff", "B": "#a07ce0", "R": "#ff7a3c", "G": "#7fd36a", "C": "#e8e0ff"}


def sigil(colours, emblem):
    """A stone disc, a bead ring in the colours by turns, and an emblem at y ~55."""
    out = [
        cyl(34, 6, STONE_D, 0, 0, 0),
        cyl(30, 3, STONE, 0, 6, 0),
        cyl(10, 2, DARK, 0, 9, 0),
    ]
    n = 16
    for i in range(n):
        a = 2 * math.pi * i / n
        c = COL[colours[i % len(colours)]]
        out.append(ball(2.6, c, 27 * math.sin(a), 11, 27 * math.cos(a), g=800))
    # a thin shaft of light up to the emblem
    out.append(part("cyl", (0, 30, 0), (3, 40, 3), COL[colours[0]], g=600))
    return out + [dict(p, p=[p["p"][0], p["p"][1] + 50, p["p"][2]]) for p in emblem]


def beads_ring(cx, cy, cz, r, n, c, size=3, axis="y", g=0):
    """Beads round a circle: about the vertical (axis y) or face on (axis z)."""
    out = []
    for i in range(n):
        a = 2 * math.pi * i / n
        if axis == "y":
            p = (cx + r * math.sin(a), cy, cz + r * math.cos(a))
        else:
            p = (cx + r * math.sin(a), cy + r * math.cos(a), cz)
        out.append(ball(size, c, *p, g=g))
    return out


# ---------------------------------------------------------------- the emblems

def arrest():
    # two shackles, linked, with a chain between
    return beads_ring(-8, 6, 0, 8, 12, STEEL, 2.4, "z") + beads_ring(8, 6, 0, 8, 12, STEEL, 2.4, "z") + [
        ball(3, COL["W"], 0, 6, 0, g=700)
    ]


def ethereal_armor():
    # a breastplate of light
    return [
        ball(14, COL["W"], 0, 6, 0, 1.0, 1.2, 0.55, g=500),
        ball(9, "#ffffff", 0, 10, 4, 0.8, 0.7, 0.4, g=900),
        box(26, 3, 8, COL["W"], 0, 18, 0),
    ]


def paralyzing_grasp():
    # an icy hand closing
    out = [ball(9, "#bfe3ff", 0, 0, 0, 1.0, 0.8, 0.7, g=300)]
    for i, x in enumerate((-7, -3.5, 0, 3.5)):
        out.append(part("cone", (x, 12, 2), (3.5, 14 - abs(i - 1.5) * 2, 3.5), "#9fd2ff", t=(-80, 0, 0), g=400))
    out.append(part("cone", (9, 4, 2), (3.5, 10, 3.5), "#9fd2ff", t=(0, -250, 0), g=400))
    return out


def stab_wound():
    # a black dagger, point down, a red drop below it
    return [
        part("cone", (0, -6, 0), (5, 26, 2), STEEL, t=(0, 500, 0)),
        box(16, 3, 4, DARK, 0, 8, 0),
        box(3, 10, 3, "#4a2a2a", 0, 14, 0),
        ball(3, DARK, 0, 20, 0),
        ball(2.5, "#c0202a", 0, -22, 0, 1, 1.4, 1, g=500),
    ]


def shrieking_affliction():
    # a wailing ghost face
    return [
        ball(13, "#d9cfee", 0, 0, 0, 1, 1.25, 0.8, g=300),
        ball(3, DARK, -5, 4, 9), ball(3, DARK, 5, 4, 9),
        ball(5, DARK, 0, -7, 9, 0.8, 1.4, 0.5),
        part("cone", (0, -16, -3), (14, 12, 8), "#d9cfee", t=(500, 0, 0), g=200),
    ]


def pyroconvergence():
    # flames rising from a ring
    out = beads_ring(0, -6, 0, 10, 8, BRASS, 2.5)
    for i in range(5):
        a = 2 * math.pi * i / 5
        out.append(part("cone", (7 * math.sin(a), 6, 7 * math.cos(a)), (7, 18, 7), COL["R"], g=800))
    out.append(part("cone", (0, 10, 0), (9, 26, 9), "#ffd24a", g=900))
    return out


def deaths_presence():
    # a skull sprouting vines
    out = [
        ball(10, "#e8e0cc", 0, 0, 0, 1, 0.95, 1),
        ball(2.6, DARK, -4, 1, 8), ball(2.6, DARK, 4, 1, 8),
        box(10, 4, 6, "#e8e0cc", 0, -8, 3),
    ]
    for i in range(4):
        a = 2 * math.pi * i / 4 + 0.4
        out.append(part("cone", (9 * math.sin(a), 12, 9 * math.cos(a)), (4, 16, 4), COL["G"], t=(80 * math.cos(a), 0, 80 * math.sin(a))))
        out.append(ball(3, "#4f8f3a", 13 * math.sin(a), 20, 13 * math.cos(a)))
    return out


def detention_sphere():
    # a sphere of light inside two crossing bands
    return [ball(10, COL["U"], 0, 0, 0, g=900)] + beads_ring(0, 0, 0, 15, 14, COL["W"], 2.2, "z", g=500) + [
        dict(p, p=[p["p"][2], p["p"][1], p["p"][0]]) for p in beads_ring(0, 0, 0, 15, 14, COL["W"], 2.2, "z", g=500)
    ]


def growing_ranks():
    # a sapling with paired leaves
    out = [cyl(2, 26, "#7a5236", 0, -8, 0)]
    for y, s in ((2, 1.0), (10, 0.85), (18, 0.7)):
        out.append(ball(7 * s, COL["G"], -7 * s, y, 0, 1.3, 0.5, 0.8, t=(0, 120, 0)))
        out.append(ball(7 * s, COL["G"], 7 * s, y, 0, 1.3, 0.5, 0.8, t=(0, -120, 0)))
    out.append(ball(4, COL["W"], 0, 22, 0, g=700))
    return out


def knightly_valor():
    # a knight's helm with a white plume
    return [ball(10, STEEL, 0, 0, 0, 1, 1.1, 1), box(14, 3, 3, DARK, 0, 0, 9),
            part("cone", (0, 14, -2), (5, 16, 10), COL["W"], t=(-150, 0, 0), g=500)]


def martial_law():
    # a raised gavel over a seal
    return [cyl(12, 3, BRASS, 0, -10, 0), box(18, 8, 8, "#6a4a2a", 0, 8, 0), box(3, 20, 3, "#5a3a2a", 0, -2, 0),
            ball(3, COL["W"], 0, -8, 12, g=700)]


def rest_in_peace():
    # a gravestone, a white flower on it
    return [box(18, 24, 5, STONE, 0, 0, 0), ball(9, STONE, 0, 12, 0, 1, 0.6, 0.28), ball(4, COL["W"], 0, 2, 4, g=700)]


def security_blockade():
    # a tower shield with a bar across it
    return [box(18, 26, 3, STEEL, 0, 0, 0), box(22, 4, 5, BRASS, 0, 0, 2), ball(3, COL["W"], 0, 8, 3, g=700)]


def soul_tithe():
    # a pair of scales
    out = [box(2, 22, 2, BRASS, 0, 0, 0), box(26, 2, 2, BRASS, 0, 10, 0)]
    for x in (-12, 12):
        out += [cyl(6, 1.5, BRASS, x, 0, 0), ball(2.5, COL["W"], x, 3, 0, g=700)]
    return out


def sphere_of_safety():
    # a shell of light round a small tower
    return [box(6, 14, 6, STONE, 0, -4, 0), part("cone", (0, 6, 0), (8, 6, 8), COL["W"]),
            ball(15, COL["W"], 0, 0, 0, g=200)]


def chronic_flooding():
    # a wave curling over
    out = []
    for i in range(6):
        a = math.pi * i / 5
        out.append(ball(5 - i * 0.4, COL["U"], 12 * math.cos(a), 10 * math.sin(a), 0, g=400))
    return out + [ball(4, "#e8f6ff", -12, 0, 2, g=700)]


def conjured_currency():
    # a stack of coins of light
    return [part("cyl", (0, -7 + 3 * i, 0), (16, 2, 16), BRASS, g=200 * i) for i in range(5)] + [ball(3, COL["U"], 0, 10, 0, g=900)]


def search_the_city():
    # a magnifying glass over five face-down cards
    out = [part("cube", (-14 + 7 * i, -10, 0), (6, 1, 9), COL["U"], g=300) for i in range(5)]
    return out + [cyl(9, 2, BRASS, 0, 6, 0, t=(250, 0, 0)), box(3, 12, 3, "#5a3a2a", 6, -2, 0)]


def deviant_glee():
    # a grinning mask
    return [ball(12, "#e8e0cc", 0, 0, 0, 1, 1.1, 0.5), ball(3, DARK, -5, 3, 6), ball(3, DARK, 5, 3, 6),
            part("cube", (0, -5, 6), (12, 2, 2), COL["B"], g=600)]


def grave_betrayal():
    # a hand rising from an open grave
    return [box(22, 4, 12, STONE_D, 0, -12, 0), box(4, 14, 4, "#a8b0a0", 0, -2, 0)] + [
        part("cone", (x, 8, 0), (2.5, 8, 2.5), "#a8b0a0") for x in (-3, -1, 1, 3)] + [ball(4, COL["B"], 0, -10, 6, g=800)]


def underworld_connections():
    # a chain of black links running down
    return beads_ring(0, 6, 0, 6, 10, DARK, 2, "z") + beads_ring(0, -6, 0, 6, 10, DARK, 2, "z") + [ball(3, COL["B"], 0, 0, 0, g=800)]


def guild_feud():
    # two crossed swords, red and black
    return [part("cone", (0, 0, 0), (4, 34, 2), STEEL, t=(0, 0, 300)), part("cone", (0, 0, 0), (4, 34, 2), STEEL, t=(0, 0, -300)),
            ball(4, COL["R"], -8, -10, 2, g=700), ball(4, COL["B"], 8, -10, 2, g=700)]


def pursuit_of_flight():
    # a single red wing
    return [part("cone", (-6 + 4 * i, 4 * i, 0), (5, 16 + 4 * i, 2), COL["R"], t=(0, 0, -200 + 60 * i), g=400) for i in range(4)]


def racecourse_fury():
    # a hoof print in fire
    return [ball(8, DARK, 0, -6, 0, 1, 0.5, 1)] + [part("cone", (x, 6, 0), (5, 16, 5), COL["R"], g=800) for x in (-6, 0, 6)]


def mana_bloom():
    # a flower opening, charge at its heart
    out = [ball(4, "#ffd24a", 0, 0, 0, g=900)]
    for i in range(6):
        a = 2 * math.pi * i / 6
        out.append(ball(6, COL["G"], 8 * math.sin(a), 0, 8 * math.cos(a), 1, 0.4, 1))
    return out


def urban_burgeoning():
    # a sprout through a broken paving stone
    return [box(20, 3, 20, STONE, 0, -10, 0), cyl(1.5, 16, "#4f8f3a", 0, -8, 0), ball(5, COL["G"], -4, 6, 0, 1.4, 0.5, 0.8),
            ball(5, COL["G"], 4, 8, 0, 1.4, 0.5, 0.8)]


def collective_blessing():
    # three figures under a halo
    out = [ball(4, COL["G"], x, -4, 0) for x in (-8, 0, 8)] + [part("cone", (x, -12, 0), (7, 12, 7), COL["W"]) for x in (-8, 0, 8)]
    return out + beads_ring(0, 8, 0, 12, 10, "#ffd24a", 2, "y", g=900)


def havoc_festival():
    # a flaming jester's cap
    return [part("cone", (x, 4, 0), (6, 20, 6), c, t=(0, 0, 160 * s)) for x, c, s in ((-6, COL["R"], 1), (6, COL["B"], -1))] + [
        ball(3, "#ffd24a", -12, 12, 0, g=900), ball(3, "#ffd24a", 12, 12, 0, g=900), ball(6, DARK, 0, -6, 0)]


def righteous_authority():
    # a crown with a blue gem
    return [cyl(10, 6, BRASS, 0, 0, 0)] + [part("cone", (8 * math.sin(a), 6, 8 * math.cos(a)), (3, 8, 3), BRASS) for a in (0, 1.6, 3.2, 4.8)] + [
        ball(3.5, COL["U"], 0, 0, 10, g=900)]


# ---------------------------------------------------------------- the objects

def keyrune(c1, c2, emblem):
    """A keystone of carved stone on its point, the guild's emblem glowing on its face."""
    out = [
        part("gem", (0, 28, 0), (34, 56, 16), STONE),
        part("gem", (0, 28, 1), (26, 44, 16), STONE_D),
        ball(6, COL[c1], 0, 30, 9, 1, 1, 0.4, g=900),
        ball(4, COL[c2], 0, 18, 8, 1, 1, 0.4, g=900),
    ]
    return out + emblem


def bird_marks(c):
    return [part("cone", (-9, 34, 8), (5, 14, 2), COL[c], t=(0, 300, 0), g=600), part("cone", (9, 34, 8), (5, 14, 2), COL[c], t=(0, -300, 0), g=600)]


def insect_marks(c):
    return [ball(3, COL[c], x, 38, 8, g=600) for x in (-8, 8)] + [part("cone", (0, 44, 8), (2, 10, 2), COL[c], g=600)]


def chromatic_lantern():
    out = [
        cyl(16, 4, BRASS, 0, 0, 0),
        cyl(3, 10, BRASS, 0, 4, 0),
        cyl(14, 3, BRASS, 0, 14, 0),
        cyl(14, 3, BRASS, 0, 50, 0),
        part("cone", (0, 60, 0), (24, 14, 24), BRASS),
        ball(4, BRASS, 0, 70, 0),
        ball(9, "#fff6d8", 0, 33, 0, g=1000),
    ]
    for i, c in enumerate("WUBRG"):
        a = 2 * math.pi * i / 5
        out.append(box(11, 30, 2, COL[c], 12 * math.sin(a), 32, 12 * math.cos(a), t=(0, 0, round(i * 200))))
        out.append(box(2, 36, 2, BRASS, 14 * math.sin(a + math.pi / 5), 32, 14 * math.cos(a + math.pi / 5)))
    return out


def civic_saber():
    # a sword standing in a stone, point down
    return [
        cyl(12, 8, STONE, 0, 0, 0),
        part("cone", (0, 30, 0), (6, 50, 2), STEEL, t=(0, 500, 0)),
        box(4, 30, 1.5, "#dfe6ee", 0, 38, 1),
        box(26, 4, 6, BRASS, 0, 56, 0),
        box(4, 14, 4, "#5a3a2a", 0, 64, 0),
        ball(4, BRASS, 0, 73, 0),
        ball(3, COL["W"], -10, 56, 3, g=700), ball(3, COL["U"], 10, 56, 3, g=700),
    ]


def codex_shredder():
    # a carved stone beast's head, mouth full of grinding teeth
    out = [ball(26, STONE, 0, 30, 0, 1, 1.1, 1.2), ball(18, STONE_D, 0, 24, 22, 1.1, 0.8, 0.8)]
    for y in (16, 22, 28):
        out.append(box(28, 3, 4, DARK, 0, y, 34))
    out += [ball(4, COL["U"], -10, 42, 22, g=800), ball(4, COL["U"], 10, 42, 22, g=800)]
    for i in range(3):
        out.append(part("cone", (-12 + 12 * i, 56, -4), (8, 14, 8), STONE_D))
    return out


def pithing_needle():
    # a long needle through a stone ring
    return [cyl(12, 6, STONE, 0, 0, 0), part("cone", (0, 34, 0), (4, 60, 4), STEEL), ball(4, BRASS, 0, 66, 0), ball(3, COL["C"], 0, 10, 10, g=700)]


def tablet_of_the_guilds():
    # a stone tablet with ten guild marks
    out = [box(36, 48, 6, STONE, 0, 24, 0), box(40, 4, 8, STONE_D, 0, 2, 0)]
    for i, c in enumerate("WUBRGWUBRG"):
        out.append(ball(2.6, COL[c], -12 + 6 * (i % 5), 34 - 14 * (i // 5), 3.5, g=800))
    return out


MODELS = {
    "c3": sigil("W", arrest()),
    "c9": sigil("W", ethereal_armor()),
    "c46": sigil("U", paralyzing_grasp()),
    "c78": sigil("B", stab_wound()),
    "c76": sigil("B", shrieking_affliction()),
    "c103": sigil("R", pyroconvergence()),
    "c121": sigil("G", deaths_presence()),
    "c155": sigil("WU", detention_sphere()),
    "c217": sigil("GW", growing_ranks()),
    "c225": keyrune("W", "U", bird_marks("W")),
    "c229": keyrune("B", "G", insect_marks("G")),
    "c226": chromatic_lantern(),
    "c227": civic_saber(),
    "c13": sigil("W", knightly_valor()),
    "c14": sigil("W", martial_law()),
    "c18": sigil("W", rest_in_peace()),
    "c20": sigil("W", security_blockade()),
    "c23": sigil("W", soul_tithe()),
    "c24": sigil("W", sphere_of_safety()),
    "c32": sigil("U", chronic_flooding()),
    "c33": sigil("U", conjured_currency()),
    "c49": sigil("U", search_the_city()),
    "c65": sigil("B", deviant_glee()),
    "c67": sigil("B", grave_betrayal()),
    "c83": sigil("B", underworld_connections()),
    "c97": sigil("R", guild_feud()),
    "c102": sigil("R", pursuit_of_flight()),
    "c104": sigil("R", racecourse_fury()),
    "c130": sigil("G", mana_bloom()),
    "c138": sigil("G", urban_burgeoning()),
    "c150": sigil("GW", collective_blessing()),
    "c166": sigil("BR", havoc_festival()),
    "c189": sigil("UW", righteous_authority()),
    "c230": keyrune("U", "R", bird_marks("R")),
    "c232": keyrune("B", "R", insect_marks("R")),
    "c233": keyrune("G", "W", bird_marks("G")),
    "c228": codex_shredder(),
    "c231": pithing_needle(),
    "c235": tablet_of_the_guilds(),
}
