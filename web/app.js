// The page: loads the module and the data, then each display frame hands
// the time to the core, uploads the draw list it returns to WebGL, and
// redraws the panels when the core's view changes. Keys and clicks go back
// as actions. It decides nothing (Builder R6).
import init, { Game, numbers, mesh, uniforms } from "./pkg/tool_shim.js";
import { makeRenderer } from "./gl.js";
import { makeUi } from "./ui.js";
import { readSave, writeSave } from "./store.js";

const el = (id) => document.getElementById(id);
const text = async (path) => (await fetch(path)).text();

async function main() {
  await init();
  const manifest = JSON.parse(await text("data/manifest.json"));
  const bundle = {};
  const texts = await Promise.all(manifest.files.map((f) => text(`data/${f}`)));
  manifest.files.forEach((f, i) => { bundle[f] = texts[i]; });
  bundle["glossary.md"] = await text("data/glossary.md");
  const seed = new Uint32Array(1);
  crypto.getRandomValues(seed);
  const game = new Game(JSON.stringify(bundle), readSave(), seed[0], location.search);
  const N = JSON.parse(numbers());
  const controls = JSON.parse(bundle["controls.json"]);
  const strings = JSON.parse(bundle["strings.json"]);
  const canvas = el("gl");
  const renderer = makeRenderer(canvas, N, { vertex: bundle["shaders/scene.vert"], fragment: bundle["shaders/scene.frag"], software: [] }, (id) => mesh(id), uniforms());
  if (!renderer) el("message").textContent = strings["error.no_webgl"];

  let screen = "";
  function act(name, value) {
    const r = JSON.parse(game.act(name, value));
    el("message").textContent = r.error || "";
  }
  const ui = makeUi(el("panels"), el("labels"), act);

  window.addEventListener("keydown", (e) => {
    if (e.target && (e.target.tagName === "INPUT" || e.target.tagName === "TEXTAREA")) return;
    const a = (controls[screen] || {})[e.code];
    if (!a) return;
    e.preventDefault();
    act(a[0], a[1]);
  });
  // a drag turns the view (the fight's table); a drag is not a click
  let dragging = false;
  let moved = false;
  canvas.addEventListener("pointerdown", () => { dragging = true; moved = false; });
  window.addEventListener("pointerup", () => { dragging = false; });
  canvas.addEventListener("pointermove", (e) => {
    if (!dragging || !e.movementX) return;
    moved = true;
    game.act("view.drag", `${e.movementX}`);
  });
  canvas.addEventListener("click", (e) => {
    if (moved) { moved = false; return; }
    const r = JSON.parse(game.pick(e.offsetX, e.offsetY));
    el("message").textContent = r.error || "";
  });

  function resize() {
    game.act("resize", `${canvas.clientWidth},${canvas.clientHeight},${window.devicePixelRatio || 1}`);
    const [w, h] = game.pixels();
    if (renderer) renderer.resize(w, h);
  }
  resize();
  window.addEventListener("resize", resize);

  let version = null;
  let shown = null;
  function frame(now) {
    game.frame(now);
    if (renderer) {
      const v = game.static_version();
      if (v !== version) {
        renderer.setStatic(game.static_values());
        version = v;
      }
      renderer.frame(game.views(), game.values(), game.commands());
    }
    const u = game.ui_version();
    if (u !== shown) {
      shown = u;
      const view = JSON.parse(game.ui());
      screen = view.screen;
      document.body.dataset.screen = screen;
      document.title = view.title;
      el("keys").textContent = view.keys;
      if (view.message) el("message").textContent = view.message;
      ui.draw(view);
    }
    const save = game.take_save();
    if (save) writeSave(save);
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
  document.body.dataset.ready = "true";
  if (location.hash === "#models") act("screen.show", "models");
}

main();
