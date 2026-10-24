"""Scenery: CHUMWORLD's town buildings for the cities, and the board's
terrain pieces, places and markers.

PART 1, the town buildings, ported from CHUMWORLD (the person's own game,
MIT): chumworld-towns/crates/core/src/models/props.rs, `model(kind)` for each
`Prop` of chumworld-towns/crates/core/src/models.rs. Colours are CHUMWORLD's
data/models.json `colours`. CHUMWORLD works in millimetres with its front to
-z; `Kit` below takes the same numbers and writes centimetres with the front
to +z, so each port reads line for line against its function:

    chum_house       fn house (and fn gabled, its walls and roof)
    chum_hall        fn hall (fn gabled)
    chum_inn         fn inn (fn gabled)
    chum_stall       fn stall(cart_cloth, cart_stripe, fruit)   Prop::Stall
                     with fn stall(barn_trim, shutter, cloth)   Prop::StallCloth
                     beside it, the two as one model
    chum_watchtower  fn watchtower (fn Kit::slab for its stair)
    chum_well        fn well
    chum_gate        fn town_gate(shut = false)
    chum_notice      fn notice_board
    chum_mill        fn mill (fn gabled)
    chum_shrine      fn shrine
    chum_tent        fn tent
    chum_fence       fn fence
    chum_bridge      fn bridge

CHUMWORLD's meshes this format lacks, and what stands in: Facet and Dot are
spheres, Taper a frustum, Tube a cylinder (the mill's wheel rims are rings of
blocks), Pyramid the top half of a gem turned an eighth (its lower half sits
inside what it caps), Wedge a cone flattened to a triangle (the gable ends,
the tent's ends, the pennants) with slabs for its slopes. Parts below the
ground in CHUMWORLD (the gate's footings, the bridge's piles) stop at y = 0.
They keep CHUMWORLD's sizes, so a house is 6 m wide: a city scales them.

PART 2, the board (each on a 200 cm tile, centred, on y = 0), the places and
the markers: trees, mountain, peak, dead tree, swamp pool, plains columns,
canals, the five guild towers, Niv-Mizzet's aerie, the card stand, the five
land crystals, the guildgate marker, lamp, fountain, plaza, arena, pedestal.
"""
import math

from common import ball, box, cyl, part


def tt(deg):
    """Degrees to thousandths of a turn."""
    return deg * 1000 / 360


# ---------------------------------------------------------------- helpers

def bar(m, a, b, thick, c, g=0):
    """A bar of mesh m from a to b (cm), thick across."""
    dx, dy, dz = b[0] - a[0], b[1] - a[1], b[2] - a[2]
    L = math.sqrt(dx * dx + dy * dy + dz * dz)
    pitch = math.degrees(math.acos(max(-1, min(1, dy / L))))
    yaw = math.degrees(math.atan2(dx, dz))
    mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, (a[2] + b[2]) / 2)
    return part(m, mid, (thick, L, thick), c, t=(tt(pitch), 0, tt(yaw)), g=g)


def slab(a, b, width, thick, c, m="cube"):
    """A slab width across (x) whose top face runs from a to b (same x, cm)."""
    dy, dz = b[1] - a[1], b[2] - a[2]
    L = math.hypot(dy, dz)
    ny, nz = abs(dz) / L, (-dy if dz > 0 else dy) / L
    theta = math.atan2(nz, ny)
    mid = (a[0], (a[1] + b[1]) / 2 - ny * thick / 2, (a[2] + b[2]) / 2 - nz * thick / 2)
    return part(m, mid, (width, thick, L), c, t=(tt(math.degrees(theta)), 0, 0))


def hring(n, r, y, size, c, x0=0, z0=0, g=0, mesh="cube", phase=0):
    """n blocks round a level ring of radius r, each along the ring."""
    out = []
    for i in range(n):
        a = 2 * math.pi * i / n + phase
        out.append(part(mesh, (x0 + r * math.cos(a), y, z0 + r * math.sin(a)), size, c,
                        t=(0, 0, tt(-math.degrees(a) - 90)), g=g))
    return out


def tilt_axis(roll, pitch):
    """Where a part's +y points after roll then pitch (degrees)."""
    r, p = math.radians(roll), math.radians(pitch)
    return (-math.sin(r), math.cos(r) * math.cos(p), math.cos(r) * math.sin(p))


def pyramid(x, base, z, w, h, d, c):
    """A four-sided roof w x d, its base at y = base, h tall: a gem's top half."""
    return part("gem", (x, base, z), (w * 1.414, 2 * h, d * 1.414), c, t=(0, 0, 125))


# ------------------------------------------------- CHUMWORLD's kit, ported

CW = {
    "plaster": "#eee2c6", "timber": "#765236", "roof_tile": "#b45e42", "glass": "#4c5c7e",
    "shutter": "#608498", "door": "#8e623e", "brass": "#cea24c", "flower_red": "#de4e46",
    "flower_yellow": "#f6ce52", "stone": "#a29a8e", "stone_dark": "#807a72",
    "stone_light": "#beb8ac", "slate": "#5c6274", "hollow": "#302422", "candle": "#f2e8ce",
    "char_dark": "#2c2426", "tent_flag": "#b23838", "wood": "#926a44", "wood_dark": "#68482e",
    "water": "#487c9c", "rope": "#ccb684", "iron": "#5e606e", "fence": "#967856",
    "fence_rail": "#886c4e", "canvas": "#d2b280", "tent_dark": "#3e2e32",
    "canvas_patch": "#b07052", "shrine_stone": "#b0aea6", "moss": "#709252",
    "shrine_orb": "#aae6ff", "flame": "#ffa040", "plank_0": "#9e764e", "plank_1": "#926c46",
    "plank_2": "#a67e54", "sign_board": "#b08858", "notice_paper": "#eae0c4",
    "sign_paint": "#d65446", "cart_cloth": "#eee4ca", "cart_stripe": "#c44c40",
    "barn_trim": "#eee4d0", "leaf": "#56944a", "straw": "#e8c670", "quilt": "#7090c4",
    "crate": "#ba925c", "sack": "#d6c096", "lamp_glow": "#ffd68c", "barrel": "#8c5e3a",
}

MESH = {"Cube": "cube", "Facet": "sphere", "Dot": "sphere", "Cyl": "cyl", "Cone": "cone",
        "Taper": "frustum", "Tube": "cyl"}


class Kit:
    """CHUMWORLD's Kit: millimetres, front to -z, turns in degrees (rot_x, rot_z,
    rot_y). Writes parts in centimetres, front to +z (z and the x and y turns
    mirror)."""

    def __init__(self, dx=0, dz=0):
        self.out, self.dx, self.dz = [], dx, dz

    def c(self, name):
        return CW[name]

    def cm(self, at):
        return ((at[0] + self.dx) / 10, at[1] / 10, -(at[2] + self.dz) / 10)

    def shape(self, mesh, at, size, rgb, rx=0, rz=0, ry=0, g=0):
        m = MESH.get(mesh, mesh)
        self.out.append(part(m, self.cm(at), (size[0] / 10, size[1] / 10, size[2] / 10), rgb,
                             t=(tt(-rx), tt(rz), tt(-ry)), g=g))

    def bx(self, at, size, rgb):
        self.shape("Cube", at, size, rgb)

    def up(self, mesh, at, size, rgb):
        if mesh == "Pyramid":
            x, y, z = self.cm(at)
            self.out.append(pyramid(x, y - size[1] / 20, z, size[0] / 10, size[1] / 10, size[2] / 10, rgb))
        elif mesh == "Wedge":
            # a triangle (ridge along z, thin): a flattened cone
            self.shape("cone", at, size, rgb)
        else:
            self.shape(mesh, at, size, rgb)

    def glow(self, mesh, at, size, rgb, g=800):
        self.shape(mesh, at, size, rgb, g=g)

    def bar(self, mesh, a, b, thick, rgb):
        self.out.append(bar(MESH.get(mesh, mesh), self.cm(a), self.cm(b), thick / 10, rgb))

    def slab(self, a, b, width, thick, rgb):
        self.out.append(slab(self.cm(a), self.cm(b), width / 10, thick / 10, rgb))


def gabled(k, w, d, h, footing, wall, roof, cap):
    stone = k.c("stone_dark")
    k.bx((0, footing / 2, 0), (w + 300, footing, d + 300), stone)
    top = footing + h
    k.bx((0, footing + h / 2 - 10, 0), (w, h + 20, d), wall)
    rise = d / 2 * 839 / 1000
    # the gable: a triangle at each end wall (CHUMWORLD's Wedge)
    for s in (-1, 1):
        k.shape("cone", (s * (w / 2 - 20), top + rise / 2, 0), (30, rise, d), wall)
    slope = d / 2 * 1305 / 1000
    length, thick = slope + 450 + 120, 220
    shift = (450 - 120) / 2
    for side in (-1, 1):
        s40, c40 = 643, 766
        mid_y = top + rise / 2
        mid_z = side * d / 4
        out_n = thick / 2 - 25
        y = mid_y + out_n * c40 / 1000 - shift * s40 / 1000
        z = mid_z + side * (out_n * s40 / 1000 + shift * c40 / 1000)
        k.shape("Cube", (0, y, z), (w + 700, thick, length), roof, rx=side * 40)
    k.shape("Cyl", (0, top + rise + 150, 0), (300, w + 760, 300), cap, rz=90)
    return top


