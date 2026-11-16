// devices/burner/page/dictionary.js: how a page reads the MENU of a module - where its rows stand, what they are called, which of its settings are
// folded, what its curves say - beside the table of texts it kept before. The data here is made up: what is tested is the mechanism,
// never a phrase, and never a name of the burner.
//
//     node --test ./tools/test_page_menu.js
const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");

const ROOT = path.dirname(__dirname);
const { load } = require("./page_sources");
const scope = {};
load(["form.js", "wire.js", "burning.js", "dictionary.js", "board.js", "scheme.js"], scope);
const d = scope.dictionary;
const { Board } = scope.boards;

// A module `m` the menu places; a module `o` and a card `x` it does not, which keep to the table of texts.
const DICT = {
    contract: 3,
    version: "k+1",
    machine: { card: "auto", fixed: "fixed", stage: "s", says: "s", cause: "s", press: [], aside: [], sections: [], now: [{ card: "x", row: "first" }, { card: "o", row: "rate" }] },
    things: [
        { card: "m", name: "state", kinds: ["word", "unknown"], words: ["a", "b"], reasons: ["gone"] },
        { card: "m", name: "pin", kinds: ["number", "unknown"], unit: "mV", curves: ["m_fast"] },
        { card: "settings", name: "m_lo", kinds: ["text"] },
        { card: "settings", name: "m_hi", kinds: ["text"] },
        { card: "settings", name: "order", part_of: "m_hi", kinds: ["word"], words: ["in", "crossed"] },
        { card: "o", name: "rate", kinds: ["number"], unit: "u", curves: ["o_fast"] },
        { card: "x", name: "first", kinds: ["number"], unit: "u" },
    ],
    // what the board says of the settings of a module the menu places: no card
    settings: [
        { name: "m_lo", default: "5", kind: "whole", from: 1, to: 9, unit: "mV" },
        { name: "m_hi", default: "7", kind: "whole", from: 1, to: 9, unit: "mV" },
        { name: "o_set", card: "o", default: "1", kind: "whole", from: 1, to: 9 },
    ],
};
const MENU = {
    version: 1,
    modules: {
        m: {
            card: { after: "x", title: { en: "the m card", ru: "" } },
            rows: {
                state: {
                    label: { en: "State", ru: "Состояние" },
                    says: [["a", { en: "is a", ru: "это a" }], ["unknown:gone", { en: "gone", ru: "" }]],
                    level: [["a", "ok"], ["unknown:gone", "bad"]],
                    now: { after: "x/first", as: { en: "the m" }, beside: "pin", spec: "volts" },
                    scene: { place: "pot", number: "pin" },
                },
                pin: {
                    label: { en: "Pin" },
                    says: [["number", { en: "{} mV" }]],
                    curves: { m_fast: { title: { en: "minute" }, lines: [{ legend: { en: "what the pin held" }, span: [0, 3300] }] } },
                },
            },
            settings: {
                m_lo: { label: { en: "Low" }, unit: "mV", level: "work" },
                m_hi: { label: { en: "High" }, unit: "mV", level: "tuning", note: "order" },
            },
            notes: { "m_hi.order": { says: [["in", { en: "in order" }], ["crossed", { en: "crossed at {m_hi} mV" }]] } },
        },
    },
};
const TEXTS = {
    rows: { "o/rate": { label: "Rate", curves: { o_fast: "hour" } }, "x/first": { label: "First" } },
    settings: { o_set: { label: "O set" } },
};
const WITH = { ...TEXTS, menu: MENU };

const VALUES = [[0, "a"], [1, 3153], [2, { text: "5" }], [3, { text: "7" }], [4, "crossed"], [5, 4], [6, 1]];
// the values by address, as the board's records put them in place
const byAddress = (pairs) => {
    const all = new Array(DICT.things.length).fill(undefined);
    for (const [at, v] of pairs) all[at] = v;
    return all;
};
const state = (texts, pairs = VALUES) => d.state(DICT, texts, byAddress(pairs));
const row = (s, group, key) => s.things.find((t) => t.group === group && t.key === key);

test("a row of a module the menu places is called, worded and levelled by the menu", () => {
    const one = row(state(WITH), "m", "state");
    assert.deepStrictEqual([one.name, one.state, one.level], ["State", "is a", "ok"]);
    const gone = row(state(WITH, [[0, { unknown: "gone" }]]), "m", "state");
    assert.deepStrictEqual([gone.state, gone.level], ["gone", "bad"], "an empty ru falls back to en");
});

test("a row of a module the menu does not place is drawn by the table of texts, with the menu beside it", () => {
    const one = row(state(WITH), "o", "rate");
    assert.deepStrictEqual([one.name, one.state], ["Rate", "4 u"]);
    assert.deepStrictEqual(row(state(TEXTS), "o", "rate"), one, "the same row without any menu");
});

test("the language the page asks for is the one the menu says, and en where it says none", () => {
    const ru = state({ ...WITH, lang: "ru" });
    assert.strictEqual(row(ru, "m", "state").name, "Состояние");
    assert.strictEqual(row(ru, "m", "state").state, "это a");
    assert.strictEqual(row(ru, "m", "pin").name, "Pin", "no ru for it");
});

test("the settings of a module the menu places stand on its card, folded where the menu folds them", () => {
    const s = state(WITH).settings;
    const by = (name) => s.find((x) => x.name === name);
    assert.deepStrictEqual([by("m_lo").card, by("m_lo").label, by("m_lo").level], ["m", "Low", undefined]);
    assert.deepStrictEqual([by("m_hi").card, by("m_hi").level], ["m", "tuning"]);
    assert.strictEqual(by("m_hi").note, "crossed at 7 mV", "the note is the menu's words for the code the board says");
});

test("a setting the board gives a card, of a module the menu does not place, stands where the board says", () => {
    const one = state(WITH).settings.find((x) => x.name === "o_set");
    assert.deepStrictEqual([one.card, one.label], ["o", "O set"]);
    assert.deepStrictEqual(state(TEXTS).settings.find((x) => x.name === "o_set"), one);
});

test("the cards stand in the order the menu gives, the others as they come", () => {
    assert.deepStrictEqual(d.cardOrder(["x", "o", "m"], WITH), ["x", "m", "o"]);
    assert.deepStrictEqual(d.cardOrder(["x", "o", "m"], TEXTS), ["x", "o", "m"], "no menu, no move");
    assert.deepStrictEqual(d.cardOrder(["o", "m"], WITH), ["o", "m"], "a card to stand after one that is not there stays where it is");
});

test("a row the menu puts in the card Now stands where it says, with the number beside it", () => {
    const s = state(WITH);
    const one = row(s, "m", "state");
    assert.ok(one.now > 0 && one.now < 1, `the place ${one.now} is not between the board's first and second row`);
    assert.deepStrictEqual([one.as, one.alongside], ["the m", "3.15 V"]);
    const rows = d.nowRows(s.things, WITH, DICT.machine);
    assert.deepStrictEqual(scope.wire.now(rows).map((r) => r.name), ["First", "the m", "Rate"]);
    // without the menu the card Now is the board's list alone
    const bare = state(TEXTS);
    assert.deepStrictEqual(scope.wire.now(d.nowRows(bare.things, TEXTS, DICT.machine)).map((r) => r.name), ["First", "Rate"]);
});

test("the curves of a row carry the lines the menu words, and the board's own where it words none", () => {
    const m = row(state(WITH), "m", "pin").curves[0];
    assert.deepStrictEqual(m, { name: "m_fast", words: "minute", lines: [{ words: "what the pin held", span: [0, 3300] }] });
    const o = row(state(WITH), "o", "rate").curves[0];
    assert.deepStrictEqual(o, { name: "o_fast", words: "hour" }, "a curve of a module the menu does not place has no lines of its own");
    const board = [{ unit: "mV", points: [1, 2] }];
    assert.deepStrictEqual(d.curveLines(board, m), [{ unit: "mV", points: [1, 2], words: "what the pin held", span: [0, 3300] }]);
    const theirs = [{ words: "asked", unit: "Pa", points: [1], span: [0, 300] }];
    assert.deepStrictEqual(d.curveLines(theirs, o), theirs, "the board's words and span stand when the menu has none");
    assert.deepStrictEqual(d.curveLines(theirs, undefined), theirs);
});

test("a row stands on the drawing where the menu puts it, with the key of the row its number is read from", () => {
    const s = state(WITH);
    const one = scope.scheme.standing(s.things, "pot");
    assert.strictEqual(one.key, "state");
    assert.strictEqual(one.placeNumber, "pin");
    assert.strictEqual(scope.scheme.standing(state(TEXTS).things, "pot"), null, "no menu, nothing on the drawing from it");
    assert.strictEqual(scope.scheme.standing(s.things, "nowhere"), null);
    assert.strictEqual(row(s, "m", "pin").place, undefined, "a row the menu does not place has no place");
});

test("the number on the drawing is read from the row the menu names, and from none else", () => {
    const s = state(WITH);
    assert.strictEqual(scope.scheme.flameVolts(s.things, { group: "m", key: "pin" }), "3.15 V");
    assert.strictEqual(scope.scheme.flameVolts(s.things, { group: "m", key: "nothing" }), "-");
});

test("a card is headed by the title the menu gives it, and by its own name where the menu gives none", () => {
    assert.strictEqual(d.cardTitle(WITH, "m"), "the m card");
    assert.strictEqual(d.cardTitle(WITH, "o"), "o");
    assert.strictEqual(d.cardTitle(TEXTS, "m"), "m", "no menu, the card's own name");
    assert.strictEqual(d.cardTitle({ ...WITH, lang: "ru" }, "m"), "the m card", "an empty ru falls back to en");
});

test("a board that still lists the row in the card Now draws it as the menu words it, at the place the board gives", () => {
    // a page kit put on a board whose image is not the new one: the row stands in `machine.now` and in the menu, and the menu's word and number win
    const OLD = { ...DICT, machine: { ...DICT.machine, now: [{ card: "x", row: "first" }, { card: "m", row: "state", with: "pin" }, { card: "o", row: "rate" }] } };
    const s = d.state(OLD, WITH, byAddress(VALUES));
    const one = row(s, "m", "state");
    assert.deepStrictEqual([one.now, one.as, one.alongside], [1, "the m", "3.15 V"]);
});

// ---- a note that says nothing when all is well (REF_Device_Page_Look, rule 7) ----

const QUIET = {
    ...MENU,
    modules: { m: { ...MENU.modules.m, notes: { "m_hi.order": { says: [["in", { en: "" }], ["crossed", { en: "crossed at {m_hi} mV" }]] } } } },
};

test("a note the menu words with nothing is no note: the setting says none while all is well, and speaks when it is not", () => {
    const calm = state({ ...TEXTS, menu: QUIET }, [[0, "a"], [1, 3153], [2, { text: "5" }], [3, { text: "7" }], [4, "in"]]).settings.find((x) => x.name === "m_hi");
    assert.strictEqual(calm.note, undefined, `a note was drawn in order: ${JSON.stringify(calm.note)}`);
    const wrong = state({ ...TEXTS, menu: QUIET }, [[0, "a"], [1, 3153], [2, { text: "5" }], [3, { text: "7" }], [4, "crossed"]]).settings.find((x) => x.name === "m_hi");
    assert.strictEqual(wrong.note, "crossed at 7 mV");
    // the pair: a note with words in order is drawn, as before
    const talking = state(WITH, [[0, "a"], [3, { text: "7" }], [4, "in"]]).settings.find((x) => x.name === "m_hi");
    assert.strictEqual(talking.note, "in order");
});

