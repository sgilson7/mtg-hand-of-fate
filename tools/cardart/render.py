#!/usr/bin/env python3
"""Render card art from TikZ: tools/cardart/src/<number>.tex -> art/cards/<number>.webp.

    python3 tools/cardart/render.py            every card with a source
    python3 tools/cardart/render.py 183 158    some cards
    python3 tools/cardart/render.py --check    no TeX: check what is committed

Each picture recreates the composition of the card's original art as flat,
ink-outlined shapes. The rules this script holds:
- a picture has no words, letters or digits (TeX that sets text is refused);
- the source names no image file (no \\includegraphics: nothing traced or pasted);
- the output is 616 x 452 and at most MAX_BYTES;
- every card in data/cards.json has a picture (--check);
- no reference art is kept in the repository (--check fails on any jpg or png
  under tools/cardart/).
"""
import json
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
SRC = HERE / "src"
OUT = ROOT / "art" / "cards"
W, H = 616, 452
MAX_BYTES = 60_000
REFUSED = re.compile(r"\\(node|text|put|char|pgftext|includegraphics|input\{(?!\.\./preamble)|write|immediate|openout)\b|label\s*=")


def cards():
    return json.loads((ROOT / "data" / "cards.json").read_text())["cards"]


def refuse(tex):
    body = "\n".join(l.split("%", 1)[0] for l in tex.splitlines())
    m = REFUSED.search(body)
    return m.group(0) if m else None


def render(number):
    from PIL import Image  # only rendering needs Pillow
    tex = (SRC / f"{number}.tex").read_text()
    bad = refuse(tex)
    if bad:
        return f"{number}: refused, the source uses {bad}"
    with tempfile.TemporaryDirectory() as tmp:
        t = pathlib.Path(tmp)
        (t / "src").mkdir()
        shutil.copy(HERE / "preamble.tex", t / "preamble.tex")
        (t / "src" / "card.tex").write_text(tex)
        r = subprocess.run(["pdflatex", "-interaction=nonstopmode", "-halt-on-error", "card.tex"], cwd=t / "src", capture_output=True, text=True)
        if r.returncode:
            lines = [l for l in r.stdout.splitlines() if l.startswith("!")]
            return f"{number}: TeX failed: {' '.join(lines[:2]) or r.stdout[-300:]}"
        subprocess.run(["pdftocairo", "-png", "-singlefile", "-scale-to-x", str(W), "-scale-to-y", str(H), "card.pdf", "card"], cwd=t / "src", check=True)
        img = Image.open(t / "src" / "card.png").convert("RGB")
        if img.size != (W, H):
            img = img.resize((W, H))
        OUT.mkdir(parents=True, exist_ok=True)
        q = 82
        while True:
            img.save(OUT / f"{number}.webp", "WEBP", quality=q, method=6)
            if (OUT / f"{number}.webp").stat().st_size <= MAX_BYTES or q <= 40:
                break
            q -= 8
    size = (OUT / f"{number}.webp").stat().st_size
    return None if size <= MAX_BYTES else f"{number}: {size} bytes, over {MAX_BYTES}"


def check():
    problems = []
    for c in cards():
        n = c["number"]
        if not (SRC / f"{n}.tex").exists():
            problems.append(f"{n} {c['name']}: no source")
            continue
        bad = refuse((SRC / f"{n}.tex").read_text())
        if bad:
            problems.append(f"{n}: the source uses {bad}")
        p = OUT / f"{n}.webp"
        if not p.exists():
            problems.append(f"{n} {c['name']}: no picture")
        elif p.stat().st_size > MAX_BYTES:
            problems.append(f"{n}: {p.stat().st_size} bytes, over {MAX_BYTES}")
    for f in HERE.rglob("*"):
        if f.suffix.lower() in (".jpg", ".jpeg", ".png", ".webp", ".gif"):
            problems.append(f"{f.relative_to(ROOT)}: an image under tools/cardart; reference art is never kept in the repository")
    return problems


def main(args):
    if args[:1] == ["--check"]:
        problems = check()
        for p in problems:
            print("FAIL", p)
        print("card art:", "ok" if not problems else f"{len(problems)} problems")
        return 1 if problems else 0
    if not shutil.which("pdflatex") or not shutil.which("pdftocairo"):
        print("card art: pdflatex and pdftocairo are needed to render")
        return 3
    numbers = args or sorted(p.stem for p in SRC.glob("*.tex"))
    failed = [m for m in (render(n) for n in numbers) if m]
    for m in failed:
        print("FAIL", m)
    print(f"rendered {len(numbers) - len(failed)} of {len(numbers)}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
