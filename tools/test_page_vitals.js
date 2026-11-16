// page/page.js, the Vitals component - the rows the System tab opens with: what the board is, how much memory it has, how long it has run, how busy each of its cores is.
//
//     bun test ./tools/test_page_vitals.js
//
// The class is taken OUT OF THE PAGE ITSELF, as tools/test_page_curve.js does it: a copy would go on passing while the page changed. OWL is not loaded: the getters touch neither the DOM nor the framework.
const test = require("node:test");
const assert = require("node:assert");

const { load, menu, read } = require("./page_sources");
const src = read("page.js");

function classSource(name) {
    const head = src.indexOf(`class ${name} extends`);
    assert.notStrictEqual(head, -1, `page.js has no class ${name}`);
    let depth = 0;
    for (let at = src.indexOf("{", head); at < src.length; at += 1) {
        if (src[at] === "{") depth += 1;
        if (src[at] === "}") {
            depth -= 1;
            if (depth === 0) return src.slice(head, at + 1);
        }
    }
    assert.fail(`the class ${name} is not closed`);
}

const scope = {};
load(["wire.js", "dictionary.js"], scope);
const MENU = menu();
scope.dictionary.useUnits(scope.dictionary.menuUnits({ menu: MENU, lang: "en" }));
const WORDS = scope.dictionary.uiOf({ menu: MENU, lang: "en" });
const Vitals = new Function(
    "dictionary", "WORDS", "Row",
    `const Component = class {}; const xml = (s) => s; const useBoard = () => ({}); const onWillDestroy = () => {};
     const ui = () => WORDS; const fill = dictionary.spell;
     ${classSource("Vitals")} return Vitals;`
)(scope.dictionary, WORDS, class {});

/// A Vitals of a board that says HEAD of itself, without OWL ever running.
function showing(head) {
    const one = Object.create(Vitals.prototype);
    one.b = { vitals: head, since: 0 };
    return one;
}

test("each core is a row of its own with its load in per cent", () => {
    const rows = showing({ cores: [21, 4] }).cores;
    assert.deepStrictEqual(rows, [{ name: "core 0", value: "21 %" }, { name: "core 1", value: "4 %" }]);
});

test("a core not measured yet is a row with no value, and the other one still says its own", () => {
    assert.deepStrictEqual(showing({ cores: [null, 4] }).cores, [{ name: "core 0", value: "" }, { name: "core 1", value: "4 %" }]);
});

test("a core with nothing to do says zero and not nothing", () => {
    assert.strictEqual(showing({ cores: [0, 0] }).cores[0].value, "0 %");
});

test("a board that says nothing of its cores, or has not answered yet, has no rows for them", () => {
    assert.deepStrictEqual(showing({ cores: null }).cores, []);
    assert.deepStrictEqual(showing(null).cores, []);
});

test("the rows of the cores stand in the tab after the uptime", () => {
    const template = [].concat(Vitals.template).join("");
    assert.ok(template.indexOf("ui.rowUptime") < template.indexOf("cores"), "the template does not draw the cores after the uptime: " + template);
});
