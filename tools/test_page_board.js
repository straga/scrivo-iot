// devices/burner/page/board.js: what the page knows of the board, kept outside the components' reactivity and told to them by
// address. The board here is made up: what is tested is the mechanism - which rows an event makes again,
// who is told, what is kept - and never a phrase.
//
//     node --test tools/test_page_board.js
const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");

const { load } = require("./page_sources");
const scope = {};
load(["wire.js", "dictionary.js", "board.js"], scope);
const { Board, same } = scope.boards;

// A made-up board: two plain numbers, a row with a part it says, a switch that hangs on another, a clock, a row its texts hide, and
// the rows of the board itself; and a setting that has a number the board worked out of it, its note.
const DICT = {
    contract: 3,
    version: "k+1",
    things: [
        { card: "a", name: "one", kinds: ["number"], unit: "u" },                                  // 0
        { card: "a", name: "two", kinds: ["number"], unit: "u" },                                  // 1
        { card: "a", name: "spread", part_of: "one", kinds: ["number"], unit: "u" },               // 2
        { card: "b", name: "sw", kinds: ["on"] },                                                  // 3
        { card: "b", name: "main", kinds: ["on"] },                                                // 4
        { card: "b", name: "left", kinds: ["seconds"] },                                           // 5
        { card: "b", name: "ghost", kinds: ["number"], unit: "u" },                                // 6
        { card: "system", name: "uptime", kinds: ["number"], unit: "ms" },                         // 7
        { card: "system", name: "free", kinds: ["number"], unit: "B" },                            // 8
        { card: "c", name: "k", kinds: ["number"], unit: "q", fixed: 0.5 },                        // 9
        { card: "settings", name: "up_kp", kinds: ["text"] },                                      // 10
        { card: "settings", name: "other", kinds: ["text"] },                                      // 11
        { card: "fan", name: "holding", kinds: ["word"], words: ["auto"] },                        // 12
        { card: "settings", name: "moves", part_of: "up_kp", kinds: ["number"] },                  // 13
        { card: "b", name: "lamp", kinds: ["on"] },                                                // 14
    ],
    settings: [
        { name: "up_kp", default: "400", kind: "whole", from: 0, to: 9000 },
        { name: "other", default: "1", kind: "whole", from: 0, to: 9 },
    ],
};
const TEXTS = {
    rows: {
        "a/one": { says: [["number", "{:.1} {unit}, spread {spread}"]] },
        "b/sw": { says: [["on&main=off", "held"], ["on", "on"], ["off", "off"]], level: [["on", "ok"]] },
        "b/ghost": { hide: ["*&main=on"] },
        "b/lamp": { says: [["on&left=0", "zero"], ["on", "on"]] },
    },
    // the note of up_kp is the words of a number the board worked out of it (its part `moves`); the page words it
    menu: { modules: { fan: { settings: { up_kp: { label: { en: "up" }, note: "moves" } }, notes: { "up_kp.moves": { says: [["number", { en: "a miss moves the gate {:sig} % up" }]] } } } } },
};

const rows = (b) => b.rows;
// what an event made again, as a sorted list of words
const moved = (b, records) => b.take(records).map(String).sort();
const loaded = (pairs = []) => {
    const b = new Board();
    b.take(pairs);
    b.load(DICT, TEXTS);
    return b;
};
const BASE = [[0, 1], [1, 2], [2, 7], [3, true], [4, false], [5, 30], [6, 5], [7, 1000], [8, 100000], [12, "auto"], [13, 2], [14, true]];

test("an event with one address makes the row that reads it again and leaves every other row the same object", () => {
    const b = loaded(BASE);
    const before = [...rows(b)];
    assert.deepStrictEqual(moved(b, [[1, 3]]), ["1"]);
    assert.notStrictEqual(rows(b)[1], before[1]);
    assert.strictEqual(rows(b)[1].value.number.value, 3);
    for (let at = 0; at < before.length; at++) if (at !== 1) assert.strictEqual(rows(b)[at], before[at], "row " + at + " is the same object");
});

test("a row is made again when a part it says changes, and a switch when the neighbour its words hang on does - not before", () => {
    const b = loaded(BASE);
    assert.deepStrictEqual(moved(b, [[2, 8]]), ["0"], "the part changes the row that says it, not itself: a part is no row");
    assert.strictEqual(rows(b)[0].state, "1.0 u, spread 8");
    // the switch is on, so its words ask what the neighbour is: main going on makes it read "on" and hides the ghost that hangs on main
    assert.deepStrictEqual(moved(b, [[4, true]]), ["3", "4", "6", "shape"]);
    assert.strictEqual(rows(b)[3].state, "on");
});

