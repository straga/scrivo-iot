// A clock that has not begun: the board says `{"unknown": "not_begun"}` for the blow behind a burning out, the watch over a flame that has come back and the climb to a full step
// while their thing has not begun, and the page draws a dash - from the word the MENU gives, on the real dictionary and the real menu of the burner - and does not count it down. From the
// second the thing begins the board says a number and the page counts it down as it counts every clock.
//
//     node --test tools/test_page_clock.js
const test = require("node:test");
const assert = require("node:assert");
const { execSync } = require("node:child_process");

const { ROOT, load, menu, texts } = require("./page_sources");
const scope = {};
load(["form.js", "wire.js", "dictionary.js", "board.js", "burning.js"], scope);
const d = scope.dictionary;
const { Board } = scope.boards;

const dict = JSON.parse(execSync("cargo +stable run -q -p burner_api --example served_json", { cwd: ROOT, encoding: "utf8", maxBuffer: 1 << 26 }));
const all = texts("burner");
all.menu = menu("burner");
const at = (name) => dict.things.findIndex((t) => t.card === "automaton" && t.name === name && !t.part_of);
const words = (said, name) => d.render(dict, all, said, at(name)).words;

const MAY_NOT_HAVE_BEGUN = ["blowing_clear_ends_in", "fed_after_flame_for", "full_step_in"];
const ALWAYS_RUNNING = ["lighting_in", "dose_ends_in", "igniter_off_in", "attempt_ends_in", "auger_switches_in", "relight_in", "no_relight_burning_off_in", "burning_off_in", "burning_out_ends_in"];

test("a clock that has not begun is a dash on the page, in each of the three, and its reason is not drawn as its code", () => {
    for (const name of MAY_NOT_HAVE_BEGUN) {
        const said = new Array(dict.things.length).fill(undefined);
        said[at(name)] = { unknown: "not_begun" };
        assert.strictEqual(words(said, name), "-", name);
    }
});

test("a clock that has begun is its seconds, in each of the three", () => {
    for (const name of MAY_NOT_HAVE_BEGUN) {
        const said = new Array(dict.things.length).fill(undefined);
        said[at(name)] = 20;
        assert.match(words(said, name), /20/, name);
    }
});

test("every clock the board counts is a count down, and only the three may be unknown", () => {
    for (const name of [...MAY_NOT_HAVE_BEGUN, ...ALWAYS_RUNNING]) {
        const thing = dict.things[at(name)];
        assert.strictEqual(thing.counts, "down", name);
        assert.strictEqual(thing.kinds.includes("unknown"), MAY_NOT_HAVE_BEGUN.includes(name), name);
    }
});

const T0 = 1_000_000;
const counting = (name, first) => {
    const b = new Board();
    b.take([[at(name), first]], T0);
    b.load(dict, all, T0);
    return b;
};

test("a count the board said is counted down by the page, and one it said is unknown is counted by nobody", () => {
    for (const name of MAY_NOT_HAVE_BEGUN) {
        const b = counting(name, 20);
        b.tick(T0 + 4000);
        assert.strictEqual(b.values[at(name)], 16, `${name}: a running clock loses the seconds that went`);
        b.take([[at(name), { unknown: "not_begun" }]], T0 + 5000);
        b.tick(T0 + 30_000);
        assert.deepStrictEqual(b.values[at(name)], { unknown: "not_begun" }, `${name}: a clock that has not begun is not counted down`);
        b.take([[at(name), 20]], T0 + 31_000);
        b.tick(T0 + 33_000);
        assert.strictEqual(b.values[at(name)], 18, `${name}: begun, it counts from the number the board said`);
    }
});

test("a clock that has not begun is no clock running: the row of what comes next does not take it", () => {
    const things = MAY_NOT_HAVE_BEGUN.map((name) => ({ group: "automaton", name, clock: true, state: "-", value: { unknown: "not_begun" } }));
    assert.deepStrictEqual(scope.burning.next(things), { value: "-", level: "dim" });
});
