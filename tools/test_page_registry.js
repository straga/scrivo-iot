// page/registry.js: what a device adds to the engine's page - a tab, a part for a section of the menu - and what the page does when the adding is wrong.
// A wrong adding is never left to the order the scripts happen to load in: a part registered twice, or a section that asks for a part nobody registered,
// is a problem the page says in words, and the part that came first stands.
//
//     node --test ./tools/test_page_registry.js
const test = require("node:test");
const assert = require("node:assert");
const { load } = require("./page_sources");

const scope = {};
load(["registry.js"], scope);
const { registry } = scope;

const part = (name) => function Part() { return name; };

test("a tab and a part are added once each, and nothing is wrong", () => {
    const parts = registry.make();
    parts.tab({ id: "scheme", label: "scheme", component: part("scheme") });
    parts.section("readings", part("readings"));
    assert.deepStrictEqual(parts.problems, []);
    assert.deepStrictEqual(parts.tabs.map((one) => one.id), ["scheme"]);
    assert.deepStrictEqual(Object.keys(parts.sections), ["readings"]);
});

test("a part added twice under one name is a problem that names it, and the first one stands", () => {
    const parts = registry.make();
    const first = part("first");
    parts.section("readings", first);
    parts.section("readings", part("second"));
    assert.deepStrictEqual(parts.problems, [{ kind: "sectionTwice", key: "readings" }]);
    assert.strictEqual(parts.sections.readings, first);
});

test("a tab added twice under one id is a problem that names it, and the first one stands", () => {
    const parts = registry.make();
    parts.tab({ id: "scheme", label: "scheme", component: part("first") });
    parts.tab({ id: "scheme", label: "scheme", component: part("second") });
    assert.deepStrictEqual(parts.problems, [{ kind: "tabTwice", key: "scheme" }]);
    assert.strictEqual(parts.tabs.length, 1);
    assert.strictEqual(parts.tabs[0].component(), "first");
});

test("two parts under two names, and two tabs under two ids, are not a problem", () => {
    const parts = registry.make();
    parts.section("a", part("a"));
    parts.section("b", part("b"));
    parts.tab({ id: "x", label: "x", component: part("x") });
    parts.tab({ id: "y", label: "y", component: part("y") });
    assert.deepStrictEqual(parts.problems, []);
});

test("a section that does not ask for a part is a plain section", () => {
    const parts = registry.make();
    parts.section("readings", part("readings"));
    assert.strictEqual(parts.partOf({ id: "steps", title: {} }), null);
});

test("a section that asks for a part by its `part` field is given the part and what it asks it for", () => {
    const parts = registry.make();
    const readings = part("readings");
    parts.section("readings", readings);
    assert.deepStrictEqual(parts.partOf({ id: "run", part: "readings", given: "panel" }), { component: readings, given: "panel" });
    assert.deepStrictEqual(parts.partOf({ id: "run", part: "readings" }), { component: readings, given: undefined });
});

test("a section that asks for a part nobody added is a problem that names the part and the section - and is not drawn as a plain one", () => {
    const parts = registry.make();
    parts.section("readings", part("readings"));
    assert.deepStrictEqual(parts.partOf({ id: "run", part: "gauge" }), { problem: { kind: "noPart", key: "gauge", section: "run" } });
});

test("a key of the section that happens to be the name of a part is no asking: only the `part` field asks", () => {
    const parts = registry.make();
    parts.section("readings", part("readings"));
    assert.strictEqual(parts.partOf({ id: "run", readings: "panel" }), null);
});

// WHAT THE PAGE SAYS of each problem is the menu's words, in both languages, with a place for each thing the problem names: the engine's menu, not a device's.
const { menu } = require("./page_sources");
load(["wire.js", "dictionary.js"], scope);
test("every problem the registry can make has words in both languages that name what is wrong", () => {
    const words = menu().ui;
    const made = [
        { kind: "sectionTwice", key: "marked" },
        { kind: "tabTwice", key: "mine" },
        { kind: "noPart", key: "gauge", section: "asks" },
    ];
    for (const lang of ["en", "ru"]) {
        const ui = scope.dictionary.uiOf({ menu: { ui: words, units: {} }, lang });
        for (const wrong of made) {
            const said = scope.dictionary.spell(ui[wrong.kind], wrong);
            assert.ok(said.includes(wrong.key), `${wrong.kind} in ${lang} does not name the ${wrong.key}: ${said}`);
            if (wrong.section) assert.ok(said.includes(wrong.section), `${wrong.kind} in ${lang} does not name the section`);
            assert.ok(!/\{\w+\}/.test(said), `${wrong.kind} in ${lang} leaves a place unfilled: ${said}`);
        }
    }
});
