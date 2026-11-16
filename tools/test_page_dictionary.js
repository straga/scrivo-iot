// devices/burner/page/dictionary.js: how a page makes words and levels out of a board's values of contract 3 and its own
// table of texts. The data here is made up: what is tested is the mechanism, never a phrase.
//
//     bun test ./tools/test_page_dictionary.js
const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");

const ROOT = path.dirname(__dirname);
const { load, menu, texts, read } = require("./page_sources");
const scope = {};
load(["wire.js", "dictionary.js"], scope);
const d = scope.dictionary;

// A made-up board: a row of codes, a row with a part, a switch, a count down, a row that hangs on another.
const DICT = {
    contract: 3,
    version: "k+1",
    things: [
        { card: "a", name: "mode", kinds: ["word", "unknown"], words: ["x", "y"], reasons: ["r1", "r2"] },
        { card: "a", name: "level", kinds: ["number"], unit: "u" },
        { card: "a", name: "spread", part_of: "level", kinds: ["number"], unit: "u" },
        { card: "b", name: "sw", kinds: ["on"] },
        { card: "b", name: "left", kinds: ["seconds"] },
        { card: "b", name: "main", kinds: ["on"] },
        { card: "a", name: "why", part_of: "mode", kinds: ["word"], words: ["p"] },
        { card: "a", name: "n", part_of: "mode", kinds: ["number"], unit: "things" },
    ],
};

const TEXTS = {
    rows: {
        "a/mode": { label: "Mode", says: [["x", "ex"], ["y", "why: {why}"], ["unknown", "none now"]], level: [["x", "ok"], ["unknown:r2", "bad"]] },
        "a/level": { label: "Level", says: [["number", "{:.1} u, spread {spread}"]], level: [["none", "dim"]] },
        "a/mode.why": { says: [["p", "p makes {n}"]] },
        "b/sw": { label: "Switch", says: [["on&main=off", "held"], ["on", "on"], ["off", "off"]], level: [["on", "ok"]] },
        "b/left": { label: "Left", says: [["0", "done"], ["seconds", "{:span}"]], level: [["0", "dim"]] },
    },
};

function values(pairs) {
    const all = new Array(DICT.things.length).fill(undefined);
    for (const [at, v] of pairs) all[at] = v;
    return all;
}

const said = (at, pairs) => d.render(DICT, TEXTS, values(pairs), at);

test("a code says the words its case is given, and its level", () => {
    assert.deepStrictEqual(said(0, [[0, "x"]]), { words: "ex", level: "ok" });
});

test("a number fills its template, with the places asked, and a part by its name", () => {
    assert.deepStrictEqual(said(1, [[1, 3.25], [2, 7]]), { words: "3.2 u, spread 7", level: null });
});

test("a number with a unit and no words of its own says the number and the unit", () => {
    const bare = { rows: {} };
    assert.strictEqual(d.render(DICT, bare, values([[1, 20]]), 1).words, "20 u");
    assert.strictEqual(d.render(DICT, bare, values([[1, 0.1]]), 1).words, "0.1 u");
});

test("a reason takes the words of its own case first, then those of its kind", () => {
    assert.deepStrictEqual(said(0, [[0, { unknown: "r2" }]]), { words: "none now", level: "bad" });
    assert.deepStrictEqual(said(0, [[0, { unknown: "r1" }]]), { words: "none now", level: null });
});

test("nothing held is a dash, and takes the level given to nothing", () => {
    assert.deepStrictEqual(said(1, [[1, null]]), { words: "-", level: "dim" });
    assert.deepStrictEqual(said(1, []), { words: "-", level: "dim" }, "never sent is nothing too");
});

test("a part named bare says its own words, which may name a part beside it", () => {
    assert.strictEqual(said(0, [[0, "y"], [6, "p"], [7, 3]]).words, "why: p makes 3");
});

test("words may hang on a row beside, and the first that holds in the order written is said", () => {
    assert.strictEqual(said(3, [[3, true], [5, false]]).words, "held");
    assert.strictEqual(said(3, [[3, true], [5, true]]).words, "on");
    assert.deepStrictEqual(said(3, [[3, false], [5, false]]), { words: "off", level: null });
});

