// devices/burner/page/burning.js: what the main screen shows of the automaton, read off the board's dictionary and the page's table of texts -
// which stage of the strip is lit, which numbers stand in which stage, which of a card's settings a person can change, what
// comes next, the card Now - and how a control, the contract and the board's address are read. The scheme's own are in test_page_scheme.js.
//
//     node --test tools/test_page_burning.js
const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");

const ROOT = path.dirname(__dirname);
const { load, menu, texts } = require("./page_sources");
const scope = {};
load(["wire.js", "dictionary.js", "burning.js"], scope);
const burning = scope.burning;
const wire = scope.wire;

// the burner's dictionary as its image carries it, from the crate that makes it, and the page's own table of texts
const BURNER = JSON.parse(require("node:child_process").execSync(
    "cargo run -q -p burner_api --example first_burner_json", { cwd: ROOT, encoding: "utf8" }));
const TEXTS = texts();
TEXTS.menu = menu();
// the automaton's words are the menu's: what the page says of the first burner's rows is read from there
const STAGE = scope.dictionary.textsOf(TEXTS, { card: "automaton", name: "stage" });

// A CELL IS KNOWN BY ITS KEY - the board's code of the stage - and said by its words, which are the menu's and are never compared here
const lit = (stage, brokeAt) => burning.strip(STAGE.strip, stage, brokeAt).filter((c) => c.now).map((c) => c.key);
const marked = (stage, brokeAt) =>
    burning.strip(STAGE.strip, stage, brokeAt).filter((c) => c.level).map((c) => `${c.key} ${c.level}`);

test("the strip lights the stage the automaton is in", () => {
    assert.deepStrictEqual(burning.strip(STAGE.strip, "stop").map((c) => c.key),
        ["stop", "purge", "kindling", "rising", "working", "burning_off"]);
    assert.deepStrictEqual(lit("purge"), ["purge"]);
    assert.deepStrictEqual(lit("kindling"), ["kindling"]);
    assert.deepStrictEqual(lit("burning_off"), ["burning_off"]);
});

test("a fault lights in red the stage it happened on", () => {
    assert.deepStrictEqual(lit("fault", "kindling"), ["kindling"]);
    assert.deepStrictEqual(marked("fault", "kindling"), ["kindling bad"]);
    assert.deepStrictEqual(marked("fault", "smouldering"), ["working bad"]);
});

test("burning off after a failure lights burning off and marks the stage it happened on", () => {
    assert.deepStrictEqual(lit("burning_off", "kindling"), ["burning_off"]);
    assert.deepStrictEqual(marked("burning_off", "kindling"), ["kindling warn"]);
});

test("a stage that went well is marked nothing, and neither is a burning off a person asked for", () => {
    for (const stage of ["stop", "purge", "kindling", "rising", "working", "smouldering", "burning_off"]) {
        assert.deepStrictEqual(marked(stage), [], stage);
    }
    // the stage it broke on is the board's word; a stage the strip has no cell for marks nothing
    assert.deepStrictEqual(marked("fault", "nonsense"), []);
    assert.deepStrictEqual(marked("fault", "fault"), []);
});

test("every cell of the strip is one word, so no cell wraps to a second line", () => {
    for (const cell of burning.strip(STAGE.strip, "stop")) {
        assert.ok(!/\s/.test(cell.words), `'${cell.words}' breaks in two and makes the strip twice as tall`);
    }
});

test("smouldering lights working: the fire is carried, only lower", () => {
    assert.deepStrictEqual(lit("smouldering"), ["working"]);
});

test("a fault the board has not said the stage of lights nothing, and a stage nobody knows lights none either", () => {
    assert.deepStrictEqual(lit("fault"), []);
    assert.deepStrictEqual(lit("nonsense"), []);
    assert.deepStrictEqual(lit(undefined), []);
});

// A strip is read off the page's table and the board's words: the table's cells, which stage carries another's cell, which stages light the
// cell the fault happened on and in what level - and no stage is written in the code. The stages here are made up.
test("a strip names no stage of its own: its cells, its stages that carry another's cell and its marks are the table's", () => {
    const table = { cells: [["a", "A"], ["b", "B"], ["c", "C"]], as: { b2: "b" }, cause: ["broke"], marks: { broke: "bad", late: "warn" } };
    const cells = (stage, cause) => burning.strip(table, stage, cause);
    const on = (stage, cause) => cells(stage, cause).filter((c) => c.now).map((c) => c.key);
    const levels = (stage, cause) => cells(stage, cause).filter((c) => c.level).map((c) => `${c.key} ${c.level}`);
    assert.deepStrictEqual(on("a"), ["a"]);
    assert.deepStrictEqual(on("b2"), ["b"], "a stage that carries another's cell lights that cell");
    assert.deepStrictEqual(on("broke", "c"), ["c"]);
    assert.deepStrictEqual(levels("broke", "c"), ["c bad"]);
    assert.deepStrictEqual(on("late", "a"), [], "a stage of no cell lights none");
    assert.deepStrictEqual(levels("late", "a"), ["a warn"]);
    assert.deepStrictEqual(levels("a", "b"), [], "a stage the table marks nothing for marks nothing");
    assert.deepStrictEqual(burning.strip(undefined, "a"), [], "no table - no cells");
});

