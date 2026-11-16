// devices/burner/page/app.js, the Curve component: how it turns the lines a board sent into a picture.
//
//     bun test ./tools/test_page_curve.js
//
// The board's curve is a LIST of lines, not a fixed pair, because a regulator needs three on one
// field - what was asked, what came out, and the angle it is held by.
// These hold the part of that which is arithmetic: which line the picture is about, which lines share its
// scale, and that a reading which meant nothing breaks the line instead of being drawn as a zero.
//
// The class is taken OUT OF THE PAGE ITSELF. A copy would go on passing while the page it stands for changed.
const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");

const { load, menu, read } = require("./page_sources");
const src = read("page.js");

/// The source of the class called NAME, from its head to the brace that closes it.
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

// OWL is not loaded: the getters under test touch neither the DOM nor the framework, so the base class and the
// template tag are stubs, and the component is made without its lifecycle.
// The words of the pace are the page's own dictionary.js over the menu's `ui` and `units` in English, so the getter is held with the real function it calls.
const scope = {};
load(["wire.js", "dictionary.js"], scope);
const MENU = menu();
scope.dictionary.useUnits(scope.dictionary.menuUnits({ menu: MENU, lang: "en" }));
const WORDS = scope.dictionary.uiOf({ menu: MENU, lang: "en" });
const Curve = new Function(
    "dictionary", "WORDS",
    `const Component = class {}; const xml = (s) => s; const useState = (s) => s;
     const onWillUnmount = () => {}; const useRef = () => {};
     const ui = () => WORDS;
     ${classSource("Curve")} return Curve;`
)(scope.dictionary, WORDS);

/// A component holding DRAWN, without OWL ever running.
function showing(drawn, fitted) {
    const one = Object.create(Curve.prototype);
    one.state = { which: "x", drawn, fitted: Boolean(fitted) };
    one.props = { curves: [{ name: "x", words: "x" }] };
    return one;
}

const line = (words, unit, points, span) => ({ words, unit, points, span });

/// The three lines a regulator sends.
const HOLDING = {
    step_ms: 1000,
    lines: [
        line("asked", "Pa", [51, 51, 51], [0, 300]),
        line("came out", "Pa", [12, 31, 49], [0, 300]),
        line("by angle", "%", [33, 24, 18], [0, 100]),
    ],
};

test("the picture is about the first line the board sent", () => {
    // the board puts it first on purpose: for the draught that is the pascals, not the millivolts under them
    assert.strictEqual(showing(HOLDING).main.words, "asked");
    assert.strictEqual(showing(HOLDING).main.unit, "Pa");
});

test("a first line that means nothing for its whole span is passed over", () => {
    // a sensor gone, a divider nobody filled in: every point is null, and the numbers down the side would have
    // nothing to say. The line that does have numbers becomes the one the picture is about.
    const drawn = { step_ms: 100, lines: [line("what it means", "Pa", [null, null]), line("what the pin held", "mV", [161, 162])] };
    assert.strictEqual(showing(drawn).main.words, "what the pin held");
});

test("every line the board sent is drawn, all of one weight and none thinner than the first used to be", () => {
    const drawn = showing(HOLDING).drawnLines;
    assert.strictEqual(drawn.length, 3);
    assert.deepStrictEqual(new Set(drawn.map((one) => one.width)).size, 1, "the lines are not of one weight: " + drawn.map((one) => one.width));
    assert.ok(drawn[0].width >= 1, "the lines are thinner than the first one was: " + drawn[0].width);
    assert.notStrictEqual(drawn[0].paint, drawn[1].paint, "two lines are painted the same colour");
    assert.notStrictEqual(drawn[1].paint, drawn[2].paint);
});

// WHAT A PERSON SEES OF A LINE, besides its colour: the dash and the end of the dash.
const look = (one) => `${one.dashed}|${one.cap}`;

test("the lines of one field look different by their order - solid, dashed, dotted - whatever their scales", () => {
    // the regulator's own field: the miss in Pa, and P and I in one unit that is not the picture's, sharing one scale
    for (const drawn of [HOLDING, REGULATOR]) {
        const lines = showing(drawn).drawnLines;
        assert.strictEqual(lines[0].dashed, null, "the line the picture is about is not solid");
        assert.ok(lines[1].dashed, "the second line is solid");
        assert.ok(lines[2].dashed, "the third line is solid");
        assert.strictEqual(new Set(lines.map(look)).size, 3, "two lines of one field look the same: " + lines.map(look));
    }
});

test("two lines of one scale are told apart by more than their colour", () => {
    // the pair that sent a person back to the picture on 21.09: asked and came out, both pascals, both on one scale
    const lines = showing(HOLDING).drawnLines;
    assert.strictEqual(lines[0].own || lines[1].own, false);
    assert.notStrictEqual(look(lines[0]), look(lines[1]));
});