def chum_house():
    k = Kit()
    plaster, timber, roof, glass, shutter = (k.c(n) for n in ("plaster", "timber", "roof_tile", "glass", "shutter"))
    w, d, h, f = 6000, 5000, 2600, 400
    top = gabled(k, w, d, h, f, plaster, roof, timber)
    for x in (-1, 1):
        for z in (-1, 1):
            k.bx((x * w / 2, f - 20 + (h - 20) / 2, z * d / 2), (260, h - 20, 260), timber)
    for y in (top - 140, f + 300):
        for x in (-1, 1):
            k.bx((x * w / 2, y, 0), (200, 200, d - 200), timber)
    for z in (-1, 1):
        k.bx((0, top - 140, z * d / 2), (w - 200, 200, 200), timber)
    for x in (-1, 1):
        a, b = 710 - 20, w / 2 - 100
        k.bx((x * (a + b) / 2, f + 300, -d / 2), (b - a, 200, 200), timber)
    k.bx((0, f + 300, d / 2), (w - 200, 200, 200), timber)
    for x in (-1, 1):
        for z in (-1, 1):
            k.shape("Cube", (x * (w / 2 + 30), f + 620, z * 1200), (140, 1300, 120), timber, rx=z * 45)
    door = k.c("door")
    front = -d / 2
    k.bx((0, f - 10 + 1000, front), (1100, 2000, 160), door)
    k.bx((0, f + 1300, front - 90), (1000, 110, 40), timber)
    for x in (-1, 1):
        k.bx((x * 630, f - 10 + 1050, front), (160, 2100, 200), timber)
    k.bx((0, f + 2080, front), (1380, 160, 200), timber)
    k.up("Facet", (380, f + 1000, front - 110), (80, 80, 80), k.c("brass"))
    flowers = [k.c("flower_red"), k.c("flower_yellow"), k.c("flower_red")]
    for x in (-1900, 1900):
        y = f + 1300
        k.bx((x, y, front), (900, 800, 120), glass)
        k.bx((x, y, front - 70), (60, 800, 40), timber)
        k.bx((x, y, front - 70), (900, 60, 40), timber)
        for s in (-1, 1):
            k.bx((x + s * 680, y, front - 50), (420, 860, 80), shutter)
        k.bx((x, y - 460, front - 60), (1100, 100, 240), timber)
        k.bx((x, y - 640, front - 130), (1000, 220, 260), door)
        for i, c in enumerate(flowers):
            k.up("Facet", (x - 300 + 300 * i, y - 500, front - 130), (220, 160, 200), c)
    for x in (-1, 1):
        k.bx((x * w / 2, f + 1300, 0), (120, 800, 900), glass)
        k.bx((x * (w / 2 + 70), f + 1300, 0), (40, 800, 60), timber)
    k.bx((1800, top + 1200, 900), (600, 2800, 600), k.c("stone"))
    k.bx((1800, top + 2680, 900), (760, 160, 760), k.c("stone_dark"))
    return k.out


def chum_hall():
    k = Kit()
    plaster, timber, slate, stone = (k.c(n) for n in ("plaster", "timber", "slate", "stone"))
    light, glass, door, gold = (k.c(n) for n in ("stone_light", "glass", "door", "brass"))
    w, d, h, f = 14000, 9000, 7000, 700
    top = gabled(k, w, d, h, f, plaster, slate, timber)
    front = -d / 2
    k.bx((0, 2475, 0), (w + 80, 3650, d + 80), stone)
    for x in (-1, 1):
        for z in (-1, 1):
            k.bx((x * w / 2, 5980, z * d / 2), (240, 3360, 240), timber)
    for x in (-4200, -1400, 1400, 4200):
        k.bx((x, 5980, front), (200, 3360, 240), timber)
    k.bx((0, 7500, front), (13600, 200, 160), timber)
    for x in (-5000, -2500, 2500, 5000):
        k.bx((x, 2400, front - 40), (1000, 1800, 120), glass)
        k.bx((x, 1440, front - 100), (1300, 120, 260), light)
        k.bx((x, 3400, front - 60), (1300, 200, 200), light)
    for x in (-5600, -2800, 2800, 5600):
        k.bx((x, 6000, front - 20), (900, 1200, 120), glass)
    # the round window: its timber ring behind the glass (CHUMWORLD's Tube)
    k.shape("Cyl", (0, 6000, front - 30), (1400, 120, 1400), glass, rx=90)
    k.shape("Cyl", (0, 6000, front - 5), (1640, 60, 1640), timber, rx=90)
    k.bx((0, 2300, front - 80), (2400, 3200, 120), door)
    k.bx((0, 2300, front - 150), (60, 3100, 40), timber)
    for x in (-1, 1):
        k.bx((x * 1350, 2450, front - 100), (300, 3500, 300), light)
        k.up("Facet", (x * 300, 2300, front - 170), (110, 110, 110), gold)
    k.bx((0, 4050, front - 100), (3000, 300, 300), light)
    k.bx((0, 330, -(d + 300) / 2 - 300), (4200, 660, 600), light)
    k.bx((0, 165, -(d + 300) / 2 - 900), (4200, 330, 600), stone)
    for x in (-1900, 1900):
        k.up("Cyl", (x, 2460, -5000), (500, 3600, 500), light)
        k.bx((x, 4330, -5000), (700, 140, 700), stone)
    # the pediment (CHUMWORLD's Wedge 4600 x 1200 x 1400): its face and slopes
    k.up("Wedge", (0, 5000, -5830), (4600, 1200, 40), slate)
    run = math.degrees(math.atan2(1200, 2300))
    for s in (-1, 1):
        k.shape("Cube", (s * 1150, 5000 - 10, -5150), (2650, 80, 1460), slate, rz=-s * run)
    k.bx((0, 4430, -5150), (4800, 120, 1460), light)
    red = k.c("tent_flag")
    for x in (-3700, 3700):
        k.bx((x, 2700, front - 80), (800, 2600, 40), red)
        k.bx((x, 1700, front - 120), (500, 300, 40), gold)
        k.shape("Cyl", (x, 4040, front - 120), (70, 1000, 70), timber, rz=90)
    rise = d / 2 * 839 / 1000
    base = top + rise - 700
    hollow = k.c("hollow")
    k.bx((0, base + 1400, 0), (2000, 2800, 2000), plaster)
    k.bx((0, base + 1000, -1030), (1100, 1200, 60), hollow)
    k.bx((0, base + 1000, 1030), (1100, 1200, 60), hollow)
    for x in (-1030, 1030):
        k.bx((x, base + 1000, 0), (60, 1200, 1100), hollow)
    k.up("Taper", (0, base + 1150, -1080), (600, 700, 60), gold)
    k.shape("Cyl", (0, base + 2150, -1040), (900, 80, 900), k.c("candle"), rx=90)
    k.bx((0, base + 2280, -1110), (60, 380, 40), k.c("char_dark"))
    k.shape("Cube", (90, base + 2110, -1110), (50, 260, 40), k.c("char_dark"), rz=-60)
    k.bx((0, base + 2800, 0), (2300, 160, 2300), timber)
    k.up("Pyramid", (0, base + 3700, 0), (2600, 1800, 2600), slate)
    k.up("Facet", (0, base + 4700, 0), (320, 320, 320), gold)
    return k.out


def chum_inn():
    k = Kit()
    plaster, timber, roof, glass, shutter = (k.c(n) for n in ("plaster", "timber", "roof_tile", "glass", "shutter"))
    w, d, h, f = 10000, 7000, 5600, 400
    top = gabled(k, w, d, h, f, plaster, roof, timber)
    front = -d / 2
    for x in (-1, 1):
        for z in (-1, 1):
            k.bx((x * w / 2, f - 20 + (h - 20) / 2, z * d / 2), (260, h - 20, 260), timber)
    for y in (top - 140, f + 2800):
        for z in (-1, 1):
            k.bx((0, y, z * d / 2), (w - 200, 200, 200), timber)
        for x in (-1, 1):
            k.bx((x * w / 2, y, 0), (200, 200, d - 200), timber)
    door = k.c("door")
    k.bx((0, f - 10 + 1100, front), (1400, 2200, 160), door)
    for x in (-1, 1):
        k.bx((x * 800, f - 10 + 1150, front), (200, 2300, 220), timber)
    k.bx((0, f + 2310, front), (1800, 200, 220), timber)
    for x in (-3300, -1900, 1900, 3300):
        k.bx((x, f + 1300, front), (1000, 900, 120), glass)
        k.bx((x, f + 1300, front - 70), (60, 900, 40), timber)
    flowers = [k.c("flower_red"), k.c("flower_yellow")]
    for i, x in enumerate((-3300, -1100, 1100, 3300)):
        y = f + 4100
        k.bx((x, y, front), (1000, 1000, 120), glass)
        for s in (-1, 1):
            k.bx((x + s * 720, y, front - 50), (440, 1060, 80), shutter)
        k.bx((x, y - 640, front - 130), (1200, 220, 260), door)
        for j in range(3):
            k.up("Facet", (x - 340 + 340 * j, y - 500, front - 130), (240, 170, 200), flowers[(i + j) % 2])
    for x in (-1, 1):
        k.bx((x * w / 2, f + 4100, 0), (120, 1000, 1000), glass)
    k.bx((0, 180, -(d + 300) / 2 - 675), (2800, 360, 1350), k.c("plank_1"))
    for x in (-1200, 1200):
        k.up("Cyl", (x, 1530, -4850), (160, 2700, 160), timber)
    k.slab((0, 2780, -5050), (0, 3100, -3560), 2700, 120, roof)
    iron = k.c("iron")
    k.bar("Cyl", (2600, 3500, front - 60), (2600, 3500, front - 1300), 70, iron)
    for z in (front - 480, front - 1080):
        k.bar("Cyl", (2600, 3500, z), (2600, 3110, z), 24, iron)
    k.bx((2600, 2700, front - 780), (80, 800, 1000), k.c("sign_board"))
    k.bx((2600, 2700, front - 780), (180, 420, 500), k.c("sign_paint"))
    k.bx((2600, 2700, front - 1080), (180, 260, 120), k.c("sign_paint"))
    for x in (-1100, 1100):
        k.up("Cyl", (x, f + 2000, front - 160), (160, 220, 160), k.c("brass"))
        k.glow("Dot", (x, f + 2000, front - 160), (130, 170, 130), k.c("lamp_glow"))
    for x, z in ((-2000, -4100), (-2650, -4250)):
        k.up("Cyl", (x, 400, z), (600, 800, 600), k.c("barrel"))
        k.up("Tube", (x, 560, z), (640, 60, 640), iron)
    stone, dark = k.c("stone"), k.c("stone_dark")
    for x in (-3500, 3500):
        k.bx((x, 7350, 1500), (700, 3700, 700), stone)
        k.bx((x, 9260, 1500), (860, 160, 860), dark)
    return k.out