// THE TABLE OF WHAT THE AUTOMATON GOES BY, made of the burner's own dictionary and the page's table of texts: the numbers said on the card here
// are what the board holds in 6.7's table, the rest its defaults and the image's
const SET = {
    auto_purge: "30", auto_igniter: "30", auto_wait: "90", auto_tries: "8", auto_kindle_pa: "42", auto_pa: "51", auto_dose: "32",
    auto_rise_s: "90", auto_no_flame_s: "30", auto_flue_win_s: "600", auto_burn_off_s: "600", auto_feed_on_s: "1", auto_feed_off_s: "24",
    auto_pump_on_c: "45", auto_dose_again: "10", auto_relight_1: "20", auto_relight_2: "10", auto_relight_3: "10",
};
// what the page reads of the burner with the settings SAID (name: text), as it draws it
const read = (told = SET) => {
    const values = [];
    for (const [name, text] of Object.entries(told)) values[BURNER.things.findIndex((t) => t.card === "settings" && t.name === name)] = { text };
    return scope.dictionary.state(BURNER, TEXTS, values);
};
// the table, CHANGE being a way to put the settings and the numbers of the image the page is given other than the board gave them
const table = (told = SET, change = (settings, fixed) => [settings, fixed]) => {
    const state = read(told);
    const [settings, fixed] = change(state.settings, state.things.filter((t) => t.group === BURNER.machine.fixed));
    return burning.parameters(BURNER.machine, scope.dictionary.cycleOf(TEXTS).words, settings, fixed);
};
// A STAGE AND AN ITEM ARE KNOWN BY THEIR NAMES - the board's codes: a stage's, a setting's, a number of the image's - and said by the menu's words, which are never compared here
const part = (stage, name, told, change) => table(told, change).find((s) => s.name === stage)?.parts.find((p) => p.name === name);
const shown = (p) => p && { value: p.value, set: p.set };
const partNames = (stage, told) => table(told).find((s) => s.name === stage).parts.map((p) => p.name);

test("the lighting says the plug burns with air until a flame: the igniter's most and the wait, both set on the card", () => {
    // the plug burns until the flame, and its setting is the most an attempt gives it
    assert.deepStrictEqual(shown(part("lighting", "auto_igniter")), { value: "30 s", set: true });
    assert.deepStrictEqual(shown(part("lighting", "auto_wait")), { value: "90 s", set: true });
});

test("the lighting says how a lost flame is lit again: the card's three doses and the flue it goes without the igniter over", () => {
    assert.deepStrictEqual(shown(part("lighting", "auto_relight_1")), { value: "20 · 10 · 10 s", set: true });
    assert.deepStrictEqual(shown(part("lighting", "relight_hot_c")), { value: "45 °C", set: false });
    assert.deepStrictEqual(shown(part("lighting", "auto_dose_again")), { value: "10 %", set: true });
});

test("the pump runs from the water set on the card", () => {
    assert.deepStrictEqual(shown(part("pump", "auto_pump_on_c")), { value: "45 °C", set: true });
});

test("a number set on the card is marked as set there, with its unit", () => {
    assert.deepStrictEqual(shown(part("purge", "auto_purge")), { value: "30 s", set: true });
    assert.deepStrictEqual(shown(part("lighting", "auto_tries")), { value: "8", set: true });
    assert.deepStrictEqual(shown(part("working", "auto_pa")), { value: "51 Pa", set: true });
});

test("lighting shows its own draught, and the climb runs from it to the step's", () => {
    assert.deepStrictEqual(shown(part("lighting", "auto_kindle_pa")), { value: "42 Pa", set: true });
    assert.deepStrictEqual(shown(part("rising", "auto_kindle_pa")), { value: "42 Pa", set: true });
    assert.deepStrictEqual(shown(part("rising", "auto_pa")), { value: "51 Pa", set: true });
    assert.deepStrictEqual(shown(part("working", "auto_pa")), { value: "51 Pa", set: true });
});

test("a board that does not send the draught of lighting says so, and the step's draught does not stand in", () => {
    const got = part("lighting", "auto_kindle_pa", SET, (settings, fixed) => [settings.filter((s) => s.name !== "auto_kindle_pa"), fixed]);
    assert.deepStrictEqual({ missing: got.missing, set: got.set }, { missing: true, set: false });
});

