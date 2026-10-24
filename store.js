// The only page file that may name browser storage (a change to Builder's
// D17, SECOND-ORDER R2.2): one text, the core's save of the game:
// the map, the player and their cards. It decides nothing about what is kept. When
// storage is refused (a private window), the save lasts the visit.
const KEY = "tenth-district.save";

export function readSave() {
  try {
    return localStorage.getItem(KEY) || "";
  } catch (e) {
    return "";
  }
}

export function writeSave(text) {
  try {
    localStorage.setItem(KEY, text);
  } catch (e) {
    // refused: the save lasts this visit only
  }
}
