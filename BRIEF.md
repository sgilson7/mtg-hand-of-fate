# Brief

Stage 1. The person's own words come first; the rest can be written by the agent from them and read by the person. `C1 Frame the problem and set the rules`, `B1 Establish context`.

## The description, word for word

The request arrived in two parts in one message: a pasted block, then the description of the game. Both are kept unchanged.

> get yourself familiarized with the builder repository and clone it from https://github.com/sgilson7/builder-setting into this repo then remove the .git. You will be using builder setting to help constrain and direct your actions as you help act as as a video game  creator and co-designer / developer, with the goal of deploying it to a github pages so its freely browser accessible, with an emphasis on optimization for running in the browser.  Please also see https://sgilson7.github.io/projects/ for examples of other browser based tools / games built in builder like the PDF redactor, youtube to mp3, long time coming, extremely lifelike fishing  etc. Also see ptonkle.github.io for games built using builder by my good friend. First thing is I want you to do is interpret builder, and write an analysis of how you will use it to help you develop this game, and write into the first tab of the design journal, the design journal process is described in builder; when you write the design journal, use the local ape-race directories design journal as a representative of a good journaling process. you may access the other builder made games within the documents directory on this host machine.

> the game I want to make is as inspired by a game id play as a kid to pass time with my siblings, and hand of fate. the game is played on a 2d grid, where each spot on the grid is controlled by a possible pool of magic cards that can be drawn, or obstacles of some kinds like mountains, or cities with shops and the such. the game should be 3d in the perspective of mage tower battle and in the style of protection of the spheres, but the game is really 2d underneath the hood. When you go on a tile controleld by a magic card, the possible deck of cards is drawn from, then each magic card has 3 modes, based on their mana cost, abilities, and type. if the card draw is a creature, then you battle it using an auto battling system like graftwood or level twenty. if the card is an enchantment or an artifact, you have a story card with text and a little choice to make related to either the magic spell or atfiact you find, that can begin / advance  questlines like hand of fate 1/2. if you get a land, you can attach them to any tile you've visited for bonuses. if the card is an instant or sorcery, you get a story node as well but more magic related, when you clear a tile and complete the card, you get it for your inventory, and you can build your own 20 card deck slowly over time that you use to auto battle. the auto battleing should pretty much be magic but instead you get mana per turn equal to lands you've placed on the grid (potentially within a range based on a stat), and then your mana gets spent to cast a card from your 5 card hand if possible, or nothing happens and you bank mana for the next turn. generally you bank mana turn to turn. besides that, you should summon creatures like normal, and their abilities should go off last in the chain if you have mana. your max health is dependent on your armor you acquire, and other stats like charisma, style, etc are affected by your look and your armor. for the magic cards, start with the set return to ravnica, i leave you to figure out how to fit the colors into the game. you should start at the center of a 2d grid and if you get to an edge, you add an additional 5x5 set of tiles to the map. cities should allow you to buy armor, cards, abilities, start quests, complete events related to quests, etc. the armor should be modeled in 3d and spin around in the shop, and the mtg cards should be in a 3d deck of yours, but really be 2d. the art for each card should be remade using the tikz prompt, but the text of the mtg card should be maintained verbatim, and the art should be also verbatim just recreated. each tile should be themed by its placement on the map, like a mountain tile should have a lil 3d mountain poking out of it, and a river tile should have a river through it, the city should be rendered in 3d, use buildings from chumworld to fill the cities with assets, but its really just a glorified menu. the game is about building a big deck and then defeating the local guardian, who also has a sophisticed deck of their own. you can use https://mtgjson.com for cards, or mtg preview or the wizards sites for searching cards to get cards. start by design journaling to explain how you will use builder to create this game, and then build out a mockup of the game for my review with screenshots of scenes mocked up, flow diagrams, maps, etc. i want to review before you begin building the real thing. recall the main inspiration is hand of fate 1 and 2, so do a research paper on the gameplay mechanics of that game in its own design tab and how it will affect this game as ive described