test("nought takes its own words before those of its kind, and seconds read as a span", () => {
    assert.deepStrictEqual(said(4, [[4, 0]]), { words: "done", level: "dim" });
    assert.strictEqual(said(4, [[4, 45]]).words, "45 s");
    assert.strictEqual(said(4, [[4, 300]]).words, "5 min");
    assert.strictEqual(said(4, [[4, 262]]).words, "4 min 22 s");
});

test("a number rounds as the board's formatting did: a tie of the number as the board held it goes to the even digit", () => {
    const one = (n) => d.shown(n, ".1");
    assert.strictEqual(one(19.25), "19.2");
    assert.strictEqual(one(19.35), "19.4");
    assert.strictEqual(one(0.05), "0.1");
    assert.strictEqual(d.shown(2.5, ".0"), "2");
    assert.strictEqual(d.shown(3.5, ".0"), "4");
    assert.strictEqual(d.shown(-19.25, ".1"), "-19.2");
    assert.strictEqual(d.shown(262, "clock"), "4:22");
    assert.strictEqual(d.shown(7, "clock"), "0:07");
});

// ---- the state a page draws, made of the dictionary, the values and the texts ----

const BOARD = {
    contract: 3,
    version: "kit-1+abcd",
    device: "dev",
    board: "B",
    terminal: "/console",
    cards: ["system", "settings", "other", "c"],
    things: [
        { card: "system", name: "free", kinds: ["number"], unit: "bytes" },
        { card: "system", name: "uptime", kinds: ["number"], unit: "ms", once: true },
        { card: "system", name: "time", kinds: ["text", "unknown"], reasons: ["late"] },
        { card: "system", name: "low_memory", kinds: ["on"] },
        { card: "system", name: "used", kinds: ["number"], unit: "bytes" },
        { card: "c", name: "sw", kinds: ["on"], can: ["switch", "toggle"] },
        { card: "c", name: "knob", kinds: ["number", "word"], unit: "u", words: ["off"], can: ["hold"], range: { min: 0, max: 9, step: 1, at: 7 } },
        { card: "c", name: "slider", part_of: "knob", kinds: ["number"], unit: "u" },
        { card: "c", name: "go", kinds: ["button"], can: ["press"] },
        { card: "c", name: "k", kinds: ["number"], unit: "q", fixed: 0.5 },
        { card: "c", name: "slot", kinds: ["number", "unknown"], unit: "t", reasons: ["empty"], curves: ["slot_curve"] },
        { card: "c", name: "role", part_of: "slot", kinds: ["word", "text"], words: ["free", "hot"] },
        { card: "system", name: "net", kinds: ["word"], words: ["up"] },
        { card: "system", name: "signal", part_of: "net", kinds: ["number"], unit: "dBm" },
        { card: "settings", name: "mode", kinds: ["text"] },
        { card: "settings", name: "span", kinds: ["text"] },
        { card: "settings", name: "note", part_of: "mode", kinds: ["word"], words: ["fine"] },
    ],
    settings: [
        { name: "mode", default: "a", kind: "choice", choices: [{ value: "a" }, { value: "b", takes: true }] },
        { name: "span", card: "c", default: "60", kind: "seconds", from: 0, to: 600 },
    ],
};

const BOARD_TEXTS = {
    rows: {
        "system/time": { says: [["text", "{} here"], ["unknown:late", "no time"]] },
        "c/sw": { label: "Switch" },
        "c/knob": { label: "Knob", unit: "U" },
        "c/go": { label: "Go", says: [["none", "go"]] },
        "c/k": { label: "K", says: [["number", "{} of it"]], unit: "of it" },
        "c/slot": { label: "{role}", hide: ["*&role=free"], curves: { slot_curve: "an hour" } },
        "c/slot.role": { says: [["hot", "the hot one"], ["text", "{}"]] },
        "settings/mode.note": { says: [["fine", "fine with {span}"]] },
        "settings/span": { says: [["text", "{:span}"]] },
    },
    settings: {
        mode: { label: "Mode", choices: [{ value: "a", words: "ay" }, { value: "b", words: "bee", takes: "a word", means: "the word" }], note: "note" },
        span: { label: "Span", note: "" },
    },
};

