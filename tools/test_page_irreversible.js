// What the page does with a command that cannot be taken back: the board says so in its dictionary (`irreversible`), the page's row carries it,
// and the button's control says to ask first. The asking itself - the second press, the words - is the page's browser check, tools/page_parts.py.
//
//     node --test tools/test_page_irreversible.js
const test = require("node:test");
const assert = require("node:assert");

const { load } = require("./page_sources");
const scope = {};
load(["wire.js", "dictionary.js"], scope);
const d = scope.dictionary;
const wire = scope.wire;

const DICT = {
    contract: 9,
    version: "k+1",
    things: [
        { card: "energy", name: "reset", kinds: ["button"], can: ["press"], irreversible: true },
        { card: "energy", name: "mark", kinds: ["button"], can: ["press"] },
        { card: "energy", name: "lamp", kinds: ["on"], can: ["switch", "toggle"], irreversible: true },
    ],
};

const row = (at) => d.rowOf(DICT, { rows: {} }, new Array(DICT.things.length).fill(undefined), at);

test("a row the board calls irreversible says so, and a row it does not call so does not", () => {
    assert.strictEqual(row(0).irreversible, true);
    assert.strictEqual(Boolean(row(1).irreversible), false);
});

test("the button of an irreversible press is asked about first", () => {
    assert.deepStrictEqual(wire.control(row(0)), { path: "/press/0", words: "Reset", ask: true });
});

test("the button of a press that can be undone is sent as it always was, with nothing said of asking", () => {
    assert.deepStrictEqual(wire.control(row(1)), { path: "/press/1", words: "Mark" });
});

test("what is asked about is the command the row takes, whichever it is", () => {
    assert.strictEqual(wire.control(row(2)).ask, true);
    assert.strictEqual(wire.control({ ...row(2), irreversible: false }).ask, undefined);
});