// the lighting pours one dose, rising feeds nothing for as long as the card says, and the work
// feeds by a pair of its own - the table says which number feeds which
test("the work feeds by its own pair, and rising feeds nothing for the time set on the card", () => {
    assert.deepStrictEqual(shown(part("working", "auto_feed_on_s")), { value: "1 s", set: true });
    assert.deepStrictEqual(shown(part("working", "auto_feed_off_s")), { value: "24 s", set: true });
    assert.deepStrictEqual(shown(part("rising", "auto_rise_s")), { value: "90 s", set: true });
    assert.ok(!partNames("rising").some((n) => /feed|dose/.test(n)), partNames("rising").join(", "));
    assert.deepStrictEqual(shown(part("lighting", "auto_dose")), { value: "32 s", set: true });
});

test("the lighting's dose is not shown as the work's feed", () => {
    const working = table().find((s) => s.name === "working").parts;
    assert.ok(!working.some((p) => p.value === "32 s" || p.value === "15 s"), JSON.stringify(working));
});

test("a number fixed in the image is marked as fixed", () => {
    assert.deepStrictEqual(shown(part("burning_off", "purge_after_s")), { value: "20 s", set: false });
});

// the fire burns out on the step's draught, a lost flame is waited for and the flue is given minutes,
// all three set on the card
test("burning off burns out on the step's draught, and the waits for a flame and for the flue are set on the card", () => {
    assert.deepStrictEqual(shown(part("burning_off", "auto_pa")), { value: "51 Pa", set: true });
    // burning out ends by the fire, a minute without a flame; its ceiling is set on the card
    assert.deepStrictEqual(shown(part("burning_off", "burn_out_min_s")), { value: "60 s", set: false });
    assert.deepStrictEqual(shown(part("burning_off", "auto_burn_off_s")), { value: "600 s", set: true });
    // a flame dark past its wait is lit again, not given up on: the wait stands with the lighting
    assert.deepStrictEqual(shown(part("lighting", "auto_no_flame_s")), { value: "30 s", set: true });
    assert.equal(part("gives_up", "auto_no_flame_s"), undefined, "the wait for a flame is not among what the burner gives up on");
    // the flue is given a window of minutes, and what it must do in it - stay cold, or fall - is said beside it
    assert.deepStrictEqual(shown(part("gives_up", "auto_flue_win_s")), { value: "600 s", set: true });
});

// a fixed number is read off its value and never off its words
test("a number fixed in the image is read off its value, whatever its words say", () => {
    const got = part("burning_off", "purge_after_s", SET,
        (settings, fixed) => [settings, fixed.map((r) => (r.key === "purge_after_s" ? { ...r, state: "about twenty seconds" } : r))]);
    assert.deepStrictEqual(shown(got), { value: "20 s", set: false });
});

test("a fixed number with words and no value is said not to be sent, not shown as its words", () => {
    const got = part("burning_off", "purge_after_s", SET,
        (settings, fixed) => [settings, fixed.map((r) => (r.key === "purge_after_s" ? { ...r, value: undefined } : r))]);
    assert.deepStrictEqual({ missing: got.missing, set: got.set }, { missing: true, set: false });
});

test("a number the board did not send says so, and is never undefined", () => {
    const stages = burning.parameters(BURNER.machine, scope.dictionary.cycleOf(TEXTS).words, [], []);
    assert.ok(stages.length >= 5, "the table vanished with the numbers");
    for (const s of stages) {
        for (const p of s.parts) {
            // a number not sent is none and says so by `missing`: the page puts the menu's words where it stands
            assert.ok(p.value === null ? p.missing === true : !/undefined|NaN/.test(p.value), `${s.stage} / ${p.words}: '${p.value}'`);
        }
    }
});


test("a relay's button offers what the value is not", () => {
    assert.strictEqual(wire.relayButton({ state: "on - off by itself in 4:22", value: { on: true } }), "off");
    assert.strictEqual(wire.relayButton({ state: "on", value: { on: false } }), "on");
});


// a button says what it wants - on, off, or a press - and never "switch over"; by
// the thing's address, never its name
test("a relay's button sends the state it offers, on or off, to the relay's address, and never a toggle", () => {
    const relay = (on) => ({ name: "pump", at: 70, can: ["switch", "toggle"], value: { on } });
    assert.deepStrictEqual(wire.control(relay(false)), { path: "/on/70", words: "on" });
    assert.deepStrictEqual(wire.control(relay(true)), { path: "/off/70", words: "off" });
});

test("a button of the automaton is pressed at its address", () => {
    assert.deepStrictEqual(wire.control({ name: "stop", at: 38, can: ["press"], value: { button: {} } }), { path: "/press/38", words: "Stop" });
});

test("a thing that answers to no command, or has no address, has no button", () => {
    assert.strictEqual(wire.control({ name: "water", at: 3, can: [], value: { number: { value: 40, unit: "°C" } } }), null);
    assert.strictEqual(wire.control({ name: "wifi" }), null);
    assert.strictEqual(wire.control({ name: "only toggle", at: 4, can: ["toggle"], value: { on: true } }), null);
    assert.strictEqual(wire.control({ name: "pump", can: ["switch"], value: { on: false } }), null, "a button to nowhere");
});

