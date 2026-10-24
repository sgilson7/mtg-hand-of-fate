"""Black and red Return to Ravnica creatures (Rakdos and its streets), after the cards' art."""
import math

from common import ball, box, cyl, part, mirror, eyes

INK = "#1d1626"
BONE = "#e6dcc0"
IRON = "#4a4650"
STEEL = "#b8b8c4"
FIRE = "#ff8a2a"
FLAME = "#ffd25a"


def _tilt(a, b):
    dx, dy, dz = b[0] - a[0], b[1] - a[1], b[2] - a[2]
    length = math.sqrt(dx * dx + dy * dy + dz * dz) or 1
    th = math.acos(max(-1, min(1, dy / length)))
    ph = math.atan2(dx, dz) if (dx or dz) else 0
    return length, (th / (2 * math.pi) * 1000, 0, ph / (2 * math.pi) * 1000)


def seg(a, b, r, c, n="body", mesh="sphere", stretch=1.15, rz=None, g=0):
    """A mesh stretched from point a to point b (a cone's apex lands on b)."""
    length, t = _tilt(a, b)
    mid = [(a[i] + b[i]) / 2 for i in range(3)]
    k = stretch if mesh in ("sphere", "blob") else 1
    return part(mesh, mid, (2 * r, length * k, 2 * (rz or r)), c, n, t, g)


def spike(a, b, r, c, n="body", g=0):
    return seg(a, b, r, c, n, mesh="cone", g=g)


def chain(a, b, k, r=4, c="#8a8a94", n="body", sag=0):
    out = []
    for i in range(k + 1):
        f = i / k
        p = [a[j] + (b[j] - a[j]) * f for j in range(3)]
        p[1] -= sag * 4 * f * (1 - f)
        out.append(ball(r, c, *p, n=n, mesh="blob"))
    return out


def face(y, z, spread, r, c=INK, x=0, n="head", g=0):
    out = [ball(r, c, x - spread, y, z, n=n, g=g), ball(r, c, x + spread, y, z, n=n, g=g)]
    if c == INK:
        out += [ball(r * 0.35, "#ffffff", x - spread + r * 0.3, y + r * 0.35, z + r * 0.6, n=n),
                ball(r * 0.35, "#ffffff", x + spread + r * 0.3, y + r * 0.35, z + r * 0.6, n=n)]
    return out


def person(skin, top, pants, boot, hl, hr, *, arm=None, fl=(-20, 0, 6), fr=(20, 0, 6), hip=52, sh=106,
           head=136, hr_=28, tr=34, ar=10, lr=13, hand=None, barefoot=False):
    """A chunky upright figure; hl, hr are the hands' positions."""
    arm = arm or top
    hand = hand or skin
    out = []
    for s, f in ((-1, fl), (1, fr)):
        k = hip / 52
        knee = ((s * 15 * k + f[0]) / 2, (hip + 14 * k) / 2 + 8 * k, f[2] * 0.6 + 4 * k)
        out.append(seg((s * 15 * k, hip, 0), knee, lr, pants))
        out.append(seg(knee, (f[0], 14, f[2]), lr * 0.9, pants))
        out.append(ball(13 * k, skin if barefoot else boot, f[0], 8 * k, f[2] + 5 * k, 0.95, 0.65, 1.45))
    out.append(ball(tr * 0.9, pants, 0, hip + 4, 0, 1.05, 0.7, 0.85))
    out.append(ball(tr, top, 0, (hip + sh) / 2 + 6, 0, 1.08, (sh - hip + 16) / (2 * tr), 0.82))
    for s, hp in ((-1, hl), (1, hr)):
        shp = (s * (tr + 2), sh - 6, 0)
        out.append(ball(ar * 1.35, top, *shp))
        out.append(seg(shp, hp, ar, arm))
        out.append(ball(ar * 1.15, hand, *hp))
    out.append(cyl(tr * 0.27, tr * 0.4, skin, 0, sh - 2, 0))
    out.append(ball(hr_, skin, 0, head, 4, n="head"))
    return out


# ---------------------------------------------------------------- black


def daggerdrome_imp():
    sk, dk, red = "#3d3550", "#2a2436", "#a41f2c"
    out = [
        seg((0, 78, -16), (0, 104, 14), 16, sk),
        ball(11, "#564a68", 0, 88, 6, 1, 1.3, 0.8),
        ball(16, sk, 0, 114, 26, n="head"),
        ball(10, sk, 0, 106, 40, 1, 0.8, 1.1, n="head"),
        ball(8, "#c0303a", 0, 101, 42, 1.05, 0.7, 0.9, n="head"),
        ball(4, "#ff3a2a", -6, 117, 39, n="head", g=900),
        ball(4, "#ff3a2a", 6, 117, 39, n="head", g=900),
    ]
    side = [
        spike((4, 104, 46), (4, 97, 47), 2, "#f4efe4", n="head"),
        spike((8, 104, 42), (9, 98, 43), 1.6, "#f4efe4", n="head"),
        seg((9, 120, 24), (22, 138, 16), 6, dk, n="head", mesh="cone", rz=2),
        spike((6, 126, 30), (8, 136, 38), 2.5, "#1e1a26", n="head"),
        seg((13, 100, 16), (18, 86, 32), 5, sk),
        seg((18, 86, 32), (16, 76, 40), 4, sk),
        spike((16, 76, 40), (17, 68, 46), 2, "#d8d0e0"),
        seg((9, 82, -12), (15, 72, 2), 7, sk),
        seg((15, 72, 2), (13, 54, -12), 5, sk),
        spike((13, 54, -12), (13, 46, -2), 3, "#d8d0e0"),
        # wing: arm bone, three fingers, red membrane
        seg((10, 108, -2), (46, 130, -4), 4, dk, n="wing_l"),
        seg((46, 130, -4), (104, 138, -10), 3, dk, n="wing_l"),
        seg((46, 130, -4), (94, 120, -46), 3, dk, n="wing_l"),
        seg((46, 130, -4), (62, 106, -58), 3, dk, n="wing_l"),
        spike((46, 130, -4), (42, 146, 4), 3.5, "#d8d0e0", n="wing_l"),
        part("blob", (54, 122, -26), (84, 4, 60), red, "wing_l", (0, 40, 0)),
        part("blob", (86, 128, -26), (40, 4, 40), red, "wing_l", (0, 30, 0)),
        part("blob", (40, 126, -10), (56, 5, 20), "#4b4060", "wing_l", (0, 60, 0)),
    ]
    out += mirror(side)
    out += [
        seg((0, 82, -24), (0, 72, -48), 4, sk, n="tail"),
        seg((0, 72, -48), (0, 80, -74), 3, sk, n="tail"),
        spike((0, 80, -74), (0, 94, -80), 3, "#1e1a26", n="tail"),
    ]
    return out


