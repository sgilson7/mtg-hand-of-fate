// The panels: built from the core's view model without knowing what any
// panel means. Each panel names where it sits (place) and holds any of:
// a heading, text, lines, rows, a health meter, mana pips, a list, a menu,
// choices, cards, fate faces, a battle log, bars and actions. Every button
// sends its action name and value back through act(). Card faces are drawn
// from the core's description: the text arrives already split into words,
// mana symbols and reminder text, so nothing here reads a card's rules.

const make = (tag, cls, text) => {
  const n = document.createElement(tag);
  if (cls) n.className = cls;
  if (text !== undefined && text !== null) n.textContent = text;
  return n;
};

export function makeUi(root, labelRoot, act) {
  let hints = [];

  function mark(node) {
    const hint = hints.find((h) => h.term === node.textContent);
    if (hint) {
      node.dataset.term = hint.term;
      node.title = hint.definition;
    }
  }

  function button(a, cls) {
    const b = make("button", cls || "act", a.label);
    b.disabled = !a.enabled;
    if (a.key) b.dataset.key = a.key;
    if (a.sub) b.append(make("small", "", a.sub));
    b.addEventListener("click", () => act(a.act, a.value));
    return b;
  }

  function symbol(s) {
    const k = s.replace("/", "");
    const n = make("span", `m m-${k}`, k);
    n.setAttribute("aria-label", `{${s}}`);
    return n;
  }

  function cardFace(f) {
    const c = make("div", `card ${f.size || "md"} ${f.frame.split(" ").map((x) => "f-" + x).join(" ")}`);
    if (f.dim) c.classList.add("dim");
    const name = make("div", "nm");
    name.append(make("span", "nm-t", f.name));
    const cost = make("span", "cost");
    f.cost.forEach((s) => cost.append(symbol(s)));
    name.append(cost);
    const art = make("div", "art");
    const img = make("img");
    img.loading = "lazy";
    img.alt = "";
    img.src = f.art;
    art.append(img);
    const ty = make("div", "ty");
    ty.append(make("span", "", f.type_line), make("span", "rar", f.rarity));
    const tx = make("div", "tx");
    f.lines.forEach((line) => {
      const p = make("p");
      line.forEach((seg) => {
        if (seg.s !== undefined) p.append(symbol(seg.s));
        else if (seg.i !== undefined) p.append(make("i", "", seg.i));
        else p.append(document.createTextNode(seg.t));
      });
      tx.append(p);
    });
    if (f.flavor) tx.append(make("p", "fl", f.flavor));
    const ft = make("div", "ft");
    ft.append(make("span", "", `${f.number} RTR · ${f.artist}`));
    if (f.pt) ft.append(make("span", "pt", f.pt));
    c.append(make("div", "face"), name, art, ty, tx, ft);
    if (f.badge) c.append(make("div", "badge", f.badge));
    if (f.act) {
      c.classList.add("clickable");
      c.tabIndex = 0;
      c.addEventListener("click", () => act(f.act, f.value));
    }
    if (f.actions) {
      const row = make("div", "card-acts");
      f.actions.forEach((a) => row.append(button(a, "act small")));
      c.append(row);
    }
    return c;
  }

  function panel(p) {
    const box = make("section", `panel place-${p.place || "center"} kind-${p.kind || "glass"}`);
    box.id = `panel-${p.id}`;
    if (p.eyebrow) box.append(make("div", "eyebrow", p.eyebrow));
    if (p.heading) {
      const h = make("h2", "", p.heading);
      mark(h);
      box.append(h);
    }
    if (p.meter) {
      const m = make("div", "meter");
      const bar = make("progress");
      bar.max = p.meter.max;
      bar.value = p.meter.value;
      m.append(make("span", "meter-label", p.meter.label), bar);
      box.append(m);
    }
    if (p.text) box.append(make("p", "text", p.text));
    (p.lines || []).forEach((l) => box.append(make("p", "line", l)));
    if (p.rows && p.rows.length) {
      const t = make("dl", "rows");
      p.rows.forEach(([k, v]) => {
        const dt = make("dt", "", k);
        mark(dt);
        t.append(dt, make("dd", "", v));
      });
      box.append(t);
    }
    (p.pips || []).forEach((pip) => {
      const row = make("div", "pips");
      row.append(make("span", "pips-label", pip.label));
      pip.units.forEach((u) => row.append(symbol(u)));
      box.append(row);
    });
    if (p.menu) {
      const m = make("div", "menu");
      p.menu.forEach((a) => m.append(button(a, "act menu-item")));
      box.append(m);
    }
    if (p.list) {
      const l = make("div", "list");
      p.list.forEach((a) => {
        const b = button(a, "act list-item");
        if (a.hot) b.classList.add("hot");
        l.append(b);
      });
      box.append(l);
    }
    if (p.choices) {
      const ch = make("div", "choices");
      p.choices.forEach((c) => {
        const b = make("button", "choice");
        b.disabled = !c.enabled;
        b.append(make("b", "", c.key), make("span", "", c.text));
        if (c.sub) b.append(make("small", "", c.sub));
        if (c.guild) b.dataset.guild = c.guild;
        b.addEventListener("click", () => act(c.act, c.value));
        ch.append(b);
      });
      box.append(ch);
    }
    if (p.cards) {
      const row = make("div", "cards");
      p.cards.forEach((f) => row.append(cardFace(f)));
      box.append(row);
    }
    if (p.faces) {
      const row = make("div", "faces");
      p.faces.forEach((f) => {
        const b = make("button", `fate ${f.up ? "up" : "down"} ${f.up ? (f.good ? "good" : "bad") : ""}`, f.label);
        b.disabled = !f.enabled;
        b.addEventListener("click", () => act(f.act, f.value));
        row.append(b);
      });
      box.append(row);
    }
    if (p.log) {
      const l = make("ol", "log");
      p.log.forEach((e) => {
        const li = make("li", e.now ? "now" : "", e.text);
        li.dataset.side = e.side;
        l.append(li);
      });
      box.append(l);
    }
    if (p.bars) {
      const b = make("div", "bars");
      p.bars.forEach(([k, n]) => {
        const col = make("div", "bar");
        col.style.setProperty("--n", n);
        col.append(make("em", "", n), make("span", "", k));
        b.append(col);
      });
      box.append(b);
    }
    if (p.colours) {
      const c = make("div", "pips");
      p.colours.forEach(([k, n]) => {
        c.append(symbol(k), make("span", "pip-n", n));
      });
      box.append(c);
    }
    if (p.actions && p.actions.length) {
      const row = make("div", "actions");
      p.actions.forEach((a) => row.append(button(a)));
      box.append(row);
    }
    return box;
  }

  function label(l) {
    const n = make("div", `lbl ${l.hot ? "hot" : ""}`, l.text);
    if (l.sub) n.append(make("small", "", l.sub));
    n.style.setProperty("--x", l.x);
    n.style.setProperty("--y", l.y);
    return n;
  }

  return {
    draw(view) {
      hints = view.hints || [];
      root.replaceChildren(...view.panels.map(panel));
      labelRoot.replaceChildren(...view.labels.map(label));
    },
  };
}