// the page reads contract N of docs/CONTRACT_Device_Api.md, and an answer of another contract is
// refused in words before anything is drawn from it.
const DOC = fs.readFileSync(path.join(ROOT, "docs/CONTRACT_Device_Api.md"), "utf8");

test("the page reads the contract the document is", () => {
    const number = Number(/^Contract version: `(\d+)`/m.exec(DOC)[1]);
    assert.strictEqual(wire.CONTRACT, number);
});

test("an answer of the page's own contract is drawn", () => {
    assert.strictEqual(wire.foreign({ contract: wire.CONTRACT, things: [] }), null);
});

test("an answer of another contract, or of none, is refused with the number the board speaks and the one the page reads", () => {
    const other = wire.foreign({ contract: wire.CONTRACT + 1, things: [] });
    assert.deepStrictEqual(other, { kind: "contractOther", number: wire.CONTRACT + 1, mine: wire.CONTRACT });
    assert.deepStrictEqual(wire.foreign({ things: [] }), { kind: "noContract", number: "", mine: wire.CONTRACT });
});

// the automaton's card and buttons, the clocks and the choices' bearings are read out of the dictionary, and the board that sends none of them
// says contract 2 - a page taking its dictionary as its own would hide what it cannot find and say nothing of why.
test("a real dictionary of the contract before, one with no automaton in it, is refused with both numbers", () => {
    const old = JSON.parse(fs.readFileSync(path.join(__dirname, "test_old_dictionary.json"), "utf8"));
    assert.strictEqual(old.contract, 2);
    assert.deepStrictEqual(wire.foreign(old), { kind: "contractOther", number: 2, mine: wire.CONTRACT });
});

// the answer of /settings and the row `refused` changed their shape (a code with fields, not a sentence), so a page of this image does not draw a board of the one before, and the other way round
test("a board of contract 4, which answered a save in words, is refused by the page of contract 10 with both numbers", () => {
    assert.strictEqual(wire.CONTRACT, 10);
    assert.deepStrictEqual(wire.foreign({ contract: 4, things: [] }), { kind: "contractOther", number: 4, mine: 10 });
});

// the buttons and the switch of the heat moved to a card of their own, the thermostat's, and a page of contract 5 looks for them on the automaton's
test("a board of contract 5, whose automaton still held the buttons, is refused by the page of contract 10 with both numbers", () => {
    assert.deepStrictEqual(wire.foreign({ contract: 5, things: [] }), { kind: "contractOther", number: 5, mine: 10 });
});

// the silence of the return and the air moved to the thermostat's card, and a page of contract 6 looks for it on the automaton's
test("a board of contract 6, whose automaton still said the silence of the return and the air, is refused by the page of contract 10 with both numbers", () => {
    assert.deepStrictEqual(wire.foreign({ contract: 6, things: [] }), { kind: "contractOther", number: 6, mine: 10 });
});

// the profile, the windows and the silence of the room moved to the demand's card, and a page of contract 7 looks for them on the thermostat's
test("a board of contract 7, whose thermostat still said the profile and the silence of the room, is refused by the page of contract 10 with both numbers", () => {
    assert.deepStrictEqual(wire.foreign({ contract: 7, things: [] }), { kind: "contractOther", number: 7, mine: 10 });
});

// the card `thermostat` has a third button, `light`, and the button `start` no longer lights the fire; a page of contract 8 would send `start` for a light
test("a board of contract 8, whose Start lit the fire, is refused by the page of contract 10 with both numbers", () => {
    assert.deepStrictEqual(wire.foreign({ contract: 8, things: [] }), { kind: "contractOther", number: 8, mine: 10 });
});

// the times of the auger are a setting of the kind `tenths` since 10, and a page of contract 9 would read "0.8" as a number of seconds it cannot take
test("a board of contract 9, whose times of the auger were whole seconds, is refused by the page of contract 10 with both numbers", () => {
    assert.deepStrictEqual(wire.foreign({ contract: 9, things: [] }), { kind: "contractOther", number: 9, mine: 10 });
});

test("a dictionary of the contract the page reads is drawn whether or not the board has an automaton", () => {
    assert.strictEqual(wire.foreign({ contract: wire.CONTRACT, things: [], machine: { card: "automaton" } }), null);
    assert.strictEqual(wire.foreign({ contract: wire.CONTRACT, things: [] }), null);
});

// the same page runs on a computer and is pointed at a board by its address - ?board=<host> -;
// without one it speaks to the board that served it.
test("a page given a board's address speaks to that board", () => {
    assert.deepStrictEqual(wire.boardAt("?board=192.0.2.39", "http://127.0.0.1:8790"),
        { base: "http://192.0.2.39", error: null });
    assert.deepStrictEqual(wire.boardAt("?tab=1&board=burner.local:8080", "http://127.0.0.1:8790"),
        { base: "http://burner.local:8080", error: null });
});

