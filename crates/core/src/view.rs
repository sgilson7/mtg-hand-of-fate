//! The screen as a view model: panels with their text, rows, cards, choices
//! and actions, labels placed over the 3D view, and the glossary's hints.
//! The page builds the DOM from this without knowing what a panel means
//! (Builder R6; after Peter's Mandible).

use serde_json::{json, Value};

use crate::battle::{Outcome, P};
use crate::content::{ty, Card, Kind};
use crate::encounter;
use crate::geom::{self, V3};
use crate::mana;
use crate::player::{self, ABILITIES};
use crate::rules::kw;
use crate::scene;
use crate::session::{Next, Room, Screen, Session};
use crate::world::{Feature, Guild, Pos, Terrain};

fn action(label: String, name: &str, value: &str, enabled: bool) -> Value {
    json!({"label": label, "act": name, "value": value, "enabled": enabled})
}

/// A card's text as lines of segments: plain text, mana symbols, reminder text.
pub fn segments(text: &str) -> Value {
    let mut lines = Vec::new();
    for line in text.split('\n') {
        let mut segs: Vec<Value> = Vec::new();
        let mut buf = String::new();
        let mut italic = false;
        let mut chars = line.chars().peekable();
        let flush = |buf: &mut String, segs: &mut Vec<Value>, italic: bool| {
            if !buf.is_empty() {
                segs.push(if italic {
                    json!({"i": buf.clone()})
                } else {
                    json!({"t": buf.clone()})
                });
                buf.clear();
            }
        };
        while let Some(ch) = chars.next() {
            match ch {
                '{' => {
                    flush(&mut buf, &mut segs, italic);
                    let mut sym = String::new();
                    for c in chars.by_ref() {
                        if c == '}' {
                            break;
                        }
                        sym.push(c);
                    }
                    segs.push(json!({"s": sym}));
                }
                '(' => {
                    flush(&mut buf, &mut segs, italic);
                    italic = true;
                    buf.push(ch);
                }
                ')' => {
                    buf.push(ch);
                    flush(&mut buf, &mut segs, italic);
                    italic = false;
                }
                _ => buf.push(ch),
            }
        }
        flush(&mut buf, &mut segs, italic);
        lines.push(Value::Array(segs));
    }
    Value::Array(lines)
}

fn cost_tokens(cost: &str) -> Value {
    Value::Array(
        cost.split('}')
            .filter_map(|t| t.strip_prefix('{'))
            .map(|t| json!(t))
            .collect(),
    )
}

pub fn frame_of(c: &Card) -> String {
    if c.is(ty::LAND) {
        return "land".into();
    }
    let n = mana::count_colours(c.colours);
    if n == 0 {
        return "artifact".into();
    }
    let letters: Vec<&str> = mana::COLOURS
        .iter()
        .filter(|k| c.colours & **k != 0)
        .map(|k| mana::letter_of(*k))
        .collect();
    if n >= 2 && c.hybrid {
        return format!("hybrid {}", letters.join(" "));
    }
    if n >= 2 {
        return "gold".into();
    }
    letters[0].to_string()
}

/// A card face: everything the page needs to draw it, text verbatim.
pub fn face(s: &Session, c: usize, size: &str) -> Value {
    let k = &s.cards[c];
    let pt = if k.is(ty::CREATURE) {
        format!("{}/{}", k.power_text, k.toughness_text)
    } else {
        String::new()
    };
    json!({
        "id": c,
        "number": k.number,
        "name": k.name,
        "cost": cost_tokens(&k.cost_text),
        "type_line": k.type_line,
        "lines": segments(&k.text),
        "flavor": k.flavor,
        "pt": pt,
        "frame": frame_of(k),
        "art": format!("art/cards/{}.webp", k.number),
        "rarity": s.s(&format!("rarity.{}", k.rarity)),
        "artist": k.artist,
        "size": size,
    })
}

fn kw_names(s: &Session, k: u32) -> Vec<String> {
    let names = [
        (kw::FLYING, "kw.flying"),
        (kw::FIRST_STRIKE, "kw.first_strike"),
        (kw::DOUBLE_STRIKE, "kw.double_strike"),
        (kw::DEATHTOUCH, "kw.deathtouch"),
        (kw::LIFELINK, "kw.lifelink"),
        (kw::TRAMPLE, "kw.trample"),
        (kw::HASTE, "kw.haste"),
        (kw::VIGILANCE, "kw.vigilance"),
        (kw::REACH, "kw.reach"),
        (kw::DEFENDER, "kw.defender"),
        (kw::HEXPROOF, "kw.hexproof"),
        (kw::UNBLOCKABLE, "kw.unblockable"),
        (kw::SWAMPWALK, "kw.swampwalk"),
        (kw::FLASH, "kw.flash"),
    ];
    names
        .iter()
        .filter(|(b, _)| k & b != 0)
        .map(|(_, key)| s.s(key))
        .collect()
}