test("a switch that is off asks nothing of its neighbour, so the neighbour changing makes it nothing", () => {
    const b = loaded(BASE.map(([at, v]) => (at === 3 ? [3, false] : [at, v])));
    const before = rows(b)[3];
    assert.ok(!moved(b, [[4, true]]).includes("3"));
    assert.strictEqual(rows(b)[3], before);
});

test("a value that is what it was makes nothing again and tells nobody", () => {
    const b = loaded(BASE);
    const before = [...rows(b)];
    let told = 0;
    b.follows({ fn: () => told++ }, [0, 1, 3]);
    assert.deepStrictEqual(b.take([[0, 1], [1, 2], [3, true], [2, 7]]), []);
    assert.strictEqual(told, 0);
    assert.deepStrictEqual([...rows(b)], before);
});

test("a row that reads a neighbour is made again when it changes and comes out the same, and the same object is kept and nobody told", () => {
    // the lamp says "zero" when the clock is at nought and "on" otherwise: 30 becoming 31 changes the clock's row and not the lamp's words
    const b = loaded(BASE);
    const lamp = rows(b)[14];
    let told = 0;
    b.follows({ fn: () => told++ }, [14]);
    assert.deepStrictEqual(moved(b, [[5, 31]]), ["5"]);
    assert.strictEqual(rows(b)[14], lamp);
    assert.strictEqual(told, 0);
    assert.deepStrictEqual(moved(b, [[5, 0]]), ["14", "5"]);
    assert.strictEqual(rows(b)[14].state, "zero");
    assert.strictEqual(told, 1);
});

test("a component is told of the rows it read and of no others, once for an event, and lets go of what it stops reading", () => {
    const b = loaded(BASE);
    const calls = [];
    const a = { fn: () => calls.push("a") }, c = { fn: () => calls.push("c") };
    b.follows(a, [0, 1]);
    b.follows(c, [1, 5]);
    b.take([[1, 3], [5, 20]]);
    assert.deepStrictEqual(calls.sort(), ["a", "c"], "each once, though two of its rows changed");
    calls.length = 0;
    b.follows(c, [5]);
    b.take([[1, 4]]);
    assert.deepStrictEqual(calls, ["a"], "c no longer reads row 1");
    calls.length = 0;
    b.forgets(a);
    b.take([[1, 5], [5, 10]]);
    assert.deepStrictEqual(calls, ["c"]);
    assert.strictEqual(b.watchers.has(0), false, "nothing is kept for a key nobody reads");
});

test("what a component reads while it draws is what it depends on: a value, not the name a row is picked by", () => {
    const b = loaded(BASE);
    const reads = new Set();
    b.reader = reads;
    b.things.filter((t) => t.group === "a");
    assert.deepStrictEqual([...reads], ["shape"], "picking by group depends on which rows there are, and on nothing else");
    b.row(0).state;
    b.row(1).level;
    b.reader = null;
    assert.deepStrictEqual([...reads].map(String).sort(), ["0", "1", "shape"]);
});

test("a row the texts hide is gone from the list when they hide it, back when they stop, and the list's readers are told", () => {
    const b = loaded(BASE);
    const at = () => b.things.map((t) => t.at);
    assert.ok(at().includes(6));
    const seen = [];
    b.follows({ fn: () => seen.push(at()) }, ["shape"]);
    b.take([[4, true]]);
    assert.strictEqual(rows(b)[6], null);
    assert.ok(!at().includes(6));
    assert.strictEqual(seen.length, 1);
    b.take([[4, false]]);
    assert.ok(at().includes(6));
    assert.strictEqual(seen.length, 2);
});

test("what the board says of itself is one key, and the time it has run is counted from the moment it said it", () => {
    const real = Date.now;
    try {
        Date.now = () => 1000;
        const b = loaded(BASE);
        assert.strictEqual(b.since, 1000);
        const told = [];
        b.follows({ fn: () => told.push("head") }, ["head"]);
        Date.now = () => 2000;
        b.take([[8, 90000]]);
        assert.strictEqual(told.length, 1);
        assert.strictEqual(b.since, 1000, "the memory going down is not the uptime moving");
        b.take([[7, 5000]]);
        assert.strictEqual(b.since, 2000);
        assert.strictEqual(b.vitals.uptime, 5000);
    } finally {
        Date.now = real;
    }
});

test("a setting is made again when its own value or a number its note is worded from changes, and not for another setting", () => {
    const b = loaded([...BASE, [10, { text: "500" }], [11, { text: "2" }]]);
    const before = [...b.settings];
    const told = [];
    b.follows({ fn: () => told.push("settings") }, ["settings"]);
    assert.ok(before[0].note, "the knob has its note");
    assert.deepStrictEqual(moved(b, [[11, { text: "3" }]]), ["s:other"]);
    assert.deepStrictEqual(told, ["settings"], "and the components that draw the settings are told, once, that they changed");
    assert.notStrictEqual(b.settings[1], before[1]);
    assert.strictEqual(b.settings[0], before[0], "up_kp reads neither the other setting nor a number that moved");
    told.length = 0;
    assert.deepStrictEqual(moved(b, [[13, 4]]), ["s:up_kp"], "the note of the knob is worded from the number the board worked out of it");
    assert.notStrictEqual(b.settings[0].note, before[0].note);
    assert.deepStrictEqual(told, ["settings"]);
});

