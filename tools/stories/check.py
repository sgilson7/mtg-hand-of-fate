#!/usr/bin/env python3
"""Check data/stories/*.json against tools/stories/SCHEMA.md and data/cards.json.

    python3 tools/stories/check.py            all groups; fails while a card has no story
    python3 tools/stories/check.py W U         some groups only
"""
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
STOCK = ["delve", "tapestry", "realm", "robust", "seamless", "elevate", "it is worth noting", "crucially", "leverage", "underscore", "landscape", "game-changer", "empower"]
EFFECTS = {"take": bool, "fight": bool, "leave": bool, "gold": int, "supplies": int, "health": int, "pool_card": bool, "style": list, "quest_start": str,
           "piles": bool, "contract": bool, "petrify": bool, "assassins": int, "rares": int,
           "enchant": str, "install": bool, "open": int, "rebuild": bool}
MARKS = {"shrine", "hex"}
# a patron's effects belong to patrons; the map effects to story and magic cards
PATRON_ONLY = {"piles", "contract", "petrify", "assassins", "rares"}
RANGES = {"gold": (-25, 30), "supplies": (-4, 6), "health": (-6, 6)}
NEEDS = {"charisma": (1, 5), "style": (1, 5), "gold": (5, 25), "supplies": (1, 4)}
GUILDS = {"Azorius", "Izzet", "Rakdos", "Golgari", "Selesnya"}
STARTERS = {"155": "writ", "226": "lantern", "76": "festival", "229": "undercity", "217": "conclave"}
QUESTS = {q["id"] for q in json.loads((ROOT / "data" / "quests.json").read_text())["quests"]}


def kind_of(c):
    t = c["types"]
    # a planeswalker is met as a patron (T37)
    if "Planeswalker" in t:
        return "patron"
    if "Creature" in t:
        return "creature"
    if "Land" in t:
        return "land"
    if "Instant" in t or "Sorcery" in t:
        return "magic"
    return "story"


def text_problems(where, s):
    out = []
    if not isinstance(s, str) or not s.strip():
        return [f"{where}: empty text"]
    if re.search(r"\d", s):
        out.append(f"{where}: a digit in text ({s[:40]}...); the game shows amounts")
    low = s.lower()
    for w in STOCK:
        if re.search(rf"\b{re.escape(w)}\b", low):
            out.append(f"{where}: stock phrase '{w}'")
    for sent in re.split(r"(?<=[.!?])\s+", s):
        if len(sent.split()) > 25:
            out.append(f"{where}: a sentence over 25 words")
    return out


def outcome_problems(where, o, depth=0):
    out = []
    if not isinstance(o, dict):
        return [f"{where}: outcome is not an object"]
    out += text_problems(f"{where}.text", o.get("text"))
    if "fate" in o:
        if depth:
            return out + [f"{where}: fate inside fate"]
        f = o["fate"]
        faces = f.get("faces")
        if not (isinstance(faces, list) and len(faces) == 4 and all(isinstance(n, int) and n >= 0 for n in faces) and sum(faces) == 4):
            out.append(f"{where}.fate.faces: four counts that add up to 4")
            faces = [1, 1, 1, 1]
        for k, n in zip(["huge_success", "success", "failure", "huge_failure"], faces):
            if n and k not in f:
                out.append(f"{where}.fate: has {k} faces and no {k} branch")
            if k in f:
                out += outcome_problems(f"{where}.fate.{k}", f[k], depth + 1)
        return out
    effects = o.get("effects")
    if not isinstance(effects, list) or not effects:
        return out + [f"{where}: no effects"]
    for e in effects:
        if not (isinstance(e, dict) and len(e) == 1):
            out.append(f"{where}: an effect is one key")
            continue
        (k, v), = e.items()
        if k not in EFFECTS or not isinstance(v, EFFECTS[k]):
            out.append(f"{where}: unknown effect {k}={v!r}")
            continue
        if k in RANGES and not RANGES[k][0] <= v <= RANGES[k][1]:
            out.append(f"{where}: {k} {v} outside {RANGES[k]}")
        if k == "style" and not (len(v) == 2 and v[0] in GUILDS and v[1] in (1, 2)):
            out.append(f"{where}: style is [guild, 1 or 2]")
        if k == "quest_start" and v not in QUESTS:
            out.append(f"{where}: no quest {v}")
        if k == "enchant" and v not in MARKS:
            out.append(f"{where}: enchant is one of {sorted(MARKS)}")
        if k == "open" and v not in (1, 4):
            out.append(f"{where}: open is 1, or 4 for every neighbour")
        if k in ("assassins", "rares") and not 1 <= v <= 3:
            out.append(f"{where}: {k} {v} outside (1, 3)")
    return out