/// The person's three readings of a card: type, mana value, abilities.
fn readings(s: &Session, c: usize, at: Pos) -> Value {
    let k = &s.cards[c];
    let kind = s.s(&format!("reading.kind.{}", k.kind().name()));
    let tier = k.tier();
    let mv = match k.kind() {
        Kind::Creature => s.f(
            "reading.tier.creature",
            &[
                ("tier", tier.to_string()),
                ("life", encounter::FOE_LIFE[tier as usize - 1].to_string()),
                ("mana", tier.to_string()),
            ],
        ),
        Kind::Land => s.s("reading.tier.land"),
        _ => s.f("reading.tier.story", &[("tier", tier.to_string())]),
    };
    let r = &k.rules;
    let mut abilities = kw_names(s, r.kw);
    for (flag, key) in [
        (r.unleash, "kw.unleash"),
        (r.scavenge.is_some(), "kw.scavenge"),
        (r.overload.is_some(), "kw.overload"),
        (
            r.etb.iter().any(|e| matches!(e, crate::rules::Eff::Detain(_))),
            "kw.detain",
        ),
        (
            r.etb.contains(&crate::rules::Eff::Populate)
                || r.modes
                    .iter()
                    .flatten()
                    .any(|e| *e == crate::rules::Eff::Populate),
            "kw.populate",
        ),
        (!r.activated.is_empty(), "kw.activated"),
        (!r.etb.is_empty(), "kw.enters"),
        (!r.dies.is_empty(), "kw.dies"),
        (!r.statics.is_empty() || r.on_cast.is_some(), "kw.static"),
    ] {
        if flag {
            abilities.push(s.s(key));
        }
    }
    let ab = if abilities.is_empty() {
        s.s("reading.abilities.none")
    } else {
        s.f("reading.abilities", &[("list", abilities.join(", "))])
    };
    let lands = s.world().lands_in_reach(at, s.player().reach(&s.armoury)).len();
    json!({
        "id": "readings", "place": "right", "heading": s.s("reading.heading"),
        "rows": [[s.s("reading.type"), kind], [s.s("reading.value"), mv], [s.s("reading.abilities_label"), ab],
                 [s.s("reading.lands"), s.f("reading.lands_n", &[("n", lands.to_string())])]],
    })
}

fn hud(s: &Session) -> Value {
    let p = s.player();
    let a = &s.armoury;
    let lands: usize = s.world().tiles.values().map(|t| t.lands.len()).sum();
    json!({
        "id": "hud", "place": "tl", "heading": s.s(&format!("guild.{}", s.guild.name())),
        "meter": {"value": p.health, "max": p.max_health(a), "label": s.f("hud.health", &[("n", p.health.to_string()), ("max", p.max_health(a).to_string())])},
        "rows": [
            [s.s("hud.supplies"), format!("{} / {}", p.supplies, p.max_supplies())],
            [s.s("hud.gold"), p.gold.to_string()],
            [s.s("hud.reach"), p.reach(a).to_string()],
            [s.s("hud.charisma"), p.charisma(a).to_string()],
            [s.s("hud.style"), p.style(a).to_string()],
            [s.s("hud.deck"), format!("{} / {}", p.deck.len(), player::DECK_MAX)],
            [s.s("hud.lands"), lands.to_string()],
        ],
        "actions": [action(s.s("hud.open_deck"), "deck.open", "", true)],
    })
}

fn quests_panel(s: &Session) -> Value {
    let p = s.player();
    let mut rows = Vec::new();
    for q in &s.quests {
        let Some(&done) = p.quests.get(&q.id) else {
            continue;
        };
        let name = s.s(&format!("quest.{}", q.id));
        let state = if done as usize >= q.steps.len() {
            s.s("quest.done")
        } else {
            s.f(
                "quest.step",
                &[
                    ("n", (done + 1).to_string()),
                    ("of", q.steps.len().to_string()),
                    ("next", s.s(&format!("quest.{}.{}", q.id, done))),
                ],
            )
        };
        rows.push(json!([name, state]));
    }
    json!({"id": "quests", "place": "tr", "heading": s.s("quest.heading"), "rows": rows})
}

fn terrain_name(s: &Session, t: Terrain) -> String {
    s.s(&format!("terrain.{}", t.key()))
}

