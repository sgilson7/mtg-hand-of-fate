"""Helpers for writing models as parts. See tools/models/README.md.

A part: a unit mesh (one unit across, centred on its origin; a cone's apex
points up +y), placed at p (cm) with size s (cm, the full width on each axis),
coloured c ("#rrggbb" or a palette role from data/palette.json), tilted by t
(pitch about x, roll about z, yaw about y, in thousandths of a turn, applied
roll then pitch then yaw), moving with node n, glowing by g (thousandths).

Meshes: cube, sphere, blob (a low sphere), cyl (nine sides), cone, frustum (a
cylinder narrowing to three fifths at the top), gem (an octahedron), ico,
rock (a nudged ico), chunk (a nudged cube), ground (a flat square facing up).
Nodes: body, head, wing_l, wing_r, tail.
Models stand on y = 0 and face +z.
"""


def part(m, p, s, c, n="body", t=(0, 0, 0), g=0):
    out = {"m": m, "p": [round(v) for v in p], "s": [max(1, round(v)) for v in s], "c": c}
    if n != "body":
        out["n"] = n
    if any(t):
        out["t"] = [round(v) for v in t]
    if g:
        out["g"] = round(g)
    return out


def ball(r, c, x, y, z, sx=1, sy=1, sz=1, n="body", t=(0, 0, 0), mesh="sphere", g=0):
    """A sphere of radius r cm at (x, y, z), squashed by sx, sy, sz."""
    return part(mesh, (x, y, z), (2 * r * sx, 2 * r * sy, 2 * r * sz), c, n, t, g)


def box(w, h, d, c, x, y, z, n="body", t=(0, 0, 0), mesh="cube"):
    """A block w x h x d cm whose centre is at (x, y, z)."""
    return part(mesh, (x, y, z), (w, h, d), c, n, t)


def cyl(r, h, c, x, y, z, n="body", t=(0, 0, 0), mesh="cyl"):
    """A cylinder (or cone, frustum) of radius r and height h, its base at y."""
    return part(mesh, (x, y + h / 2, z), (2 * r, h, 2 * r), c, n, t)


def mirror(parts):
    """The parts and their copies mirrored across x = 0 (wing_l <-> wing_r)."""
    out = list(parts)
    for p in parts:
        q = dict(p)
        q["p"] = [-p["p"][0], p["p"][1], p["p"][2]]
        if "t" in p:
            pitch, roll, yaw = (p["t"] + [0, 0, 0])[:3]
            q["t"] = [pitch, -roll, -yaw]
        if p.get("n") == "wing_l":
            q["n"] = "wing_r"
        elif p.get("n") == "wing_r":
            q["n"] = "wing_l"
        out.append(q)
    return out


def eyes(y, z, spread, r, c="#1d1626", n="head"):
    return [ball(r, c, -spread, y, z, n=n), ball(r, c, spread, y, z, n=n)]