def grim_roustabout():
    coat, dk, bone = "#4e2232", "#26202a", "#d9ceb0"
    out = person(bone, coat, "#3a2a3a", dk, (-34, 82, 40), (36, 150, 26), arm=bone, hand=dk, ar=6, lr=8,
                 tr=30, hr_=24, head=138)
    out += [
        ball(14, bone, 0, 124, 14, 1, 0.7, 0.9, n="head"),
        box(16, 4, 4, "#f4efe4", 0, 121, 26, n="head"),
        box(3, 26, 30, "#6a2a3a", 0, 156, 4, n="head"),
        ball(2.5, INK, 0, 132, 29, n="head"),
    ]
    out += face(140, 23, 9, 6, "#2a1820") + [ball(2, "#ff3a3a", -9, 140, 28, n="head", g=900),
                                            ball(2, "#ff3a3a", 9, 140, 28, n="head", g=900)]
    out += [part("frustum", (0, 50, -2), (70, 50, 52), coat, t=(0, 0, 0)),
            ball(10, "#f0e6cc", 0, 92, 24, 1.5, 0.3, 0.5), ball(10, "#f0e6cc", 0, 84, 24, 1.4, 0.3, 0.5),
            box(10, 40, 8, "#2a1e24", 0, 86, 26)]
    collar = []
    for i in range(7):
        a = math.pi * (0.15 + 0.7 * i / 6)
        x, z = 34 * math.cos(a), -6 + 18 * math.sin(a) * 0 - 10 * math.sin(a)
        collar.append(spike((x, 108, z), (x * 1.35, 128, z - 8), 5, dk))
    out += collar
    out += mirror([ball(16, dk, 32, 104, 0, 1, 0.7, 1), spike((34, 112, 0), (40, 126, 2), 4, "#5a5260"),
                   spike((40, 108, 10), (48, 118, 16), 3.5, "#5a5260")])
    # two hooked axes, chained
    for hx, hy, hz, up in ((-34, 82, 40, 1), (36, 150, 26, 1)):
        out += [seg((hx, hy - 20, hz - 6), (hx, hy + 24, hz + 6), 2.5, "#6a4a30", mesh="cyl"),
                box(3, 18, 22, STEEL, hx, hy + 20, hz + 14, t=(-60, 0, 0)),
                spike((hx, hy + 12, hz + 22), (hx, hy + 2, hz + 30), 3, STEEL)]
    out += chain((-34, 64, 36), (36, 132, 22), 9, 2.5, sag=24)
    return out


def thrill_kill_assassin():
    sk, arm, pale, red, hair = "#9a7868", "#2a2226", "#ece0d8", "#8e1e24", "#1c1418"
    out = person(sk, arm, "#2a2226", "#2a2226", (-40, 104, 34), (42, 122, 10), fl=(-22, 0, 34), fr=(22, 0, -30),
                 tr=28, hr_=25, head=136, ar=8, lr=11, hand=arm)
    out += [
        ball(24, red, 0, 140, 12, 1.02, 0.75, 0.95, n="head"),
        ball(3, "#ffe0c0", -8, 140, 34, n="head", g=600), ball(3, "#ffe0c0", 8, 140, 34, n="head", g=600),
        ball(6, sk, 0, 124, 26, 1.4, 0.8, 0.6, n="head"),
        seg((10, 152, 6), (34, 166, -14), 5, "#3a2a28", n="head", mesh="cone"),
        seg((34, 166, -14), (50, 162, -40), 3, "#3a2a28", n="head", mesh="cone"),
        ball(26, hair, 0, 148, -6, 1.05, 0.9, 1, n="head"),
    ]
    out += [seg((-4, 146, -10), (-40, 124, -70), 20, hair, n="head", rz=12),
            seg((-30, 130, -50), (-70, 112, -96), 12, hair, n="head", mesh="cone", rz=6),
            seg((10, 140, -14), (30, 116, -60), 12, hair, n="head", mesh="cone", rz=6),
            seg((-14, 150, -6), (-24, 160, -60), 10, hair, n="head", mesh="cone", rz=5)]
    out += [
        spike((-26, 96, 4), (-36, 112, -2), 5, pale), spike((26, 96, 4), (38, 112, 0), 5, pale),
        spike((-22, 34, 24), (-26, 46, 46), 4, pale), spike((22, 34, -18), (28, 48, -2), 4, pale),
        ball(12, pale, -22, 30, 22, 1, 0.6, 0.8), ball(12, pale, 22, 32, -20, 1, 0.6, 0.8),
        box(40, 6, 26, "#5a1a1e", 0, 56, 2),
        # the curved bone blade
        seg((42, 122, 10), (60, 176, 4), 9, pale, mesh="cone", rz=3),
        seg((42, 122, 10), (52, 92, 36), 10, pale, rz=3),
        seg((52, 92, 36), (44, 82, 74), 8, pale, mesh="cone", rz=3),
        spike((50, 104, 22), (68, 110, 34), 4, pale),
        ball(5, "#8e1e24", 42, 122, 10),
        # the knife and chain
        seg((-40, 104, 34), (-46, 100, 62), 3, pale, mesh="cone", rz=1.5),
    ]
    out += chain((-40, 100, 30), (-70, 60, -10), 7, 2.5, sag=-10)
    return out


def dead_reveler():
    sk, shirt, red = "#b8a088", "#3a2a26", "#b8303a"
    out = person(sk, shirt, "#4a3a30", "#2a2020", (-30, 76, 26), (40, 178, 16), arm=sk, tr=36, hr_=27,
                 head=140)
    out += face(146, 25, 9, 4.5, "#2a1418")
    out += [
        ball(12, "#3a1014", 0, 130, 22, 1, 1.05, 0.8, n="head"),
        box(14, 3, 3, "#f0e8d0", 0, 140, 30, n="head"), box(12, 3, 3, "#f0e8d0", 0, 120, 30, n="head"),
        ball(6, "#8a5a50", 14, 132, 22, n="head"), ball(5, "#8a5a50", -12, 128, 22, n="head"),
        ball(27, "#2a2020", 0, 152, -6, 1.05, 0.8, 1, n="head"),
        spike((-14, 160, 0), (-22, 176, 8), 7, "#2a2020", n="head"),
        spike((8, 166, -4), (14, 180, 2), 7, "#2a2020", n="head"),
        ball(16, "#7a6050", -38, 104, 0, 1.1, 0.7, 1.1),
        box(10, 10, 10, "#2a1e1e", 40, 158, 16),
        ball(9, "#6a2a5a", 0, 114, 8, 2.2, 0.5, 1.8),
    ]
    for i in range(5):
        x = -24 + i * 12
        out.append(part("gem", (x, 96, 29), (9, 9, 4), red))
        if i < 4:
            out.append(part("gem", (x + 6, 88, 30), (9, 9, 4), red))
    out += chain((-30, 72, 26), (34, 66, 26), 8, 3, sag=-6)
    out += [seg((36, 140, 10), (52, 226, 24), 2.5, "#8a5a35", mesh="cyl"),
            spike((52, 226, 24), (54, 242, 26), 4, STEEL)]
    for i, (dx, dz, c) in enumerate(((-30, 8, "#e8a0b8"), (-10, -24, "#f4efe4"), (-38, -6, "#c870a0"))):
        out.append(seg((50, 220, 22), (50 + dx, 186 - i * 10, 22 + dz), 2.5, c, mesh="cube", rz=0.5))
    return out