fn tile_panel(s: &Session) -> Value {
    let at = s.player().pos;
    let Some(t) = s.world().tile(at) else {
        return json!(null);
    };
    let mut heading = terrain_name(s, t.terrain);
    if let Some(g) = t.guild {
        heading = s.f(
            "tile.district",
            &[("guild", g.name().to_string()), ("terrain", heading)],
        );
    }
    let mut lines = Vec::new();
    match t.feature {
        Feature::City(c) => lines.push(s.s(&format!("city.{}", c.id()))),
        Feature::Lair => lines.push(s.s("tile.lair")),
        _ if t.cleared => lines.push(s.s("tile.cleared")),
        _ => {}
    }
    if !t.lands.is_empty() {
        let names: Vec<String> = t.lands.iter().map(|&l| s.cards[l].name.clone()).collect();
        lines.push(s.f("tile.lands", &[("list", names.join(", "))]));
    }
    // the pools of the tiles around you, so you can choose where to step
    let mut rows = Vec::new();
    for (dir, d) in [("n", (0, -1)), ("e", (1, 0)), ("s", (0, 1)), ("w", (-1, 0))] {
        let p = (at.0 + d.0, at.1 + d.1);
        let Some(n) = s.world().tile(p) else { continue };
        let what = match n.feature {
            Feature::City(c) => s.s(&format!("city.{}", c.id())),
            Feature::Lair => s.s("tile.lair"),
            Feature::Peak => s.s("tile.peak"),
            Feature::Canal => s.s("tile.canal"),
            Feature::Cards if n.cleared || n.pool.is_empty() => s.s("tile.cleared"),
            Feature::Cards => {
                let names: Vec<String> = n.pool.iter().map(|&c| s.cards[c].name.clone()).collect();
                names.join(", ")
            }
        };
        let mut place = terrain_name(s, n.terrain);
        if let Some(g) = n.guild {
            place = s.f(
                "tile.district",
                &[("guild", g.name().to_string()), ("terrain", place)],
            );
        }
        rows.push(json!([s.s(&format!("dir.{dir}")), format!("{place} · {what}")]));
    }
    json!({"id": "tile", "place": "bl", "heading": heading, "lines": lines, "rows": rows})
}

fn labels_board(s: &Session) -> Vec<Value> {
    let mut out = Vec::new();
    let w = s.world();
    for (p, t) in &w.tiles {
        let (key, sub) = match t.feature {
            Feature::City(c) => (format!("city.{}", c.id()), s.s("label.city")),
            Feature::Lair => ("tile.lair".to_string(), s.s("label.lair")),
            _ => continue,
        };
        out.push((scene::tile_point(*p, 2), s.s(&key), sub, false));
    }
    let you = s.player().pos;
    let mut labels: Vec<Value> = out
        .into_iter()
        .filter_map(|(pt, text, sub, hot)| place(s, pt, text, sub, hot))
        .collect();
    if let Some(v) = place(
        s,
        scene::tile_point(you, 1),
        s.s("label.you"),
        String::new(),
        true,
    ) {
        labels.push(v);
    }
    labels
}

fn place(s: &Session, pt: V3, text: String, sub: String, hot: bool) -> Option<Value> {
    let m = scene::camera_for(s).matrix(geom_aspect(s));
    let (x, y) = geom::project(&m, pt)?;
    if !(0..=1000).contains(&x) || !(0..=1000).contains(&y) {
        return None;
    }
    Some(json!({"x": x, "y": y, "text": text, "sub": sub, "hot": hot}))
}

fn geom_aspect(s: &Session) -> i32 {
    (s.size.0 as i64 * crate::fx::ONE as i64 / s.size.1.max(1) as i64) as i32
}

fn result_panel(s: &Session, lines: &[String], next: &Next) -> Value {
    let label = match next {
        Next::Fight { .. } => s.s("result.fight"),
        Next::City(_) => s.s("result.city"),
        Next::Victory => s.s("result.victory"),
        Next::Board => s.s("result.continue"),
    };
    json!({"id": "result", "place": "center", "kind": "paper", "lines": lines, "actions": [action(label, "result.next", "", true)]})
}

fn story_panel(s: &Session, card: usize) -> Value {
    let k = &s.cards[card];
    let Some(st) = s.stories.get(&k.number) else {
        return json!(null);
    };
    let choices: Vec<Value> = st
        .choices
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let need = match &c.needs {
                None => String::new(),
                Some(crate::story::Need::Charisma(n)) => s.f("need.charisma", &[("n", n.to_string())]),
                Some(crate::story::Need::Style((g, n))) => s.f("need.style", &[("guild", g.clone()), ("n", n.to_string())]),
                Some(crate::story::Need::Gold(n)) => s.f("need.gold", &[("n", n.to_string())]),
                Some(crate::story::Need::Supplies(n)) => s.f("need.supplies", &[("n", n.to_string())]),
            };
            let gamble = c.outcome.fate.as_ref().map(|f| {
                s.f(
                    "need.fate",
                    &[("good", (f.faces[0] + f.faces[1]).to_string()), ("bad", (f.faces[2] + f.faces[3]).to_string())],
                )
            });
            let sub = [need, gamble.unwrap_or_default()].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
            json!({"key": (i + 1).to_string(), "text": c.text, "sub": sub, "enabled": s.can_choose(c), "act": "story.choose", "value": i.to_string()})
        })
        .collect();
    let tag = s.s(&format!("story.tag.{}", k.kind().name()));
    json!({"id": "story", "place": "center-right", "kind": "paper", "eyebrow": tag, "heading": st.title, "text": st.text, "choices": choices})
}

