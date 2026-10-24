# Handoff

Stage 9, written by the agent. `C5 Consolidate in your own words`. A fresh session starts from this file, not from memory.

## What this is

Tenth District (a working title): a Hand of Fate–style deck-building adventure on Ravnica's grid with 108 Return to Ravnica cards, for the browser. This MVP is the first region, from a new game in the Tenth District to the fight with Niv-Mizzet in Nivix. Since MVP 2 (TRIAGE T1) every fight is a game of Magic the player plays against a computer opponent, with a land pile instead of lands in the deck. MVP 1 is public at https://sgilson7.github.io/mtg-hand-of-fate/, source at https://github.com/sgilson7/mtg-hand-of-fate. MVP 2 is committed locally and not pushed or published.

## Rules that hold it up

The rules in `CLAUDE.md` under "Rules agreed for this project" (MTG-1 to MTG-11). The ones that break silently: card text verbatim (`a_face_keeps_the_card_text_verbatim`), a fight from its seed (`a_fight_replays_the_same_from_its_seed`), every card with rules (`a_card_without_rules_is_refused_by_name`), no reference art in the repository (`tools/cardart/render.py --check`).

## Shape of the code

| Where | What |
| --- | --- |
| `crates/core/src/rules.rs` | what each card does in a game of Magic, as data |
| `crates/core/src/duel.rs` | the rules engine: turns, priority, the stack, combat, state-based actions; every decision is a `Pending` ask |
| `crates/core/src/duel_ai.rs` | the computer's answers to those asks |
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
- The bot in `playthrough.rs` reaches Niv-Mizzet but does not beat him (R2.14); the computer plays its fights for it (`duel.human = [false, false]`).
- A fight waits on the player: `Session::act("duel.*")` answers the pending ask. Tests that only need a result set both sides to the computer and call `run()`.
- `cargo fmt` rewraps code that `tests/breaks.json` finds; run `python3 scripts/break_tests.py` after formatting.

## The journal

`../mtg-hand-of-fate-journal/index.html`, published privately at https://claude.ai/artifact/4PQW6KtNaSEEYUsNVxXayp. Newest entry: the first region, built.

## Open for the person

- R1.1: whether and how the game may use Wizards' card text and art in public (MTG-Q1). The person asked for a public page; the Fan Content notice is the only safeguard.
- R1.2: the game's public name (MTG-Q2).
- R1.3: the rules of the childhood grid game (MTG-Q3).
- R1.10: read the story text and labels the agent wrote for the mockup.
- R2.10: read the cards whose text this game can only partly honour (Street Sweeper, Ash Zealot, the keyrunes, Faerie Impostor).
- R2.11: the working title "Tenth District" and the Fan Content notice on the title screen.
- R2.12: read the agent-written strings and the 108 stories.
- R2.13: decide whether the weaker TikZ pictures get a second pass (MTG-Q12).
- R2.17: set `git config user.name` (and email) so `scripts/publish.sh` can commit the page.
- R3.5: the four choices the game makes itself.
- R4.1: half your life, rounded up, for an empty library.
- R4.2: six as both the opening hand and the cleanup limit.
- R4.3: what "the 3d summoning terrain should still work" meant.
- R3.10: armour and ability rules carried over to Magic.
- R3.12: read the fight's new strings.
- Whether to publish MVP 2: the grant stands, but this request did not ask for a deploy.
- R2.16: the repository's name, `mtg-hand-of-fate`, chosen by the agent from the project folder.

## Mistakes so far

- Balance: starting decks of cheapest commons lost to tier-two foes; foes had too much mana; decking punished twelve-card decks (R2.5–R2.7). Each now has a balance test.
- Tests: two tests passed with their behaviour broken (R2.15); both strengthened, and `tests/breaks.json` holds 16 rows.
- Layout: room panels overlapped; fixed by a side list and a main panel.

## What passed

2026-10-24: `./scripts/check.sh` ended `BROWSER CHECK PASSED in chromium, firefox, webkit` and `BUILDER CHECK PASSED`; `./scripts/count-tests.sh`: 116 passed, 0 failed; `scripts/break_tests.py`: 16 of 16 caught.

2026-10-24, deploy: `./scripts/check.sh` passed again (`BUILDER CHECK PASSED`); `dist/` built from b39eda7 (build 86992f96dcd0) pushed to gh-pages; `BUILDER_ORIGIN=https://sgilson7.github.io/mtg-hand-of-fate python3 tests/browser/check.py` ended `BROWSER CHECK PASSED in chromium, firefox, webkit`.

2026-10-24, MVP 2: `./scripts/check.sh` ended `BROWSER CHECK PASSED in chromium, firefox, webkit` and `BUILDER CHECK PASSED`; `./scripts/count-tests.sh`: 133 passed, 0 failed; `scripts/break_tests.py`: 34 of 34 caught.

2026-10-24, T2 to T7: `./scripts/check.sh` ended `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `./scripts/count-tests.sh`: 135 passed, 0 failed; `scripts/break_tests.py`: 36 of 36 caught.

## The next action

The person plays MVP 2's fights, says whether to publish it, and answers MTG-Q2 and Q3; their requests become `TRIAGE.md` rows.
