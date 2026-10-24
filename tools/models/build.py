#!/usr/bin/env python3
"""Writes data/models.json from every tools/models/parts/*.py.

Each parts module defines MODELS = {name: [part, ...]}. Names must not
repeat across modules. A creature card's model is "c<collector number>".

    python3 tools/models/build.py                 writes data/models.json
    python3 tools/models/build.py --out FILE      writes FILE instead (for previews while others build)
"""
import importlib.util
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
MESHES = {"cube", "sphere", "blob", "cyl", "cone", "frustum", "gem", "ico", "rock", "chunk", "ground"}
NODES = {"body", "head", "wing_l", "wing_r", "tail"}


def main(args):
    sys.path.insert(0, str(HERE))
    palette = json.loads((ROOT / "data" / "palette.json").read_text())
    models, origin, problems = {}, {}, []
    for f in sorted((HERE / "parts").glob("*.py")):
        spec = importlib.util.spec_from_file_location(f.stem, f)
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        for name, parts in mod.MODELS.items():
            if name in models:
                problems.append(f"{name}: in {origin[name]} and {f.name}")
            models[name], origin[name] = parts, f.name
            for p in parts:
                if p["m"] not in MESHES:
                    problems.append(f"{name}: unknown mesh {p['m']}")
                if p.get("n", "body") not in NODES:
                    problems.append(f"{name}: unknown node {p.get('n')}")
                c = p["c"]
                if not (c.startswith("#") and len(c) == 7) and c not in palette:
                    problems.append(f"{name}: unknown colour {c}")
            if len(parts) > 120:
                problems.append(f"{name}: {len(parts)} parts, more than 120")
    for p in problems:
        print("FAIL", p)
    if problems:
        return 1
    out = {"_about": "Models as parts, written by tools/models/build.py from tools/models/parts/*.py.", "models": dict(sorted(models.items()))}
    dest = pathlib.Path(args[1]) if args[:1] == ["--out"] else ROOT / "data" / "models.json"
    dest.write_text(json.dumps(out, separators=(",", ":")))
    print(f"models: {len(models)}, {sum(len(v) for v in models.values())} parts")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