def sewer_shambler():
    moss, root, glow, pur = "#34442a", "#566a38", "#9ad85a", "#5e4e72"
    out = [
        seg((0, 96, -4), (0, 140, 6), 20, moss),
        seg((0, 80, -4), (0, 100, -4), 14, root),
        ball(17, "#8aa870", 0, 168, 18, 0.9, 1.05, 1, n="head"),
        ball(13, glow, 6, 176, 14, n="head", g=350),
        ball(9, "#4a5a3a", 0, 156, 30, 1, 0.6, 0.6, n="head"),
    ] + face(170, 31, 7, 4.5, "#1a2014")
    out += [ball(7, glow, -14, 142, 10, 0.6, 1.2, 0.8, g=300), ball(6, glow, 16, 112, 4, 0.6, 1.4, 0.8, g=300),
            ball(8, pur, 8, 140, -14)]
    for x, z in ((-12, 14), (0, 18), (12, 12), (-6, -16), (10, -14)):
        out.append(seg((x, 150, z), (x * 1.4, 96, z * 1.1), 3, "#4a5c30"))
    # arms: right flung out to a wall, left hanging low
    out += [seg((18, 140, 4), (52, 146, 18), 7, root), seg((52, 146, 18), (92, 154, 34), 5, root),
            seg((-18, 138, 4), (-40, 108, 16), 7, root), seg((-40, 108, 16), (-50, 72, 28), 5, root)]
    for tip in ((106, 160, 42), (104, 150, 46), (100, 166, 36)):
        out.append(spike((92, 154, 34), tip, 2.5, moss))
    for tip in ((-56, 56, 36), (-46, 58, 40), (-58, 60, 24)):
        out.append(spike((-50, 72, 28), tip, 2.5, moss))
    for p, q in (((20, 144, 6), (24, 112, 10)), ((60, 148, 22), (62, 122, 24)), ((-30, 120, 10), (-30, 92, 4))):
        out.append(spike(p, q, 2, "#7a9a50"))
    for s in (-1, 1):
        out += [seg((s * 10, 84, -4), (s * 28, 46, 6), 8, root), seg((s * 28, 46, 6), (s * 36, 10, 10), 6, moss)]
        for dx, dz in ((-10, 16), (6, 22), (14, 0), (-2, -14)):
            out.append(spike((s * 36, 12, 10), (s * 36 + dx, 1, 10 + dz), 3, root))
    out += [spike((-6, 96, 16), (-10, 50, 20), 3, "#7a9a50"), spike((8, 100, 14), (14, 60, 18), 2.5, glow),
            ball(7, pur, -14, 100, 6)]
    return out


def perilous_shadow():
    dk, mid, lt = "#262c3c", "#3c4660", "#7484a8"
    out = [
        ball(50, dk, 0, 160, 0, 1.1, 0.75, 1.2),
        ball(34, mid, 0, 128, 0, 1, 0.8, 1),
        ball(26, dk, 0, 158, 54, 1, 0.8, 1, n="head"),
        ball(14, mid, 0, 146, 74, 1.2, 0.6, 0.8, n="head"),
    ]
    out += face(164, 76, 10, 5, "#bfe4ff", n="head", g=800)
    out += [ball(3, "#bfe4ff", -20, 170, 70, n="head", g=800), ball(3, "#bfe4ff", 20, 170, 70, n="head", g=800)]
    out += mirror([seg((10, 146, 78), (22, 120, 102), 4, lt, n="head", mesh="cone")])
    out.append(spike((0, 186, -4), (0, 290, -16), 14, mid))
    for i in range(10):
        a = i / 10 * 2 * math.pi
        x, z = 36 * math.sin(a), 36 * math.cos(a)
        out.append(spike((x * 0.6, 172, z * 0.6), (x * 1.6, 220 + 14 * (i % 2), z * 1.6 - 10), 10, mid if i % 2 else lt))
    for i in range(4):
        z = 40 - i * 28
        knee = (120, 200 - i * 8, z * 1.8)
        foot = (160, 0, z * 2.5)
        out += mirror([seg((36, 160, z * 0.6), knee, 8, mid), seg(knee, foot, 6, dk, mesh="cone"),
                       spike(knee, (knee[0] + 8, knee[1] + 30, knee[2] - 6), 5, lt)])
    for i in range(6):
        a = i / 6 * 2 * math.pi
        out.append(spike((20 * math.sin(a), 110, 20 * math.cos(a)), (34 * math.sin(a), 50, 34 * math.cos(a)), 8, dk))
    return out


def dark_revenant():
    dk, glow, face_c = "#1e2a16", "#a8e04a", "#d8d0b0"
    out = [
        ball(26, dk, 0, 150, 0, 1, 1.2, 0.9),
        ball(20, "#2c3a1e", 0, 116, -6, 1, 1.1, 0.9),
        ball(30, glow, 0, 150, -16, 1.2, 1.3, 0.5, g=500),
        seg((0, 104, -8), (0, 52, -30), 14, dk, n="tail", mesh="cone"),
        seg((0, 76, -20), (10, 30, -50), 6, "#2c3a1e", n="tail", mesh="cone"),
        ball(20, "#2a2a1e", 0, 186, 10, 1, 1.1, 1, n="head"),
        ball(13, face_c, 0, 182, 22, 0.9, 1.1, 0.8, n="head"),
    ]
    out += face(186, 31, 5, 3.5, glow, g=900)
    out += [ball(3, "#3a1a1a", 0, 175, 32, 1.2, 0.6, 0.6, n="head")]
    for x, z, y in ((-18, -4, 150), (18, -6, 160), (-8, -20, 140), (12, -18, 128)):
        out.append(spike((x * 0.7, 196, z), (x * 1.8, y, z - 26), 4, "#141c10", n="head"))
    # the clawed hand, reaching
    out += [seg((14, 160, 8), (30, 130, 38), 6, "#2a2a1e"), seg((30, 130, 38), (34, 100, 62), 5, "#3a3a2a"),
            ball(8, "#3a3a2a", 34, 98, 64)]
    for dx in (-8, -3, 3, 8):
        out.append(spike((34 + dx * 0.5, 96, 66), (34 + dx * 1.6, 76, 80), 2, "#d8d0b0"))
    out += [seg((-14, 160, 8), (-34, 140, 20), 5, "#2a2a1e"), seg((-34, 140, 20), (-44, 118, 34), 4, "#2a2a1e")]
    wisp = [part("blob", (46, 156, -12), (70, 4, 34), dk, "wing_l", (0, -60, 60)),
            part("blob", (60, 146, -26), (50, 4, 20), glow, "wing_l", (0, -80, 80), g=400),
            spike((30, 170, -8), (96, 140, -40), 5, dk, n="wing_l")]
    out += mirror(wisp)
    return out


