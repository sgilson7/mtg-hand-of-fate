# Story data

One file per group in `data/stories/<group>.json`: `{"stories": [ ... ]}`, one entry per card in that group of `data/cards.json`. The core (`story.rs`) reads them; `python3 tools/stories/check.py` checks them.

The card itself is always shown beside its story with its text verbatim. The story is the game's interpretation of the card as a moment in the Tenth District of Ravnica: who you meet, what is happening, what you might do. It does not restate the card's rules text.

## Kinds

The kind follows the card's type: a creature (including an artifact creature) is `creature`; an enchantment or a non-creature artifact is `story`; an instant or sorcery is `magic`; a land is `land`.

```json
{"number": "158", "name": "Dreg Mangler", "kind": "creature",
 "intro": "Two or three sentences: you meet it, and it comes for you.",
 "win": "One or two sentences after you win: it is yours now.",
 "lose": "One sentence after you lose.",
 "flee": "One sentence after you run."}

{"number": "226", "name": "Chromatic Lantern", "kind": "story",
 "title": "A short title",
 "text": "Three to five sentences: the scene.",
 "choices": [
   {"text": "What you do, as a short command", "needs": {"charisma": 3}, "outcome": OUTCOME},
   {"text": "...", "outcome": OUTCOME},
   {"text": "...", "outcome": OUTCOME}
 ]}

{"number": "265", "name": "Mountain", "kind": "land",
 "text": "Two sentences: you find this land, and you can claim it for a tile you have walked."}
```

`magic` has the same shape as `story`; its scene is about a spell being cast, a spell's residue, or a mage.

Two or three choices. At least one path lets you take the card. `needs` is optional and one of: `{"charisma": n}`, `{"style": ["Izzet", n]}`, `{"gold": n}` (paid when chosen), `{"supplies": n}` (paid when chosen). n is 1 to 5 for charisma and style, 5 to 25 for gold, 1 to 4 for supplies.

## Outcomes

```json
OUTCOME = {"text": "What happens, one or two sentences.", "effects": [EFFECT, ...]}
        | {"text": "What is at stake, one sentence.", "fate": {"faces": [hs, s, f, hf],
             "huge_success": OUTCOME, "success": OUTCOME, "failure": OUTCOME, "huge_failure": OUTCOME}}
```

`faces` counts the four kinds of fate card shown before the shuffle: huge success, success, failure, huge failure. They add up to 4. A branch whose count is 0 may be left out. A fate branch's outcome has `effects` (no nested fate).

Effects, each an object with one key:

| Effect | Meaning |
| --- | --- |
| `{"take": true}` | You gain this card and the tile is cleared. |
| `{"fight": true}` | You fight a creature from this tile's pool; if you win you also take this card. |
| `{"leave": true}` | Nothing more happens and the tile is not cleared; you may come back. |
| `{"gold": n}` | Gain (or lose, negative) n gold. -25 to 30. |
| `{"supplies": n}` | Gain or lose supplies. -4 to 6. |
| `{"health": n}` | Heal (positive) or take damage (negative). -6 to 6. |
| `{"pool_card": true}` | You also gain another card from this tile's pool. |
| `{"style": ["Golgari", n]}` | Standing with a guild, 1 or 2. |
| `{"quest_start": "lantern"}` | Starts a quest (see data/quests.json). |

Every outcome either clears the tile (`take` or a won `fight`) or says `leave`, except a fate branch that loses: a failure may simply cost something, and then the tile is cleared without the card. The core applies that rule; write the text to match ("you leave with nothing but the bruise").

## Quests that cards start

These cards must have a choice whose outcome (or success branch) includes `quest_start` with the id shown, as well as `take`:

| Card | Quest | The quest |
| --- | --- | --- |
| 155 Detention Sphere | `writ` | An Azorius writ of detention has gone missing; carry it to New Prahv, win a hearing by force in an Azorius district, and return to the Tenth District. |
| 226 Chromatic Lantern | `lantern` | A lantern that answers to six colours; find what it lights in an Izzet district, beat what guards it, and take it to New Prahv's archive. |
| 76 Shrieking Affliction | `festival` | A Rakdos curse spreading through a festival; trace it to a Rakdos district, beat its performer, and bring the cure back to the Tenth District. |
| 229 Golgari Keyrune | `undercity` | A key to the undercity; open its door in a Golgari district, survive what lives there, and report to Vitu-Ghazi. |
| 217 Growing Ranks | `conclave` | The Conclave asks for help at Vitu-Ghazi; plant its seed in a Selesnya district and defend the sapling. |

The places: **the Tenth District** (the city at the centre, where you start), **New Prahv** (the Azorius city to the north), **Vitu-Ghazi** (the Selesnya city to the west), **Rix Maadi** (the Rakdos city to the south), **Svogthos** (the Golgari city to the south-west), **Nivix** (Niv-Mizzet's aerie and laboratory, the guardian's lair in the north-east), and districts of each guild scattered between them.

## Words

Follow `TONE.md` and `GLOSSARY.md`. Second person, present tense. Plain words, sentences under 25 words. Never type a number of gold, supplies or health in text: the game shows the amounts. No stock phrases (delve, tapestry, realm, robust, seamless, elevate, "it is worth noting"). Name colours only alongside something else ("a red-robed Rakdos performer", not "the red one"). Stay inside Ravnica's Return to Ravnica era: guilds, guildmages, districts, the Guildpact's absence, Jace as the Living Guildpact is later and not mentioned.