function stateOf(pairs) {
    const values = new Array(BOARD.things.length).fill(undefined);
    for (const [at, v] of pairs) values[at] = v;
    return d.state(BOARD, BOARD_TEXTS, values);
}

const FULL = [[0, 48128], [1, 7774], [2, { text: "12:00" }], [3, false], [4, 274432], [5, true], [6, 31], [7, 31],
    [10, 61.5], [11, "hot"], [12, "up"], [13, -59], [14, { text: "b:x" }], [15, { text: "90" }], [16, "fine"]];

test("what the board is stands beside the rows, and the rows of the system that are no rows are not drawn", () => {
    const s = stateOf(FULL);
    assert.strictEqual(s.contract, 3);
    assert.deepStrictEqual([s.device, s.board, s.kit, s.terminal], ["dev", "B", "kit-1", "/console"]);
    assert.deepStrictEqual([s.free, s.used, s.uptime, s.low_memory, s.time], [48128, 274432, 7774, false, "12:00 here"]);
    assert.deepStrictEqual(s.things.map((t) => t.name), ["Switch", "Knob", "Go", "K", "the hot one", "net"]);
});

test("a thing carries its words, its level, its value as a page reads it, its commands, its slider and its curves", () => {
    const [sw, knob, go, k, slot, net] = stateOf(FULL).things;
    // the name a person reads is the page's word; the board's own name for the row (`key`) is what a page finds it by
    assert.deepStrictEqual(sw, { name: "Switch", key: "sw", at: 5, group: "c", state: "on", level: null, value: { on: true }, can: ["switch", "toggle"] });
    assert.deepStrictEqual(knob.range, { min: 0, max: 9, step: 1, value: 31, unit: "U" });
    assert.deepStrictEqual(knob.value, { number: { value: 31, unit: "U" } });
    assert.deepStrictEqual(go.value, { button: {} });
    assert.deepStrictEqual([k.state, k.value], ["0.5 of it", { number: { value: 0.5, unit: "of it" } }]);
    assert.deepStrictEqual(slot.curves, [{ name: "slot_curve", words: "an hour" }]);
    assert.deepStrictEqual(slot.value, { number: { value: 61.5, unit: "t" } });
    assert.strictEqual(net.signal, -59);
});

test("a row hidden by its texts is not drawn, and a reading with no number now is unknown with its words", () => {
    const s = stateOf([...FULL.filter(([at]) => at !== 11 && at !== 10), [11, "free"], [10, { unknown: "empty" }]]);
    assert.ok(!s.things.some((t) => t.group === "c" && t.name.includes("hot")), "a free slot drawn");
    const unknown = stateOf([...FULL.filter(([at]) => at !== 10), [10, { unknown: "empty" }]]).things.find((t) => t.curves);
    assert.ok(unknown, "the slot is not drawn at all");
    assert.deepStrictEqual(unknown.value, { unknown: {} });
});

test("the settings carry their label, their value as kept, their choices in words and what they mean now", () => {
    const [mode, span] = stateOf(FULL).settings;
    assert.deepStrictEqual(mode, {
        name: "mode", label: "Mode", value: "b:x",
        options: [{ value: "a", words: "ay" }, { value: "b", words: "bee", takes: "a word", means: "the word" }],
        note: "fine with 1 min 30 s",
    });
    // a span of seconds carries the unit it is counted in
    assert.deepStrictEqual(span, { name: "span", label: "Span", value: "90", seconds: [0, 600], card: "c", unit: "s", note: "1 min 30 s" });
});

test("a board that sent nothing yet is drawn with dashes, not left out", () => {
    const s = stateOf([]);
    assert.deepStrictEqual(s.things.map((t) => t.state), ["-", "-", "go", "0.5 of it", "-", "-"]);
});

test("a board whose dictionary is not the one the page draws by is refused by a code with both versions", () => {
    assert.strictEqual(d.stale({ version: "k+1" }, { version: "k+1" }), null);
    assert.deepStrictEqual(d.stale({ version: "k+1" }, { version: "k2+9" }), { kind: "speaksDictionary", version: "k2+9", mine: "k+1" });
    assert.strictEqual(d.stale({ version: "k+1" }, {}).kind, "namesNoDictionary", "a dictionary with no version is not the one held");
});