fn battle_panels(s: &Session, f: &crate::session::Fight) -> Vec<Value> {
    let ev = &f.record.events[f.shown];
    let snap = &ev.snap;
    let bank = |side: usize| -> Value { Value::Array(snap.bank[side].iter().map(|u| json!(u)).collect()) };
    let foe_name = s.cards[f.creature].name.clone();
    let mut panels = vec![json!({
        "id": "sides", "place": "tl", "heading": s.f("battle.turn", &[("n", ev.turn.div_ceil(2).to_string())]),
        "rows": [
            [s.s("battle.you"), s.f("battle.life", &[("n", snap.life[0].to_string())])],
            [foe_name, s.f("battle.life", &[("n", snap.life[1].to_string())])],
            [s.s("battle.cards"), s.f("battle.cards_n", &[("hand", snap.hand[1].to_string()), ("library", snap.library[1].to_string())])],
        ],
        "pips": [{"label": s.s("battle.your_mana"), "units": bank(0)}, {"label": s.s("battle.foe_mana"), "units": bank(1)}],
    })];
    let from = f.shown.saturating_sub(9);
    let log: Vec<Value> = f.record.events[from..=f.shown]
        .iter()
        .enumerate()
        .map(|(i, e)| json!({"text": s.f(e.key, &e.args.iter().map(|(k, v)| (*k, v.clone())).collect::<Vec<_>>()), "now": from + i == f.shown, "side": e.side}))
        .collect();
    panels.push(json!({"id": "log", "place": "tr", "heading": s.s("battle.log"), "log": log}));
    let hand: Vec<Value> = snap.player_hand.iter().map(|&c| face(s, c, "sm")).collect();
    panels.push(json!({"id": "hand", "place": "bottom", "cards": hand}));
    let done = f.shown + 1 >= f.record.events.len();
    let mut actions = vec![
        action(
            s.s(if f.paused { "battle.play" } else { "battle.pause" }),
            "battle.pause",
            "",
            !done,
        ),
        action(s.s("battle.step"), "battle.step", "", !done),
        action(
            s.f("battle.speed", &[("n", f.speed.to_string())]),
            "battle.speed",
            "",
            !done,
        ),
        action(s.s("battle.skip"), "battle.skip", "", !done),
    ];
    if done {
        let label = match f.record.outcome {
            Outcome::Won => s.s("battle.won"),
            Outcome::Lost => s.s("battle.lost"),
            Outcome::Withdrew => s.s("battle.withdrew_short"),
        };
        actions = vec![action(label, "battle.done", "", true)];
    }
    panels.push(json!({"id": "controls", "place": "br", "actions": actions}));
    panels
}

fn battle_labels(s: &Session, f: &crate::session::Fight) -> Vec<Value> {
    let ev = &f.record.events[f.shown];
    let mut out = Vec::new();
    for (pt, p) in scene::battle_spots(s, &ev.snap) {
        if !p.creature {
            continue;
        }
        let hot = ev.focus.contains(&p.id);
        let mut sub = format!("{}/{}", p.power, p.toughness - p.damage.min(p.toughness));
        if p.detained {
            sub = s.f("battle.detained", &[("pt", sub)]);
        }
        if let Some(v) = place(s, pt, p.name.clone(), sub, hot) {
            out.push(v);
        }
    }
    let _ = P;
    out
}

