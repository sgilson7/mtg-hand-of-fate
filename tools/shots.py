#!/usr/bin/env python3
"""Screenshots of the running build for the design journal.

Serves dist/ on a free port, opens it in Chromium at 1280x720 (device scale
1.5), runs a list of steps, and saves pictures. The page has no test hooks:
the steps press keys and click buttons the way a player would.

    .venv/bin/python tools/shots.py OUT_DIR steps.json

Steps (JSON list): ["key", "ArrowRight"], ["click", "button text"],
["clicksel", "css selector"], ["wait", ms], ["screen", "board"] (wait for body[data-screen]),
["shot", "name"], ["hash", "models"], ["reload"].
Fails on a console error or page error.
"""
import functools
import http.server
import json
import pathlib
import sys
import threading

from playwright.sync_api import sync_playwright

ROOT = pathlib.Path(__file__).resolve().parents[1]
DIST = ROOT / "dist"


class Quiet(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm", ".js": "text/javascript"}

    def log_message(self, *a):
        pass


def main(out, steps_path, query=""):
    steps = json.loads(pathlib.Path(steps_path).read_text())
    out = pathlib.Path(out)
    out.mkdir(parents=True, exist_ok=True)
    srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Quiet, directory=str(DIST)))
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    base = f"http://127.0.0.1:{srv.server_address[1]}/{query}"
    errors = []
    with sync_playwright() as p:
        b = p.chromium.launch(args=["--enable-gpu", "--ignore-gpu-blocklist", "--use-angle=metal"])
        ctx = b.new_context(viewport={"width": 1280, "height": 720}, device_scale_factor=1.5)
        pg = ctx.new_page()
        pg.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)
        pg.on("pageerror", lambda e: errors.append(str(e)))
        pg.goto(base)
        pg.wait_for_selector("body[data-ready='true']", timeout=60000)
        for st in steps:
            kind, arg = st[0], (st[1] if len(st) > 1 else None)
            if kind == "key":
                pg.keyboard.press(arg)
            elif kind == "click":
                pg.get_by_role("button", name=arg).first.click()
            elif kind == "clicksel":
                pg.locator(arg).first.click()
            elif kind == "wait":
                pg.wait_for_timeout(arg)
            elif kind == "screen":
                pg.wait_for_selector(f"body[data-screen='{arg}']", timeout=30000)
            elif kind == "hash":
                pg.goto(base + "#" + arg)
                pg.wait_for_selector("body[data-ready='true']", timeout=60000)
            elif kind == "reload":
                pg.reload()
                pg.wait_for_selector("body[data-ready='true']", timeout=60000)
            elif kind == "shot":
                pg.wait_for_timeout(250)
                pg.screenshot(path=str(out / f"{arg}.jpg"), type="jpeg", quality=84)
                print("saved", arg)
            elif kind == "eval_screen":
                print("screen:", pg.evaluate("document.body.dataset.screen"))
        b.close()
    srv.shutdown()
    if errors:
        sys.exit("errors:\n" + "\n".join(errors))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2], sys.argv[3] if len(sys.argv) > 3 else "")