## The tool

A single-player deck-building adventure in the browser, for anyone with a link, on a desktop or laptop first. The player walks a 2D grid of tiles drawn in 3D. Most tiles hold a pool of Magic: The Gathering cards from Return to Ravnica; stepping on one draws a card, and the card's type decides the encounter (a creature is an auto-battle, an enchantment or artifact is a story choice, an instant or sorcery is a magic-flavoured story choice, a land is attached to a visited tile). Clearing a tile adds its card to the player's collection, from which they build a 20-card deck. Cities are menus drawn as 3D towns where the player buys armour, cards and abilities and starts and finishes quests. The goal is to build a deck strong enough to beat the local guardian, who has a deck of their own. Hand of Fate 1 and 2 are the main inspiration.

## Done means

Written for the mockup stage, which is the only stage this brief currently covers. The full game's conditions are drafted in `PLAN.md` and settled when the person has read the mockup.

- The design journal exists beside the project with an opening entry (how Builder will be used), a Hand of Fate research tab, and a mockup tab.
- The mockup tab shows each main scene (the grid, a card draw, an auto-battle, a story card, a land placement, a city, the armour shop, the deck builder, the guardian fight), a flow diagram of the game loop, a map of the starting grid and its growth, and the open questions, each with a default.
- Nothing is built in `crates/` beyond the template until the person has reviewed the mockup.

## Decisions that stay with the person

- Use of Magic: The Gathering card names, text and recreated art in a free public game, under Wizards of the Coast's Fan Content Policy (MTG-Q1).
- The game's public name and repository name (MTG-Q2).
- The rules of the childhood grid game the description names, which only the person knows (MTG-Q3).
- Whether CHUMWORLD's building models may be copied in, and their terms (MTG-Q4).
- The publish grant: which repository, and whether the agent may publish (MTG-Q5).
- Whether the game keeps a save between visits (browser storage, D17).

## Accessibility

Colour is never the only cue: each of the five Magic colours carries its mana symbol shape as well as its hue, and guilds carry their sigil. Card text is real text on the page, readable by a screen reader and at browser zoom. Mouse and keyboard both work: arrow keys or WASD to move on the grid, number keys for story choices. Auto-battles can be paused and stepped.

## Style

A description, not a pack yet: a tilted 3D board in the camera of Mage Tower Battle, in the look of Protection of the Spheres (to be confirmed from the local and published copies). Card art is TikZ through `tools/art/`, recreating each card's original composition. A style pack will be written in `styles/ravnica/` once the person has approved the mockup.

## Words

Magic: The Gathering's own terms keep their Magic meaning (mana, mana value, creature, instant, sorcery, enchantment, artifact, land, guild, detain, overload, unleash, scavenge, populate). Hand of Fate's terms are borrowed where the description names them (story card, quest line). The game's own terms go into `GLOSSARY.md` once the mockup is approved.

## Autonomy

The plan was not approved in advance: the person reviewed the mockup first. On 2026-10-24 the person asked for the MVP of the first region (DECISIONS.md), which approves `PLAN.md` with the journal's defaults. The agent builds stages 4 to 7 and the handoff without stopping. The agent may not loosen a Builder requirement or a boundary test, and may not add a server, an account, analytics or any off-origin request. The agent does not create a repository, push or publish until the person names the repository and grants it (MTG-Q5).

## Journal

`default`

Where the design journal lives. `default` is a folder beside this project, `../<this folder>-journal/`, kept as its own repository and never pushed; a relative path puts it elsewhere. Write `none`, with the reason, to build without a journal. With any other value the agent starts the journal and writes its opening entry before the plan, and `scripts/journal_check.py` fails while it is missing.

## Allowed core dependencies

`serde`, `serde_json` (reference default). Add others only here, with the reason.
