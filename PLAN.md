# Plan

Stage 2, written by the agent from `BRIEF.md`. `B2 Require step-by-step justification`.

**Status: draft for the person's review with the mockup.** The person asked to review a mockup before the real game is built, so nothing below is started. The numbers in it are placeholders (R1.9) until the questions in the journal (MTG-Q1 to MTG-Q13) are answered.

| # | Decision | Reason | Test that shows it holds |
| --- | --- | --- | --- |
| 1 | Copy `kits/canvas/` (draw list) and the seeded `rng.rs` and `fx.rs` from `kits/realtime/`; no other kit yet. | The board, battle and city are drawn from a draw list the core builds; chance and geometry must be deterministic. | The kits' own tests, each broken once in `tests/breaks.json`. |
| 2 | `map.rs`: the world is integer grid cells. The first 5×5 is centred on a city at (0, 0); stepping onto an edge cell adds the 5×5 beyond it, generated from the seed and the chunk's coordinates. | The description: "you start at the center … if you get to an edge, you add an additional 5x5". A chunk depends only on seed and position, so the same seed grows the same world in any order. | A chunk generated before and after its neighbours is identical; stepping onto an edge adds exactly one chunk; the first chunk's centre is a city. |
| 3 | `terrain.rs`: five basic terrains (Plains, Island, Swamp, Mountain, Forest) carry the five colours; guild districts appear where a guild's two terrains touch; peaks and deep canals block movement. | Placement themes each tile; Ravnica's guilds are colour pairs. | Every district borders both of its guild's terrains; no generated chunk cuts the player off from its edges. |
| 4 | `pool.rs`: each card tile's pool is drawn from `data/cards.json` by its terrain, its neighbours and the distance from the start. | Placement decides the mix; farther tiles are harder. | Pools only hold cards of the tile's colours (or colourless); mean mana value rises with distance on a fixed seed. |
| 5 | `encounter.rs`: the drawn card's type routes the encounter (creature → battle; enchantment or artifact → story; instant or sorcery → magic story; land → attach), with mana value and abilities as the tier and twist (MTG-Q4). | The description's encounter table. | One test per type; a land never starts a battle. |
| 6 | `battle.rs`: two decks, turn by turn as in the battle-turn diagram; mana per turn from lands within Reach of the fight tile; unspent mana banks; casts by a fixed priority; abilities last in the chain. The whole fight is decided from its seed and recorded as events for the page to play back (Graftwood's pattern). | "Pretty much Magic" with map mana; a recorded fight replays and can be stepped. | Hand-worked fights for each guild mechanic (detain, overload, unleash, scavenge, populate); a replay hash; banking carries mana across turns. |
| 7 | `rules/`: one module per supported card behaviour, starting with keywords (flying, haste, trample …) and the five guild mechanics, then card-specific text. A card whose text no module covers is held out of pools and listed. | 254 cards cannot all be hand-ruled on day one; a card that does nothing is a silent bug. | A test lists every card and fails on one in a pool without a rule. |
| 8 | `story.rs` and `quest.rs`: story cards with two or three choices, fate draws (four faces, shuffled), quest steps that span districts. Story text in `data/stories.json`. | Hand of Fate's encounter cards and chains. | A quest started in one district advances only in the one it names; the fate draw's faces are what the card shows. |
| 9 | `deck.rs`, `shop.rs`, `stats.rs`: a 20-card deck from the collection; armour slots give max health and the look stats (Charisma, Style); prices in cities. | The description's deck, armour and city. | Deck size is enforced; health is the sum of armour; a price falls with Charisma by the rule's amount. |
| 10 | `guardian.rs`: each region's guardian has its own 40-card deck that grows with the player's collection. | "Defeating the local guardian, who also has a sophisticated deck"; Hand of Fate's dealer grows his deck with yours. | The guardian's deck is legal, and grows by the stated rule. |
| 11 | Page: hand-written WebGL 2 replaying the draw list (after Mage Tower Battle's `gl.js`), instanced low-poly models built in the core, card faces as DOM so text stays real. | Builder D3, and browser speed. | Browser check in three engines; a frame-time probe on the board. |
| 12 | Art: one TikZ picture per card in `tools/art/art_cards.py`, by guild, first-region cards first; PNG converted to WebP and loaded when a card is shown. Reference art never committed. | "The art should be also verbatim just recreated"; load size. | `art.py --check-only`; a size cap per picture; a check that no reference image is in the repository. |
| 13 | Load budget: first load at most 2.5 MB over the wire before card art. | Mage Tower Battle and PotS load about 2 MB; the brief stresses browser speed. | A build step that fails over budget. |

## Open questions for the person

The questions are in the journal's opening entry (MTG-Q1 to MTG-Q5) and its *for you* entry (MTG-Q6 to MTG-Q13), each with the default this plan is built on.

## Status

- [ ] Read by the person on:
