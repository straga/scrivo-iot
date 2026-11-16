// Where a module's action stands: a row of a card that is a command - the zeroing of the energy - is placed by the MENU, in the section of the module's page it belongs to,
// as a setting is: `actions: ["<card>/<name>"]`. A row named there is drawn in that section, after its settings, and nowhere else; the same card shown on the main screen keeps its readings
// and loses the command. A menu with no `actions` draws as it always did. What is held here is where the rows go - numbers and names, never a word of a title - and that the command still asks first.
//
//     node --test tools/test_page_actions.js
const test = require("node:test");
const assert = require("node:assert");
const { execSync } = require("node:child_process");

const { ROOT, load, menu } = require("./page_sources");
const scope = {};
load(["form.js", "wire.js", "dictionary.js"], scope);
const d = scope.dictionary;
const wire = scope.wire;

// ---- a small board: the energy card with three readings and the zeroing, and a relay card the menu does not touch ----
const DICT = {
    contract: 9,
    version: "k+1",
    things: [
        { card: "energy", name: "today", kinds: [], can: [] },
        { card: "energy", name: "month", kinds: [], can: [] },
        { card: "energy", name: "reset", kinds: ["button"], can: ["press"], irreversible: true },
        { card: "relays", name: "relay1", kinds: ["on"], can: ["switch", "toggle"] },
        { card: "energy", name: "mark", kinds: ["button"], can: ["press"] },
    ],
};
const rows = (dict, texts = { rows: {} }) => dict.things.map((_, at) => d.rowOf(dict, texts, new Array(dict.things.length).fill(undefined), at)).filter(Boolean);
const keys = (list) => list.map((t) => `${t.group}/${t.key}`);

const MAIN = { id: "energy", cards: ["energy"] };
const PAGE = { id: "energy", cards: ["energy"], settings: ["rows", "keep"], actions: ["energy/reset"] };
const WITH = { main: { sections: [MAIN, { id: "relays", cards: ["relays"] }] }, modules: { groups: [], pages: [{ id: "energy", sections: [PAGE] }] } };
const WITHOUT = { main: { sections: [MAIN] }, modules: { groups: [], pages: [{ id: "energy", sections: [{ id: "energy", cards: ["energy"], settings: ["rows", "keep"] }] }] } };

test("an action the menu puts in a module's section is drawn there, and the card on the main screen keeps its readings only", () => {
    const things = rows(DICT);
    const main = d.sectionRows(WITH, MAIN, things);
    assert.deepStrictEqual(keys(main.shown), ["energy/today", "energy/month", "energy/mark"]);
    assert.deepStrictEqual(main.actions, []);
    const page = d.sectionRows(WITH, PAGE, things);
    assert.deepStrictEqual(keys(page.shown), ["energy/today", "energy/month", "energy/mark"], "the readings stand on the module's page too");
    assert.deepStrictEqual(keys(page.actions), ["energy/reset"]);
});

test("a menu that places no action draws every row of the cards a section names, as it always did", () => {
    const things = rows(DICT);
    for (const tree of [WITHOUT, null, undefined, {}]) {
        const out = d.sectionRows(tree, MAIN, things);
        assert.deepStrictEqual(keys(out.shown), ["energy/today", "energy/month", "energy/reset", "energy/mark"]);
        assert.deepStrictEqual(out.actions, []);
    }
});

test("the order of the cards a section names is the menu's, and the board's within each", () => {
    const things = rows(DICT);
    const out = d.sectionRows(WITHOUT, { id: "both", cards: ["relays", "energy"] }, things);
    assert.deepStrictEqual(keys(out.shown), ["relays/relay1", "energy/today", "energy/month", "energy/reset", "energy/mark"]);
});

test("an action the board does not have is no row and no failure, and the readings are the same", () => {
    const tree = { ...WITH, modules: { groups: [], pages: [{ id: "energy", sections: [{ ...PAGE, actions: ["energy/ghost", "energy/reset"] }] }] } };
    const out = d.sectionRows(tree, tree.modules.pages[0].sections[0], rows(DICT));
    assert.deepStrictEqual(keys(out.actions), ["energy/reset"]);
    assert.deepStrictEqual(keys(out.shown), ["energy/today", "energy/month", "energy/mark"]);
});

test("an action is the one place of a row: two sections that name the same card show it only where it is named", () => {
    const other = { id: "other", cards: ["energy"] };
    const tree = { main: { sections: [other] }, modules: { groups: [], pages: [{ id: "energy", sections: [PAGE] }] } };
    assert.deepStrictEqual(keys(d.sectionRows(tree, other, rows(DICT)).shown), ["energy/today", "energy/month", "energy/mark"]);
});

test("a person finds the action through the tree like a setting: its place says which tab, which module's page and which section", () => {
    assert.deepStrictEqual(d.placeOf(WITH, "energy/reset"), { tab: "modules", module: "energy", section: "energy" });
    assert.ok(d.sectionNames(PAGE).includes("energy/reset"));
    assert.deepStrictEqual(d.sectionNames(PAGE).filter((n) => n !== "energy/reset"), ["rows", "keep"], "the settings are named as before");
    assert.strictEqual(d.placeOf(WITH, "energy/ghost"), null);
});