fn city_panels(s: &mut Session, city: crate::world::City, room: Room) -> Vec<Value> {
    let name = s.s(&format!("city.{}", city.id()));
    let back = action(s.s("city.back"), "city.room", "menu", true);
    match room {
        Room::Menu => {
            let rooms = ["armourer", "cards", "hall", "inn", "trainer"];
            let mut actions: Vec<Value> = rooms
                .iter()
                .enumerate()
                .map(|(i, r)| json!({"label": s.s(&format!("room.{r}")), "sub": s.s(&format!("room.{r}.sub")), "act": "city.room", "value": r, "enabled": true, "key": (i + 1).to_string()}))
                .collect();
            actions.push(action(s.s("hud.open_deck"), "deck.open", "", true));
            actions.push(action(s.s("city.leave"), "city.leave", "", true));
            vec![
                json!({"id": "city", "place": "bl", "kind": "menu", "heading": name, "text": s.s(&format!("city.{}.text", city.id())), "menu": actions}),
            ]
        }
        Room::Armourer => {
            let a = s.armoury.clone();
            let p = s.player().clone();
            let sold = encounter::shop_armour(&a, city);
            let mut rows = Vec::new();
            let mut ids: Vec<String> = sold.clone();
            for o in &p.owned {
                if !ids.contains(o) {
                    ids.push(o.clone());
                }
            }
            for id in ids {
                let Some(piece) = a.pieces.iter().find(|x| x.id == id) else {
                    continue;
                };
                let owned = p.owned.contains(&id);
                let worn = p.worn.get(&piece.slot) == Some(&id);
                let price = p.price(&a, piece.price);
                let status = if worn {
                    s.s("armour.worn")
                } else if owned {
                    s.s("armour.owned")
                } else {
                    s.f("armour.price", &[("n", price.to_string())])
                };
                rows.push(json!({"label": s.s(&format!("armour.{id}")), "sub": format!("{} · {}", s.s(&format!("slot.{}", piece.slot)), status),
                    "act": "armour.show", "value": id, "enabled": true, "hot": s.showing == id}));
            }
            let mut detail = json!(null);
            if let Some(piece) = a.pieces.iter().find(|x| x.id == s.showing) {
                let owned = p.owned.contains(&piece.id);
                let price = p.price(&a, piece.price);
                let mut rows2 = vec![
                    json!([
                        s.s("armour.health"),
                        s.f("armour.plus", &[("n", piece.health.to_string())])
                    ]),
                    json!([
                        s.s("hud.charisma"),
                        s.f("armour.plus", &[("n", piece.charisma.to_string())])
                    ]),
                    json!([
                        s.s("hud.style"),
                        s.f("armour.plus", &[("n", piece.style.to_string())])
                    ]),
                ];
                if !piece.effect.is_empty() {
                    rows2.push(json!([
                        s.s("armour.rule"),
                        s.s(&format!("effect.{}", piece.effect))
                    ]));
                }
                if piece.style_needed > 0 {
                    rows2.push(json!([
                        s.s("armour.needs"),
                        s.f(
                            "need.style",
                            &[
                                ("guild", piece.guild.clone()),
                                ("n", piece.style_needed.to_string())
                            ]
                        )
                    ]));
                }
                let worn = p.worn.get(&piece.slot) == Some(&piece.id);
                let act = if owned {
                    action(
                        s.s(if worn { "armour.worn" } else { "armour.wear" }),
                        "armour.wear",
                        &piece.id,
                        !worn,
                    )
                } else {
                    action(
                        s.f("armour.buy", &[("n", price.to_string())]),
                        "armour.buy",
                        &piece.id,
                        sold.contains(&piece.id),
                    )
                };
                detail = json!({"id": "piece", "place": "right", "heading": s.s(&format!("armour.{}", piece.id)),
                    "eyebrow": s.f("armour.eyebrow", &[("guild", if piece.guild.is_empty() { s.s("armour.plain") } else { piece.guild.clone() }), ("slot", s.s(&format!("slot.{}", piece.slot)))]),
                    "rows": rows2, "actions": [act]});
            }
            let gold = s.f("armour.gold", &[("n", p.gold.to_string())]);
            let mut out = vec![
                json!({"id": "armoury", "place": "side", "heading": s.s("room.armourer"), "text": gold, "list": rows, "actions": [back]}),
            ];
            if !detail.is_null() {
                out.push(detail);
            }
            out
        }
        Room::Cards => {
            let stock = s.stock_of(city);
            let p = s.player().clone();
            let buy: Vec<Value> = stock
                .iter()
                .map(|&c| {
                    let price = p.price(&s.armoury, encounter::card_price(&s.cards[c]));
                    let mut f = face(s, c, "sm");
                    f["act"] = json!("cards.buy");
                    f["value"] = json!(c.to_string());
                    f["badge"] = json!(s.f("cards.price", &[("n", price.to_string())]));
                    f
                })
                .collect();
            let mut owned: Vec<usize> = p.collection.clone();
            owned.sort_by_key(|&c| (s.cards[c].value, s.cards[c].name.clone()));
            owned.dedup();
            let sell: Vec<Value> = owned
                .iter()
                .map(|&c| {
                    let n = p.collection.iter().filter(|&&x| x == c).count();
                    json!({"label": s.f("cards.owned", &[("card", s.cards[c].name.clone()), ("n", n.to_string())]),
                        "sub": s.f("cards.sell", &[("n", (encounter::card_price(&s.cards[c]) / encounter::SELL_DIVISOR).to_string())]),
                        "act": "cards.sell", "value": c.to_string(), "enabled": true})
                })
                .collect();
            vec![
                json!({"id": "shop", "place": "top", "heading": s.s("room.cards"), "text": s.s("cards.text"), "cards": buy, "actions": [back]}),
                json!({"id": "sell", "place": "left", "heading": s.s("cards.sell_heading"), "list": sell}),
            ]
        }
        Room::Hall => {
            let mut rows: Vec<Value> = Vec::new();
            for g in Guild::ALL {
                rows.push(json!([
                    s.f("hall.standing", &[("guild", g.name().to_string())]),
                    s.player().style_with(&s.armoury, g.name()).to_string()
                ]));
            }
            let q = quests_panel(s);
            vec![
                json!({"id": "hall", "place": "left", "heading": s.s("room.hall"), "text": s.s(&format!("hall.{}", city.id())), "rows": rows, "actions": [back]}),
                json!({"id": "quests", "place": "right", "heading": q["heading"], "rows": q["rows"]}),
            ]
        }
        Room::Inn => {
            let p = s.player();
            let a = &s.armoury;
            vec![
                json!({"id": "inn", "place": "left", "heading": s.s("room.inn"), "text": s.s("inn.text"),
                "actions": [
                    action(s.f("inn.rest", &[("n", p.price(a, encounter::INN_REST).to_string())]), "inn.rest", "", p.health < p.max_health(a)),
                    action(s.f("inn.supplies", &[("n", p.price(a, encounter::INN_SUPPLIES_PRICE).to_string()), ("k", encounter::INN_SUPPLIES.to_string())]), "inn.supplies", "", p.supplies < p.max_supplies()),
                    back,
                ]}),
            ]
        }
        Room::Trainer => {
            let p = s.player();
            let a = &s.armoury;
            let list: Vec<Value> = ABILITIES
                .iter()
                .map(|(id, base, max)| {
                    let have = p.abilities.get(*id).copied().unwrap_or(0);
                    let price = p.price(a, base * (have as i32 + 1));
                    json!({"label": s.s(&format!("ability.{id}")), "sub": if have >= *max { s.s("ability.learned") } else { s.f("ability.price", &[("n", price.to_string())]) },
                        "act": "trainer.learn", "value": id, "enabled": have < *max})
                })
                .collect();
            vec![
                json!({"id": "trainer", "place": "left", "heading": s.s("room.trainer"), "text": s.s("trainer.text"), "list": list, "actions": [back]}),
            ]
        }
    }
}

