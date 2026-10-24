"""The game's browser interactions, run by check.py in every engine.

check.py has already loaded the page, waited for it to report ready, and let
it idle. This plays the opening the way a person would: start a new game as
the Izzet with Ral's deck, read the opening, enter the Tenth District, leave it, and take a
step. Expected values are worked out by hand from the data, never copied from
the game's output: the starting health is the armour's base 10 plus the
wanderer's hood 2, coat 4, wraps 0 and boots 2, which is 18.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
STRINGS = json.loads((ROOT / "data" / "strings.json").read_text())


def run(page, check, engine):
    check(page.get_attribute("body", "data-screen") == "title", "the game opens on its title screen")
    check(STRINGS["title"] in page.inner_text("#panel-title"), "the title from data/strings.json is drawn")
    page.get_by_role("button", name=STRINGS["title.new"]).click()
    page.wait_for_selector("body[data-screen='newgame']")
    check(page.locator(".choice").count() == 5, "five guilds are offered")
    page.get_by_role("button", name=STRINGS["guild.Izzet"]).click()
    page.wait_for_selector("body[data-screen='starter']")
    check(page.locator(".choice").count() == 6, "the Izzet offer six starting decks")
    check(page.locator("#panel-starterdeck .card").count() == 12, "a starting deck is shown as its twelve cards")
    page.get_by_role("button", name=STRINGS["starter.Izzet.ral"]).click()
    # wait for the redraw; the heading is drawn in capitals, so the comparison ignores case
    page.locator("#panel-starterdeck", has_text=STRINGS["starter.Izzet.ral"]).wait_for(timeout=5000)
    shown = page.inner_text("#panel-starterdeck").lower()
    check(STRINGS["starter.Izzet.ral"].lower() in shown, "the deck pointed at is the one shown")
    page.get_by_role("button", name=STRINGS["starter.start"]).click()
    page.wait_for_selector("body[data-screen='result']")
    check(STRINGS["story.opening"][:40] in page.inner_text("#panel-result"), "the opening story is shown")
    health = STRINGS["hud.health"].replace("{n}", "18").replace("{max}", "18")
    check(health in page.inner_text("#panel-hud"), "a new player has 18 health of 18")
    page.get_by_role("button", name=STRINGS["result.city"]).click()
    page.wait_for_selector("body[data-screen='city']")
    check(page.locator(".menu-item").count() == 7, "the city offers five rooms, the deck and the way out")
    page.get_by_role("button", name=STRINGS["city.leave"]).click()
    page.wait_for_selector("body[data-screen='board']")
    check("12 / 20" in page.inner_text("#panel-hud"), "the Izzet starting deck holds twelve cards")
    check(page.inner_text("#message") == "", "no error message is shown")
