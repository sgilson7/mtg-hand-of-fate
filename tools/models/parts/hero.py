"""The player: a round wanderer after Mage Tower Battle's mages, in armour
whose colours the game swaps by what is worn (roles W, U and gold here)."""
from common import ball, cyl, part, eyes

SKIN = "skin"


def hero():
    out = [
        ball(45, "U", 0, 52, 0, 1, 0.9, 0.95),
        ball(34, "gold", 0, 22, 0, 1.25, 0.22, 1.2),
        ball(30, "U", -44, 82, 0, 1, 0.85, 1),
        ball(30, "U", 44, 82, 0, 1, 0.85, 1),
        ball(13, SKIN, -50, 52, 14),
        ball(13, SKIN, 50, 52, 14),
        ball(16, "#5a3a2a", -18, 8, 4, 1, 0.6, 1.4),
        ball(16, "#5a3a2a", 18, 8, 4, 1, 0.6, 1.4),
        ball(34, SKIN, 0, 112, 0, n="head"),
        ball(36, "U", 0, 126, -4, 1.02, 0.7, 1.02, n="head"),
        part("cone", (0, 148, -2), (16, 26, 16), "gold", "head"),
    ] + eyes(114, 30, 11, 5)
    out.append(cyl(3, 140, "wood", 56, 0, 18))
    out.append(ball(9, "#9be7ff", 56, 146, 18, g=900))
    return out


MODELS = {"hero": hero()}