test("a setting the board's dictionary folds under the everyday ones carries its level, and one that does not carries none", () => {
    const folded = { ...BOARD, settings: [...BOARD.settings, { name: "tune", card: "c", default: "1", kind: "whole", from: 1, to: 9, level: "tuning" }] };
    const values = new Array(BOARD.things.length).fill(undefined);
    const [mode, span, tune] = d.state(folded, BOARD_TEXTS, values).settings;
    assert.strictEqual(tune.level, "tuning", "the level the dictionary sends did not reach the page");
    assert.ok(!("level" in mode) && !("level" in span), "an everyday setting was given a level");
});

// what a choice bears on, as the board says it, reaches the page's setting untouched - and a choice that bears on
// nothing is what it was
test("a choice the board says shows settings or needs a role carries both to the page, and a plain one carries neither", () => {
    const dict = {
        ...BOARD,
        settings: [
            { name: "mode", default: "a", kind: "choice", choices: [
                { value: "a", shows: ["span"], needs: { among: ["mode"], holds: "a" } },
                { value: "b", takes: true },
            ] },
            BOARD.settings[1],
        ],
    };
    const made = d.state(dict, BOARD_TEXTS, values([])).settings[0];
    assert.deepStrictEqual(made.options[0].shows, ["span"]);
    assert.deepStrictEqual(made.options[0].needs, { among: ["mode"], holds: "a" });
    assert.ok(!("shows" in made.options[1]) && !("needs" in made.options[1]), JSON.stringify(made.options[1]));
});

test("a setting says the unit the board gives its number, or seconds for a span of them, and a choice says none", () => {
    const dict = {
        ...BOARD,
        settings: [
            { name: "a", card: "c", default: "1", kind: "whole", from: 1, to: 9, unit: "Pa" },
            { name: "b", card: "c", default: "1", kind: "seconds", from: 1, to: 9 },
            { name: "c", card: "c", default: "1", kind: "whole", from: 1, to: 9 },
            { name: "d", card: "c", default: "x", kind: "choice", choices: [{ value: "x" }] },
        ],
    };
    const [a, b, c, dd] = d.state(dict, BOARD_TEXTS, values([])).settings;
    assert.deepStrictEqual([a.unit, b.unit, c.unit, dd.unit], ["Pa", "s", undefined, undefined]);
});

// ---- what the card Now is made of and what a knob's line is worked out of are the board's to say ----
//
// Made-up names throughout: the board says which rows Now takes, in what order and which one stands beside which (`machine.now`), and which
// settings and numbers a setting's line is worked out of (`effect`); the page says what to call a row there and how to show the number beside it.
const NOW_DICT = {
    contract: 3,
    version: "k+1",
    things: [
        { card: "p", name: "one", kinds: ["word"], words: ["up", "down"] },
        { card: "p", name: "beside", kinds: ["number"], unit: "mV" },
        { card: "q", name: "s1", kinds: ["number"], unit: "u" },
        { card: "q", name: "tag", part_of: "s1", kinds: ["word"], words: ["hot", "cold"] },
        { card: "q", name: "s2", kinds: ["number"], unit: "u" },
        { card: "q", name: "tag", part_of: "s2", kinds: ["word"], words: ["hot", "cold"] },
        { card: "p", name: "left_out", kinds: ["number"] },
    ],
    machine: { card: "p", now: [{ card: "p", row: "one", with: "beside" }, { card: "q", part: "tag", is: "cold" }] },
};
const NOW_TEXTS = {
    rows: {},
    now: { "p/one": { as: "the one", spec: "volts" }, "q/tag=cold": { as: "the cold one" } },
};
const nowValues = (pairs) => {
    const all = new Array(NOW_DICT.things.length).fill(undefined);
    for (const [at, v] of pairs) all[at] = v;
    return all;
};
const rowNow = (at, pairs) => d.rowOf(NOW_DICT, NOW_TEXTS, nowValues(pairs), at);

test("a row the board lists for the card Now carries its place in the list and the page's word for it", () => {
    const one = rowNow(0, [[0, "up"]]);
    assert.strictEqual(one.now, 0);
    assert.strictEqual(one.as, "the one");
});