def slum_reaper():
    blk, sheen, org = "#2a2420", "#5a4f45", "#e8843a"
    out = [
        ball(46, blk, 0, 150, -10, 1, 0.75, 1.3),
        ball(40, sheen, 0, 138, -86, 1, 0.8, 1.2),
        seg((0, 150, 30), (0, 160, 90), 24, blk),
        ball(26, blk, 0, 158, 108, 1, 0.9, 1.2, n="head"),
        seg((0, 176, 104), (0, 250, 70), 14, sheen, n="head", mesh="cone", rz=8),
        ball(14, org, 0, 140, 128, 1, 0.9, 0.7, n="head", g=700),
    ]
    for dx in (-8, 0, 8):
        out.append(spike((dx, 140, 128), (dx * 1.8, 112, 140), 3, org, n="head", g=500))
    out += face(166, 128, 13, 4.5, "#ffa050", n="head", g=800)
    for i, z in enumerate((-40, -16, 8)):
        out.append(spike((0, 176, z), (0, 230 + i * 8, z + 6), 7 - i, sheen))
        out += mirror([spike((14, 172, z), (22, 214, z + 4), 4, blk)])
    legs = [
        seg((30, 140, 20), (90, 190, 60), 8, blk), seg((90, 190, 60), (110, 0, 110), 5, blk, mesh="cone"),
        seg((30, 140, -40), (100, 170, -60), 8, blk), seg((100, 170, -60), (120, 0, -80), 5, blk, mesh="cone"),
        seg((30, 130, -90), (80, 150, -150), 7, blk), seg((80, 150, -150), (90, 0, -170), 5, blk, mesh="cone"),
        # the scythe arms curling forward
        seg((26, 160, 60), (60, 230, 90), 7, sheen), seg((60, 230, 90), (80, 210, 150), 5, sheen, mesh="cone"),
    ]
    out += mirror(legs)
    out += [seg((0, 130, -120), (0, 90, -190), 10, sheen, n="tail", mesh="cone")]
    return out


def catacomb_slug():
    pur, dkp, fin, mot = "#9a7aa8", "#6b4f7a", "#8a5a9c", "#a8b07a"
    out = [
        ball(110, "#5a4466", 0, 20, -20, 0.9, 0.18, 2.0),
        ball(55, pur, 0, 50, -150, 0.9, 0.6, 1.2),
        ball(80, pur, 0, 90, -60, 1, 0.9, 1.1),
        ball(76, pur, 0, 140, 30, 1, 1.05, 1),
        ball(38, "#2a1424", 0, 150, 90, 0.9, 1.05, 0.5, n="head"),
        ball(30, "#120810", 0, 150, 96, 0.8, 0.95, 0.4, n="head"),
    ]
    for i in range(7):
        a = i / 7 * 2 * math.pi
        out.append(spike((26 * math.cos(a), 150 + 32 * math.sin(a), 100), (24 * math.cos(a), 148 + 30 * math.sin(a), 92),
                         5, "#f0e0d0", n="head"))
    for i, (x, y, z, tx, ty, tz) in enumerate(((-30, 196, 50, -44, 232, 76), (30, 196, 50, 44, 232, 76),
                                                (0, 210, 30, 0, 260, 34), (-50, 170, 70, -82, 196, 94),
                                                (50, 170, 70, 82, 196, 94), (-56, 130, 74, -90, 120, 96),
                                                (56, 130, 74, 90, 120, 96))):
        out.append(seg((x, y, z), (tx, ty, tz), 6, "#c8a8b8", n="head", mesh="cone"))
    out += [ball(7, INK, -44, 232, 76, n="head"), ball(7, INK, 44, 232, 76, n="head"),
            ball(2.5, "#ffffff", -42, 235, 81, n="head"), ball(2.5, "#ffffff", 46, 235, 81, n="head")]
    for i, (y, z, h) in enumerate(((210, -10, 50), (186, -60, 56), (160, -110, 50), (120, -150, 40), (90, -190, 30))):
        out.append(part("cone", (0, y + h / 2 - 10, z), (6, h, 56), fin, t=(-60, 0, 0)))
    for x, y, z, r in ((-60, 120, 20, 16), (64, 150, 10, 14), (-70, 80, -60, 18), (72, 96, -80, 14),
                       (-40, 190, -10, 12), (40, 60, -140, 12), (-48, 54, -150, 10)):
        out.append(ball(r, mot if r % 4 else "#c86a8a", x, y, z, 0.6, 1, 1, mesh="blob"))
    out += [ball(20, dkp, 0, 30, -230, 1, 0.5, 1.4, n="tail"), ball(10, fin, 0, 50, -250, 1.4, 0.4, 1, n="tail")]
    return out


def zanikev_locust():
    shell, sheen, w1, w2 = "#2e3850", "#6a7aa0", "#7a5a8a", "#b87a40"
    out = [
        ball(22, shell, 0, 110, 6, 1, 0.9, 1.1),
        ball(26, sheen, 0, 106, -36, 0.9, 0.8, 1.4),
        ball(18, shell, 0, 104, -70, 0.8, 0.7, 1.2),
        ball(18, shell, 0, 114, 36, 1.1, 0.9, 1, n="head"),
        ball(12, "#3a4a6a", 0, 120, 46, 1.3, 0.5, 0.8, n="head"),
    ]
    out += face(116, 52, 8, 4, "#e8f070", g=800)
    side = [
        seg((10, 112, 48), (26, 116, 80), 6, sheen, n="head", mesh="cone"),
        seg((10, 112, 48), (18, 98, 72), 3.5, shell, n="head", mesh="cone"),
        spike((22, 114, 68), (32, 126, 68), 3, sheen, n="head"),
        seg((16, 104, 20), (50, 96, 50), 4, shell), seg((50, 96, 50), (62, 76, 66), 3, sheen, mesh="cone"),
        seg((18, 102, 4), (60, 90, 4), 4, shell), seg((60, 90, 4), (74, 66, 0), 3, sheen, mesh="cone"),
        seg((16, 100, -16), (54, 88, -40), 4, shell), seg((54, 88, -40), (66, 64, -56), 3, sheen, mesh="cone"),
        spike((50, 96, 50), (56, 104, 52), 2, sheen), spike((60, 90, 4), (66, 98, 4), 2, sheen),
        part("blob", (55, 131, -29), (96, 3, 34), w1, "wing_l", (0, 15, 70)),
        part("blob", (42, 126, -42), (84, 3, 30), w2, "wing_l", (0, 10, 139)),
        ball(10, sheen, 14, 122, -6, 1, 0.6, 1.4),
    ]
    out += mirror(side)
    return out


# ---------------------------------------------------------------- red


def bellows_lizard():
    sk, belly, sp, fire = "#3e5a48", "#c8602a", "#243a30", "#ffb040"
    out = [
        ball(28, sk, 0, 40, -24, 0.95, 0.8, 1.2),
        seg((0, 44, -10), (0, 90, 30), 22, sk),
        ball(16, belly, 0, 70, 36, 1, 1.5, 0.6),
        seg((0, 90, 30), (0, 118, 44), 15, sk),
        ball(17, sk, 0, 128, 50, 1, 0.9, 1, n="head"),
        seg((0, 140, 54), (0, 170, 80), 10, sk, n="head", rz=12),
        seg((0, 120, 56), (0, 106, 84), 8, sk, n="head", rz=10),
        ball(12, fire, 0, 130, 66, 0.9, 1.3, 0.8, n="head", g=900),
        ball(8, FLAME, 0, 134, 76, 0.8, 1.2, 0.8, n="head", g=1000),
        ball(5, "#2a1a14", 0, 172, 88, 1, 0.6, 1, n="head"),
    ]
    out += face(140, 58, 10, 3.5, "#ffcc40", g=900)
    for i in range(4):
        out += mirror([spike((7, 146 + i * 6, 62 + i * 5.5), (7, 140 + i * 6, 66 + i * 5.5), 2, "#f4efe4", n="head"),
                       spike((6, 114 - i * 3, 64 + i * 6), (6, 120 - i * 3, 66 + i * 6), 2, "#f4efe4", n="head")])
    for y, z in ((146, 40), (124, 30), (104, 18), (86, 2), (68, -14), (58, -32), (50, -50)):
        out.append(spike((0, y - 4, z), (0, y + 12, z - 10), 5, sp))
    out += mirror([
        seg((18, 80, 30), (24, 46, 44), 8, sk), seg((24, 46, 44), (26, 6, 50), 6, sk),
        ball(7, sp, 26, 4, 56, 1, 0.5, 1.4),
        seg((18, 38, -30), (30, 26, -10), 10, sk), seg((30, 26, -10), (28, 4, -16), 6, sk),
        ball(7, sp, 28, 4, -8, 1, 0.5, 1.4),
    ])
    out += [seg((0, 34, -46), (0, 18, -80), 11, sk, n="tail"), seg((0, 18, -80), (18, 16, -112), 7, sk, n="tail"),
            seg((18, 16, -112), (40, 34, -120), 5, sk, n="tail", mesh="cone")]
    return out


