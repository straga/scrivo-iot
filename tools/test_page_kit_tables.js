// The menu and the table of texts reach the page inside its one file (tools/page_kit.py, `kit-<name>` data elements) and,
// where a page is served as separate files (a stand of a revision taken out of git), by a request. wire.kitFile gives
// the same parsed table either way.
//
//     node --test tools/test_page_kit_tables.js
const test = require("node:test");
const assert = require("node:assert");
const { load } = require("./page_sources");

const scope = {};
load(["wire.js"], scope);
const wire = scope.wire;

const doc = (elements) => ({ getElementById: (id) => elements[id] || null });
const neverAsked = () => assert.fail("the table is inside the page, and a request was made for it");

test("a table inside the page is read from the page and no request is made", async () => {
    const page = doc({ "kit-menu.json": { textContent: '{"ui":{"save":"Save \\u003c/script>"}}' } });
    assert.deepStrictEqual(await wire.kitFile("menu.json", page, neverAsked), { ui: { save: "Save </script>" } });
});

test("a table the page does not carry is asked for by name", async () => {
    const asked = [];
    const get = async (name) => { asked.push(name); return { rows: {} }; };
    assert.deepStrictEqual(await wire.kitFile("texts.json", doc({}), get), { rows: {} });
    assert.deepStrictEqual(asked, ["texts.json"]);
});

test("the table of one name is not the table of the other", async () => {
    const page = doc({ "kit-menu.json": { textContent: '{"a":1}' } });
    assert.deepStrictEqual(await wire.kitFile("menu.json", page, neverAsked), { a: 1 });
    await assert.rejects(wire.kitFile("texts.json", page, async () => { throw new Error("asked"); }), /asked/);
});

test("a table in the page that is not JSON is an error and not a silent request", async () => {
    const page = doc({ "kit-menu.json": { textContent: "{broken" } });
    await assert.rejects(wire.kitFile("menu.json", page, neverAsked), SyntaxError);
});