def stall(k, cloth, stripe, fruit):
    wood, dark, plank = k.c("timber"), k.c("wood_dark"), k.c("plank_1")
    for x in (-1350, 1350):
        k.bx((x, 1230, -1000), (130, 2460, 130), wood)
        k.bx((x, 1450, 1000), (130, 2900, 130), wood)
    k.bx((0, 440, -700), (2400, 880, 480), plank)
    k.bx((0, 905, -700), (2600, 60, 600), dark)
    k.slab((0, 2440, -1250), (0, 2940, 1100), 3100, 80, cloth)
    for x in (-1200, -600, 0, 600, 1200):
        k.slab((x, 2467, -1310), (x, 2993, 1160), 300, 80, stripe)
    k.bx((0, 2290, -1240), (3180, 300, 40), stripe)
    if fruit:
        fr = [k.c("flower_red"), k.c("flower_yellow"), k.c("leaf")]
        for i, x in enumerate((-800, 0, 800)):
            k.up("Taper", (x, 1065, -720), (560, 260, 420), k.c("straw"))
            for j, dx in enumerate((-130, 0, 130)):
                k.up("Facet", (x + dx, 1230, -720 + 70 * (j - 1)), (170, 170, 170), fr[(i + j) % 3])
    else:
        bolts = [k.c("quilt"), k.c("flower_red"), k.c("canvas")]
        for i, x in enumerate((-900, -560, -220)):
            k.shape("Cyl", (x, 1090, -700), (300, 520 + 40 * i, 300), bolts[i], rx=90)
        for x, h in ((380, 420), (760, 520)):
            k.up("Taper", (x, 935 + h / 2, -700), (320, h, 320), k.c("roof_tile"))
    k.bx((-640, 300, 420), (600, 600, 600), k.c("crate"))
    k.up("Facet", (620, 340, 440), (620, 700, 520), k.c("sack"))


def chum_stall():
    """Prop::Stall (fruit, red stripes) and Prop::StallCloth (bolts, blue) side by side."""
    a, b = Kit(dx=-1800), Kit(dx=1800)
    stall(a, a.c("cart_cloth"), a.c("cart_stripe"), True)
    stall(b, b.c("barn_trim"), b.c("shutter"), False)
    return a.out + b.out


WATCH_Z, WATCH_HALF, WATCH_TOP, WATCH_FOOT, WATCH_STAIR_HALF = 3500, 1600, 4900, -5500, 700


def chum_watchtower():
    k = Kit()
    wood, dark, roof, flag = k.c("timber"), k.c("wood_dark"), k.c("roof_tile"), k.c("tent_flag")
    z0, p = WATCH_Z, 1450
    for x, z in ((-p, z0 - p), (p, z0 - p), (-p, z0 + p), (p, z0 + p)):
        k.bx((x, 3700, z), (200, 7400, 200), wood)
    k.bx((0, WATCH_TOP - 80, z0), (2 * WATCH_HALF, 160, 2 * WATCH_HALF), k.c("plank_1"))
    for z in (z0 - p, z0 + p):
        k.bx((0, 4670, z), (3000, 180, 160), dark)
    for x in (-p, p):
        k.bx((x, 4660, z0), (160, 180, 3000), dark)
    k.bx((0, 5800, z0 + p), (3000, 90, 90), wood)
    for x in (-p, p):
        k.bx((x, 5810, z0), (90, 90, 3000), wood)
    for x in (-1125, 1125):
        k.bx((x, 5790, z0 - p), (740, 90, 90), wood)
    k.bar("Cube", (-p, 300, z0 + 1430), (p, 4500, z0 + 1430), 90, dark)
    k.bar("Cube", (p, 300, z0 + 1470), (-p, 4500, z0 + 1470), 90, dark)
    for s in (-1, 1):
        k.bar("Cube", (s * 1430, 300, z0 - p), (s * 1430, 4500, z0 + p), 90, dark)
        k.bar("Cube", (s * 1470, 300, z0 + p), (s * 1470, 4500, z0 - p), 90, dark)
    k.up("Pyramid", (0, 7900, z0), (3900, 1100, 3900), roof)
    k.bar("Cyl", (0, 8300, z0), (0, 10300, z0), 70, dark)
    k.shape("cone", (430, 9950, z0), (760, 380, 40), flag, rz=-90)
    foot = (0, 0, WATCH_FOOT)
    head = (0, WATCH_TOP, z0 - WATCH_HALF)
    k.slab(foot, head, 2 * WATCH_STAIR_HALF - 100, 140, wood)
    n = 16
    for i in range(1, n):
        at = (0, foot[1] + (head[1] - foot[1]) * i / n, foot[2] + (head[2] - foot[2]) * i / n)
        a = (at[0], at[1] + 25, at[2] - 110)
        b = (at[0], at[1] + 25 + 150, at[2] + 110)
        k.slab(a, b, 2 * WATCH_STAIR_HALF - 240, 60, dark)
    for x in (-WATCH_STAIR_HALF, WATCH_STAIR_HALF):
        k.bar("Cyl", (x, 900, WATCH_FOOT + 60), (x, WATCH_TOP + 880, z0 - WATCH_HALF - 40), 70, wood)
        k.bx((x, 470, WATCH_FOOT + 60), (90, 940, 90), wood)
        k.bx((x * 8 / 10, 1300, -1500), (140, 2600, 140), dark)
    return k.out


def chum_well():
    k = Kit()
    stone, light, wood, water, roof = (k.c(n) for n in ("stone", "stone_light", "wood", "water", "roof_tile"))
    k.up("Tube", (0, 350, 0), (1700, 700, 1700), stone)
    k.up("Tube", (0, 740, 0), (1800, 80, 1800), light)
    k.up("Cyl", (0, 790, 0), (1250, 20, 1250), water)
    for x in (-1, 1):
        k.bx((x * 760, 700 + 950 - 40, 0), (130, 1980, 130), wood)
    k.shape("Cyl", (0, 2150, 0), (150, 1700, 150), wood, rz=90)
    k.bx((930, 2050, 0), (60, 260, 60), wood)
    # the little roof (CHUMWORLD's Wedge 1400 x 720 x 2100, ridge along x)
    for s in (-1, 1):
        k.slab((0, 3220, 0), (0, 2470, s * 760), 2100, 60, roof)
    for x in (-1030, 1030):
        k.shape("cone", (x, 2850, 0), (30, 720, 1400), roof, ry=90)
    k.bx((0, 2420, 0), (2100, 60, 1500), wood)
    k.bar("Cyl", (0, 2100, 0), (0, 1650, 0), 24, k.c("rope"))
    k.up("Taper", (0, 1500, 0), (300, 300, 300), wood)
    k.up("Tube", (0, 1500, 0), (320, 40, 320), k.c("iron"))
    return k.out


GATE_HALF, GATE_TOWER_X, GATE_TOWER_HALF, GATE_WING_END, GATE_WING_HALF = 2400, 4600, 2200, 18800, 1000
GATE_TOWER_TOP, GATE_WING_TOP = 11000, 7000