test("a page given no address speaks to the board that served it", () => {
    assert.deepStrictEqual(wire.boardAt("", "http://192.0.2.39"), { base: "", error: null });
    assert.deepStrictEqual(wire.boardAt("?board=", "http://192.0.2.39"), { base: "", error: null });
});

test("an address that is no board's is refused by a code naming it, and nothing is asked of it", () => {
    for (const bad of ["http://192.0.2.39", "192.0.2.39/state", "a b", "evil.com@192.0.2.39", "host:port"]) {
        const got = wire.boardAt("?board=" + encodeURIComponent(bad), "http://127.0.0.1:8790");
        assert.strictEqual(got.base, null, bad);
        assert.deepStrictEqual(got.error, { kind: "badBoard", named: bad }, bad);
    }
});

test("every request and the console go to the board the page speaks to", () => {
    assert.strictEqual(wire.at("http://192.0.2.39", "/state"), "http://192.0.2.39/state");
    assert.strictEqual(wire.at("", "/state"), "/state");
    assert.strictEqual(wire.socketAt("http://192.0.2.39", "/console", "http://127.0.0.1:8790/"), "ws://192.0.2.39/console");
    assert.strictEqual(wire.socketAt("", "/console", "https://192.0.2.39/"), "wss://192.0.2.39/console");
});

// THE CLOCKS IN ONE ROW: the board sends every clock the automaton counts, the page says the
// nearest one that runs.
const clock = (name, s) => ({ group: "automaton", name, clock: true, state: `${s} s`, value: { seconds: s } });
const upto = (name, s) => ({ group: "automaton", name, state: `${s} s`, value: { seconds: s } });
const dash = (name) => ({ group: "automaton", name, clock: true, state: "-", level: "dim", value: { unknown: {} } });
const done = (name) => ({ group: "automaton", name, clock: true, state: "done", level: "dim", value: { seconds: 0 } });

test("next says the nearest clock that runs, by its own word and the time left", () => {
    const things = [upto("for", 3), clock("attempt ends in", 80), clock("igniter off in", 40), dash("dose ends in")];
    assert.deepStrictEqual(burning.next(things), { value: "igniter off in \u00b7 40 s", level: null });
});

test("a clock that has run out and the time in the stage are not what comes next", () => {
    const things = [upto("for", 1), done("lighting in"), clock("auger switches in", 12)];
    assert.deepStrictEqual(burning.next(things), { value: "auger switches in \u00b7 12 s", level: null });
});

test("with no clock running next says a dash and stays a row", () => {
    const things = [upto("for", 370), dash("lighting in"), done("dose ends in")];
    assert.deepStrictEqual(burning.next(things), { value: "-", level: "dim" });
    assert.deepStrictEqual(burning.next([]), { value: "-", level: "dim" });
});

test("a row the board does not call a clock, or one with no seconds in its value, is not taken", () => {
    const things = [{ group: "automaton", name: "auger in", state: "5 s", value: { seconds: 5 } },
                    { group: "automaton", name: "odd in", clock: true, state: "7 s", value: { number: { value: 7, unit: "s" } } }];
    assert.deepStrictEqual(burning.next(things), { value: "-", level: "dim" });
});


test("the rows the page draws as clocks are the ones the board calls clocks, and they are the automaton's things of seconds but the time in the stage and the sensors' silences", () => {
    // "for" and "silent_*" count up - how long the stage has lasted, how long a sensor has been quiet - and a clock
    // counts down to something
    const byRule = BURNER.things
        .filter((t) => t.card === "automaton" && !t.part_of && t.kinds.includes("seconds") && t.name !== "for" && !t.name.startsWith("silent_"))
        .map((t) => scope.dictionary.textsOf(TEXTS, { card: "automaton", name: t.name }).label || t.name);
    assert.ok(byRule.length >= 12, `the dictionary gives ${byRule.length} clocks`);
    const drawn = scope.dictionary.state(BURNER, TEXTS, []).things.filter(burning.isClock).map((t) => t.name);
    assert.deepStrictEqual(drawn.sort(), byRule.sort());
});

test("a clock with nothing to say in this stage is still a clock, and a row the board does not call one is none", () => {
    assert.strictEqual(burning.isClock(dash("lighting in")), true);
    assert.strictEqual(burning.isClock(upto("for", 3)), false);
    assert.strictEqual(burning.isClock({ group: "relays", name: "lighting in", value: { seconds: 3 } }), false);
});


// THE CARD NOW: which rows, in what order, is the board's word (`machine.now` in its dictionary, which a row carries as `now`, its place in the list),
// and what a person reads each row as - `as`, and the number beside its word - is the page's (texts.json, `now`)
const row = (group, name, state, value, level = null, more = {}) => ({ group, name, state, value, level, ...more });