def effects_in(o):
    if "fate" in o:
        for k in ("huge_success", "success", "failure", "huge_failure"):
            if k in o["fate"]:
                yield from effects_in(o["fate"][k])
    else:
        for e in o.get("effects", []):
            yield from e.items()


def check_entry(c, s):
    n = c["number"]
    where = f"{n} {c['name']}"
    out = []
    k = kind_of(c)
    if s.get("kind") != k:
        out.append(f"{where}: kind should be {k}")
        return out
    if s.get("name") != c["name"]:
        out.append(f"{where}: name does not match the card")
    if k == "creature":
        for f in ("intro", "win", "lose", "flee"):
            out += text_problems(f"{where}.{f}", s.get(f))
    elif k == "land":
        out += text_problems(f"{where}.text", s.get("text"))
    elif k == "patron":
        for f in ("title", "text", "win", "lose"):
            out += text_problems(f"{where}.{f}", s.get(f))
        ch = s.get("choices")
        if not (isinstance(ch, list) and 3 <= len(ch) <= 4):
            return out + [f"{where}: three or four choices"]
        if sorted(x["ability"] for x in ch if "ability" in x) != [0, 1, 2]:
            out.append(f"{where}: each of the three loyalty abilities once")
        fights = False
        for i, x in enumerate(ch):
            out += text_problems(f"{where}.choices[{i}].text", x.get("text"))
            o = x.get("outcome") or {}
            if "ability" in x and "fate" in o:
                out.append(f"{where}.choices[{i}]: a loyalty ability has no fate draw")
            out += outcome_problems(f"{where}.choices[{i}]", o)
            fights |= "ability" not in x and any(k2 == "fight" for k2, _ in effects_in(o))
        if not fights:
            out.append(f"{where}: no choice challenges the patron")
    else:
        out += text_problems(f"{where}.title", s.get("title"))
        out += text_problems(f"{where}.text", s.get("text"))
        ch = s.get("choices")
        if not (isinstance(ch, list) and 2 <= len(ch) <= 3):
            return out + [f"{where}: two or three choices"]
        takes = False
        quest = False
        for i, x in enumerate(ch):
            out += text_problems(f"{where}.choices[{i}].text", x.get("text"))
            if "needs" in x:
                nd = x["needs"]
                if not (isinstance(nd, dict) and len(nd) == 1 and next(iter(nd)) in NEEDS):
                    out.append(f"{where}.choices[{i}].needs: one of {list(NEEDS)}")
                else:
                    (nk, nv), = nd.items()
                    v = nv[1] if nk == "style" else nv
                    if nk == "style" and nv[0] not in GUILDS:
                        out.append(f"{where}.choices[{i}].needs.style: a guild")
                    if not (isinstance(v, int) and NEEDS[nk][0] <= v <= NEEDS[nk][1]):
                        out.append(f"{where}.choices[{i}].needs: {nk} outside {NEEDS[nk]}")
            out += outcome_problems(f"{where}.choices[{i}]", x.get("outcome"))
            effs = list(effects_in(x.get("outcome") or {}))
            for k2, _ in effs:
                if k2 in PATRON_ONLY:
                    out.append(f"{where}.choices[{i}]: {k2} is a patron's effect")
            takes |= any(k2 in ("take", "fight") for k2, _ in effs)
            quest |= any(k2 == "quest_start" and v2 == STARTERS.get(n) for k2, v2 in effs)
        if not takes:
            out.append(f"{where}: no choice lets you take the card")
        if n in STARTERS and not quest:
            out.append(f"{where}: must start quest {STARTERS[n]}")
    return out


def main(groups):
    cards = json.loads((ROOT / "data" / "cards.json").read_text())["cards"]
    groups = groups or sorted({c["group"] for c in cards})
    problems = []
    for g in groups:
        path = ROOT / "data" / "stories" / f"{g}.json"
        want = [c for c in cards if c["group"] == g]
        if not path.exists():
            problems.append(f"{g}: no data/stories/{g}.json")
            continue
        try:
            got = {s.get("number"): s for s in json.loads(path.read_text())["stories"]}
        except (ValueError, KeyError, TypeError) as e:
            problems.append(f"{g}: unreadable ({e})")
            continue
        for c in want:
            if c["number"] not in got:
                problems.append(f"{c['number']} {c['name']}: no story")
            else:
                problems += check_entry(c, got[c["number"]])
        extra = set(got) - {c["number"] for c in want}
        problems += [f"{g}: story {x} is not a card of this group" for x in sorted(extra)]
    for p in problems:
        print("FAIL", p)
    print("stories:", "ok" if not problems else f"{len(problems)} problems")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