def gore_house_chainwalker():
    sk, pants, rust = "#c8906a", "#3a2a24", "#7a3a24"
    out = person(sk, sk, pants, sk, (-56, 84, 20), (40, 172, 0), fl=(-12, 0, 10), fr=(24, 0, -6), barefoot=True,
                 tr=30, hr_=24, ar=8, lr=11, head=140)
    out += face(140, 25, 8, 4) + [ball(24, "#5a3020", 0, 132, 6, 1.02, 0.4, 1.02, n="head")]
    for i in range(9):
        a = i / 9 * 2 * math.pi
        out.append(seg((28 * math.sin(a), 120, 4 + 28 * math.cos(a)), (28 * math.sin(a), 170, 4 + 28 * math.cos(a)),
                       2, rust, n="head", mesh="cyl"))
        out.append(spike((26 * math.sin(a), 168, 4 + 26 * math.cos(a)), (34 * math.sin(a), 186, 4 + 34 * math.cos(a)),
                         2.5, rust, n="head"))
    out += [cyl(30, 4, rust, 0, 168, 4, n="head"), cyl(30, 4, rust, 0, 144, 4, n="head")]
    out += [seg((-28, 104, 22), (26, 60, 26), 3, "#2a1a18", mesh="cube", rz=2),
            ball(14, "#2a1a18", -36, 106, 0, 1.1, 0.7, 1.1),
            spike((-36, 112, 0), (-44, 126, -4), 3, "#c8c0b0"), spike((-42, 106, 10), (-54, 116, 14), 3, "#c8c0b0"),
            box(50, 8, 30, "#2a1a18", 0, 56, 2), box(4, 30, 4, "#2a1a18", 30, 44, 8, t=(0, 40, 0)),
            ball(9, "#2a1a18", 40, 158, 0, 1, 1.4, 1), part("blob", (46, 160, 6), (6, 24, 3), "#b8303a"),
            seg((40, 172, 0), (46, 200, 6), 1.5, IRON, mesh="cyl"), spike((44, 198, 6), (42, 212, 6), 1.5, IRON),
            spike((48, 198, 6), (50, 212, 6), 1.5, IRON),
            spike((-14, 30, 12), (-10, 20, 12), 8, "#3a2a24"), spike((24, 30, -4), (28, 20, -4), 8, "#3a2a24")]
    out += chain((-56, 80, 20), (-74, 20, 54), 9, 3, sag=-6)
    out += [ball(13, "#8a3a30", -78, 14, 58, 1, 1.1, 0.9), ball(8, "#2a1a1a", -78, 26, 58, 1, 0.5, 1),
            spike((-78, 20, 58), (-78, 0, 66), 3, IRON)]
    return out


def ash_zealot():
    sk, gold, bronze, red, hair = "#e8b890", "#c8963c", "#8a6030", "#8a2420", "#c8602a"
    out = person(sk, gold, red, bronze, (-36, 90, 36), (44, 164, 8), fl=(-20, 0, 14), fr=(20, 0, -6), arm=bronze,
                 hand=bronze, tr=30, hr_=25, head=138)
    out += face(140, 27, 8, 4)
    out += [ball(4, "#8a3a2a", 0, 130, 27, 1.2, 0.6, 0.6, n="head"),
            ball(27, hair, 0, 148, -2, 1.05, 0.75, 1.02, n="head"),
            ball(18, hair, -20, 130, -6, 0.5, 1.1, 0.9, n="head"), ball(18, hair, 20, 130, -6, 0.5, 1.1, 0.9, n="head"),
            ball(20, hair, 0, 132, -16, 1, 1, 0.6, n="head"),
            part("frustum", (0, 48, 2), (62, 44, 46), red),
            box(22, 40, 6, red, 0, 48, 24),
            box(46, 6, 32, bronze, 0, 66, 2),
            part("blob", (0, 74, -28), (80, 130, 12), "#2a1c1c", t=(-30, 0, 0)),
            ball(12, "#e0b860", 0, 96, 28, 1, 1, 0.3), ball(5, "#c8302a", 0, 96, 31),
            seg((-30, 116, -8), (30, 116, -8), 9, "#2a1c1c", mesh="cyl")]
    out += mirror([ball(17, gold, 34, 108, 0, 1, 0.6, 1.05), ball(7, "#f0d080", 38, 112, 14, 1, 1, 0.4)])
    out += [seg((44, 150, 8), (50, 196, 14), 3, "#5a3a24", mesh="cyl"),
            ball(12, IRON, 50, 200, 14),
            ball(16, FIRE, 52, 212, 10, g=900), ball(10, FLAME, 50, 222, 16, g=1000),
            spike((50, 214, 6), (44, 246, -6), 8, FIRE, g=900)]
    return out


