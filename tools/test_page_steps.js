// The power steps are a table a person edits: the user's section `Power steps` holds a row a step and in it three numbers - the dose, the pause, the draught -, the row the burner burns on is known, and the
// automaton's table of "How it runs" says the numbers of that row and never a pair of its own. What is held here is where a number stands, which row is chosen and what the table of the stages says of it - numbers and
// codes, never the words of a title.
//
//     node --test tools/test_page_steps.js
const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");
const { execSync } = require("node:child_process");

const ROOT = path.dirname(__dirname);
const { load, menu, texts } = require("./page_sources");
const scope = {};
load(["form.js", "wire.js", "burning.js", "dictionary.js", "board.js", "scheme.js"], scope);
const d = scope.dictionary;
const burning = scope.burning;

const DICT = JSON.parse(execSync("cargo +stable run -q -p burner_api --example served_json", { cwd: ROOT, encoding: "utf8", maxBuffer: 1 << 26 }));
const MENU = menu();

// the settings as the page holds them for a board that keeps TOLD over the defaults: made by the page's own `state`, from the board's dictionary and the values it would send
const TEXTS_FILE = texts();
const TEXTS_ALL = { ...TEXTS_FILE, menu: MENU };
const settingsOf = (told = {}) => {
    const values = [];
    DICT.things.forEach((t, at) => {
        if (t.card !== "settings" || t.part_of) return;
        const own = DICT.settings.find((s) => s.name === t.name);
        values[at] = { text: told[t.name] ?? (own && own.default) };
    });
    return d.state(DICT, TEXTS_ALL, values).settings;
};

const stepsSection = () => d.sectionsOf(MENU.tree).find((s) => s.table);
const STEPS = DICT.settings.find((s) => s.name === "auto_step").choices.map((c) => c.value);

test("the automaton's page has a section whose settings are a table of rows, one for each level of power", () => {
    const section = stepsSection();
    assert.ok(section, "no section of the tree holds a table");
    assert.deepStrictEqual(section.table.rows.map((r) => r.step), STEPS, "a row for each level the board offers, in the order it offers them, and no other");
    assert.strictEqual(section.table.chosen, "auto_step", "the table says which setting names the level the burner burns on");
});

test("a row holds three numbers - a dose, a pause, a draught - and the columns say which is which", () => {
    const { columns, rows } = stepsSection().table;
    assert.deepStrictEqual(columns.map((c) => c.key), ["on_s", "off_s", "pa"]);
    for (const row of rows) {
        assert.strictEqual(row.settings.length, 3, `row ${row.step}`);
        const [on, off, pa] = row.settings.map((name) => DICT.settings.find((s) => s.name === name) || assert.fail(`${name} is no setting of the board`));
        assert.deepStrictEqual([on.kind, off.kind, pa.kind], ["tenths", "tenths", "whole"], `row ${row.step}`);
        assert.strictEqual(pa.unit, "Pa", `row ${row.step}`);
    }
});

test("every number of the table stands in this section and in no other place of the tree", () => {
    const where = new Map();
    for (const section of d.sectionsOf(MENU.tree)) for (const name of d.sectionNames(section)) where.set(name, [...(where.get(name) || []), section.id]);
    for (const row of stepsSection().table.rows) for (const name of row.settings) assert.deepStrictEqual(where.get(name), [stepsSection().id], name);
});

test("the names of a section are its settings, and the numbers of its table after them", () => {
    assert.deepStrictEqual(d.sectionNames({ settings: ["a", "b"] }), ["a", "b"]);
    assert.deepStrictEqual(d.sectionNames({ settings: ["a"], table: { rows: [{ step: "1", settings: ["x", "y"] }, { step: "2", settings: ["z"] }] } }), ["a", "x", "y", "z"]);
    assert.deepStrictEqual(d.sectionNames({ controls: [["a", "b"]] }), []);
});

// ---- which row the burner burns on ----

