# Plan

Stage 2, written by the agent from `BRIEF.md`. `B2 Require step-by-step justification`.

**Status: approved by the person on 2026-10-24 ("build the mvp of the first region up to the niv mizzet fight", DECISIONS.md) and built.** The table is the plan as built, in build order; where the build departed from the draft, the notebook row says why.

| # | Decision | Reason | Test that shows it holds |
| --- | --- | --- | --- |
| 1 | Mage Tower Battle's renderer stack (Q16.16 maths, unit meshes, instanced WebGL 2, uniform-only shaders, seeded chance) with an ink-outline pass added. | Browser speed and Protection of the Spheres' look; the person's own MIT code. | `render.rs` tests; boundary scans; browser check |
| 2 | The region: 3×3 chunks of 5×5 (15×15 tiles). The first chunk is centred on the Tenth District; stepping onto an edge adds the chunk beyond it; nothing past the region until the guardian falls. | "if you get to an edge, you add an additional 5x5"; a region small enough to finish in one sitting. | `world.rs`: growth, region edge, order independence, borders never blocked |
| 3 | Guild quarters: N Azorius (New Prahv), NE Izzet (Nivix, the lair), E Izzet, SE and S Rakdos, SW Golgari, W Selesnya (Vitu-Ghazi), NW Selesnya, centre mixed. Terrain is mostly the quarter's two colours; two districts a quarter chunk; two obstacles inside each chunk. | Placement themes each tile; the colours fit as terrain (Plains white, Island blue, Swamp black, Mountain red, Forest green). | `world.rs` pool test |
| 4 | Pools of four from the tile's colours (or its district's guild); mana value 3 and common or uncommon in the first chunk, 5 further out, anything in the lair's chunk. | Farther is harder. | `world.rs` pool test |
| 5 | A card's three readings: type picks the encounter, mana value the foe's tier (life 6/9/13, one mana a tier, a 12-card deck), abilities its rules. | MTG-Q4's default. | `encounter.rs` foe test |
| 6 | (Replaced in MVP 2 by decisions 16 to 19.) The auto-battle as in `battle.rs`'s header: mana from lands within Reach, banking to 12, a hand of 5, cast the dearest affordable card, abilities last in the chain, then attack; the other side may counter, shrink an attacker or flash in at end of turn. | The person's rules for the fight (R2.8 for the order). | 12 `battle.rs` tests, `balance.rs` |
| 7 | Rules for all 108 cards (`rules.rs`), the five guild mechanics, and an empty library reshuffling discards (R2.6). | "rules for every card". | `content.rs`: a card without rules is refused |
| 8 | Stories for all 108 cards (`data/stories/`), fate draws of four shown, shuffled and picked. | Hand of Fate's encounter cards. | `story.rs`, `tools/stories/check.py` |
| 9 | Six quests of three steps: the Dracogenius (main) and one a guild, started by a guild card's story. | "recommended number of quests". | `quest.rs` |
| 10 | Armour: four slots; a starter set and a set a guild (24 pieces); health, Charisma and Style a piece, one rule a piece, a rule for a full set. | "armor types and benefits". | `player.rs` |
| 11 | Three cities with an armourer (turning piece), card merchant, guild hall, inn and trainer (three abilities). | The city as a menu. | `session.rs` tests |
| 12 | Niv-Mizzet: 30 life, five mana a turn, a 40-card blue-red deck, in play from the start; Nivix sealed until New Prahv. | The region's end. | `balance.rs`; `playthrough.rs` |
| 13 | TikZ art for all 108 cards, 3D models for all 59 creatures and 6 tokens, 13 CHUMWORLD buildings, board scenery, 24 armour pieces. | The person's scope. | `render.py --check`; `look.rs` test that every creature has a model |
| 14 | A save in `localStorage` (D17 changed, R2.2). | A long game. | `no_storage.rs` |

## MVP 2: a fight is a game of Magic (T1)

Asked on 2026-10-24 (TRIAGE T1, DECISIONS.md). Built in this order; the notebook rows R3.1 to R3.13 say what was decided alone.

| # | Decision | Reason | Test that shows it holds |
| --- | --- | --- | --- |
| 15 | A literature review of open-source Magic engines first, on the journal's Magic engines tab. | "start with a literature review". | the journal |
| 16 | The engine is the project's own (`duel.rs`). It follows the Comprehensive Rules sections a two-player game of these cards needs: turns and steps, priority, the stack, casting with targets checked again on resolution, triggers in APNAP order, activated and mana abilities, combat, state-based actions, layers for the region's effects. It has a step cap. | Forge is GPL; phase.rs breaks the core's rules (R3.6). | `duel.rs` tests; `tests/breaks.json` |
| 17 | Every card's rules in `rules.rs` rewritten as Magic: targets, modes, overload, triggers, activated and mana abilities, statics, auras, equipment, the five guild mechanics. | "get the 100 cards you've created now to work as if you were playing magic". | `content.rs` (a card without rules is refused); the card audit (R3.7) |
| 18 | The land pile: no lands in decks, one land a turn from a face-up pile, the player's made up to 5. Opening hand of 5, no mulligan. | "guaranteed lands or some solution to mana scre/flood" (R3.2). | `a_short_land_pile_is_made_up_with_the_colour_it_lacks`; `a_land_is_played_from_the_pile_once_a_turn` |
| 19 | Every decision is an ask. The computer answers its own (`duel_ai.rs`); the page sends the player's through `act("duel.*")`. The player is asked only when they have something to do: their main phases, combat, the foe's spells and the foe's end step. | Arena's way of not asking for every pass. | `a_counterspell_stops_a_spell_and_both_go_to_the_graveyard`; `the_same_seed_and_answers_give_the_same_game` |
| 20 | The same 3D table: lands as crystals in each side's corner, a tapped creature turned a quarter, the stack and the steps in the panels, cards cast from the hand, creatures picked on the table. | "the rest of the game is great so far though including the 3d area". | browser check; screenshots in the journal |
| 21 | Balance re-tuned for Magic (R3.11). | Niv-Mizzet in play from the start is far stronger in real Magic. | `balance.rs` |

## Open questions for the person

MTG-Q1 (Wizards' content in public), MTG-Q2 (the name), MTG-Q3 (the childhood game), MTG-Q5 (where to publish). The rest have the defaults in the journal.

## Status

- [x] Read by the person on: 2026-10-24 (approved by asking for the MVP)