def lobber_crew():
    wood, dkw, ironb = "#7a5a3a", "#5a3e26", "#3e4a50"
    out = []
    frame = [
        box(10, 12, 220, wood, 46, 46, 0),
        box(10, 70, 12, wood, 46, 90, 40, t=(0, 0, 0)),
        seg((46, 40, -40), (46, 170, 30), 7, wood, mesh="cube"),
        seg((46, 40, 90), (46, 170, 34), 7, wood, mesh="cube"),
        cyl(28, 12, dkw, 58, 22, 70, t=(0, 250, 0)), cyl(28, 12, dkw, 58, 22, -70, t=(0, 250, 0)),
        ball(6, IRON, 66, 28, 70), ball(6, IRON, 66, 28, -70),
    ]
    for i in range(len(frame)):
        frame[i] = frame[i]
    out += mirror(frame)
    for i, p in enumerate(out):
        if p["m"] == "cyl" and p["s"][0] == 56:
            p["p"][1] = 28
    out += [box(100, 12, 12, wood, 0, 170, 32), box(100, 10, 10, wood, 0, 50, -70), box(100, 10, 10, wood, 0, 50, 70),
            box(100, 8, 120, dkw, 0, 44, 0),
            cyl(10, 100, dkw, -50, 70, -10, t=(0, 250, 0)),
            seg((0, 62, 70), (0, 150, -60), 7, wood, mesh="cube"),
            ball(20, dkw, 0, 156, -66, 1.1, 0.6, 1.1),
            ball(15, "#3a2a2a", 0, 166, -66, g=0),
            ball(14, FIRE, 0, 176, -66, g=900), spike((0, 178, -66), (0, 212, -72), 10, FLAME, g=1000)]
    for x, y, z in ((80, 12, 100), (100, 12, 80), (90, 12, 120), (110, 12, 106), (94, 32, 98), (120, 12, 130),
                    (-90, 12, -100), (-80, 12, -120)):
        out.append(ball(13, ironb, x, y, z))
    # two goblins, one heaving a ball, one with a torch
    for gx, gz, arms_up in ((86, 50, True), (-84, 10, False)):
        out += [ball(20, "#6a5a3a", gx, 38, gz, 1, 1.3, 0.9), ball(9, "#4a3a2a", gx - 10, 8, gz, 1, 0.6, 1.4),
                ball(9, "#4a3a2a", gx + 10, 8, gz, 1, 0.6, 1.4),
                ball(19, "#8a9a5a", gx, 74, gz + 6, n="head"), ball(19, "#7a6a4a", gx, 82, gz + 3, 1.05, 0.6, 1.05, n="head"),
                seg((gx - 14, 78, gz + 8), (gx - 34, 86, gz + 2), 7, "#8a9a5a", n="head", mesh="cone", rz=3),
                seg((gx + 14, 78, gz + 8), (gx + 34, 86, gz + 2), 7, "#8a9a5a", n="head", mesh="cone", rz=3),
                ball(6, "#8a9a5a", gx, 68, gz + 25, n="head")]
        out += face(76, gz + 22, 7, 3.5, x=gx)
        if arms_up:
            out += [seg((gx - 16, 50, gz), (gx - 14, 84, gz + 28), 6, "#8a9a5a"),
                    seg((gx + 16, 50, gz), (gx + 14, 84, gz + 28), 6, "#8a9a5a"), ball(16, ironb, gx, 92, gz + 34)]
        else:
            out += [seg((gx + 16, 50, gz), (gx + 26, 84, gz + 10), 6, "#8a9a5a"),
                    seg((gx + 26, 64, gz + 10), (gx + 28, 110, gz + 12), 3, wood, mesh="cyl"),
                    ball(8, FIRE, gx + 28, 114, gz + 12, g=900), seg((gx - 16, 50, gz), (gx - 24, 30, gz + 10), 6, "#8a9a5a")]
    return out


def splatter_thug():
    sk, mask, hair, pants, rag = "#c87a5a", "#ece4d4", "#1e1614", "#3a2a2c", "#8a2a2a"
    out = person(sk, sk, pants, "#2a2224", (-58, 70, 34), (54, 82, 40), fl=(-28, 0, 20), fr=(28, 0, -12),
                 tr=40, hr_=24, head=124, ar=13, lr=15)
    out += [ball(22, sk, -46, 112, 6), ball(22, sk, 46, 112, 6), ball(28, sk, 0, 104, -10, 1.3, 0.8, 1),
            ball(20, mask, 0, 124, 16, 0.85, 1.2, 0.8, n="head")]
    out += face(126, 31, 6, 4, "#2a1a1a")
    out += [ball(26, hair, 0, 134, -4, 1, 0.8, 1, n="head")]
    out += [seg((0, 136, -10), (-24, 116, -66), 18, hair, n="head", rz=12),
            seg((-16, 122, -50), (-44, 100, -96), 11, hair, n="head", mesh="cone", rz=6),
            seg((12, 132, -14), (20, 110, -60), 11, hair, n="head", mesh="cone", rz=6)]
    for x, c in ((-14, rag), (10, "#5a2224"), (0, rag)):
        out.append(box(14, 34, 4, c, x, 38, 26, t=(30, 0, 0)))
    out += mirror([spike((52, 82, 40), (64, 86, 50), 3, STEEL), spike((44, 72, 38), (52, 64, 48), 3, STEEL),
                   ball(11, "#2a2224", 50, 78, 36, 1.2, 0.9, 1.2)])
    out += [ball(13, "#2a2224", -24, 26, 26), spike((-24, 26, 34), (-24, 28, 48), 4, STEEL)]
    out += chain((-58, 66, 34), (-90, 4, -30), 10, 4, sag=-12)
    out += chain((54, 78, 40), (90, 4, 80), 9, 4, sag=-8)
    out += [ball(12, IRON, 94, 10, 84)] + [spike((94, 10, 84), (94 + dx, 10 + dy, 84 + dz), 3, STEEL)
                                           for dx, dy, dz in ((14, 0, 0), (-14, 0, 0), (0, 14, 0), (0, 0, 14))]
    out += chain((-38, 104, 20), (34, 64, 30), 7, 3)
    return out


def guttersnipe():
    sk, robe, scale = "#b8946a", "#6c5c9a", "#6a7a4a"
    out = [
        part("frustum", (0, 34, -4), (64, 68, 56), robe, t=(-30, 0, 0)),
        ball(30, robe, 0, 62, 4, 1, 0.9, 0.9),
        ball(28, scale, 0, 76, -4, 1.05, 0.6, 1),
        ball(16, robe, 0, 80, -20, 1.2, 1, 0.8),
        ball(20, sk, 0, 86, 22, n="head"),
        seg((0, 86, 36), (0, 76, 50), 5, sk, n="head", mesh="cone"),
        ball(20, "#7a6a4a", 0, 94, 18, 1.02, 0.55, 1.02, n="head"),
        box(26, 3, 4, "#5a4030", 0, 92, 40, n="head"),
        ball(5, "#7a4a3a", 0, 76, 34, 1.4, 0.4, 0.6, n="head"),
    ]
    out += face(88, 38, 8, 3)
    out += mirror([seg((14, 92, 16), (36, 104, 6), 7, sk, n="head", mesh="cone", rz=3),
                   ball(8, "#5a4a3a", 8, 4, 18, 1, 0.5, 1.5)])
    out += [seg((-24, 70, 6), (-34, 48, 20), 7, robe), ball(8, sk, -34, 42, 22),
            ball(8, "#ff3040", -38, 36, 28, g=900), ball(4, "#ffb0b8", -38, 36, 32, g=1000), ball(5, "#ff3040", -44, 48, 24, g=700),
            seg((24, 70, 6), (22, 48, 22), 7, robe), ball(8, sk, 22, 44, 24)]
    return out


def cobblebrute():
    dk, mid, lt, warm = "#4e4a48", "#6a6660", "#8a857c", "#a07a5a"
    out = [
        ball(80, dk, 0, 170, -10, 1.1, 0.95, 0.9, mesh="rock"),
        ball(60, mid, 0, 210, -30, 1.2, 0.7, 0.9, mesh="rock"),
        ball(48, dk, 0, 110, -20, 1, 0.8, 0.9, mesh="rock"),
        ball(30, mid, 0, 182, 64, 1, 0.9, 1, n="head", mesh="rock"),
        box(40, 10, 10, lt, 0, 196, 86, n="head", mesh="chunk"),
        ball(4, "#f0a050", -10, 186, 92, n="head", g=700), ball(4, "#f0a050", 10, 186, 92, n="head", g=700),
    ]
    out += mirror([seg((24, 100, -16), (40, 50, -6), 24, dk, mesh="rock"), seg((40, 50, -6), (44, 12, 4), 22, mid, mesh="rock"),
                   box(44, 20, 54, dk, 44, 10, 10, mesh="chunk")])
    out += [ball(40, mid, -84, 220, -4, 1, 0.9, 1, mesh="rock"), seg((-90, 200, 0), (-110, 120, 40), 28, dk, mesh="rock"),
            seg((-110, 120, 40), (-104, 50, 80), 26, mid, mesh="rock"), ball(38, warm, -104, 34, 92, mesh="rock"),
            ball(40, mid, 84, 220, -4, 1, 0.9, 1, mesh="rock"), seg((90, 210, 0), (130, 200, 60), 28, dk, mesh="rock"),
            seg((130, 200, 60), (128, 240, 110), 24, mid, mesh="rock"), ball(36, warm, 126, 254, 124, mesh="rock")]
    for x, y, z, r in ((-40, 230, 40, 14), (30, 160, 70, 16), (60, 130, 40, 12), (-50, 150, 60, 14),
                       (0, 250, -60, 16), (-30, 100, 40, 12), (160, 280, 150, 9), (-150, 40, 130, 10)):
        out.append(box(r * 2, r * 1.2, r * 1.6, lt if r % 4 == 0 else mid, x, y, z, mesh="chunk",
                       t=(x % 90, y % 70, z % 110)))
    return out


def bloodfray_giant():
    sk, flush, pants, club, bonec = "#e2c0ae", "#c8583a", "#2a2228", "#8a2e1e", "#ecd8b8"
    out = person(sk, sk, pants, "#3a2a2a", (-150, 230, 60), (130, 300, 20), fl=(-70, 0, 30), fr=(70, 0, -30),
                 hip=150, sh=340, head=330, hr_=60, tr=120, ar=42, lr=48)
    out += [ball(64, flush, -96, 356, -16, 1, 0.65, 1), ball(64, flush, 96, 356, -16, 1, 0.65, 1),
            ball(70, flush, 0, 340, -60, 1.4, 0.8, 1),
            ball(54, sk, 0, 336, 76, n="head"),
            ball(56, "#3a2a2a", 0, 352, 70, 1.05, 0.75, 1.05, n="head"),
            ball(30, "#5a2a22", 0, 318, 120, 1.2, 0.5, 0.5, n="head")]
    for i in range(10):
        a = i / 10 * 2 * math.pi
        out.append(spike((50 * math.sin(a), 356 + 30 * math.cos(a), 70 + 34 * math.cos(a)),
                          (78 * math.sin(a), 370 + 54 * math.cos(a), 70 + 50 * math.cos(a)), 9, "#c8c0b0", n="head"))
    out += face(334, 124, 18, 9, "#ffb040", g=600)
    out += [seg((130, 300, 20), (40, 470, -60), 30, club, mesh="frustum"),
            ball(34, club, 30, 480, -70)]
    for f in (0.35, 0.5, 0.65, 0.8, 0.95):
        x, y, z = 130 - 90 * f, 300 + 170 * f, 20 - 80 * f
        out += [spike((x, y, z + 20), (x + 6, y + 6, z + 60), 9, bonec), spike((x + 20, y, z), (x + 56, y + 10, z), 9, bonec)]
    out += chain((-120, 360, 70), (100, 200, 90), 10, 9, sag=-20)
    out += chain((110, 380, -40), (60, 160, 100), 9, 9)
    return out


def tenement_crasher():
    body, belly, mane, horn, claw = "#c88a3a", "#e0b070", "#b8382a", "#3a3230", "#2a2220"
    out = [
        ball(70, body, 0, 150, 40, 1, 0.95, 1.1),
        ball(56, body, 0, 130, -70, 0.95, 0.85, 1.1),
        ball(44, belly, 0, 112, 30, 0.9, 0.7, 1.2),
        ball(50, body, 0, 152, 130, 1, 0.9, 1, n="head"),
        ball(34, belly, 0, 126, 170, 1, 0.8, 0.9, n="head"),
        ball(12, "#5a2a20", 0, 136, 194, 1.4, 0.7, 0.6, n="head"),
    ]
    out += face(160, 166, 20, 6, "#2a1a10")
    curve = [(30, 180, 130), (60, 210, 110), (80, 220, 70), (78, 206, 30), (60, 180, 10)]
    horns = [spike((20, 120, 176), (28, 150, 186), 6, "#f0e6cc", n="head")]
    for i in range(len(curve) - 1):
        horns.append(seg(curve[i], curve[i + 1], 14 - i * 2.5, horn, n="head"))
    horns.append(spike(curve[-1], (40, 170, 0), 5, horn, n="head"))
    horns += [seg((0, 0, 0), (0, 0, 0), 1, horn, n="head")] if False else []
    out += mirror(horns)
    for i, (y, z) in enumerate(((196, 130), (210, 90), (214, 50), (204, 10), (190, -30), (176, -70))):
        out.append(spike((0, y - 16, z), (0, y + 24, z - 34), 22 - i * 2, mane))
        out += mirror([spike((16, y - 20, z), (30, y + 10, z - 30), 12, mane)])
    out += mirror([
        seg((50, 130, 70), (70, 70, 120), 26, body), seg((70, 70, 120), (60, 14, 150), 20, body),
        ball(22, body, 60, 12, 160, 1, 0.5, 1.3),
        spike((50, 10, 172), (50, 4, 190), 5, claw), spike((68, 10, 172), (70, 4, 190), 5, claw),
        seg((44, 120, -80), (60, 70, -110), 26, body), seg((60, 70, -110), (54, 14, -90), 18, body),
        ball(18, body, 54, 10, -82, 1, 0.5, 1.4),
    ])
    out += [seg((0, 140, -120), (0, 120, -200), 10, body, n="tail"), seg((0, 120, -200), (20, 150, -260), 7, body, n="tail"),
            ball(14, mane, 22, 156, -268, 1, 1, 1.4, n="tail")]
    return out


# ---------------------------------------------------------------- Rakdos