test("a row is found in the list by a part that holds a code, and the row whose part holds another code is not", () => {
    const cold = rowNow(2, [[2, 9], [3, "hot"], [4, 3], [5, "cold"]]);
    const hot = rowNow(4, [[2, 9], [3, "hot"], [4, 3], [5, "cold"]]);
    assert.strictEqual(hot.now, 1, "s2 holds cold");
    assert.strictEqual(hot.as, "the cold one");
    assert.ok(!("now" in cold), "s1 holds hot and is no row of the card");
});

test("a row the board does not list is no row of the card Now", () => {
    assert.ok(!("now" in rowNow(6, [[6, 1]])));
});

test("the number beside a row's word is said as the page's table says, and stands only when that row has a number", () => {
    assert.strictEqual(rowNow(0, [[0, "up"], [1, 900]]).alongside, "0.90 V");
    assert.ok(!("alongside" in rowNow(0, [[0, "up"]])), "no number - nothing beside the word");
});

test("a board that lists no rows for the card Now draws no row for it", () => {
    const bare = { ...NOW_DICT, machine: { card: "p" } };
    assert.ok(!("now" in d.rowOf(bare, NOW_TEXTS, nowValues([[0, "up"]]), 0)));
    assert.ok(!("now" in d.rowOf({ ...NOW_DICT, machine: undefined }, NOW_TEXTS, nowValues([[0, "up"]]), 0)));
});

// A setting whose note is the words of a number the board worked out of it: the board says `moves`, the page words it and works out none
const NOTE_DICT = {
    contract: 3,
    version: "k+1",
    things: [
        { card: "settings", name: "k_one", kinds: ["text"] },
        { card: "settings", name: "moves", part_of: "k_one", kinds: ["number"], unit: "%" },
        { card: "settings", name: "miss", part_of: "k_one", kinds: ["number"], unit: "Pa", fixed: 10 },
        { card: "settings", name: "takes", part_of: "k_one", kinds: ["number", "unknown"], reasons: ["no_step"], unit: "s" },
    ],
    settings: [{ name: "k_one", default: "200", kind: "whole", from: 0, to: 900 }],
};
const noted = (note, says) => ({ menu: { modules: { g: { settings: { k_one: { label: { en: "k" }, note } }, notes: { [`k_one.${note}`]: { says } } } } } });
const NOTE_TEXTS = noted("moves", [["number", { en: "a miss of {miss} Pa moves it {moves:sig} % up" }]]);
const noteOf = (all, texts = NOTE_TEXTS) => d.settingOf(NOTE_DICT, texts, all, NOTE_DICT.settings[0]).note;

test("a quantity is said as two figures from 1 up and one under it", () => {
    for (const [number, said] of [[14.4, "14"], [0.5866, "0.6"], [0.7824, "0.8"], [0.0587, "0.06"], [0.0147, "0.01"], [8, "8"], [0.0004, "0.0004"], [123.4, "120"]]) {
        assert.strictEqual(d.shown(number, "sig"), said, String(number));
    }
});

test("a setting's note says the number the board worked out of it, in the figures a person reads, and the size of the miss it was worked for", () => {
    // 0.3 * 10 Pa / 5.1 Pa a percent = 0.59 %, worked out by the board; the size of the miss is the board's too, fixed in its image
    assert.strictEqual(noteOf([undefined, 0.5866, 10]), "a miss of 10 Pa moves it 0.6 % up");
    assert.strictEqual(noteOf([undefined, 0.0587, 10]), "a miss of 10 Pa moves it 0.06 % up");
});

test("a number the board has not said gives no note, and a note the menu words with nothing says nothing", () => {
    assert.strictEqual(noteOf([undefined, undefined, 10]), undefined);
    const none = noted("takes", [["number", { en: "from here to there in {} s" }], ["unknown:no_step", { en: "" }]]);
    assert.strictEqual(noteOf([undefined, undefined, 10, { unknown: "no_step" }], none), undefined);
    assert.strictEqual(noteOf([undefined, undefined, 10, 14.4], none), "from here to there in 14.4 s");
});

test("the page works out nothing of a knob: a setting that is given numbers and no words for them has no note", () => {
    assert.strictEqual(noteOf([undefined, 0.5866, 10], { menu: { modules: { g: { settings: { k_one: { label: { en: "k" } } } } } } }), undefined);
});