test("the card Now takes the rows the board puts in it, in the board's order, by the words the page gives them", () => {
    const things = [
        row("temperatures", "x", "47 °C", { number: 47 }, "ok", { now: 2, as: "boiler" }),
        row("automaton", "other", "here", { on: true }),
        row("relays", "pump", "on", { on: true }, null, { now: 0, as: "pump" }),
    ];
    assert.deepStrictEqual(wire.now(things), [
        { name: "pump", value: "on", level: null },
        { name: "boiler", value: "47 °C", level: "ok" },
    ]);
});

test("a row the board does not put in the card is not drawn, and no rows are none", () => {
    assert.deepStrictEqual(wire.now([row("relays", "igniter", "off", { on: false })]), []);
    assert.deepStrictEqual(wire.now([]), []);
    assert.deepStrictEqual(wire.now(undefined), []);
});

test("where several rows are the one the board means - two sensors playing the same role - the first stands for it, as one row of the card", () => {
    const things = [
        row("temperatures", "s1", "18 °C", {}, null, { now: 0, as: "boiler" }),
        row("temperatures", "s2", "40 °C", {}, null, { now: 0, as: "boiler" }),
        row("temperatures", "s3", "22 °C", {}, null, { now: 1, as: "supply" }),
    ];
    assert.deepStrictEqual(wire.now(things).map((r) => [r.name, r.value]), [["boiler", "18 °C"], ["supply", "22 °C"]]);
});

test("a row that has a number beside its word says both, and one that has none says its word", () => {
    assert.deepStrictEqual(wire.now([row("flame", "flame", "steady", {}, undefined, { now: 5, as: "flame", alongside: "0.90 V" })]),
        [{ name: "flame", value: "steady \u00b7 0.90 V", level: null }]);
    assert.strictEqual(wire.now([row("flame", "flame", "not read yet", {}, undefined, { now: 5, as: "flame" })])[0].value, "not read yet");
});

test("the table says the flame's two thresholds in volts, from the flame's own card", () => {
    const row = table({ ...SET, flame_lit: "2400", flame_steady: "1100" }).find((s) => s.name === "flame");
    assert.deepStrictEqual(row && row.parts.map((p) => ({ name: p.name, value: p.value, set: p.set })), [
        { name: "flame_lit", value: "2.40 V", set: true },
        { name: "flame_steady", value: "1.10 V", set: true },
    ]);
});

test("thresholds the board did not send are said to be missing, not 0", () => {
    const row = table(SET, (settings, fixed) => [settings.filter((s) => !s.name.startsWith("flame_")), fixed]).find((s) => s.name === "flame");
    assert.ok(row, "no row of the flame");
    assert.ok(row.parts.every((p) => p.missing), "a threshold the board did not send stands as sent");
});

// which settings of a section are in force with the mode chosen - never by where the page happens to draw them
const SOCKETS = ["role_ntc_1", "role_ntc_2", "role_ntc_3", "role_ntc_4", "role_ntc_5", "role_ntc_6", "role_ntc_7"];
const SENSED = ["auto_hold_c", "auto_drop_c", "auto_pa", "auto_feed_on_s", "auto_feed_off_s"];
// what the board's dictionary says of each mode (crates/settings, BEARINGS): the numbers it shows and the role its sensor plays
const modes = [
    { value: "power", words: "a level of power", shows: ["auto_step"] },
    ...["boiler", "return", "room"].map((role) => ({
        value: role,
        words: { boiler: "the boiler", return: "the return", room: "the boiler room" }[role],
        shows: SENSED,
        needs: { among: SOCKETS, holds: role },
    })),
];
const automatonCard = [
    { name: "auto_hold", card: "automaton", value: "boiler", options: modes },
    { name: "auto_hold_c", card: "automaton", value: "60", months: [5, 85] },
    { name: "auto_drop_c", card: "automaton", value: "4", months: [1, 20] },
    { name: "auto_step", card: "automaton", value: "6", options: ["14", "10", "8", "6", "5", "4", "3"].map((v) => ({ value: v, words: `${v} kW` })) },
    ...Object.entries(SET).map(([name, value]) => ({ name, card: "automaton", value, level: "tuning" })),
    { name: "auto_pump_off_c", card: "automaton", value: "4", whole: [1, 20], level: "tuning" },
];
const socket = (n, role) => ({ name: `role_ntc_${n}`, value: role });
const names = (list) => list.map((s) => s.name);
// the card and the sockets the board has now, as the page reads every setting of the board
const board = (card, sockets) => [...card, ...sockets];