// ---- a row the menu puts in the card Now only while another part of it holds a code: the sensor whose role is the boiler's ----

const SOCKETS = {
    contract: 3,
    version: "k+1",
    things: [
        { card: "x", name: "first", kinds: ["number"], unit: "u" },
        { card: "t", name: "a", kinds: ["number", "unknown"], unit: "u" },
        { card: "t", name: "role", part_of: "a", kinds: ["word"], words: ["hot", "cold", "free"] },
        { card: "t", name: "b", kinds: ["number", "unknown"], unit: "u" },
        { card: "t", name: "role", part_of: "b", kinds: ["word"], words: ["hot", "cold", "free"] },
    ],
    settings: [],
};
// each socket stands in the card Now as the word of the role it plays now, and in the order of the roles, not of the sockets
const inNow = (rank_hot, rank_cold) => [
    { when: "*&role=hot", after: "x/first", as: { en: "the hot one" }, rank: rank_hot },
    { when: "*&role=cold", after: "x/first", as: { en: "the cold one" }, rank: rank_cold },
];
const SOCKETS_MENU = {
    version: 1,
    modules: { t: { rows: { a: { label: { en: "A" }, now: inNow(2, 1) }, b: { label: { en: "B" }, now: inNow(2, 1) } } } },
};
// what the card Now says of the sockets, by the word it gives each; the first row is the board's own
const sockets = (pairs, menu = SOCKETS_MENU) => {
    const texts = { rows: { "x/first": { label: "First" } }, menu };
    const all = new Array(SOCKETS.things.length).fill(undefined);
    for (const [at, v] of pairs) all[at] = v;
    const machine = { card: "auto", fixed: "fixed", stage: "s", says: "s", cause: "s", press: [], aside: [], sections: [], now: [{ card: "x", row: "first" }] };
    const s = d.state({ ...SOCKETS, machine }, texts, all);
    return scope.wire.now(d.nowRows(s.things, texts, machine)).map((r) => r.name);
};

test("a row the menu puts in the card Now while its part says a code stands there as the word of that code", () => {
    // socket a is cold, socket b is hot: both stand in Now, each as what it is now
    assert.deepStrictEqual(sockets([[0, 1], [1, 20], [2, "cold"], [3, 60], [4, "hot"]]), ["First", "the cold one", "the hot one"]);
    // the sensor that plays a role the menu does not put in Now stays out of it
    assert.deepStrictEqual(sockets([[0, 1], [1, 20], [2, "free"], [3, 60], [4, "hot"]]), ["First", "the hot one"]);
    // and a socket that gives its role to another changes places with it
    assert.deepStrictEqual(sockets([[0, 1], [1, 20], [2, "hot"], [3, 60], [4, "cold"]]), ["First", "the cold one", "the hot one"]);
});

test("the rows put in the card Now by a code stand in the order the menu ranks the codes, whatever the order of the rows", () => {
    // the cold one is ranked 1 and the hot one 2, and the sockets stand a, b in either case
    const ranked = { version: 1, modules: { t: { rows: { a: { label: { en: "A" }, now: inNow(1, 2) }, b: { label: { en: "B" }, now: inNow(1, 2) } } } } };
    assert.deepStrictEqual(sockets([[0, 1], [1, 20], [2, "cold"], [3, 60], [4, "hot"]], ranked), ["First", "the hot one", "the cold one"]);
    assert.deepStrictEqual(sockets([[0, 1], [1, 20], [2, "hot"], [3, 60], [4, "cold"]], ranked), ["First", "the hot one", "the cold one"]);
});

test("a row whose part says no code the menu names is in the card Now by no entry, and a row the menu gives one plain entry keeps working", () => {
    assert.deepStrictEqual(sockets([[0, 1], [1, 20], [2, "free"], [3, 60], [4, "free"]]), ["First"]);
    const single = { version: 1, modules: { t: { rows: { a: { label: { en: "A" }, now: { after: "x/first", as: { en: "the a" } } } } } } };
    assert.deepStrictEqual(sockets([[0, 1], [1, 20], [2, "free"], [3, 60], [4, "free"]], single), ["First", "the a"]);
});

