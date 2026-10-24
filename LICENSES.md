# Licences

Every file in this repository that is not code written for it: its source, its author, its terms. The code is under the repository's licence.

## Assets

| Path | Source | Terms |
| --- | --- | --- |
| `data/cards.json` | MTGJSON 5.3.0 (Return to Ravnica), trimmed to 108 cards | the data: MIT, © Zach Halpern. The card names, text and flavour text are Wizards of the Coast's, shown under its Fan Content Policy (SECOND-ORDER R1.1, MTG-Q1) |
| `art/cards/` | drawn as TikZ in `tools/cardart/src/` by this project's agents, recreating the composition of each card's original art | the drawings: this repository's licence. The compositions they recreate belong to each card's artist and Wizards of the Coast (MTG-Q1). No original art is kept in the repository (`tools/cardart/render.py --check`) |
| `data/models.json` | built by `tools/models/build.py` from `tools/models/parts/` | this repository's licence |
| `tools/models/parts/scenery.py` (`chum_*`) | ported from CHUMWORLD `chumworld-towns/crates/core/src/models/props.rs` at 2317c3b (Sam Gilson) | MIT |
| `data/stories/` | written for this game by its agents | this repository's licence |

## Vendored code

| Path | Project and version | Author | Licence | Checksum |
| --- | --- | --- | --- | --- |

## Code taken from other repositories

| From | What | Licence |
| --- | --- | --- |
| (none) | The Magic rules engine (`duel.rs`, `duel_ai.rs`, `rules.rs`) is written for this game from the Comprehensive Rules and the cards' Oracle text. No code or card script was copied from Forge (GPL-3.0), XMage (MIT), phase.rs or any other engine; the journal's Magic engines tab lists what was read | — |
| Mage Tower Battle at 557369b (Sam Gilson) | `crates/core/src/{fx,geom,mesh,render,rng}.rs`, `web/gl.js`, `web/store.js`, `data/shaders/scene.*`, `crates/core/tests/no_storage.rs`; changed here: an ink-outline pass in `render.rs`, the shaders and `gl.js` | MIT |

A file with no licence, a likeness, a recording or any third-party picture is a worklist row in `SECOND-ORDER.md` for the person to decide, and stays out of git until they do.