test("a burner turned to a level shows its level and no temperature, and the other way round", () => {
    const sockets = [socket(1, "boiler")];
    const heat = automatonCard.filter((s) => ["auto_hold", "auto_hold_c", "auto_drop_c", "auto_step"].includes(s.name));
    const power = wire.inForce(heat, { auto_hold: { value: "power", own: "" } }, board(automatonCard, sockets));
    assert.deepStrictEqual(names(power), ["auto_hold", "auto_step"]);
    const water = wire.inForce(heat, { auto_hold: { value: "return", own: "" } }, board(automatonCard, [...sockets, socket(2, "return")]));
    assert.deepStrictEqual(names(water), ["auto_hold", "auto_hold_c", "auto_drop_c"]);
});

test("a mode by a sensor is offered while a socket plays its role, and a level always", () => {
    const offered = (roles, held = "power") => {
        const card = automatonCard.map((s) => (s.name === "auto_hold" ? { ...s, value: held } : s));
        return wire.inForce(card.filter((s) => ["auto_hold", "auto_hold_c", "auto_drop_c", "auto_step"].includes(s.name)), {}, board(card, roles))[0].options.map((o) => o.value);
    };
    assert.deepStrictEqual(offered([]), ["power"]);
    assert.deepStrictEqual(offered([socket(1, "boiler")]), ["power", "boiler"]);
    assert.deepStrictEqual(offered([socket(1, "boiler"), socket(4, "room"), socket(5, "own:buffer")]), ["power", "boiler", "room"]);
    assert.deepStrictEqual(offered([socket(2, "return"), socket(3, "supply")]), ["power", "return"]);
});

test("the mode the board holds stays on the list even when no socket plays its role now", () => {
    const card = automatonCard.map((s) => (s.name === "auto_hold" ? { ...s, value: "return" } : s));
    const one = wire.inForce(card, {}, board(card, [socket(1, "boiler")]))[0];
    assert.deepStrictEqual(one.options.map((o) => o.value), ["power", "boiler", "return"]);
});

// A board that calls its mode and its numbers by other names is drawn the same: the page reads what the board says each choice shows
// and needs and no name of a setting. The names here are made up, and a page that read them off a list of its own could not pass.
const other = [
    { name: "how", card: "c", value: "cool", options: [
        { value: "flat", words: "flat", shows: ["level"] },
        { value: "cool", words: "cool", shows: ["until", "below"], needs: { among: ["probe_1", "probe_2"], holds: "water" } },
        { value: "warm", words: "warm", shows: ["until", "below"], needs: { among: ["probe_1", "probe_2"], holds: "air" } },
    ] },
    { name: "until", card: "c", value: "60", whole: [5, 85] },
    { name: "below", card: "c", value: "4", whole: [1, 20] },
    { name: "level", card: "c", value: "6" },
    { name: "always", card: "c", value: "1" },
    { name: "fitted", card: "c", value: "1", level: "tuning" },
];
const probe = (n, v) => ({ name: `probe_${n}`, value: v });

test("a choice shows the settings the board says it shows, and a setting no choice names is always shown", () => {
    assert.deepStrictEqual(names(wire.inForce(other, { how: { value: "flat", own: "" } }, board(other, [probe(1, "water")]))), ["how", "level", "always", "fitted"]);
    assert.deepStrictEqual(names(wire.inForce(other, {}, board(other, [probe(1, "water")]))), ["how", "until", "below", "always", "fitted"]);
});

test("a setting two choices of two settings name is shown only while both say so", () => {
    const two = [
        { name: "a", card: "c", value: "x", options: [{ value: "x", shows: ["n"] }, { value: "y" }] },
        { name: "b", card: "c", value: "p", options: [{ value: "p", shows: ["n"] }, { value: "q" }] },
        { name: "n", card: "c", value: "1" },
    ];
    assert.deepStrictEqual(names(wire.inForce(two, {}, two)), ["a", "b", "n"]);
    assert.deepStrictEqual(names(wire.inForce(two, { b: { value: "q", own: "" } }, two)), ["a", "b"]);
    assert.deepStrictEqual(names(wire.inForce(two, { a: { value: "y", own: "" } }, two)), ["a", "b"]);
});

test("a choice needing a role is offered while a setting anywhere on the board holds it, and the held one stays", () => {
    const offered = (probes, value = "flat") => {
        const card = other.map((s) => (s.name === "how" ? { ...s, value } : s));
        return wire.inForce(card, {}, board(card, probes))[0].options.map((o) => o.value);
    };
    assert.deepStrictEqual(offered([]), ["flat"]);
    assert.deepStrictEqual(offered([probe(2, "water")]), ["flat", "cool"]);
    assert.deepStrictEqual(offered([probe(1, "air"), probe(2, "water")]), ["flat", "cool", "warm"]);
    assert.deepStrictEqual(offered([], "warm"), ["flat", "warm"], "the mode held stays on the list");
    assert.deepStrictEqual(offered([probe(3, "water")]), ["flat"], "a setting the choice does not name is not a probe of it");
});

