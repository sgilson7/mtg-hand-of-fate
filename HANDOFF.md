# Handoff

Stage 9, written by the agent. `C5 Consolidate in your own words`. A fresh session starts from this file, not from memory.

## What this is

Ravnica Digest (named by the person in T31; Tenth District until then): a Hand of Fate–style deck-building adventure on Ravnica's grid with 108 Return to Ravnica cards, for the browser. This MVP is the first region, from a new game in the Tenth District to the fight with Niv-Mizzet in Nivix; since T10 the pool is all of Return to Ravnica (254 cards, the first 108 first). Since MVP 2 (TRIAGE T1) every fight is a game of Magic the player plays against a computer opponent, with a land pile instead of lands in the deck. Since T32 a new game is one of two missions: the Dracogenius, or the Riot, where Rakdos razes his way across the map to New Prahv and beating him is graded by the steps taken (`riot.rs`). It is public at https://sgilson7.github.io/mtg-hand-of-fate/ (built from 7164ccf, T29 to T34), source at https://github.com/sgilson7/mtg-hand-of-fate.

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

- R9.1 to R9.3: the broken outlines, the rings inside tree crowns, and the card being read covering the left panel's buttons in a fight, left as they are in T36, since the person chose no style changes.
- R1.1: whether and how the game may use Wizards' card text and art in public (MTG-Q1). The person asked for a public page; the Fan Content notice is the only safeguard.
- R1.3: the rules of the childhood grid game (MTG-Q3).
- R1.10: read the story text and labels the agent wrote for the mockup.
- R2.10: read the cards whose text this game can only partly honour (Street Sweeper, Ash Zealot, the keyrunes, Faerie Impostor).
- R2.11: the Fan Content notice on the title screen (the title is now Ravnica Digest).
- R2.12: read the agent-written strings and the 108 stories.
- R2.13: decide whether the weaker TikZ pictures get a second pass (MTG-Q12).
- R2.17: set `git config user.name` (and email) so `scripts/publish.sh` can commit the page.
- R3.5: the four choices the game makes itself.
- R4.1: half your life, rounded up, for an empty library.
- R4.2: six as both the opening hand and the cleanup limit.
- R4.3: what "the 3d summoning terrain should still work" meant.
- R3.10: armour and ability rules carried over to Magic.
- R3.12: read the fight's new strings.
- R2.16: the repository's name, `mtg-hand-of-fate`, chosen by the agent from the project folder.
- R5.1: the choices the new cards make for both sides (Tablet's colours, Jace's piles, free casts).
- R5.2: Seek the Horizon and Gatecreeper Vine fill the land pile instead of searching.
- R5.3: a planeswalker met on the map is a fight.
- R5.7: idle enchantments and artifacts on the table carry no label; hover to read one.
- R5.10: read the 52 new strings and the 146 new stories.
- R6.1: log card names show their card on hover, not whole cards in the log.
- R6.3: the bands of difficulty and what each holds.
- R6.4: the ten new quests, their cards and rewards.
- R6.6: a health bar's full length is the most life the side has had.
- R6.8: read the new strings.
- R7.2: the second look of each terrain, and one tile in three wearing it.
- R7.3: the thirty starting decks, the real decks they follow, and Selesnya's fifth following a green and white deck that is not Selesnya's.
- R7.5: supplies at six for 2 gold.
- R7.6: Rix Maadi and Svogthos: where they stand, their gear, their quests.
- R7.10: read the new strings.
- R8.1: the Riot's shape: a mission chosen before the guild, Rakdos's road through six chunks, one move in three steps, New Prahv falling.
- R8.2: grades by steps taken: A to 70, B to 100, C to 130, D beyond.
- R8.3: razed tiles and minions: 18 life and 7 lands, black or red creatures of 3 to 5.
- R8.5: the Calliope and the Lyev Column, where they stand and what their quests do to Rakdos.
- R8.7: which places a quest's grey tile marks.
- R8.9: the rename touches the title, tab and notice; the address, repository and save key stay.
- R8.10: read the new strings.
- R8.11: the website tile: its still, its loop and its page.

## Mistakes so far

- Balance: starting decks of cheapest commons lost to tier-two foes; foes had too much mana; decking punished twelve-card decks (R2.5–R2.7). Each now has a balance test.
- Tests: two tests passed with their behaviour broken (R2.15); both strengthened, and `tests/breaks.json` holds 16 rows.
- Layout: room panels overlapped; fixed by a side list and a main panel.

## What passed

2026-10-24: `./scripts/check.sh` ended `BROWSER CHECK PASSED in chromium, firefox, webkit` and `BUILDER CHECK PASSED`; `./scripts/count-tests.sh`: 116 passed, 0 failed; `scripts/break_tests.py`: 16 of 16 caught.

2026-10-24, deploy: `./scripts/check.sh` passed again (`BUILDER CHECK PASSED`); `dist/` built from b39eda7 (build 86992f96dcd0) pushed to gh-pages; `BUILDER_ORIGIN=https://sgilson7.github.io/mtg-hand-of-fate python3 tests/browser/check.py` ended `BROWSER CHECK PASSED in chromium, firefox, webkit`.