def rakdos_cackler():
    red, dk, light, horn = "#c8382a", "#8a2018", "#e05a3a", "#2a2024"
    out = [
        ball(44, red, 0, 66, -4, 1.05, 0.95, 0.95),
        ball(30, light, 0, 56, 22, 1, 0.9, 0.6),
        ball(32, red, 0, 108, 20, 1.1, 0.9, 1, n="head"),
        ball(16, "#3a0e0e", 0, 98, 46, 1.4, 0.5, 0.5, n="head"),
    ]
    for i in range(6):
        x = -15 + i * 6
        out.append(spike((x, 103, 50), (x, 95, 52), 2.5, "#f4efe4", n="head"))
        out.append(spike((x + 3, 92, 50), (x + 3, 99, 52), 2.2, "#f4efe4", n="head"))
    out += face(116, 46, 12, 5, "#3ae0a0", g=900)
    out += [box(12, 4, 6, dk, -12, 123, 46, n="head", t=(0, 40, 0)), box(12, 4, 6, dk, 12, 123, 46, n="head", t=(0, -40, 0))]
    out += mirror([
        seg((28, 118, 14), (54, 118, 4), 9, horn, n="head"),
        seg((54, 118, 4), (66, 142, 8), 7, horn, n="head"),
        spike((66, 142, 8), (60, 162, 16), 5, horn, n="head"),
        seg((38, 80, 4), (54, 60, 26), 10, red), ball(10, red, 56, 56, 30),
        spike((56, 50, 34), (58, 40, 44), 3, horn), spike((50, 52, 36), (50, 42, 46), 3, horn),
        seg((22, 40, 0), (26, 16, 8), 13, red), ball(12, dk, 28, 6, 16, 1, 0.5, 1.4),
        spike((24, 4, 26), (24, 2, 36), 3, horn), spike((34, 4, 24), (36, 2, 34), 3, horn),
    ])
    for x, y, z in ((0, 128, -6), (-20, 110, -30), (20, 110, -30), (0, 100, -40), (-28, 80, -36), (28, 80, -36)):
        out.append(spike((x * 0.9, y - 4, z - 2), (x * 1.3, y + 18, z - 30), 7, horn))
    for x, y, z in ((-30, 60, 30), (34, 76, 26), (-36, 86, 16), (14, 40, 32)):
        out.append(ball(4, dk, x, y, z))
    out += [seg((56, 56, 30), (82, 30, 60), 2.5, IRON, mesh="cyl"), seg((82, 30, 60), (88, 10, 70), 4, STEEL, mesh="cone"),
            spike((82, 30, 60), (98, 40, 66), 3, STEEL)]
    out += chain((-56, 56, 30), (-80, 100, 10), 6, 3)
    out += [seg((0, 40, -40), (0, 20, -70), 5, red, n="tail"), seg((0, 20, -70), (14, 34, -96), 4, red, n="tail"),
            part("cone", (16, 40, -100), (12, 14, 4), horn, "tail", (-100, 0, 0))]
    return out


def rakdos_shred_freak():
    sk, hair, strap, pants = "#c07a52", "#1a1416", "#2a2024", "#2a2224"
    out = person(sk, sk, pants, strap, (-36, 120, 42), (40, 104, 52), fl=(-24, 0, 24), fr=(24, 0, -20),
                 tr=32, hr_=25, head=138, ar=10, lr=12)
    out += face(140, 26, 8, 4, "#ff3a2a", g=700)
    out += [ball(10, "#8a2a24", 0, 132, 24, 1.6, 0.6, 0.6, n="head"),
            box(18, 3, 3, "#f4efe4", 0, 129, 30, n="head"),
            ball(26, hair, 0, 148, -4, 1.02, 0.8, 1, n="head")]
    for i in range(9):
        a = (i / 8 - 0.5) * 2.6
        out.append(spike((16 * math.sin(a), 150 + 8 * math.cos(a), -6), (50 * math.sin(a), 168 + 30 * math.cos(a), -44),
                         7, hair, n="head"))
    out += [seg((-30, 108, 22), (28, 62, 26), 3, strap, mesh="cube", rz=2),
            seg((30, 108, 22), (-28, 62, 26), 3, strap, mesh="cube", rz=2),
            box(56, 8, 30, strap, 0, 58, 2), ball(4, STEEL, 0, 85, 28)]
    for hx, hy, hz, s in ((-36, 120, 42, -1), (40, 104, 52, 1)):
        bx, by, bz = hx * 0.8, hy - 4, hz - 14
        out += [ball(10, strap, bx, by, bz, 1.2, 1.2, 1.6)]
        for k in range(5):
            a = (k / 4 - 0.5) * 2.2
            out.append(seg((bx, by, bz), (bx + s * 40 * math.cos(a) + s * 6, by + 40 * math.sin(a), bz + 30), 3.5, STEEL,
                           mesh="cone", rz=1))
        out += [spike((bx, by + 8, bz), (bx, by + 18, bz - 4), 3, "#5a5260")]
    out += mirror([spike((34, 110, 0), (40, 126, -6), 4, "#5a5260"), ball(14, strap, 34, 104, 0, 1, 0.7, 1)])
    out += chain((0, 56, 26), (40, 10, 50), 5, 2.5) + [spike((40, 10, 50), (44, 0, 58), 3, STEEL)]
    return out


def carnival_hellsteed():
    blk, dk, skull, hoof = "#2a1816", "#3a2220", "#d8402c", "#ff4a2a"
    out = [
        ball(66, blk, 0, 160, -10, 0.85, 0.8, 1.45),
        ball(50, dk, 0, 170, 56, 0.9, 0.9, 0.9),
        ball(48, dk, 0, 158, -76, 0.9, 0.85, 1),
    ]
    for s in (-1, 1):
        s = s * 1.5
        out += [seg((s * 10, 186, 64), (s * 34, 262, 112), 22, blk),
                ball(30, skull, s * 38, 280, 126, 0.8, 0.85, 1.15, n="head", g=300),
                seg((s * 38, 276, 140), (s * 40, 266, 186), 14, skull, n="head", g=300, rz=10),
                seg((s * 38, 256, 138), (s * 40, 244, 174), 9, "#b8302a", n="head"),
                ball(8, INK, s * 38 - 12, 288, 146, n="head"), ball(8, INK, s * 38 + 12, 288, 146, n="head"),
                ball(3.5, FLAME, s * 38 - 12, 288, 152, n="head", g=1000), ball(3.5, FLAME, s * 38 + 12, 288, 152, n="head", g=1000),
                spike((s * 34, 302, 118), (s * 52, 326, 102), 6, skull, n="head")]
        for i in range(3):
            out.append(spike((s * 40, 264 - i * 4, 184 - i * 9), (s * 40, 252 - i * 4, 184 - i * 9), 2.5, BONE, n="head"))
        for f in (0.2, 0.45, 0.7):
            x, y, z = s * (16 + 18 * f), 186 + 76 * f, 64 + 48 * f
            out.append(spike((x, y + 10, z - 14), (x, y + 34, z - 36), 9, "#ff6a2a", g=700))
    legs = [
        ((26, 140, 70), (32, 104, 112), (30, 66, 124)),
        ((-26, 140, 70), (-30, 80, 82), (-32, 14, 92)),
        ((26, 140, -84), (32, 80, -114), (32, 14, -144)),
        ((-26, 140, -84), (-32, 90, -74), (-32, 14, -62)),
    ]
    for top, knee, foot in legs:
        out += [seg(top, knee, 16, blk), seg(knee, foot, 10, dk),
                ball(13, hoof, foot[0], foot[1] - 4, foot[2] + 4, 1, 0.8, 1.2, g=800)]
    out += [seg((0, 176, -140), (0, 124, -190), 12, blk, n="tail"),
            spike((0, 130, -186), (0, 72, -232), 14, "#ff6a2a", n="tail", g=700)]
    return out


MODELS = {
    "c60": daggerdrome_imp(),
    "c68": grim_roustabout(),
    "c81": thrill_kill_assassin(),
    "c62": dead_reveler(),
    "c75": sewer_shambler(),
    "c74": perilous_shadow(),
    "c61": dark_revenant(),
    "c77": slum_reaper(),
    "c58": catacomb_slug(),
    "c84": zanikev_locust(),
    "c88": bellows_lizard(),
    "c96": gore_house_chainwalker(),
    "c86": ash_zealot(),
    "c99": lobber_crew(),
    "c105": splatter_thug(),
    "c98": guttersnipe(),
    "c91": cobblebrute(),
    "c89": bloodfray_giant(),
    "c108": tenement_crasher(),
    "c220": rakdos_cackler(),
    "c221": rakdos_shred_freak(),
    "c147": carnival_hellsteed(),
}