fn deck_panels(s: &Session) -> Vec<Value> {
    let p = s.player();
    let mut owned: Vec<usize> = p.collection.clone();
    owned.sort_by_key(|&c| (s.cards[c].value, s.cards[c].name.clone()));
    owned.dedup();
    let cards: Vec<Value> = owned
        .iter()
        .map(|&c| {
            let have = p.collection.iter().filter(|&&x| x == c).count();
            let used = p.deck.iter().filter(|&&x| x == c).count();
            let mut f = face(s, c, "sm");
            f["badge"] = json!(s.f(
                "deck.count",
                &[("in", used.to_string()), ("of", have.to_string())]
            ));
            f["dim"] = json!(used == 0);
            f["actions"] = json!([
                action(
                    s.s("deck.add"),
                    "deck.add",
                    &c.to_string(),
                    used < have && p.deck.len() < player::DECK_MAX
                ),
                action(s.s("deck.remove"), "deck.remove", &c.to_string(), used > 0),
            ]);
            f
        })
        .collect();
    let mut curve = [0usize; 7];
    for &c in &p.deck {
        curve[s.cards[c].value.min(6)] += 1;
    }
    let colours: Vec<Value> = mana::COLOURS
        .iter()
        .map(|k| {
            json!([
                mana::letter_of(*k),
                p.deck.iter().filter(|&&c| s.cards[c].colours & k != 0).count()
            ])
        })
        .collect();
    vec![
        json!({"id": "deck", "place": "top", "heading": s.f("deck.heading", &[("n", p.deck.len().to_string()), ("max", player::DECK_MAX.to_string())]), "text": s.s("deck.text"), "cards": cards,
            "actions": [action(s.s("deck.close"), "deck.close", "", true)]}),
        json!({"id": "curve", "place": "br", "heading": s.s("deck.curve"), "bars": curve.iter().enumerate().map(|(i, n)| json!([if i == 6 { "6+".to_string() } else { i.to_string() }, n])).collect::<Vec<_>>(), "colours": colours}),
    ]
}

