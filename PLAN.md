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
| 6 | The auto-battle as in `battle.rs`'s header: mana from lands within Reach, banking to 12, a hand of 5, cast the dearest affordable card, abilities last in the chain, then attack; the other side may counter, shrink an attacker or flash in at end of turn. | The person's rules for the fight (R2.8 for the order). | 12 `battle.rs` tests, `balance.rs` |
| 7 | Rules for all 108 cards (`rules.rs`), the five guild mechanics, and an empty library reshuffling discards (R2.6). | "rules for every card". | `content.rs`: a card without rules is refused |
| 8 | Stories for all 108 cards (`data/stories/`), fate draws of four shown, shuffled and picked. | Hand of Fate's encounter cards. | `story.rs`, `tools/stories/check.py` |
| 9 | Six quests of three steps: the Dracogenius (main) and one a guild, started by a guild card's story. | "recommended number of quests". | `quest.rs` |
| 10 | Armour: four slots; a starter set and a set a guild (24 pieces); health, Charisma and Style a piece, one rule a piece, a rule for a full set. | "armor types and benefits". | `player.rs` |
| 11 | Three cities with an armourer (turning piece), card merchant, guild hall, inn and trainer (three abilities). | The city as a menu. | `session.rs` tests |
| 12 | Niv-Mizzet: 30 life, five mana a turn, a 40-card blue-red deck, in play from the start; Nivix sealed until New Prahv. | The region's end. | `balance.rs`; `playthrough.rs` |
| 13 | TikZ art for all 108 cards, 3D models for all 59 creatures and 6 tokens, 13 CHUMWORLD buildings, board scenery, 24 armour pieces. | The person's scope. | `render.py --check`; `look.rs` test that every creature has a model |
| 14 | A save in `localStorage` (D17 changed, R2.2). | A long game. | `no_storage.rs` |

## Open questions for the person

MTG-Q1 (Wizards' content in public), MTG-Q2 (the name), MTG-Q3 (the childhood game), MTG-Q5 (where to publish). The rest have the defaults in the journal.

## Status

- [x] Read by the person on: 2026-10-24 (approved by asking for the MVP)