def chum_gate():
    k = Kit()
    stone, dark, light, slate = k.c("stone"), k.c("stone_dark"), k.c("stone_light"), k.c("slate")
    tx, th = GATE_TOWER_X, GATE_TOWER_HALF
    for s in (-1, 1):
        x = s * tx
        k.bx((x, GATE_TOWER_TOP / 2, 0), (2 * th, GATE_TOWER_TOP, 2 * th), stone)
        k.bx((x, 7400, 0), (2 * th + 120, 240, 2 * th + 120), light)
        for i in (-1, 0, 1):
            for z in (-1, 1):
                k.bx((x + i * 1500, GATE_TOWER_TOP + 410, z * (th - 440)), (800, 900, 800), dark)
        for z in (-1, 1):
            for xs in (-1, 1):
                k.bx((x + xs * (th - 440), GATE_TOWER_TOP + 410, z * 700), (800, 900, 600), dark)
        k.up("Pyramid", (x, GATE_TOWER_TOP + 1700, 0), (3400, 3400, 3400), slate)
        k.bar("Cyl", (x, GATE_TOWER_TOP + 3200, 0), (x, GATE_TOWER_TOP + 4900, 0), 80, k.c("wood_dark"))
        k.shape("cone", (x + 450, GATE_TOWER_TOP + 4550, 0), (820, 420, 40), k.c("tent_flag"), rz=-90)
        k.bx((x, 8900, -th - 60), (1400, 2600, 40), k.c("tent_flag"))
        k.bx((x, 9200, -th - 100), (500, 500, 40), k.c("brass"))
        k.up("Cyl", (s * (GATE_HALF + 400), 3800, -th - 160), (220, 300, 220), k.c("brass"))
        k.glow("Dot", (s * (GATE_HALF + 400), 3800, -th - 160), (170, 220, 170), k.c("lamp_glow"))
        for z in (-1, 1):
            k.bx((x, 5200, z * (th + 30)), (220, 1400, 60), k.c("hollow"))
        a, b = tx + th - 200, GATE_WING_END
        join = (a + b) / 2
        for p, q in ((a, join), (join, b)):
            k.bx((s * (p + q) / 2, GATE_WING_TOP / 2, 0), (q - p, GATE_WING_TOP, 2 * GATE_WING_HALF), stone)
        k.bx((s * join, (GATE_WING_TOP + 300) / 2, 0), (900, GATE_WING_TOP + 300, 2 * GATE_WING_HALF + 400), light)
        m = a + 900
        while m < b - 400:
            if abs(m - join) >= 900:
                k.bx((s * m, GATE_WING_TOP + 360, -GATE_WING_HALF + 210), (700, 800, 500), dark)
            m += 1400
    lo, hi = 5600, 8000
    k.bx((0, (lo + hi) / 2, 0), (2 * GATE_HALF + 400, hi - lo, 3000), stone)
    for s in (-1, 1):
        k.shape("Cube", (s * GATE_HALF, lo, 0), (1300, 1300, 2900), stone, rz=45)
        k.bx((s * (GATE_HALF - 30), lo + 1100, -1530), (300, 600, 60), light)
    k.bx((0, lo + 500, -1530), (900, 900, 60), light)
    for x in (-1500, 0, 1500):
        for z in (-1, 1):
            k.bx((x, hi + 360, z * 1200), (800, 800, 500), dark)
    wood, iron = k.c("wood_dark"), k.c("iron")
    leaf_h = lo - 600
    for s in (-1, 1):
        x = s * (GATE_HALF - 140)
        k.bx((x, leaf_h / 2, GATE_HALF / 2 + 200), (240, leaf_h, GATE_HALF - 20), wood)
        for y in (1000, leaf_h - 1000):
            k.bx((x - s * 150, y, GATE_HALF / 2 + 200), (100, 160, GATE_HALF - 200), iron)
    return k.out


def chum_notice():
    k = Kit()
    wood, dark, board, paper, wax = (k.c(n) for n in ("wood", "wood_dark", "sign_board", "notice_paper", "sign_paint"))
    for x in (-1000, 1000):
        k.bx((x, 1250, 0), (220, 2500, 220), wood)
    k.bx((0, 1650, 0), (1900, 1100, 60), board)
    for y, h in ((1100, 90), (2200, 90)):
        k.bx((0, y, 0), (1960, h, 140), dark)
    for s in (-1, 1):
        k.shape("Cube", (0, 2560, s * 230), (2400, 60, 560), dark, rx=s * 28)
    for x, y, w, h in ((-560, 1760, 520, 700), (40, 1820, 460, 560), (580, 1700, 500, 640), (60, 1380, 380, 300)):
        k.bx((x, y, -75), (w, h, 30), paper)
        k.bx((x, y + h / 2 - 90, -120), (70, 70, 30), wax)
    k.bx((0, 40, 0), (2400, 80, 400), k.c("stone"))
    return k.out


def chum_mill():
    k = Kit()
    stone, timber, roof, wood, dark = (k.c(n) for n in ("stone", "timber", "roof_tile", "wood", "wood_dark"))
    w, d, h, f = 6500, 5500, 4200, 500
    gabled(k, w, d, h, f, stone, roof, timber)
    front = -d / 2
    k.bx((0, f - 10 + 1050, front), (1200, 2100, 160), k.c("door"))
    k.bx((0, f + 2180, front), (1600, 200, 240), timber)
    for x in (-2000, 2000):
        k.bx((x, f + 1700, front), (800, 800, 120), k.c("glass"))
        k.bx((x, f + 1240, front - 80), (1000, 100, 240), timber)
    cx, cy = w / 2 + 1100, f + 2000
    k.shape("Cyl", (cx - 600, cy, 0), (260, 2100, 260), k.c("iron"), rz=90)
    for x in (cx - 330, cx + 330):
        # the rim (CHUMWORLD's Tube 5200 across): a ring of twelve blocks
        for s in range(12):
            a = 360 * s / 12
            r = 2530
            k.shape("Cube", (x, cy + r * math.cos(math.radians(a)), r * math.sin(math.radians(a))),
                    (140, 140, 1380), dark, rx=a)
        for s in range(4):
            k.shape("Cube", (x, cy, 0), (80, 4900, 80), wood, rx=45 * s)
    k.shape("Cyl", (cx, cy, 0), (800, 1000, 800), dark, rz=90)
    for s in range(16):
        a = 360 * s / 16
        y, z = 2250 * math.cos(math.radians(a)), 2250 * math.sin(math.radians(a))
        k.shape("Cube", (cx, cy + y, z), (720, 700, 90), wood, rx=a)
    pit = k.c("stone_dark")
    for x in (cx - 620, cx + 620):
        k.bx((x, 300, 0), (180, 600, 6000), pit)
    for z in (-2960, 2960):
        k.bx((cx, 300, z), (1420, 600, 180), pit)
    k.bx((cx, 250, 0), (1100, 400, 5800), k.c("water"))
    fy = cy + 2600 + 250
    z0, z1 = -500, 9000
    mid, length = (z0 + z1) / 2, z1 - z0
    k.bx((cx, fy, mid), (760, 80, length), wood)
    for x in (cx - 340, cx + 340):
        k.bx((x, fy + 200, mid), (80, 400, length - 60), wood)
    k.bx((cx, fy + 260, mid + 40), (600, 40, length - 160), k.c("water"))
    for z in (3500, 6300):
        for x in (cx - 460, cx + 460):
            k.bx((x, fy / 2 - 300, z), (140, fy + 600, 140), dark)
        k.bx((cx, fy - 260, z), (1100, 120, 160), dark)
    k.bx((cx, fy + 500, z1 + 500), (1800, 1400, 1800), wood)
    for y in (fy + 100, fy + 900):
        k.up("Tube", (cx, y, z1 + 500), (1880, 120, 1880), k.c("iron"))
    for x, z in ((cx - 700, z1 - 200), (cx + 700, z1 - 200), (cx - 700, z1 + 1200), (cx + 700, z1 + 1200)):
        k.bx((x, fy / 2 - 200, z), (160, fy + 400, 160), dark)
    k.up("Cyl", (-2300, 150, front - 1000), (1300, 300, 1300), k.c("stone_light"))
    k.up("Cyl", (-2300, 330, front - 1000), (300, 80, 300), k.c("stone_dark"))
    for x, z in ((1500, front - 700), (2050, front - 900)):
        k.up("Facet", (x, 360, z), (620, 720, 520), k.c("sack"))
    return k.out


def chum_shrine():
    k = Kit()
    stone, light, moss, roof = k.c("shrine_stone"), k.c("stone_light"), k.c("moss"), k.c("roof_tile")
    k.bx((0, 150, 0), (2400, 300, 2400), stone)
    k.bx((0, 430, 0), (1900, 300, 1900), light)
    for x in (-700, 700):
        for z in (-700, 700):
            k.up("Cyl", (x, 560 + 950 - 20, z), (260, 1940, 260), stone)
    k.bx((0, 2420, 0), (2500, 120, 2500), stone)
    k.up("Pyramid", (0, 2930, 0), (2300, 900, 2300), roof)
    k.up("Facet", (0, 3420, 0), (260, 260, 260), light)
    k.up("Cyl", (0, 900, 0), (500, 700, 500), light)
    k.up("Cyl", (0, 1280, 0), (620, 80, 620), stone)
    k.glow("Dot", (0, 1520, 0), (380, 380, 380), k.c("shrine_orb"), 700)
    for x in (-1, 1):
        for z in (-1, 1):
            k.up("Cyl", (x * 520, 640, z * 520), (80, 200, 80), k.c("candle"))
            k.glow("Cone", (x * 520, 790, z * 520), (60, 110, 60), k.c("flame"), 700)
    for x, z in ((-1150, -1150), (1150, 1150), (1150, -1150)):
        k.up("Facet", (x, 300, z), (420, 200, 360), moss)
    return k.out


def chum_tent():
    k = Kit()
    canvas, dark, pole, rope = k.c("canvas"), k.c("tent_dark"), k.c("wood_dark"), k.c("rope")
    # CHUMWORLD's Wedge 2800 x 2100 x 3400: two slopes and two ends
    run = math.degrees(math.atan2(2100, 1400))
    for s in (-1, 1):
        k.shape("Cube", (s * 700 + s * 20, 1050, 0), (2560, 50, 3400), canvas, rz=-s * run)
    for z in (-1690, 1690):
        k.up("Wedge", (0, 1040, z), (2800, 2080, 30), canvas)
    k.up("Wedge", (0, 630, -1720), (1500, 1300, 30), dark)
    for z in (-1760, 1760):
        k.bx((0, 1250, z), (80, 2500, 80), pole)
    for z in (-1, 1):
        k.bar("Cyl", (0, 2400, z * 1760), (0, 0, z * 2700), 22, rope)
        k.bx((0, 60, z * 2700), (60, 160, 60), pole)
    k.shape("cone", (230, 2380, -1760), (220, 380, 20), k.c("tent_flag"), rz=-90)
    k.shape("Cyl", (-1000, 130, -2300), (260, 900, 260), k.c("canvas_patch"), rz=90)
    return k.out


def chum_fence():
    k = Kit()
    wood = k.c("fence")
    for x in (-2000, 0, 2000):
        k.bx((x, 550, 0), (110, 1100, 110), wood)
        k.up("Pyramid", (x, 1160, 0), (110, 120, 110), wood)
    for y in (450, 850):
        k.bx((0, y, -60), (4000, 90, 50), k.c("fence_rail"))
    return k.out


def chum_bridge():
    k = Kit()
    wood, dark = k.c("wood"), k.c("wood_dark")
    planks = [k.c("plank_0"), k.c("plank_1"), k.c("plank_2")]
    for i in range(24):
        k.bx((0, 560, -5750 + 500 * i), (3000, 80, 460), planks[i % 3])
    for x in (-1200, 1200):
        k.bx((x, 400, 0), (250, 320, 12000), dark)
    for z in (-6000, -3000, 0, 3000, 6000):
        for x in (-1420, 1420):
            k.bx((x, 1050, z), (160, 1100, 160), wood)
            k.up("Pyramid", (x, 1660, z), (160, 120, 160), wood)
    for x in (-1420, 1420):
        k.bx((x, 1500, 0), (100, 120, 12200), wood)
        k.bx((x, 1050, 0), (80, 90, 12100), wood)
    for z in (-3000, 0, 3000):
        for x in (-1200, 1200):
            k.up("Cyl", (x, 250, z), (300, 500, 300), dark)
    return k.out


# ------------------------------------------------------------ the board

LEAF = ("#4fa04a", "#3f8c42", "#6cbf5a")
TRUNK = "#6a4630"
MARBLE, MARBLE_D = "#f1ebdc", "#d9d0bc"
RED_ROCK, RED_ROCK_D, RED_ROCK_L = "#b0563d", "#86402f", "#c9704f"
SNOW = "#f7f3ea"


def tree_a():
    """A round park tree in a low stone planter."""
    out = [
        cyl(34, 10, "#cfc6b0", 0, 0, 0),
        cyl(30, 3, "#5a4030", 0, 9, 0),
        cyl(9, 70, TRUNK, 0, 8, 0, mesh="frustum"),
        bar("cyl", (0, 50, 0), (22, 78, 6), 6, TRUNK),
        ball(44, LEAF[0], 0, 112, 0, 1, 0.9, 1),
        ball(30, LEAF[1], 30, 96, 14, 1, 0.9, 1),
        ball(28, LEAF[2], -26, 128, -8, 1, 0.9, 1),
        ball(26, LEAF[1], -22, 92, 22, 1, 0.9, 1),
        ball(22, LEAF[2], 12, 142, 16, 1, 0.9, 1),
    ]
    return out


def tree_b():
    """A taller park tree: a crown in two tiers, a planter ring of hedge."""
    out = [
        cyl(30, 8, "#cfc6b0", 0, 0, 0),
        cyl(7, 120, TRUNK, 0, 6, 0, mesh="frustum"),
        bar("cyl", (0, 70, 0), (-24, 92, 0), 5, TRUNK),
        bar("cyl", (0, 86, 0), (20, 108, -8), 5, TRUNK),
        ball(30, LEAF[1], -24, 96, 4, 1, 0.8, 1),
        ball(28, LEAF[1], 22, 110, -6, 1, 0.8, 1),
        ball(38, LEAF[0], 0, 140, 0, 1, 0.85, 1),
        ball(24, LEAF[2], 6, 172, 6, 1, 0.9, 1),
        ball(20, LEAF[2], -18, 126, 26, 1, 0.9, 1),
    ]
    for i in range(5):
        a = 2 * math.pi * i / 5 + 0.3
        out.append(ball(10, "#3f8c42", 28 * math.cos(a), 10, 28 * math.sin(a), 1, 0.8, 1))
    return out


def peak_part(x, z, R, H, c, roll=0, pitch=0, cap=0.28, sink=4):
    """A cone peak with a snow cap at its tip, both tilted alike."""
    ax = tilt_axis(roll, pitch)
    cy = H / 2 - sink
    out = [part("cone", (x, cy, z), (2 * R, H, 2 * R), c, t=(tt(pitch), tt(roll), 0))]
    if cap:
        h = H * cap
        d = H / 2 - h / 2 + 1
        r = R * cap * 1.1
        out.append(part("cone", (x + ax[0] * d, cy + ax[1] * d, z + ax[2] * d), (2 * r, h + 2, 2 * r), SNOW,
                        t=(tt(pitch), tt(roll), 0)))
    return out


def mountain():
    """Two or three red-rock peaks with snow tips, about 110 cm."""
    out = []
    out += peak_part(-18, -14, 58, 112, RED_ROCK, roll=3, pitch=-2)
    out += peak_part(40, 22, 44, 82, RED_ROCK_D, roll=-5, pitch=3)
    out += peak_part(-46, 46, 32, 56, RED_ROCK_L, roll=6, pitch=4, cap=0.3)
    for x, y, z, s, c in ((22, 8, 62, 18, RED_ROCK_D), (-70, 6, -2, 16, RED_ROCK), (70, 6, -30, 15, RED_ROCK_L),
                          (8, 6, 34, 13, RED_ROCK_L), (-30, 5, -66, 14, RED_ROCK_D)):
        out.append(part("rock", (x, y, z), (2 * s, 1.3 * s, 2 * s), c, t=(0, 0, (x * 7) % 200)))
    return out


def peak():
    """An impassable jagged peak, about 260 cm."""
    out = []
    out += peak_part(0, -6, 70, 262, RED_ROCK, roll=2, pitch=-2, cap=0.24)
    out += peak_part(-46, 20, 46, 170, RED_ROCK_D, roll=10, pitch=6, cap=0.22)
    out += peak_part(44, 30, 40, 140, RED_ROCK_L, roll=-12, pitch=8, cap=0.22)
    out += peak_part(30, -48, 36, 120, RED_ROCK_D, roll=-8, pitch=-10, cap=0)
    # crags on its flanks
    for x, y, z, w, h, r, p in ((-26, 120, 24, 24, 60, 14, 8), (28, 96, 20, 22, 50, -16, 10),
                                (-14, 70, -46, 26, 44, 10, -12), (8, 170, 16, 16, 40, -6, 10)):
        out.append(part("chunk", (x, y, z), (w, h, w * 0.8), RED_ROCK_D, t=(tt(p), tt(r), 40)))
    for x, y, z, s, c in ((-74, 8, -30, 22, RED_ROCK_D), (74, 8, -6, 20, RED_ROCK), (-10, 6, 70, 18, RED_ROCK_L),
                          (52, 6, 64, 15, RED_ROCK_D), (-60, 6, 62, 14, RED_ROCK)):
        out.append(part("rock", (x, y, z), (2 * s, 1.4 * s, 2 * s), c, t=(0, 0, (x * 11) % 200)))
    return out


def dead_tree():
    """A bare black tree of the undercity, roots spread on the ground."""
    bark, dark = "#4a3f44", "#352d33"
    out = [
        cyl(12, 80, bark, 0, 0, 0, mesh="frustum"),
        bar("cyl", (0, 70, 0), (4, 118, -6), 7, bark),
        bar("cyl", (2, 60, 0), (34, 102, 8), 6, bark),
        bar("cyl", (34, 102, 8), (52, 112, 2), 3, dark),
        bar("cyl", (0, 52, 0), (-32, 88, 10), 6, bark),
        bar("cyl", (-32, 88, 10), (-44, 110, 18), 3, dark),
        bar("cyl", (4, 118, -6), (-12, 140, -12), 4, dark),
        bar("cyl", (4, 118, -6), (20, 136, -4), 3, dark),
        bar("cyl", (-6, 80, 0), (-20, 90, -26), 4, bark),
    ]
    for a in (0.3, 1.9, 3.4, 4.8):
        out.append(bar("cyl", (0, 8, 0), (30 * math.cos(a), 1, 30 * math.sin(a)), 7, dark))
    out.append(part("blob", (0, 1, 0), (60, 6, 60), "#2f2a30"))
    return out


def swamp_pool():
    """A dark flat pool with reeds, lily pads and two mushrooms."""
    mud, pool = "#3a3230", "#24302c"
    out = [
        part("cyl", (0, 2, 0), (170, 4, 140), mud),
        part("cyl", (0, 3, 4), (140, 4, 112), pool),
        part("cyl", (-30, 5, 26), (22, 1, 22), "#4d6b3c"),
        part("cyl", (20, 5, -18), (16, 1, 16), "#4d6b3c"),
        part("cyl", (40, 5, 30), (14, 1, 14), "#5b7a44"),
    ]
    for x, z, h in ((-66, -30, 60), (-58, -40, 48), (-72, -18, 52), (60, -44, 56), (68, -32, 44), (-20, -56, 50)):
        out.append(cyl(1.5, h, "#6b7a4a", x, 0, z))
        out.append(part("cyl", (x, h - 6, z), (5, 12, 5), "#5a3a26"))
    # mushrooms on the bank, softly glowing (Golgari)
    out += [
        cyl(4, 22, "#e6dcc8", 62, 0, 46),
        ball(14, "#8a5aa0", 62, 24, 46, 1, 0.5, 1, g=250),
        cyl(3, 14, "#e6dcc8", 76, 0, 34),
        ball(9, "#b98a3a", 76, 15, 34, 1, 0.5, 1, g=250),
        part("rock", (-56, 6, 52), (28, 14, 22), "#4a4046"),
    ]
    return out