test("the action still asks first, whichever section draws it; a plain button beside it does not ask", () => {
    const page = d.sectionRows(WITH, PAGE, rows(DICT));
    const reset = page.actions[0];
    assert.strictEqual(reset.irreversible, true);
    assert.deepStrictEqual(wire.control(reset), { path: `/press/${reset.at}`, words: "Reset", ask: true });
    const mark = page.shown.find((t) => t.key === "mark");
    assert.strictEqual(wire.control(mark).ask, undefined);
});

// ---- the Dual as it is served and as its menu places it ----
const DUAL = JSON.parse(execSync("cargo +stable run -q -p bench_dualr3_api --example served_json", { cwd: ROOT, encoding: "utf8", maxBuffer: 1 << 26 }));
const DUAL_MENU = menu("bench_dualr3");
const DUAL_TEXTS = { menu: DUAL_MENU };
const DUAL_TREE = d.treeOf(DUAL_TEXTS);

test("on the Dual the zeroing of the energy is not on the main screen and is on the energy page of the modules, asking first", () => {
    const things = rows(DUAL, DUAL_TEXTS);
    const main = DUAL_TREE.main.sections.find((s) => (s.cards || []).includes("energy"));
    assert.ok(main, "the main screen shows the energy card");
    const onMain = d.sectionRows(DUAL_TREE, main, things);
    assert.ok(!onMain.shown.some((t) => t.group === "energy" && t.key === "reset"), "the zeroing is on the main screen");
    assert.ok(onMain.shown.some((t) => t.group === "energy"), "the readings are gone from the main screen too");
    const page = DUAL_TREE.modules.pages.find((p) => p.id === "energy");
    const section = page.sections.find((s) => (s.actions || []).length);
    assert.ok(section, "the energy page of the modules places no action");
    assert.deepStrictEqual(section.settings, ["rows", "keep"], "beside the settings of the history");
    const mine = d.sectionRows(DUAL_TREE, section, things);
    assert.deepStrictEqual(keys(mine.actions), ["energy/reset"]);
    assert.strictEqual(wire.control(mine.actions[0]).ask, true);
    assert.deepStrictEqual(d.placeOf(DUAL_TREE, "energy/reset"), { tab: "modules", module: "energy", section: section.id });
});

test("every action the Dual's menu places is a row the board serves, and is placed once", () => {
    const served = new Set(DUAL.things.map((t) => `${t.card}/${t.name}`));
    const named = d.sectionsOf(DUAL_TREE).flatMap((s) => s.actions || []);
    assert.ok(named.length >= 1);
    assert.deepStrictEqual(named.filter((n) => !served.has(n)), [], "an action the board does not serve");
    assert.strictEqual(new Set(named).size, named.length, "an action placed twice");
});

// ---- the burner is not touched: its menu places no action and every section draws what it drew ----
const BURNER = JSON.parse(execSync("cargo +stable run -q -p burner_api --example served_json", { cwd: ROOT, encoding: "utf8", maxBuffer: 1 << 26 }));
const BURNER_TEXTS = { menu: menu("burner") };
const BURNER_TREE = d.treeOf(BURNER_TEXTS);

test("the burner's menu places the fan's hands in the Service and no other action, and each section draws the rows of its cards as before - but the rows placed there and the three the scheme's buttons press", () => {
    const things = rows(BURNER, BURNER_TEXTS);
    assert.deepStrictEqual(keys(things.filter((t) => t.apart)).sort(), ["thermostat/light", "thermostat/start", "thermostat/stop"]);
    const sections = d.sectionsOf(BURNER_TREE);
    assert.ok(sections.length > 5);
    const placed = new Set(sections.flatMap((s) => s.actions || []));
    assert.deepStrictEqual([...placed].sort(), ["fan/draught", "fan/gate", "fan/purge"]);
    for (const section of sections) {
        assert.ok(!(section.actions || []).length || section.group === "service", `${section.id} places an action outside the Service`);
        const cards = section.cards || [];
        const before = things.filter((t) => cards.includes(t.group) && !t.apart && !placed.has(`${t.group}/${t.key}`)).sort((x, y) => cards.indexOf(x.group) - cards.indexOf(y.group));
        const out = d.sectionRows(BURNER_TREE, section, things);
        assert.deepStrictEqual(out.shown.map((t) => t.at), before.map((t) => t.at), `${section.id}: rows`);
        assert.deepStrictEqual(keys(out.actions), (section.actions || []).map((n) => n), `${section.id}: actions`);
    }
});

// ---- a row the menu says a part of the page draws: it is a row of the board, and no row of the card's list ----

const APART = { rows: { "energy/mark": { apart: true } } };

test("a row the menu says is drawn apart is still a row of the page - a part finds it - and stands in no section's list", () => {
    const things = rows(DICT, APART);
    assert.deepStrictEqual(keys(things.filter((t) => t.apart)), ["energy/mark"]);
    assert.deepStrictEqual(keys(d.sectionRows(WITHOUT, MAIN, things).shown), ["energy/today", "energy/month", "energy/reset"]);
    assert.deepStrictEqual(keys(d.sectionRows(WITH, PAGE, things).shown), ["energy/today", "energy/month"]);
});

test("a row the menu does not say is apart is listed as it was, and an action placed in a section is drawn there whether or not its neighbour is apart", () => {
    const things = rows(DICT, APART);
    assert.strictEqual(things.find((t) => t.key === "reset").apart, undefined);
    assert.deepStrictEqual(keys(d.sectionRows(WITH, PAGE, things).actions), ["energy/reset"]);
});