2026-10-24, MVP 2: `./scripts/check.sh` ended `BROWSER CHECK PASSED in chromium, firefox, webkit` and `BUILDER CHECK PASSED`; `./scripts/count-tests.sh`: 133 passed, 0 failed; `scripts/break_tests.py`: 34 of 34 caught.

2026-10-24, T2 to T7: `./scripts/check.sh` ended `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `./scripts/count-tests.sh`: 135 passed, 0 failed; `scripts/break_tests.py`: 36 of 36 caught.

2026-10-24, MVP 2 deploy: `BUILDER_ORIGIN=https://sgilson7.github.io/mtg-hand-of-fate python3 tests/browser/check.py` ended `BROWSER CHECK PASSED in chromium, firefox, webkit`; the live `build.txt` reads `e8d5a5f9f04d e3c6958`.

2026-10-24, T8: `./scripts/check.sh` ended `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); 136 tests passed; 37 of 37 breaks caught.

2026-10-24, T9 to T13: `./scripts/check.sh` ended `BROWSER CHECK PASSED in chromium, firefox, webkit` and `BUILDER CHECK PASSED`; `./scripts/count-tests.sh`: 140 passed, 0 failed; `scripts/break_tests.py`: 43 of 43 caught; `tools/cardart/render.py --check`: card art ok (254 pictures). A live play-through of the new artifacts and enchantments (Playwright, build from the working tree) showed no page errors.

2026-10-24, T9 to T13 deploy: `./scripts/check.sh` passed on the clean tree at c7549cd (`BUILDER CHECK PASSED`); `dist/` pushed to gh-pages by hand, as `scripts/publish.sh` does, since it still stops on the missing `git config user.name` (R2.17); the live `build.txt` reads `43c0336dd629 c7549cd`; `BUILDER_ORIGIN=https://sgilson7.github.io/mtg-hand-of-fate .venv/bin/python tests/browser/check.py` ended `BROWSER CHECK PASSED in chromium, firefox, webkit`.

2026-10-24, T14 to T22: `./scripts/check.sh` ended `BROWSER CHECK PASSED in chromium, firefox, webkit` and `BUILDER CHECK PASSED`; `./scripts/count-tests.sh`: 146 passed, 0 failed; `scripts/break_tests.py`: 54 of 54 caught. A live play-through (Playwright) showed the stack, log and land hovers, health bars and deck sorting with no page errors.

2026-10-24, T14 to T22 deploy: `dist/` built from 4a08d8f pushed to gh-pages by hand (R2.17); the live `build.txt` reads `6d1f4375d1dd 4a08d8f`; the browser check against the live address ended `BROWSER CHECK PASSED in chromium, firefox, webkit`.

2026-10-24, T23 to T28: `./scripts/check.sh` ended `BROWSER CHECK PASSED in chromium, firefox, webkit` and `BUILDER CHECK PASSED`; `./scripts/count-tests.sh`: 154 passed, 0 failed; `scripts/break_tests.py`: 68 of 68 caught. The browser check now chooses Ral's deck before the opening, and waits for the deck panel to redraw before reading it. A live look (Playwright, build from the working tree) showed the deck window, the deck choice, the second terrain looks, Rix Maadi's hall and armourer, and the whole log, with no page errors.

2026-10-24, T23 to T28 deploy: main pushed; `dist/` built from b420f5e committed onto gh-pages by hand (0179c59, an ordinary push, R2.17); the live `build.txt` reads `8107cb816ced b420f5e`; the browser check against the live address ended `BROWSER CHECK PASSED in chromium, firefox, webkit`.

2026-10-24, T29 to T34: `./scripts/check.sh` ended `BROWSER CHECK PASSED in chromium, firefox, webkit` and `BUILDER CHECK PASSED`; `./scripts/count-tests.sh`: 173 passed, 0 failed; `scripts/break_tests.py`: 95 of 95 caught. The browser check now opens a Riot game and finds Rakdos marked five steps away. A live look (Playwright, build from the working tree) showed the mission choice, New Prahv's grey tile, the Riot on the map with razed tiles and minions, facing and fighting Rakdos, with no page errors.

2026-10-24, T29 to T34 deploy: main pushed; `dist/` built from 7164ccf committed onto gh-pages by hand (0f94374, an ordinary push, R2.17); the live `build.txt` reads `b4669bd9210d 7164ccf`; the browser check against the live address ended `BROWSER CHECK PASSED in chromium, firefox, webkit`. The website's tile and page for the game (T29, T30) are in the website's own repository and deploy.

2026-10-25, T35 and T36: `./scripts/check.sh` ended `BROWSER CHECK PASSED in chromium, firefox, webkit` and `BUILDER CHECK PASSED`; `./scripts/count-tests.sh`: 176 passed, 0 failed; `scripts/break_tests.py`: 104 of 104 caught (one break, the second main phase, was first missed because the test worked its expectation out with the function under test; the test now states it by hand). A live look (Playwright, build from the working tree) showed the glow on castable cards, a cast card flying to the stack, the stack's arc to its target, the turn strip, the main button and a damage number rising, with no page errors. A dying or entering creature was not caught on a screenshot; its timing is tested in the core.

## The next action

The person plays a fight on the live page to judge T36's Arena changes; their requests become `TRIAGE.md` rows.

The person plays MVP 2's fights on the live page and answers MTG-Q2 and Q3; their requests become `TRIAGE.md` rows.