test("the row the burner burns on is the one the board's level names, and no row when the board names none", () => {
    const section = stepsSection();
    for (const step of STEPS) {
        const table = d.tableOf(section, settingsOf({ auto_step: step }), {});
        assert.deepStrictEqual(table.rows.filter((r) => r.chosen).map((r) => r.step), [step], `the board keeps ${step}`);
    }
    const none = d.tableOf(section, settingsOf({ auto_step: "7" }), {});
    assert.deepStrictEqual(none.rows.filter((r) => r.chosen), [], "a level the table has not is no row");
});

test("a person's unsaved pick of the level does not move the row the burner burns on", () => {
    const table = d.tableOf(stepsSection(), settingsOf({ auto_step: "8" }), { auto_step: { value: "3", own: "" } });
    assert.deepStrictEqual(table.rows.filter((r) => r.chosen).map((r) => r.step), ["8"]);
});

test("a row's cells are the board's settings of that row, in the order of the columns, and a setting the board has not is a hole", () => {
    const section = stepsSection();
    const table = d.tableOf(section, settingsOf(), {});
    for (const row of table.rows) {
        const want = section.table.rows.find((r) => r.step === row.step).settings;
        assert.deepStrictEqual(row.cells.map((c) => c && c.name), want, `row ${row.step}`);
    }
    const holed = d.tableOf(section, settingsOf().filter((s) => s.name !== "auto_5_pa"), {});
    assert.strictEqual(holed.rows.find((r) => r.step === "5").cells[2], null);
});

test("a section with no table has no table", () => {
    assert.strictEqual(d.tableOf(d.sectionsOf(MENU.tree).find((s) => !s.table), settingsOf(), {}), null);
});

// ---- what "How it runs" says ----

const machine = () => d.cycleOf(TEXTS_ALL);
const parts = (stage, told) => {
    const every = settingsOf(told);
    return burning.parameters(machine(), d.cycleOf(TEXTS_ALL).words, [...every, ...d.chosenStep(MENU.tree, every)], []).find((s) => s.name === stage).parts;
};

test("the working stage says the numbers of the row the burner burns on, whichever hold it is in", () => {
    for (const hold of ["power", "boiler", "return", "room"]) {
        const got = Object.fromEntries(parts("working", { auto_step: "10", auto_hold: hold }).map((p) => [p.name, p.value]));
        assert.deepStrictEqual([got["step.on_s"], got["step.off_s"], got["step.pa"]], ["2 s", "12 s", "36 Pa"], hold);
    }
});

test("rising and burning off say the draught of the row the burner burns on", () => {
    for (const [step, pa] of [["14", "51 Pa"], ["6", "22 Pa"], ["3", "11 Pa"]]) {
        for (const stage of ["rising", "burning_off"]) {
            assert.strictEqual(parts(stage, { auto_step: step }).find((p) => p.name === "step.pa").value, pa, `${stage} at ${step}`);
        }
    }
});

test("an edit of the table moves what the stages say, and an edit of another row does not", () => {
    const base = parts("working", { auto_step: "6" }).find((p) => p.name === "step.off_s").value;
    const moved = parts("working", { auto_step: "6", auto_6_off_s: "33" }).find((p) => p.name === "step.off_s").value;
    const elsewhere = parts("working", { auto_step: "6", auto_8_off_s: "33" }).find((p) => p.name === "step.off_s").value;
    assert.deepStrictEqual([base, moved, elsewhere], ["21 s", "33 s", "21 s"]);
});

test("no stage of the cycle names the pair the burner no longer has", () => {
    for (const section of machine().sections) {
        for (const item of section.items) for (const name of item) assert.ok(!["auto_pa", "auto_feed_on_s", "auto_feed_off_s"].includes(name), `${section.name} says ${name}`);
    }
});

test("a board that names no level is not told a number: the stages say it is not sent", () => {
    const every = settingsOf({ auto_step: "7" });
    assert.deepStrictEqual(d.chosenStep(MENU.tree, every), []);
    const stage = burning.parameters(machine(), d.cycleOf(TEXTS_ALL).words, every, []).find((s) => s.name === "working").parts;
    assert.ok(stage.filter((p) => p.name.startsWith("step.")).every((p) => p.missing), JSON.stringify(stage));
});