test("the key under the picture shows each line's look beside its words", () => {
    // the sample is drawn by the same dash the line is, or the key would tell what a line looks like by a colour only
    const template = [].concat(Curve.template).join("");
    const key = template.slice(template.indexOf("WHICH LINE IS WHICH"));
    const sample = key.slice(key.indexOf("<svg"), key.indexOf("</svg>"));
    assert.ok(sample.includes('t-att-stroke-dasharray="one.dashed"'), "the key has no sample drawn by the line's dash: " + key.slice(0, 400));
});

test("a line in another unit is drawn against its own scale and says so", () => {
    const drawn = showing(HOLDING).drawnLines;
    assert.strictEqual(drawn[0].own, false, "the line the picture is about has its own scale");
    assert.strictEqual(drawn[1].own, false, "two lines of pascals did not share one scale");
    assert.strictEqual(drawn[2].own, true, "the angle was squeezed into a scale of pascals");
    assert.ok(drawn[2].dashed, "the line on its own scale is not dashed");
    assert.ok(drawn[2].shape.includes("%"), "the key does not say which unit it is in: " + drawn[2].shape);
    assert.strictEqual(drawn[0].shape, "", "a line on the picture's own scale was labelled as having another");
});

// the regulator's own curve - the miss the picture is about, and the two halves of the move
const REGULATOR = {
    step_ms: 1000,
    lines: [
        line("miss (over 0: up gains)", "Pa", [-3, 1, 4]),
        line("P", "% a second", [0.5, -0.2, 0.1]),
        line("I", "% a second", [0.02, 0.05, -0.04]),
    ],
};

test("two lines of one unit that is not the picture's share one scale, so the bigger half reads as bigger", () => {
    const one = showing(REGULATOR);
    const [p, i] = [REGULATOR.lines[1], REGULATOR.lines[2]];
    assert.deepStrictEqual(one.bounds(p), [-0.2, 0.5], "P was not drawn against the scale it shares with I");
    assert.deepStrictEqual(one.bounds(i), one.bounds(p), "I was drawn against a scale of its own, not the one it shares with P");
});

test("a line alone in its unit keeps a scale of its own", () => {
    // the pair: sharing is by unit, and a unit with one line in it has nobody to share with
    const one = showing(HOLDING, true);
    assert.deepStrictEqual(one.bounds(HOLDING.lines[2]), [18, 33], "the angle took a scale that is not its own");
});

test("the angle of a percent is not flattened onto the floor of a scale of pascals", () => {
    // THE WHOLE POINT OF THE OWN SCALE. Against 0-300 Pa an angle of 18 to 33 % is a flat line at the bottom,
    // which is exactly the shape a person is supposed to read.
    const drawn = showing(HOLDING).drawnLines;
    const ys = (runs) => runs.join(" ").split(" ").map((p) => Number(p.split(",")[1]));
    const angle = ys(drawn[2].runs);
    assert.ok(Math.max(...angle) - Math.min(...angle) > 3, "the angle is drawn flat: " + angle.join(" "));
});

test("a reading that meant nothing breaks the line instead of being drawn as a zero", () => {
    // a curve dropping to the floor says "it read zero", which is the one thing that did not happen
    const drawn = { step_ms: 100, lines: [line("what it means", "Pa", [10, 11, null, null, 40, 41], [0, 300])] };
    const runs = showing(drawn).drawnLines[0].runs;
    assert.strictEqual(runs.length, 2, "the gap did not break the line: " + JSON.stringify(runs));
});

test("what the picture spans names every line it drew", () => {
    const said = showing(HOLDING).span;
    assert.ok(said.includes("Pa"), said);
    assert.ok(said.includes("%"), "the angle is drawn and not named in the span: " + said);
});

test("a board that has not answered yet is a picture of nothing rather than a crash", () => {
    const empty = showing(null);
    assert.strictEqual(empty.main, null);
    assert.deepStrictEqual(empty.drawnLines, []);
    assert.strictEqual(empty.span, null, "no span before the board answers: the page says so in the menu's words");
    assert.strictEqual(empty.pace, "");
});

test("a curve of one line carries no key, because there is nothing to tell apart", () => {
    const drawn = { step_ms: 100, lines: [line("flue gas", "°C", [20, 21], [0, 250])] };
    assert.strictEqual(showing(drawn).drawnLines.length, 1);
});

test("the pace is counted off the line the picture is about", () => {
    assert.ok(showing(HOLDING).pace.includes("1000 ms a point"), showing(HOLDING).pace);
    assert.ok(showing(HOLDING).pace.includes("3 s"), showing(HOLDING).pace);
});