// ---- a row's short form for the drawing ----
// A row the drawing says in a few words - "eco until 06:00", a clock with a sign before it - carries a `short` the menu words, filled like the label: its own value, its parts and the rows of its card by name.

test("a row carries the short form its menu words, filled with its own value, its parts and a row beside it", () => {
    const texts = { rows: { "a/level": { label: "Level", says: [["number", "{:.1} u"]], short: "L {:.1} s{spread:.1} m={mode}" }, "a/mode": { says: [["x", "ex"]] } } };
    const all = values([[0, "x"], [1, 3.14], [2, 0.5]]);
    assert.strictEqual(d.rowOf(DICT, texts, all, 1).short, "L 3.1 s0.5 m=ex");
});

test("a row the menu gives no short form has none, and a short form that is no text is not one", () => {
    const all = values([[0, "x"], [1, 3.14], [2, 0.5]]);
    assert.ok(!("short" in d.rowOf(DICT, TEXTS, all, 1)));
    const odd = { rows: { "a/level": { label: "Level", says: [["number", "{:.1} u"]], short: 7 } } };
    assert.ok(!("short" in d.rowOf(DICT, odd, all, 1)));
});

// ---- the words that are parts of a row, by their codes ----
// `why` of the automaton's `doing` is the code of the cause; the list of what asks for a person reads it from the row and never from the row's words.

test("a row carries the parts of it that are words, by name and by their codes - and a part that is a number is not one", () => {
    const all = values([[0, "y"], [6, "p"], [7, 3]]);
    assert.deepStrictEqual(d.rowOf(DICT, TEXTS, all, 0).parts, { why: "p" });
});

test("a row with no word among its parts, or whose parts the board has not said, carries no `parts`", () => {
    assert.ok(!("parts" in d.rowOf(DICT, TEXTS, values([[1, 3.14], [2, 0.5]]), 1)), "a number part");
    assert.ok(!("parts" in d.rowOf(DICT, TEXTS, values([[0, "x"]]), 0)), "a part nobody has said");
    assert.ok(!("parts" in d.rowOf(DICT, TEXTS, values([[3, true]]), 3)), "a row with no parts at all");
});

test("a setting of tenths of a second is a number of seconds with its tenth, drawn as a number with the unit s", () => {
    const dict = {
        ...BOARD,
        settings: [
            { name: "dose", card: "c", default: "2", kind: "tenths", from: 1, to: 1200 },
            { name: "pause", card: "c", default: "20", kind: "seconds", from: 1, to: 600 },
        ],
    };
    const [dose, pause] = d.state(dict, BOARD_TEXTS, values([])).settings;
    assert.deepStrictEqual(dose.seconds, [0.1, 120]);
    assert.strictEqual(dose.tenths, true);
    assert.strictEqual(dose.unit, "s");
    assert.strictEqual(pause.tenths, undefined, "a whole span of seconds takes no tenths");
});

// ---- how busy each core of the board is: two rows of the board's card a device puts last ----

const WITH_CORES = { ...BOARD, things: [...BOARD.things, { card: "system", name: "core_0", kinds: ["number"], unit: "%" }, { card: "system", name: "core_1", kinds: ["number"], unit: "%" }] };

function coresOf(pairs, board = WITH_CORES) {
    const values = new Array(board.things.length).fill(undefined);
    for (const [at, v] of [...FULL, ...pairs]) values[at] = v;
    return d.state(board, BOARD_TEXTS, values);
}

test("the load of each core stands beside what the board is, one number a core, and its rows are not drawn as rows of the system", () => {
    const at = BOARD.things.length;
    const s = coresOf([[at, 21], [at + 1, 4]]);
    assert.deepStrictEqual(s.cores, [21, 4]);
    assert.ok(!s.things.some((t) => /core/.test(t.key)), "the load of a core is drawn as a row: " + s.things.map((t) => t.key));
});

test("a core that has not been measured yet says no number, and the other one still says its own", () => {
    const at = BOARD.things.length;
    assert.deepStrictEqual(coresOf([[at, null], [at + 1, 4]]).cores, [null, 4]);
});

test("a board that has no rows for its cores says no cores, not a pair of nothings", () => {
    assert.strictEqual(stateOf(FULL).cores, null);
});