def plains_columns():
    """A few white marble columns on a stone floor, one broken, a capital fallen."""
    out = [box(150, 4, 110, MARBLE_D, 0, 2, -10)]
    for x, z, h in ((-50, -40, 120), (0, -40, 120), (50, -40, 64)):
        out.append(box(28, 8, 28, MARBLE, x, 8, z))
        out.append(cyl(11, h, MARBLE, x, 12, z))
        if h > 100:
            out.append(cyl(15, 6, MARBLE_D, x, 12 + h, z, mesh="frustum", t=(500, 0, 0)))
            out.append(box(32, 8, 32, MARBLE, x, 12 + h + 10, z))
    out.append(box(136, 10, 32, MARBLE, -25, 12 + 120 + 19, -40))
    out.append(part("chunk", (50, 78, -40), (20, 8, 20), MARBLE_D))
    # the fallen column and its capital
    out.append(part("cyl", (-10, 11, 34), (22, 86, 22), MARBLE, t=(0, 250, 40)))
    out.append(box(32, 8, 32, MARBLE, 50, 15, 50, t=(30, 70, 60)))
    out.append(part("frustum", (46, 6, 30), (30, 12, 30), MARBLE_D, t=(0, 0, 0)))
    out.append(part("chunk", (-60, 5, 56), (14, 10, 12), MARBLE_D))
    return out


CANAL_STONE = ("#b8ad94", "#a89c84", "#c4baa2")
WATER, WATER_L = "#3b7fd0", "#6aa6e6"


def canal_straight():
    """Water straight across the tile along x between stone banks."""
    out = [part("ground", (0, 2, 0), (200, 1, 110), WATER)]
    for x, l in ((-60, 46), (40, 30)):
        out.append(part("ground", (x, 3, -16), (l, 1, 4), WATER_L))
    out.append(part("ground", (-10, 3, 18), (36, 1, 4), WATER_L))
    for s in (-1, 1):
        for i, x in enumerate((-75, -25, 25, 75)):
            out.append(box(49, 14, 44, CANAL_STONE[(i + (s > 0)) % 3], x, 7, s * 77))
        out.append(box(200, 4, 12, "#d8d0bc", 0, 16, s * 61))
    for x in (-70, 70):
        out.append(cyl(5, 16, "#6e625c", x, 14, 84))
    return out


def canal_pool():
    """Water in the middle of the tile only, in a ring of stone."""
    out = [part("ground", (0, 2, 0), (104, 1, 104), WATER),
           part("ground", (0, 2, 0), (104, 1, 104), WATER, t=(0, 0, 125)),
           part("ground", (-12, 3, -10), (30, 1, 4), WATER_L)]
    out += hring(10, 64, 8, (42, 16, 26), CANAL_STONE[0])
    out += hring(10, 56, 18, (36, 4, 10), "#d8d0bc", phase=0.31)
    return out


# --------------------------------------------------------- guild towers

def tower_azorius():
    w, wd, gold, blue, navy = "#f3ecd2", "#d9d2bf", "#f2c14e", "#2f6fc0", "#1f3f7a"
    out = [
        box(130, 14, 130, wd, 0, 7, 0),
        box(110, 10, 110, w, 0, 19, 0),
        cyl(44, 120, w, 0, 24, 0),
        cyl(47, 6, gold, 0, 24, 0),
        cyl(47, 6, gold, 0, 138, 0),
        cyl(58, 6, w, 0, 144, 0),
        cyl(34, 46, w, 0, 150, 0),
        cyl(37, 4, gold, 0, 194, 0),
        part("cone", (0, 236, 0), (74, 84, 74), w),
        part("cone", (0, 236, 0), (78, 30, 78), blue),
        ball(6, gold, 0, 282, 0),
        part("gem", (0, 296, 0), (10, 24, 10), gold, g=300),
    ]
    out[-3] = part("cone", (0, 210, 0), (78, 30, 78), blue)
    for i in range(4):
        a = math.pi / 2 * i + math.pi / 4
        x, z = 46 * math.cos(a), 46 * math.sin(a)
        yaw = tt(-math.degrees(a) + 90)
        out.append(part("cube", (x * 1.02, 96, z * 1.02), (22, 64, 2), blue, t=(0, 0, yaw)))
        out.append(part("cube", (x * 1.05, 128, z * 1.05), (26, 5, 3), gold, t=(0, 0, yaw)))
        out.append(part("cone", (x * 1.04, 62, z * 1.04), (22, 10, 2), blue, t=(500, 0, yaw)))
    for i in range(4):
        a = math.pi / 2 * i
        x, z = 35 * math.cos(a), 35 * math.sin(a)
        out.append(part("cube", (x, 170, z), (8, 22, 8), navy, t=(0, 0, tt(-math.degrees(a)))))
    for x in (-55, 55):
        for z in (-55, 55):
            out.append(cyl(6, 40, w, x, 14, z))
            out.append(part("cone", (x, 62, z), (14, 16, 14), gold))
    return out


def tower_izzet():
    copper, brass, iron, glow, red = "#c46a3c", "#d8a640", "#4e4a56", "#6fd6ff", "#d8452c"
    out = [
        cyl(58, 18, iron, 0, 0, 0),
        cyl(48, 100, copper, 0, 18, 0, mesh="frustum"),
        cyl(50, 6, brass, 0, 40, 0),
        cyl(42, 6, brass, 0, 90, 0),
        cyl(36, 40, copper, 0, 118, 0),
        cyl(52, 8, brass, 0, 158, 0),
        cyl(8, 60, brass, 0, 166, 0),
    ]
    for i, r in enumerate((26, 22, 18, 14, 10)):
        out.append(cyl(r, 3, glow, 0, 172 + 11 * i, 0))
        out[-1]["g"] = 700
    out.append(ball(16, glow, 0, 240, 0, g=900))
    for i in range(3):
        a = 2 * math.pi * i / 3
        x, z = 46 * math.cos(a), 46 * math.sin(a)
        out.append(bar("cyl", (x, 166, z), (x * 0.4, 236, z * 0.4), 4, copper))
        out.append(ball(5, glow, x * 0.4, 236, z * 0.4, g=800))
        out.append(part("gem", (x * 0.7, 210, z * 0.7), (4, 30, 4), glow, t=(0, 60, 0), g=900))
    # pipes down the side, windows aglow, a red banner
    for a in (0.6, 2.4, 4.2):
        x, z = 52 * math.cos(a), 52 * math.sin(a)
        out.append(bar("cyl", (x, 150, z), (x * 1.08, 18, z * 1.08), 7, brass))
    for a in (1.5, 3.3, 5.1):
        x, z = 42 * math.cos(a), 42 * math.sin(a)
        out.append(part("cube", (x, 70, z), (12, 22, 6), glow, t=(0, 0, tt(-math.degrees(a) + 90)), g=600))
    out.append(box(26, 50, 3, red, 0, 120, 40))
    out.append(part("cone", (0, 90, 40), (26, 10, 3), red, t=(500, 0, 0)))
    out.append(ball(6, brass, 0, 147, 41))
    return out


def tower_rakdos():
    black, dred, red, bone, fire, fire2 = "#2a1f26", "#6e1c22", "#b8302a", "#e8dcc0", "#ff7a26", "#ffd04a"
    out = [
        cyl(62, 16, black, 0, 0, 0),
        cyl(50, 90, dred, 0, 16, 0, mesh="frustum"),
        cyl(44, 8, black, 0, 70, 0),
        cyl(34, 60, black, 0, 104, 0, mesh="frustum"),
        cyl(30, 8, red, 0, 140, 0),
        cyl(36, 40, dred, 0, 146, 0),
        cyl(46, 8, black, 0, 186, 0),
        cyl(30, 12, black, 0, 194, 0, mesh="frustum"),
    ]
    for i in range(8):
        a = 2 * math.pi * i / 8
        x, z = 42 * math.cos(a), 42 * math.sin(a)
        out.append(bar("cone", (x * 0.8, 190, z * 0.8), (x * 1.3, 226, z * 1.3), 12, bone))
    for i in range(6):
        a = 2 * math.pi * i / 6 + 0.5
        x, z = 50 * math.cos(a), 50 * math.sin(a)
        out.append(bar("cone", (x * 0.9, 40, z * 0.9), (x * 1.45, 64, z * 1.45), 14, bone))
    for i in range(5):
        a = 2 * math.pi * i / 5 + 0.2
        x, z = 34 * math.cos(a), 34 * math.sin(a)
        out.append(bar("cone", (x * 0.85, 120, z * 0.85), (x * 1.6, 134, z * 1.6), 10, black))
    out += [
        part("cone", (0, 230, 0), (52, 64, 52), fire, g=900),
        part("cone", (-12, 222, 8), (26, 44, 26), fire2, g=900),
        part("cone", (14, 226, -6), (24, 52, 24), fire2, g=900),
        part("cone", (2, 262, 0), (18, 34, 18), fire2, g=900),
        part("cube", (0, 100, 47), (16, 26, 4), fire, g=700),
        part("cube", (0, 160, 37), (10, 16, 4), fire, g=700),
    ]
    for x in (-30, 30):
        out.append(box(20, 50, 2, red, x, 50, 50, t=(-30, 0, 0)))
        out.append(part("cone", (x, 21, 51), (20, 8, 2), black, t=(500, 0, 0)))
    return out