// ---- where a row stands on the drawing by a code of its part: the sensor whose role is the boiler's stands at the boiler ----

const onDrawing = (hot, cold) => [
    { when: "*&role=hot", place: hot },
    { when: "*&role=cold", place: cold },
];
const placed = (pairs, scene) => {
    const menu = { version: 1, modules: { t: { rows: { a: { label: { en: "A" }, scene }, b: { label: { en: "B" }, scene } } } } };
    const all = new Array(SOCKETS.things.length).fill(undefined);
    for (const [at, v] of pairs) all[at] = v;
    const machine = { card: "auto", fixed: "fixed", stage: "s", says: "s", cause: "s", press: [], aside: [], sections: [], now: [] };
    const s = d.state({ ...SOCKETS, machine }, { menu }, all);
    return Object.fromEntries(s.things.filter((t) => t.group === "t").map((t) => [t.key, t.place]));
};

test("a row stands at the place the menu gives for the code its part says now, and moves when the code does", () => {
    const scene = onDrawing("flue", "room");
    assert.deepStrictEqual(placed([[1, 20], [2, "cold"], [3, 60], [4, "hot"]], scene), { a: "room", b: "flue" });
    assert.deepStrictEqual(placed([[1, 20], [2, "hot"], [3, 60], [4, "cold"]], scene), { a: "flue", b: "room" }, "the sockets changed roles");
});

test("a row whose part says a code the menu gives no place for stands nowhere, and a row with one plain entry stands there whatever the code", () => {
    assert.deepStrictEqual(placed([[1, 20], [2, "free"], [3, 60], [4, "hot"]], onDrawing("flue", "room")), { a: undefined, b: "flue" });
    assert.deepStrictEqual(placed([[1, 20], [2, "free"], [3, 60], [4, "free"]], { place: "boiler" }), { a: "boiler", b: "boiler" });
});

// ---- the fold of a card's settings: the words of its button are the page's texts, never the code's ----

const FOLD = { fold: { show: "Settings", hide: "Hide settings" } };

test("the button that folds a card's settings says what it holds while shut, and says to hide them while open", () => {
    assert.strictEqual(d.foldWords(FOLD, false, 9), "Settings");
    assert.strictEqual(d.foldWords(FOLD, true, 9), "Hide settings");
    // a page whose texts want the count says it, the count being the texts' to ask for
    assert.strictEqual(d.foldWords({ fold: { show: "Settings, {n}", hide: "Hide" } }, false, 9), "Settings, 9");
});

test("the words of the fold are the texts' and no other: a page whose texts have none says nothing on the button", () => {
    assert.strictEqual(d.foldWords({}, false, 9), "");
    assert.strictEqual(d.foldWords(undefined, true, 9), "");
    assert.strictEqual(d.foldWords({ fold: { hide: "Hide" } }, false, 3), "", "no words for the shut button, none made up");
});

// ---- the cycle a module of the menu runs: the shape the board's dictionary once carried as `machine`, said by the page ----

const CYCLE_MENU = {
    version: 1,
    modules: {
        k: {
            card: { title: { en: "the cycle", ru: "цикл" } },
            cycle: {
                stage: "phase",
                says: "doing",
                cause: "broke",
                press: ["go", "halt"],
                aside: ["list"],
                sections: [
                    { name: "first", title: { en: "first", ru: "первая" }, by: "", items: [{ of: ["n_one"], label: { en: "one", ru: "" } }, { of: ["n_a", "n_b"], label: { en: "a and b", ru: "" }, spec: "volts" }], notes: [[{ en: "note", ru: "" }, { en: "said", ru: "" }]] },
                    { name: "second", title: { en: "second", ru: "" }, by: "mode", items: [{ of: ["n_two"], label: { en: "two", ru: "" } }] },
                ],
            },
            rows: { size: { label: { en: "Size" }, level: [["*", "dim"]] } },
        },
    },
};
const CYCLE_DICT = {
    contract: 3,
    version: "k+1",
    things: [
        { card: "k", name: "phase", kinds: ["word"], words: ["a"] },
        { card: "k", name: "size", kinds: ["number"], unit: "mm", fixed: 12 },
        { card: "k", name: "free", kinds: ["number"], unit: "mm" },
    ],
    settings: [],
};