pub fn ui(s: &mut Session) -> Value {
    let screen = s.screen.clone();
    let mut panels: Vec<Value> = Vec::new();
    let mut labels: Vec<Value> = Vec::new();
    let name = match &screen {
        Screen::Title => {
            panels.push(json!({"id": "title", "place": "center", "kind": "title", "heading": s.s("title"), "text": s.s("title.tagline"),
                "actions": [action(s.s("title.new"), "game.new", "", true), action(s.s("title.continue"), "game.continue", "", s.saved.is_some())],
                "lines": [s.s("title.notice")]}));
            "title"
        }
        Screen::NewGame => {
            let choices: Vec<Value> = Guild::ALL
                .iter()
                .enumerate()
                .map(|(i, g)| json!({"key": (i + 1).to_string(), "text": s.s(&format!("guild.{}", g.name())), "sub": s.s(&format!("guild.{}.sub", g.name())), "enabled": true, "act": "game.guild", "value": g.name(), "guild": g.name()}))
                .collect();
            panels.push(json!({"id": "guilds", "place": "center", "kind": "paper", "heading": s.s("newgame.heading"), "text": s.s("newgame.text"), "choices": choices}));
            "newgame"
        }
        Screen::Board => {
            panels.push(hud(s));
            panels.push(quests_panel(s));
            panels.push(tile_panel(s));
            labels = labels_board(s);
            "board"
        }
        Screen::Draw { at, card } => {
            panels.push(hud(s));
            let k = &s.cards[*card];
            let story = s.stories.get(&k.number);
            let mut actions = Vec::new();
            match k.kind() {
                Kind::Creature => {
                    actions.push(action(s.s("draw.fight"), "draw.fight", "", true));
                    actions.push(action(
                        s.f("draw.flee", &[("n", crate::session::FLEE_SUPPLIES.to_string())]),
                        "draw.flee",
                        "",
                        true,
                    ));
                }
                Kind::Land => actions.push(action(s.s("draw.claim"), "draw.claim", "", true)),
                _ => actions.push(action(s.s("draw.read"), "draw.read", "", true)),
            }
            let text = match (k.kind(), story) {
                (Kind::Creature, Some(st)) => st.intro.clone(),
                (Kind::Land, Some(st)) => st.text.clone(),
                _ => String::new(),
            };
            panels.push(json!({"id": "drawn", "place": "center-left", "kind": "card", "cards": [face(s, *card, "lg")]}));
            let mut r = readings(s, *card, *at);
            r["text"] = json!(text);
            r["actions"] = json!(actions);
            panels.push(r);
            "draw"
        }
        Screen::Story { card, .. } => {
            panels.push(
                json!({"id": "drawn", "place": "left", "kind": "card", "cards": [face(s, *card, "lg")]}),
            );
            panels.push(story_panel(s, *card));
            "story"
        }
        Screen::Fate {
            card,
            faces,
            shuffled,
            picked,
            ..
        } => {
            panels.push(
                json!({"id": "drawn", "place": "left", "kind": "card", "cards": [face(s, *card, "md")]}),
            );
            let cards: Vec<Value> = faces
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let up = !shuffled || *picked == Some(i);
                    json!({"label": if up { s.s(f.key()) } else { String::new() }, "up": up, "good": f.good(), "act": "fate.pick", "value": i.to_string(), "enabled": *shuffled && picked.is_none()})
                })
                .collect();
            let (text, actions) = if *shuffled {
                (s.s("fate.pick_text"), vec![])
            } else {
                (
                    s.s("fate.shown_text"),
                    vec![action(s.s("fate.shuffle"), "fate.shuffle", "", true)],
                )
            };
            panels.push(json!({"id": "fate", "place": "center-right", "kind": "paper", "heading": s.s("fate.heading"), "text": text, "faces": cards, "actions": actions}));
            "fate"
        }
        Screen::Result { lines, next } => {
            if s.player.is_some() {
                panels.push(hud(s));
            }
            panels.push(result_panel(s, lines, next));
            "result"
        }
        Screen::Land { card, .. } => {
            panels.push(hud(s));
            let p = s.player().pos;
            let mut options: Vec<(i32, Pos)> = s
                .world()
                .tiles
                .iter()
                .filter(|(_, t)| t.visited && t.lands.len() < t.land_room())
                .map(|(q, _)| (crate::world::distance(*q, p), *q))
                .collect();
            options.sort();
            let list: Vec<Value> = options
                .iter()
                .take(8)
                .map(|(d, q)| {
                    let t = s.world().tile(*q).expect("tile");
                    json!({"label": s.f("land.option", &[("place", terrain_name(s, t.terrain)), ("x", q.0.to_string()), ("y", q.1.to_string())]),
                        "sub": s.f("land.steps", &[("n", d.to_string())]), "act": "land.place", "value": format!("{},{}", q.0, q.1), "enabled": true})
                })
                .collect();
            panels.push(json!({"id": "land", "place": "right", "heading": s.s("land.heading"), "text": s.f("land.text", &[("reach", s.player().reach(&s.armoury).to_string())]),
                "cards": [face(s, *card, "sm")], "list": list}));
            labels = labels_board(s);
            "land"
        }
        Screen::Battle(f) => {
            panels.extend(battle_panels(s, f));
            labels = battle_labels(s, f);
            "battle"
        }
        Screen::City { city, room } => {
            if *room != Room::Armourer {
                panels.push(hud(s));
            }
            panels.extend(city_panels(s, *city, *room));
            if *room == Room::Menu {
                labels = scene::city_doors(s)
                    .into_iter()
                    .filter_map(|(pt, key)| place(s, pt, s.s(&format!("room.{key}")), String::new(), false))
                    .collect();
            }
            if *room == Room::Armourer {
                "armourer"
            } else {
                "city"
            }
        }
        Screen::Deck { .. } => {
            panels.extend(deck_panels(s));
            "deck"
        }
        Screen::Lair => {
            panels.push(hud(s));
            let niv = s
                .cards
                .iter()
                .position(|c| c.name == "Niv-Mizzet, Dracogenius")
                .unwrap_or(0);
            let intro = s
                .stories
                .get(&s.cards[niv].number)
                .map(|st| st.intro.clone())
                .unwrap_or_default();
            let open = s.player().quests.get("dracogenius").copied().unwrap_or(0) >= 2;
            let lands = s
                .world()
                .lands_in_reach(s.player().pos, s.player().reach(&s.armoury))
                .len();
            panels
                .push(json!({"id": "drawn", "place": "left", "kind": "card", "cards": [face(s, niv, "lg")]}));
            panels.push(json!({"id": "lair", "place": "center-right", "kind": "paper", "eyebrow": s.s("lair.eyebrow"), "heading": s.s("tile.lair"),
                "text": if open { intro } else { s.s("lair.sealed") },
                "lines": [s.f("lair.odds", &[("life", encounter::GUARDIAN_LIFE.to_string()), ("deck", encounter::GUARDIAN_DECK.to_string()), ("lands", lands.to_string())])],
                "actions": [action(s.s("lair.climb"), "lair.climb", "", open), action(s.s("lair.leave"), "lair.leave", "", true)]}));
            "lair"
        }
        Screen::Victory => {
            panels.push(json!({"id": "victory", "place": "center", "kind": "title", "heading": s.s("victory.heading"), "text": s.s("victory.text"),
                "actions": [action(s.s("victory.continue"), "victory.continue", "", true)]}));
            "victory"
        }
        Screen::Models { page } => {
            let names = scene::model_page(s, *page);
            labels = scene::model_spots(s, *page)
                .into_iter()
                .filter_map(|(pt, n)| place(s, pt, n, String::new(), false))
                .collect();
            let pages = scene::model_pages(s);
            panels.push(json!({"id": "models", "place": "br", "heading": s.f("models.heading", &[("n", (page + 1).to_string()), ("of", pages.to_string())]),
                "actions": [action(s.s("models.prev"), "models.page", &page.saturating_sub(1).to_string(), *page > 0),
                            action(s.s("models.next"), "models.page", &(page + 1).to_string(), page + 1 < pages)],
                "lines": [names.join(", ")]}));
            "models"
        }
    };
    let hints: Vec<Value> = s
        .hints
        .iter()
        .map(|(t, d)| json!({"term": t, "definition": d}))
        .collect();
    json!({
        "screen": name,
        "title": s.s("title"),
        "message": s.message,
        "panels": panels.into_iter().filter(|p| !p.is_null()).collect::<Vec<_>>(),
        "labels": labels,
        "hints": hints,
        "keys": s.s(&format!("keys.{name}")),
        "version": s.ui_version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_text_splits_into_symbols_and_reminder_text() {
        let v = segments("Flying\n{U}{R}: Niv-Mizzet deals 1 damage to any target.");
        assert_eq!(v[0][0]["t"], "Flying");
        assert_eq!(v[1][0]["s"], "U");
        assert_eq!(v[1][1]["s"], "R");
        assert_eq!(v[1][2]["t"], ": Niv-Mizzet deals 1 damage to any target.");
        let r = segments("Haste (This creature can attack.)");
        assert_eq!(r[0][1]["i"], "(This creature can attack.)");
    }

    #[test]
    fn a_face_keeps_the_card_text_verbatim() {
        let s = crate::session::for_tests();
        let niv = s
            .cards
            .iter()
            .position(|c| c.name == "Niv-Mizzet, Dracogenius")
            .unwrap();
        let f = face(&s, niv, "lg");
        let mut text = String::new();
        for line in f["lines"].as_array().unwrap() {
            for seg in line.as_array().unwrap() {
                let piece = seg
                    .get("t")
                    .or(seg.get("i"))
                    .and_then(|x| x.as_str())
                    .map(|x| x.to_string());
                text.push_str(&piece.unwrap_or_else(|| format!("{{{}}}", seg["s"].as_str().unwrap())));
            }
            text.push('\n');
        }
        assert_eq!(text.trim_end(), s.cards[niv].text);
        assert_eq!(f["frame"], "gold");
        assert_eq!(f["art"], "art/cards/183.webp");
    }

    #[test]
    fn every_screen_describes_itself_without_a_missing_string() {
        let mut s = crate::session::for_tests();
        // a missing string comes back as its own key; keys hold no spaces and end in a word
        let check = |s: &mut Session| {
            let v = ui(s);
            let mut texts = Vec::new();
            fn walk(v: &Value, out: &mut Vec<String>) {
                match v {
                    Value::String(t) => out.push(t.clone()),
                    Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
                    Value::Object(o) => o
                        .iter()
                        .filter(|(k, _)| {
                            !matches!(
                                k.as_str(),
                                "act"
                                    | "value"
                                    | "art"
                                    | "id"
                                    | "place"
                                    | "kind"
                                    | "screen"
                                    | "size"
                                    | "frame"
                                    | "guild"
                                    | "number"
                            )
                        })
                        .for_each(|(_, x)| walk(x, out)),
                    _ => {}
                }
            }
            walk(&v, &mut texts);
            let missing: Vec<&String> = texts
                .iter()
                .filter(|t| {
                    s.strings.contains_key(t.as_str())
                        || (t.contains('.')
                            && !t.contains(' ')
                            && t.chars().all(|c| c.is_ascii_lowercase() || c == '.' || c == '_'))
                })
                .collect();
            assert!(missing.is_empty(), "{missing:?}");
        };
        check(&mut s);
        s.act("game.new", "").unwrap();
        check(&mut s);
        s.act("game.guild", "Golgari").unwrap();
        check(&mut s);
        s.act("result.next", "").unwrap();
        for room in ["armourer", "cards", "hall", "inn", "trainer", "menu"] {
            s.act("city.room", room).unwrap();
            check(&mut s);
        }
        s.act("city.leave", "").unwrap();
        check(&mut s);
        s.act("deck.open", "").unwrap();
        check(&mut s);
    }
}
