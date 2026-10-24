# Handoff

Stage 9, written by the agent. `C5 Consolidate in your own words`. A fresh session starts from this file, not from memory.

## What this is

Tenth District (a working title): a Hand of Fate–style deck-building adventure on Ravnica's grid with 108 Return to Ravnica cards, for the browser. This MVP is the first region, from a new game in the Tenth District to the fight with Niv-Mizzet in Nivix. It is built and checked on this machine and is not published.

## Rules that hold it up

The rules in `CLAUDE.md` under "Rules agreed for this project" (MTG-1 to MTG-11). The ones that break silently: card text verbatim (`a_face_keeps_the_card_text_verbatim`), a fight from its seed (`a_fight_replays_the_same_from_its_seed`), every card with rules (`a_card_without_rules_is_refused_by_name`), no reference art in the repository (`tools/cardart/render.py --check`).

## Shape of the code

| Where | What |
| --- | --- |
| `crates/core/src/rules.rs` | what each card does in a fight |
| `crates/core/src/battle.rs` | the auto-battle, recorded as events for playback |
| `crates/core/src/world.rs` | chunks, terrain, pools, growth |
| `crates/core/src/session.rs` | the state of play, `act()`, saving |
| `crates/core/src/view.rs`, `scene.rs` | the panels and the draw list |
| `data/` | cards, stories, quests, armour, strings, models, controls |
| `art/cards/` | the TikZ pictures (source in `tools/cardart/src/`) |
| `tools/models/parts/` | the models' sources |
| `web/` | the page: `app.js`, `ui.js`, `gl.js`, `store.js` |

## Commands

- `./scripts/check.sh`: everything; `BUILDER_ENGINES=chromium` for one engine.
- `python3 scripts/break_tests.py`: 16 breaks, all caught.
- `cargo test -p tool_core --release --test balance -- --nocapture`: the balance rates.
- `python3 tools/models/build.py`, `python3 tools/cardart/render.py [numbers]`: rebuild models and art.
- `.venv/bin/python tools/shots.py OUT steps.json "?seed=1"`: screenshots of the running build.
- `python3 scripts/serve.py` after `./scripts/build.sh`: play at http://127.0.0.1:8000 (add `?seed=1` for a fixed world).

## What will bite within the hour

- `cargo fmt` rewraps long lines; a scripted edit written against the old text will not match.
- The page may hold no number but 0 and 1: sizes and positions come from the core (labels in thousandths of the view, the canvas size from `pixels()`).
- The bot in `playthrough.rs` reaches Niv-Mizzet but does not beat him (R2.14).

## The journal

`../mtg-hand-of-fate-journal/index.html`, published privately at https://claude.ai/artifact/4PQW6KtNaSEEYUsNVxXayp. Newest entry: the first region, built.

## Open for the person

- R1.1: whether and how the game may use Wizards' card text and art in public (MTG-Q1).
- R1.2: the game's public name (MTG-Q2).
- R1.3: the rules of the childhood grid game (MTG-Q3).
- R1.10: read the story text and labels the agent wrote for the mockup.
- R2.10: read the cards whose text this game can only partly honour (Street Sweeper, Ash Zealot, the keyrunes, Faerie Impostor).
- R2.11: the working title "Tenth District" and the Fan Content notice on the title screen.
- R2.12: read the agent-written strings and the 108 stories.
- R2.13: decide whether the weaker TikZ pictures get a second pass (MTG-Q12).

## Mistakes so far

- Balance: starting decks of cheapest commons lost to tier-two foes; foes had too much mana; decking punished twelve-card decks (R2.5–R2.7). Each now has a balance test.
- Tests: two tests passed with their behaviour broken (R2.15); both strengthened, and `tests/breaks.json` holds 16 rows.
- Layout: room panels overlapped; fixed by a side list and a main panel.

## What passed

2026-10-24: `./scripts/check.sh` ended `BROWSER CHECK PASSED in chromium, firefox, webkit` and `BUILDER CHECK PASSED`; `./scripts/count-tests.sh`: 116 passed, 0 failed; `scripts/break_tests.py`: 16 of 16 caught.

## The next action

The person plays the first region and answers MTG-Q1, Q2, Q3 and Q5; their requests become `TRIAGE.md` rows.