test("the buttons of a cycle stand on the card the menu names, and on the cycle's own card when it names none", () => {
    const own = d.cycleOf({ menu: CYCLE_MENU });
    assert.strictEqual(own.pressCard, "k", "no card named: the cycle's own");
    const elsewhere = JSON.parse(JSON.stringify(CYCLE_MENU));
    elsewhere.modules.k.cycle.press_card = "other";
    assert.strictEqual(d.cycleOf({ menu: elsewhere }).pressCard, "other");
    assert.strictEqual(d.cycleOf({ menu: elsewhere }).card, "k", "the rows of the cycle stay on its own card");
});

test("a module of the menu that runs a cycle says which rows are its stage, its words, its failure and its buttons, and the numbers it goes by stage by stage", () => {
    const cycle = d.cycleOf({ menu: CYCLE_MENU });
    assert.deepStrictEqual([cycle.card, cycle.fixed, cycle.stage, cycle.says, cycle.cause, cycle.press, cycle.aside], ["k", "k", "phase", "doing", "broke", ["go", "halt"], ["list"]]);
    assert.deepStrictEqual(cycle.sections, [
        { name: "first", by: "", items: [["n_one"], ["n_a", "n_b"]] },
        { name: "second", by: "mode", items: [["n_two"]] },
    ]);
    assert.deepStrictEqual(cycle.now, [], "what the card Now holds is each row's own `now` in the menu");
});

test("the words of a stage's numbers are the menu's, in the page's language, an empty ru reading as en", () => {
    const en = d.cycleOf({ menu: CYCLE_MENU });
    assert.deepStrictEqual(en.words.first, { title: "first", items: { n_one: "one", n_a: "a and b" }, specs: { n_a: "volts" }, notes: [["note", "said"]] });
    const ru = d.cycleOf({ menu: CYCLE_MENU, lang: "ru" });
    assert.deepStrictEqual([ru.words.first.title, ru.words.first.items.n_one, ru.words.second.title], ["первая", "one", "second"]);
});

test("a menu with no module that runs a cycle, and a page with no menu, run none", () => {
    assert.strictEqual(d.cycleOf({ menu: MENU }), null);
    assert.strictEqual(d.cycleOf({}), null);
    assert.strictEqual(d.cycleOf(null), null);
});

test("a number the image fixes is no row to read in its card, and a row the board sends is", () => {
    const s = d.state(CYCLE_DICT, { menu: CYCLE_MENU }, []);
    const by = (key) => s.things.find((t) => t.key === key);
    assert.strictEqual(by("size").fixed, true);
    assert.strictEqual(by("free").fixed, undefined);
    assert.deepStrictEqual(by("size").value, { number: { value: 12, unit: "mm" } }, "its value is the dictionary's own");
});

test("the board names the cycle the dictionary says, and the one the menu says where the dictionary says none", () => {
    const texts = { menu: CYCLE_MENU };
    const menus = new Board();
    menus.load(CYCLE_DICT, texts);
    assert.strictEqual(menus.machine.card, "k");
    const named = new Board();
    named.load({ ...CYCLE_DICT, machine: { card: "own", fixed: "own", stage: "s", says: "s", cause: "s", press: [], aside: [], sections: [], now: [] } }, texts);
    assert.strictEqual(named.machine.card, "own", "a dictionary of the board's own is the board's");
    const none = new Board();
    none.load(CYCLE_DICT, {});
    assert.strictEqual(none.machine, null);
});