def tower_golgari():
    brown, dbrown, dgreen, moss, cap, cap2, spore, bone = (
        "#5a4030", "#3e2c22", "#3d5a2e", "#5f7a3a", "#7a5a8a", "#a07a3a", "#c8e86a", "#e2d6b8")
    out = [
        part("blob", (0, 4, 0), (150, 20, 150), dbrown),
        cyl(40, 90, brown, 0, 0, 0, mesh="frustum"),
        part("rock", (0, 110, 0), (64, 50, 64), dbrown),
        cyl(24, 80, brown, 0, 110, 0, mesh="frustum"),
        part("rock", (6, 186, -4), (44, 30, 44), dbrown),
        ball(56, cap, 0, 214, 0, 1, 0.38, 1),
        ball(46, "#5e4470", 0, 206, 0, 1, 0.2, 1),
        ball(8, spore, 0, 236, 0, g=600),
    ]
    for a in (0.3, 1.7, 3.1, 4.5, 5.6):
        out.append(bar("cyl", (0, 30, 0), (60 * math.cos(a), 2, 60 * math.sin(a)), 14, dbrown))
    # shelf fungi round the stalk, moss, spores
    for x, y, z, r, c in ((34, 70, 14, 22, cap2), (-30, 96, -10, 20, cap), (18, 150, -20, 18, cap2),
                          (-22, 160, 18, 16, cap), (28, 40, -30, 16, cap)):
        out.append(ball(r, c, x, y, z, 1, 0.3, 1))
    for x, y, z, r in ((-36, 40, 20, 18), (20, 124, 26, 14), (-12, 176, -20, 12), (40, 10, -40, 16)):
        out.append(ball(r, moss, x, y, z, 1, 0.6, 1))
    for x, y, z in ((44, 100, 30), (-40, 140, -24), (30, 190, 34), (-34, 210, 30), (10, 120, -44), (-50, 60, -20)):
        out.append(ball(3, spore, x, y, z, g=700))
    for x, z in ((-56, 52), (60, 40), (-20, -64)):
        out.append(cyl(4, 22, bone, x, 0, z))
        out.append(ball(12, cap2 if x > 0 else cap, x, 22, z, 1, 0.5, 1, g=200))
    out.append(part("cone", (52, 10, -50), (10, 22, 10), bone, t=(40, -60, 0)))
    out.append(part("sphere", (-60, 8, -40), (16, 14, 16), bone))
    return out


def tower_selesnya():
    white, w2, green, lgreen, gold, bark = "#f4efe4", "#ddd6c2", "#3f9a4a", "#6cc070", "#e8c35a", "#e9e2cc"
    out = [
        cyl(64, 14, w2, 0, 0, 0),
        cyl(56, 8, white, 0, 14, 0),
        cyl(30, 120, bark, 0, 22, 0, mesh="frustum"),
        cyl(22, 4, gold, 0, 90, 0),
    ]
    # white ribs rising like roots and branches
    for i in range(6):
        a = 2 * math.pi * i / 6
        x, z = math.cos(a), math.sin(a)
        out.append(bar("cyl", (54 * x, 18, 54 * z), (22 * x, 80, 22 * z), 9, white))
        out.append(bar("cyl", (18 * x, 136, 18 * z), (58 * x, 176, 58 * z), 8, white))
    out += [
        ball(70, green, 0, 196, 0, 1, 0.62, 1),
        ball(46, lgreen, 0, 222, 0, 1, 0.55, 1),
        ball(28, white, 0, 244, 0, 1, 0.6, 1),
        cyl(10, 4, gold, 0, 258, 0),
        part("gem", (0, 270, 0), (12, 24, 12), gold, g=300),
    ]
    for i in range(6):
        a = 2 * math.pi * i / 6 + 0.5
        out.append(ball(24, lgreen if i % 2 else green, 60 * math.cos(a), 186, 60 * math.sin(a), 1, 0.7, 1))
    for i in range(4):
        a = math.pi / 2 * i + math.pi / 4
        out.append(cyl(7, 70, white, 58 * math.cos(a), 22, 58 * math.sin(a)))
        out.append(ball(10, green, 58 * math.cos(a), 96, 58 * math.sin(a), 1, 0.8, 1))
    out.append(part("cube", (0, 60, 30), (20, 34, 4), green))
    out.append(ball(4, gold, 0, 70, 33))
    return out


def nivix():
    """Niv-Mizzet's aerie: a brass and copper tower with furnaces and glow, about 400 cm."""
    copper, copper_d, brass, iron, glow, fire = "#c46a3c", "#8e4a2c", "#d8a640", "#4e4a56", "#6fd6ff", "#ff8a2a"
    out = [
        cyl(96, 18, iron, 0, 0, 0),
        cyl(80, 70, copper_d, 0, 18, 0),
        cyl(84, 6, brass, 0, 86, 0),
        cyl(62, 150, copper, 0, 92, 0, mesh="frustum"),
        cyl(60, 6, brass, 0, 140, 0),
        cyl(50, 6, brass, 0, 200, 0),
        cyl(40, 50, brass, 0, 240, 0),
        cyl(70, 8, brass, 0, 290, 0),
        cyl(56, 8, copper_d, 0, 298, 0),
        cyl(14, 40, iron, 0, 306, 0),
        cyl(10, 4, glow, 0, 330, 0),
        ball(22, glow, 0, 360, 0, g=900),
        part("cone", (0, 388, 0), (14, 26, 14), brass),
    ]
    out[10]["g"] = 800
    # the great ring round the shaft, its blocks glowing at the joins
    out += hring(14, 86, 180, (40, 8, 10), brass)
    for i in range(4):
        a = math.pi / 2 * i + math.pi / 4
        out.append(bar("cyl", (52 * math.cos(a), 170, 52 * math.sin(a)), (86 * math.cos(a), 180, 86 * math.sin(a)), 5, copper_d))
    # the aerie's prongs and the coil ring
    for i in range(6):
        a = 2 * math.pi * i / 6
        x, z = math.cos(a), math.sin(a)
        out.append(bar("cube", (56 * x, 300, 56 * z), (70 * x, 344, 70 * z), 8, copper))
        out.append(bar("cone", (70 * x, 344, 70 * z), (60 * x, 376, 60 * z), 9, brass))
        out.append(ball(5, glow, 66 * x, 352, 66 * z, g=800))
        out.append(part("gem", (34 * x, 356, 34 * z), (4, 36, 4), glow,
                        t=(tt(30 * z), tt(-30 * x), 0), g=900))
    # furnaces round the base, their mouths aflame, smokestacks behind
    for a in (math.pi / 2 - 0.7, math.pi / 2, math.pi / 2 + 0.7):
        x, z = 80 * math.cos(a), 80 * math.sin(a)
        yaw = tt(-math.degrees(a) + 90)
        out.append(part("cube", (x, 40, z), (34, 44, 12), iron, t=(0, 0, yaw)))
        out.append(part("cube", (x * 1.06, 36, z * 1.06), (22, 26, 4), fire, t=(0, 0, yaw), g=900))
    for a in (-math.pi / 2 - 0.8, -math.pi / 2 + 0.8):
        x, z = 74 * math.cos(a), 74 * math.sin(a)
        out.append(cyl(10, 200, iron, x, 18, z))
        out.append(cyl(13, 8, brass, x, 214, z))
        out.append(cyl(8, 3, fire, x, 222, z))
        out[-1]["g"] = 900
    for a in (0.0, 2.1, 4.2):
        x, z = 56 * math.cos(a), 56 * math.sin(a)
        out.append(part("cube", (x, 168, z), (10, 22, 6), glow, t=(0, 0, tt(-math.degrees(a) + 90)), g=700))
        out.append(part("cube", (x * 0.84, 262, z * 0.84), (8, 18, 6), fire, t=(0, 0, tt(-math.degrees(a) + 90)), g=700))
    for a in (0.9, 3.0, 5.1):
        x, z = math.cos(a), math.sin(a)
        out.append(bar("cyl", (60 * x, 120, 60 * z), (84 * x, 88, 84 * z), 7, brass))
    return out


# ------------------------------------------------------------ markers

def card_stand():
    """A face-down card on a small floating base, tilted back 35 degrees."""
    violet, violet_l, gold, dark = "#3a2456", "#4c2f70", "#f2c14e", "#2b2140"
    lean = -35
    base_y = 14
    out = [
        cyl(26, 5, dark, 0, base_y - 5, 0),
        cyl(22, 3, gold, 0, base_y, 0),
        cyl(8, 6, gold, 0, base_y - 11, 0, mesh="cone", t=(500, 0, 0)),
    ]
    c, s = math.cos(math.radians(lean)), math.sin(math.radians(lean))

    def at(x, y, z):
        # the card's own frame (y up its face, z out of its back) tilted back
        return (x, base_y + 3 + y * c - z * s, y * s + z * c)

    out.append(part("cube", at(0, 44, 0), (63, 88, 3), violet, t=(tt(lean), 0, 0)))
    out.append(part("cube", at(0, 44, 1.2), (53, 78, 2), violet_l, t=(tt(lean), 0, 0)))
    n, r = 14, 19
    for i in range(n):
        a = 360 * i / n
        x, y = r * math.cos(math.radians(a)), 44 + r * math.sin(math.radians(a))
        out.append(part("cube", at(x, y, 2.6), (3, 9.5, 2), gold, t=(tt(lean), tt(a), 0)))
    out.append(part("gem", at(0, 44, 2.4), (10, 14, 2), gold, t=(tt(lean), 0, 0), g=300))
    for y in (6, 82):
        out.append(part("cube", at(0, y, 2.4), (40, 2, 1.5), gold, t=(tt(lean), 0, 0)))
    return out


