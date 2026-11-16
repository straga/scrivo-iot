// What a device's page DRAWS, worked out and written as JSON, so two revisions of the page and the board can be compared byte for byte:
// the rows and settings every card is made of, the order of the cards, the card Now, the flame on the scheme, and
// the lines of a curve with their legends and spans.
//
//     node tools/page_picture.js --page DIR|sources --dictionary FILE --state FILE --curves FILE > PICTURE.json
//
// PAGE is the folder of the page's files - a revision taken out of git (`git archive`) -, or `sources`: the working tree's page, the engine's files and the burner's
// (tools/page_sources.js) with their menu and texts merged, DICTIONARY what the board
// serves at /dictionary, STATE the values by address a stand carries, CURVES what the board answers at /curve/<name> (by name). It runs the
// page's own code in the order index.html loads it, and for what the page has taken out of its classes - the order of the cards, the
// lines of a curve - it calls the same functions the classes call, and where a revision has none it works the old formula the class
// carried. WHAT IT DOES NOT SEE: the markup and the styles - those are judged on the stand, by the screens of tools/page_shots.py.
//
// The flame is put through every state it has, because a picture that held one would match by luck.
const fs = require("node:fs");
const path = require("node:path");

function args() {
    const out = {};
    const a = process.argv.slice(2);
    for (let i = 0; i < a.length; i += 2) out[a[i].replace(/^--/, "")] = a[i + 1];
    for (const need of ["page", "dictionary", "state", "curves"]) if (!out[need]) throw new Error(`page_picture: --${need} is needed`);
    return out;
}

function read(file) {
    return JSON.parse(fs.readFileSync(file, "utf8"));
}

const given = args();
const { load, menu: mergedMenu, texts: mergedTexts } = require("./page_sources");
const SOURCES = given.page === "sources";
const scope = {};
const FILES = ["form.js", "wire.js", "burning.js", "dictionary.js", "board.js", "scheme.js"];
// a revision from before the page was split has no wire.js: what it has is loaded, in the order index.html loaded it
if (SOURCES) load(FILES, scope);
else for (const file of FILES.filter((one) => fs.existsSync(path.join(given.page, one)))) new Function("globalThis", fs.readFileSync(path.join(given.page, file), "utf8"))(scope);
const dict = read(given.dictionary);
let texts = SOURCES ? mergedTexts() : read(path.join(given.page, "texts.json"));
const menuFile = path.join(given.page, "menu.json");
// the page reads its menu beside its texts when it has one, as app.js hands it over; a revision without one reads the texts alone
if (SOURCES) texts.menu = mergedMenu();
else if (fs.existsSync(menuFile)) texts.menu = read(menuFile);

const at = (card, name, part) => dict.things.findIndex((t) => t.card === card && t.name === name && (t.part_of || undefined) === part);
const stateValues = read(given.state).values;

// the states of the flame, each the values the board would send for its row, its pin and the order of its thresholds
const FLAME_STATES = {
    out: { flame: "out", pin: 3153, order: "in_order", lit: "2400", steady: "1100" },
    weak: { flame: "weak", pin: 1800, order: "in_order", lit: "2400", steady: "1100" },
    steady: { flame: "steady", pin: 900, order: "in_order", lit: "2400", steady: "1100" },
    crossed: { flame: { unknown: "crossed" }, pin: 1500, order: "crossed", lit: "1100", steady: "2400" },
    unread: { flame: { unknown: "not_read" }, pin: { unknown: "not_read" }, order: "in_order", lit: "2400", steady: "1100" },
    dark_top: { flame: "out", pin: { at_least: 3300 }, order: "in_order", lit: "2400", steady: "1100" },
};

function picture(state) {
    const board = new scope.boards.Board();
    board.take(stateValues);
    const set = (address, value) => { if (address >= 0) board.take([[address, value]]); };
    board.load(dict, texts);
    set(at("flame", "flame"), state.flame);
    set(at("flame", "pin"), state.pin);
    set(at("settings", "flame_lit"), { text: state.lit });
    set(at("settings", "flame_steady"), { text: state.steady });
    set(dict.things.findIndex((t) => t.card === "settings" && t.name === "order" && t.part_of === "flame_steady"), state.order);
    const m = board.machine;
    const things = board.things;
    const plain = (x) => JSON.parse(JSON.stringify(x));
    const D = scope.dictionary;
    // the cards of the Burner tab in the order drawn: first appearance among the rows, the automaton's two apart - or the page's own function
    const groups = D.cardOrder
        ? D.cardOrder([...new Set(things.map((t) => t.group))].filter((g) => !m || (g !== m.card && g !== m.fixed)), texts)
        : [...new Set(things.map((t) => t.group))].filter((g) => !m || (g !== m.card && g !== m.fixed));
    const nowCards = new Set((((m || {}).now) || []).map((n) => n.card));
    const nowRows = D.nowRows ? D.nowRows(things, texts, m) : things.filter((t) => nowCards.has(t.group));
    // the drawing's pot: the row the menu puts there and the key of the row its number is read from; a revision without a menu looks the flame up by name
    const pot = scope.scheme.standing ? scope.scheme.standing(things, "pot") : null;
    const flameRow = pot || things.find((t) => t.group === "flame" && t.name === "flame");
    const potNumber = pot && pot.placeNumber !== undefined ? { group: pot.group, key: pot.placeNumber } : undefined;
    // THE PLACE A ROW HAS IN THE CARD NOW IS A SORTING KEY, never drawn: the board's list gave it as the row's number in the list, and a row the menu puts in
    // the card stands between two of them. Two pages that order the card the same hold different keys when one list is shorter, so the keys are put as
    // their ranks - what a person sees of them is the order.
    const keys = [...new Set(things.filter((t) => t.now !== undefined).map((t) => t.now))].sort((a, b) => a - b);
    // A ROW'S CURVES CARRY THE MENU'S LINES in a revision that has a menu - what the curve is drawn with is compared below, line by line, in `curves`
    const ranked = things.map((t) => {
        const one = t.now === undefined ? { ...t } : { ...t, now: keys.indexOf(t.now) };
        if (one.curves) one.curves = one.curves.map(({ name, words }) => ({ name, words }));
        return one;
    });
    return plain({
        groups,
        things: ranked,
        settings: board.everySetting,
        now: (scope.wire || scope.burning).now(nowRows),
        scheme: {
            flame: scope.scheme.flameOnScheme(flameRow),
            volts: scope.scheme.flameVolts(things, potNumber),
        },
    });
}

function curveLines() {
    const curves = read(given.curves);
    const out = {};
    const board = new scope.boards.Board();
    board.take(stateValues);
    board.load(dict, texts);
    for (const t of board.things) {
        for (const one of t.curves || []) {
            const drawn = curves[one.name];
            if (!drawn) continue;
            const lines = scope.dictionary.curveLines ? scope.dictionary.curveLines(drawn.lines, one) : drawn.lines;
            out[`${t.group}/${t.key}/${one.name}`] = { title: one.words, lines: JSON.parse(JSON.stringify(lines)) };
        }
    }
    return out;
}

const result = { states: {}, curves: curveLines() };
for (const [name, state] of Object.entries(FLAME_STATES)) result.states[name] = picture(state);
// KEYS IN SORTED ORDER: which key an object was given first is no part of what a page draws, and a line the menu words gets its words after its points
const sorted = (x) => (Array.isArray(x) ? x.map(sorted) : x && typeof x === "object" ? Object.fromEntries(Object.keys(x).sort().map((k) => [k, sorted(x[k])])) : x);
process.stdout.write(JSON.stringify(sorted(result), null, 1) + "\n");