test("a choice the board does not offer, held, shows no setting some choice names, and a plain setting keeps its options as they were", () => {
    const stray = other.map((s) => (s.name === "how" ? { ...s, value: "gone" } : s));
    assert.deepStrictEqual(names(wire.inForce(stray, {}, board(stray, []))), ["how", "always", "fitted"]);
    const plain = wire.inForce(other, {}, board(other, [])).find((s) => s.name === "always");
    assert.strictEqual(plain, other.find((s) => s.name === "always"), "a setting whose choices need nothing is the object the board sent");
});

test("the working row says what the burner holds, and the smouldering line is the mode's number", () => {
    const by = (hold) => table({ ...SET, auto_hold: hold }).find((s) => s.name === "working").parts.reduce((all, p) => ({ ...all, [p.name]: p.value }), {});
    // a choice is said by the words the menu gives it - read from the menu, never spelled here
    const choice = (setting, value) => TEXTS.menu.modules.thermostat.settings[setting].choices.find((c) => c.value === value).words.en;
    assert.strictEqual(by("return")["auto_hold"], choice("auto_hold", "return"));
    assert.strictEqual(by("return")["auto_hold_c"], "60 °C");
    assert.strictEqual(by("return")["auto_drop_c"], "4 °C");
    assert.strictEqual(by("power")["auto_step"], choice("auto_step", "6"));
    assert.strictEqual(by("power")["auto_hold_c"], undefined, "a burner that holds no temperature does not smoulder");
});

test("what the mode decides stands in the working row only: the draught of the rising and of the burning off stand in every mode", () => {
    for (const hold of ["power", "boiler", "return", "room"]) {
        assert.ok(part("rising", "auto_pa", { ...SET, auto_hold: hold }), hold);
        assert.ok(part("burning_off", "auto_pa", { ...SET, auto_hold: hold }), hold);
    }
    // the level and the first burner's pair stand in every mode (no mode names them); what a mode by a temperature adds is the temperature and how far under it the burner works again
    assert.deepStrictEqual(partNames("working", { ...SET, auto_hold: "power" }), ["auto_hold", "auto_step", "auto_pa", "auto_feed_on_s", "auto_feed_off_s"]);
    assert.deepStrictEqual(partNames("working", { ...SET, auto_hold: "boiler" }), ["auto_hold", "auto_step", "auto_pa", "auto_feed_on_s", "auto_feed_off_s", "auto_hold_c", "auto_drop_c"]);
});

test("the setpoint is no longer a number fixed in the image, so the page does not read it from there", () => {
    assert.deepStrictEqual(shown(part("working", "auto_hold_c")), { value: "60 °C", set: true });
});

test("the pump says where it stops, its own number", () => {
    assert.deepStrictEqual(shown(part("pump", "auto_pump_on_c")), { value: "45 °C", set: true });
    assert.deepStrictEqual(shown(part("pump", "auto_pump_off_c")), { value: "4 °C", set: true });
});

// ---- Live: the page asks the board to send every change, and the board ends it by itself ----

test("the page asks for the pace of everyone until Live is pressed, and for every change while Live lasts", () => {
    const now = 1_000_000;
    assert.strictEqual(wire.streamPath(0, now), "/events");
    assert.strictEqual(wire.streamPath(now + 5000, now), "/events?every=0");
    assert.strictEqual(wire.streamPath(now - 1, now), "/events", "Live that has run out asks for nothing more than the pace of everyone");
});

test("the button of Live says the period the board sends at - from its dictionary - and, while Live lasts, the word Live and no count of seconds", () => {
    const dict = { stream: { every: 5, live_s: 600, most: 60 } };
    assert.strictEqual(wire.livePeriod(dict), 5);
    assert.strictEqual(wire.livePeriod({ stream: { every: 10, live_s: 600 } }), 10, "the period is the board's and not a five written on the page");
    assert.strictEqual(wire.liveButton(dict, false, "LIVE", "u"), "5u");
    assert.strictEqual(wire.liveButton(dict, true, "LIVE", "u"), "LIVE");
    for (const left of [600, 599, 59, 1]) assert.ok(!/\d/.test(wire.liveButton(dict, true, "LIVE", "u")), `a count of ${left} s is on the button`);
});

test("a board that names no period has no period to put on the button", () => {
    assert.strictEqual(wire.livePeriod({}), 0);
    assert.strictEqual(wire.livePeriod(null), 0);
    assert.strictEqual(wire.livePeriod({ stream: { live_s: 600 } }), 0);
    assert.strictEqual(wire.liveButton({}, false, "LIVE", "u"), "");
});

test("Live lasts as long as the board's dictionary says, and a board that says nothing has none to press", () => {
    assert.strictEqual(wire.liveSeconds({ stream: { every: 5, live_s: 600, most: 60 } }), 600);
    assert.strictEqual(wire.liveSeconds({ stream: { every: 5, live_s: 90, most: 60 } }), 90);
    assert.strictEqual(wire.liveSeconds({}), 0, "no number of the board's is made up");
    assert.strictEqual(wire.liveSeconds(null), 0);
});