test("what arrives before the dictionary is kept for it, and what the dictionary fixes never changes", () => {
    const b = new Board();
    assert.deepStrictEqual(b.take([[0, 5], [9, 99]]), []);
    b.load(DICT, TEXTS);
    assert.strictEqual(b.rows[0].value.number.value, 5, "the value that came first is drawn as soon as the rows can be");
    assert.deepStrictEqual(b.take([[9, 100]]), [], "a fixed number is the dictionary's own");
    assert.strictEqual(b.all[9], 0.5);
});

test("loading tells every component that has been watching, whatever it read", () => {
    const b = new Board();
    let told = 0;
    b.follows({ fn: () => told++ }, [0]);
    b.load(DICT, TEXTS);
    assert.strictEqual(told, 1);
});

test("two things of the wire are the same when they say the same", () => {
    assert.ok(same(1, 1) && same("a", "a") && same(true, true) && same(null, null));
    assert.ok(same({ text: "x" }, { text: "x" }) && same(["a", "b"], ["a", "b"]));
    assert.ok(!same(1, 2) && !same({ text: "x" }, { text: "y" }) && !same(["a"], ["a", "b"]) && !same(0, null) && !same({ unknown: "a" }, "a"));
});

// ---- a count of seconds that goes on by itself: the board says it when it jumps, the page counts it on ----

const COUNTING = {
    ...DICT,
    things: [...DICT.things.map((t, at) => (at === 5 ? { ...t, counts: "down", clock: true } : t)), { card: "b", name: "for", kinds: ["seconds"], counts: "up" }], // 15
};
const T0 = 1_000_000;
const counting = (pairs) => {
    const b = new Board();
    b.take(pairs, T0);
    b.load(COUNTING, TEXTS, T0);
    return b;
};
const seconds = (b, at) => b.rows[at].value.seconds;

test("a count the board said once is counted on by the page, up or down, second by second", () => {
    const b = counting([[5, 30], [15, 100]]);
    assert.strictEqual(seconds(b, 5), 30);
    assert.deepStrictEqual(b.tick(T0 + 4000).map(String).sort(), ["15", "5"], "both rows are made again");
    assert.strictEqual(seconds(b, 5), 26, "a count down loses the seconds that went");
    assert.strictEqual(seconds(b, 15), 104, "a count up gains them");
    b.tick(T0 + 29_000);
    assert.strictEqual(seconds(b, 5), 1);
    b.tick(T0 + 31_000);
    assert.strictEqual(seconds(b, 5), 0, "down to nothing and no further");
    b.tick(T0 + 90_000);
    assert.strictEqual(seconds(b, 5), 0);
});

test("a second that has not gone by is not a change, and a tick that changes nothing makes nothing", () => {
    const b = counting([[5, 30]]);
    assert.deepStrictEqual(b.tick(T0 + 400), [], "less than half a second: still 30");
    assert.strictEqual(seconds(b, 5), 30);
});

test("a count the board says again is counted on from the new number, whenever it came", () => {
    const b = counting([[15, 100]]);
    b.tick(T0 + 10_000);
    assert.strictEqual(seconds(b, 15), 110);
    b.take([[15, 0]], T0 + 10_000);
    assert.strictEqual(seconds(b, 15), 0, "a new stage begins from nought");
    b.tick(T0 + 17_000);
    assert.strictEqual(seconds(b, 15), 7);
});

test("a count that loses its number stops being counted and is counted again when it comes back", () => {
    const b = counting([[5, 30]]);
    b.take([[5, null]], T0 + 2000);
    assert.deepStrictEqual(b.tick(T0 + 50_000), [], "nothing to count on");
    b.take([[5, 20]], T0 + 50_000);
    b.tick(T0 + 55_000);
    assert.strictEqual(seconds(b, 5), 15);
});

test("a number that is no count is not moved by the clock", () => {
    const b = counting([[0, 1], [1, 2]]);
    assert.deepStrictEqual(b.tick(T0 + 60_000), []);
    assert.strictEqual(b.rows[0].value.number.value, 1);
});

test("a count that came before the dictionary is counted from the moment it could be drawn", () => {
    const b = new Board();
    b.take([[15, 50]], T0);
    b.load(COUNTING, TEXTS, T0 + 1000);
    b.tick(T0 + 4000);
    assert.strictEqual(seconds(b, 15), 53, "three seconds after the rows could be drawn");
});
