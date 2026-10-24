# Second-order notebook

Stage 6, kept by the agent as the work happens, not afterwards. `B3 Build in self-questioning`.

The code and tests are the work; this file is the record about the work. It is where a decision the agent made alone, an assumption that turned out wrong, or something only the person can do becomes visible to the person. With the plan approved in advance, it is the main place those decisions surface before the person uses the tool. [`docs/procedure.md`](https://github.com/sgilson7/builder-setting/blob/main/docs/procedure.md#the-second-order-notebook) says why.

One row per assumption, surprise or open item. **Kind** is *divergence* (what was assumed, what turned out to be true, what changed), *finding*, or *worklist* (the person's to do). Number rows by round (R1.1, R1.2, then R2.1 after the first triage) so a later round does not renumber an earlier one. **Rule** is the rule the row produced for later sessions, and where it now lives (`CLAUDE.md`, a test, `tests/breaks.json`), or blank. **Status** is *done*, *open*, or *person*; every *person* row is named in `HANDOFF.md`. `scripts/notebook_check.py` checks the form of each row and that the notebook is not empty once `PLAN.md` holds a decision, not what the rows say.

| # | Kind | Assumed or found | What is true | What changed | Rule | Status |
| --- | --- | --- | --- | --- | --- | --- |
| R1.1 | worklist | The description asks for Return to Ravnica's card text verbatim and its art recreated, in a game on public GitHub Pages. | Wizards' Fan Content Policy allows free fan content using its art and text with a notice, and also says "Don't use Wizards' IP in other games" and bars using its game mechanics without written permission. A free public game reproducing the cards may fall outside the policy. This is not legal advice. | Nothing is published. The cards live in one data file (`data/cards.json`) and the art in one folder, so the content can be swapped without touching a rule. Asked as MTG-Q1. | | person |
| R1.2 | worklist | The public name: the folder is `mtg-hand-of-fate` after the workspace. | "Magic: The Gathering", "MTG" and "Hand of Fate" are other companies' names. | Working title only, asked as MTG-Q2. | | person |
| R1.3 | worklist | The childhood grid game named in the description. | Only the person knows its rules. | The mockup does not borrow from it yet. Asked as MTG-Q3. | | person |
| R1.4 | divergence | "Each magic card has 3 modes, based on their mana cost, abilities, and type" read as three choices a player makes. | It reads at least two ways. | The mockup shows the reading I default to: type picks the encounter, mana value sets its difficulty tier, abilities set its twist and reward. Asked as MTG-Q4. | | open |
| R1.5 | finding | Card text from MTGJSON is the printed 2012 text. | MTGJSON's `text` is current Oracle wording ("any target", "This land enters tapped"), not the 2012 printing. | The mockup shows Oracle text. Default for the game is Oracle text, since the rules the battle uses are written against it; decision MTG-Q11. | | open |
| R1.6 | finding | TikZ can recreate each card's art from a description. | Recreating a composition needs the original in view. I fetched six art crops from Scryfall into the session's scratch folder and drew four. Each first draft took about 10 minutes and keeps the composition but not the painting's detail. | Reference images stay out of every repository. At this rate the 254 cards are roughly 40 hours of drafting before a second pass. Decision MTG-Q12. | Reference art is never committed: `LICENSES.md`, and a check to write | open |
| R1.7 | finding | "In the style of Protection of the Spheres" names a game I had not seen. | It is the person's DotA-style game (OGDota, `/pots/`): low-poly rounded shapes, banded toon light, violet ink outlines, warm sand haze, camera pitched 56° with a 52° field of view. Mage Tower Battle's overview is pitched about 47° with a 38° field. | The mockup uses PotS's haze, ink and HUD colours and a 52–56° pitch. | | done |
| R1.8 | divergence | The mockup is drawn with three.js, as Mage Tower Battle's and Fishing's journal mockups are. | The game cannot use it: Builder's page replays the core's draw list in hand-written WebGL (D3), as Mage Tower Battle's `gl.js` does. | three.js stays in the journal's `mockups/vendor/` with its checksum and never enters `web/`. | MTG-10 in the opening entry | done |
| R1.9 | finding | Every stat, price and size in the mockup (health 18, Reach 2, 34 gold, pools of 6, a 40-card guardian deck) is a placeholder I chose. | None of them is a rule yet. | They move into the core as named constants when the plan is approved, each with a test. | | open |
| R1.10 | worklist | Mockup story text ("The Guildless Light", "Fire from the rooftops") and every label in the mockup were written by me. | Story text is new writing, not card text, and a reader will see it. | Listed below for the person to read. | | person |

## Strings written by the agent

Each reader-visible string the agent wrote, with its key, for the person to read.

| Key | String |
| --- | --- |
| mockup.story.lantern | A lantern of gold owls hangs over a collapsed archive, and its six panes hold six colours, one more than the guilds have names for. An Izzet chemister on a ladder swears it hums when someone lies. |
| mockup.story.lantern.1 | Take it down yourself |
| mockup.story.lantern.2 | Pay the chemister to do it |
| mockup.story.lantern.3 | Leave it, and note where it hangs |
| mockup.story.mortars | Izzet mortars arc over the tenements. You can turn one with a word, if you say the word right. |
| mockup.city | The Tenth District |
| mockup.armour | Senate Plate |