CRYSTAL = {"W": ("#fff1b8", "#f2c14e"), "U": ("#4aa3ff", "#9fd0ff"), "B": ("#5a3a7a", "#8a60b0"),
           "R": ("#ff5a32", "#ffa070"), "G": ("#3fbf5a", "#8ae07a")}


def crystal(col):
    main, light = CRYSTAL[col]
    base, rim = "#3a3346", "#2b2140"
    return [
        cyl(16, 6, rim, 0, 0, 0),
        cyl(13, 6, base, 0, 6, 0, mesh="frustum"),
        part("gem", (0, 32, 0), (20, 40, 20), main, t=(0, 0, 60), g=600),
        part("gem", (9, 19, 4), (9, 18, 9), light, t=(0, -90, 30), g=600),
        part("gem", (-8, 18, -5), (8, 15, 8), light, t=(0, 100, 0), g=600),
    ]


def gate_marker():
    """A small two-pillar guildgate arch, about 60 cm."""
    st, st_d, gold = "stone", "#958a74", "gold"
    out = [box(56, 4, 22, st_d, 0, 2, 0)]
    for s in (-1, 1):
        out += [
            box(14, 6, 14, st_d, s * 20, 7, 0),
            box(11, 38, 11, st, s * 20, 29, 0),
            box(14, 5, 14, st_d, s * 20, 50, 0),
            pyramid(s * 20, 52.5, 0, 12, 10, 12, gold),
        ]
        out.append(part("cube", (s * 9, 43, 0), (10, 8, 9), st, t=(0, s * 125, 0)))
    out.append(box(30, 5, 9, st, 0, 47, 0))
    out.append(box(8, 8, 10, gold, 0, 47, 0, mesh="gem"))
    return out


def lamp():
    iron, glow, gold = "#2b2140", "#ffe08a", "gold"
    return [
        cyl(12, 6, iron, 0, 0, 0, mesh="frustum"),
        cyl(5, 10, iron, 0, 6, 0),
        cyl(3, 130, iron, 0, 16, 0),
        cyl(6, 4, gold, 0, 60, 0),
        cyl(9, 4, iron, 0, 144, 0),
        ball(10, glow, 0, 158, 0, 1, 1.15, 1, g=850),
        bar("cyl", (-9, 148, 0), (-9, 168, 0), 2, iron),
        bar("cyl", (9, 148, 0), (9, 168, 0), 2, iron),
        part("cone", (0, 176, 0), (24, 14, 24), iron),
        ball(3, gold, 0, 185, 0),
    ]


def fountain():
    stone, light, water, foam = "#cfc6b0", "#e7e0cc", "#4f95dc", "#b9e2ff"
    out = [
        cyl(70, 6, "#b8ad94", 0, 0, 0),
        cyl(62, 24, stone, 0, 6, 0),
        cyl(56, 2, water, 0, 30, 0),
        cyl(12, 50, light, 0, 30, 0),
        cyl(30, 10, stone, 0, 78, 0, mesh="frustum", t=(500, 0, 0)),
        cyl(26, 2, water, 0, 88, 0),
        cyl(6, 24, light, 0, 88, 0),
        ball(9, "gold", 0, 118, 0),
        ball(5, foam, 0, 130, 0, g=500),
    ]
    out[-6]["g"] = 200
    out += hring(9, 62, 31, (40, 5, 10), light)
    for i in range(6):
        a = 2 * math.pi * i / 6
        out.append(bar("cyl", (8 * math.cos(a), 124, 8 * math.sin(a)), (26 * math.cos(a), 92, 26 * math.sin(a)), 3, foam))
    return out


def plaza():
    """A 600 x 400 flat paved plaza with a low edge."""
    pave, pave_d, edge, gold = "#efe6d0", "#ddd2b8", "#b8ad94", "#e0b850"
    out = [box(600, 4, 400, pave, 0, 2, 0)]
    for i, x in enumerate(range(-250, 251, 100)):
        for j, z in enumerate(range(-150, 151, 100)):
            if (i + j) % 2:
                out.append(box(96, 1, 96, pave_d, x, 4.5, z))
    out.append(cyl(70, 1, "#cfc4a8", 0, 4, 0))
    out.append(cyl(54, 1, gold, 0, 4.5, 0))
    out.append(cyl(46, 1, pave, 0, 5, 0))
    for s in (-1, 1):
        out.append(box(600, 12, 14, edge, 0, 6, s * 193))
        out.append(box(14, 12, 372, edge, s * 293, 6, 0))
    for x in (-293, 293):
        for z in (-193, 193):
            out.append(box(22, 18, 22, "#a89c84", x, 9, z))
    return out


def arena():
    """A 1200 x 800 battle platform: paved, low walls, banners on two sides."""
    pave, pave_d, wall, wall_d, gold, violet = "#d9cfb8", "#c6bba2", "#a89c84", "#8a7f6c", "#e0b850", "#4c2f70"
    out = [box(1200, 20, 800, "#8a7f6c", 0, 10, 0), box(1160, 2, 760, pave, 0, 21, 0)]
    for i, x in enumerate(range(-480, 481, 160)):
        for j, z in enumerate(range(-240, 241, 160)):
            if (i + j) % 2:
                out.append(box(152, 1, 152, pave_d, x, 22.5, z))
    out.append(cyl(150, 1, gold, 0, 22, 0))
    out.append(cyl(140, 1, pave, 0, 23, 0))
    # walls: the back whole, the sides whole, the front in two with an opening
    out.append(box(1200, 40, 30, wall, 0, 40, -385))
    for s in (-1, 1):
        out.append(box(30, 40, 740, wall, s * 585, 40, 0))
        out.append(box(440, 40, 30, wall, s * 380, 40, 385))
    for x in (-600, 600):
        for z in (-400, 400):
            out.append(box(60, 80, 60, wall_d, x * 0.975, 60, z * 0.9625))
            out.append(cyl(22, 12, "#3a3346", x * 0.975, 100, z * 0.9625, mesh="frustum", t=(500, 0, 0)))
            out.append(part("cone", (x * 0.975, 124, z * 0.9625), (30, 40, 30), "#ff8a2a", g=900))
    for x in (-160, 160):
        out.append(box(50, 70, 44, wall_d, x, 55, 385))
    # banners on the two long sides (left and right), on poles over the wall
    for s in (-1, 1):
        for z in (-220, 0, 220):
            out.append(cyl(5, 220, "#3a3346", s * 600, 20, z))
            out.append(ball(8, gold, s * 600, 244, z))
            out.append(box(4, 140, 70, violet, s * 607, 160, z))
            out.append(box(5, 16, 72, gold, s * 608, 226, z))
            out.append(part("gem", (s * 610, 170, z), (3, 30, 22), gold))
    return out


def pedestal():
    """A turning stand for the armour shop, about 110 cm across, a gold rim."""
    dark, violet, gold = "#2b2140", "#4c2f70", "#f2c14e"
    out = [
        cyl(44, 10, dark, 0, 0, 0, mesh="frustum"),
        cyl(50, 14, dark, 0, 10, 0),
        cyl(55, 6, gold, 0, 24, 0),
        cyl(50, 7, violet, 0, 24, 0),
    ]
    out += hring(12, 47, 17, (8, 4, 2), gold, phase=0.26)
    return out


MODELS = {
    "chum_house": chum_house(),
    "chum_hall": chum_hall(),
    "chum_inn": chum_inn(),
    "chum_stall": chum_stall(),
    "chum_watchtower": chum_watchtower(),
    "chum_well": chum_well(),
    "chum_gate": chum_gate(),
    "chum_notice": chum_notice(),
    "chum_mill": chum_mill(),
    "chum_shrine": chum_shrine(),
    "chum_tent": chum_tent(),
    "chum_fence": chum_fence(),
    "chum_bridge": chum_bridge(),
    "tree_a": tree_a(),
    "tree_b": tree_b(),
    "mountain": mountain(),
    "peak": peak(),
    "dead_tree": dead_tree(),
    "swamp_pool": swamp_pool(),
    "plains_columns": plains_columns(),
    "canal_straight": canal_straight(),
    "canal_pool": canal_pool(),
    "tower_azorius": tower_azorius(),
    "tower_izzet": tower_izzet(),
    "tower_rakdos": tower_rakdos(),
    "tower_golgari": tower_golgari(),
    "tower_selesnya": tower_selesnya(),
    "nivix": nivix(),
    "card_stand": card_stand(),
    "crystal_W": crystal("W"),
    "crystal_U": crystal("U"),
    "crystal_B": crystal("B"),
    "crystal_R": crystal("R"),
    "crystal_G": crystal("G"),
    "gate_marker": gate_marker(),
    "lamp": lamp(),
    "fountain": fountain(),
    "plaza": plaza(),
    "arena": arena(),
    "pedestal": pedestal(),
}
