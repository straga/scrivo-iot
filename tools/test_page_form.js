// devices/burner/page/form.js: how a setting is drawn and what goes back to the board.
//
//     bun test ./tools/test_page_form.js
//
// The cases are the board's own answer, not invented ones: tools/test_page_form_state.json is /state as the burner
// sent it. A value that goes back written otherwise than the board keeps it is refused by settings::check, and on
// the page that shows up as "Socket 3: 'buffer tank' is none of free, water..." - a refusal a person cannot act on.
const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");

const ROOT = path.dirname(__dirname);
const { load, menu, texts, read } = require("./page_sources");
// Run the page's file in THIS realm, not in a vm context: objects made in another realm carry another
// Object.prototype, and deepStrictEqual then fails on values that are equal in every field.
const scope = {};
load(["form.js"], scope);
const form = scope.form;

const state = JSON.parse(fs.readFileSync(path.join(__dirname, "test_page_form_state.json"), "utf8"));
const setting = (name) => state.settings.find((s) => s.name === name);

test("a choice of four or more is a list, three or fewer a switch", () => {
    // the whole point of the stage: seven sockets of seven choices are seven lines, not fifty-six
    assert.strictEqual(setting("role_ntc_1").options.length, 7);
    assert.strictEqual(form.kind(setting("role_ntc_1")), "list");
    assert.strictEqual(setting("zone").options.length, 5);
    assert.strictEqual(form.kind(setting("zone")), "list");
    assert.strictEqual(setting("time").options.length, 3);
    assert.strictEqual(form.kind(setting("time")), "switch");
});

test("the border is four: from four a list, up to three a switch, and no row of four buttons between them", () => {
    const choices = (n) => ({ name: "x", value: "a", options: Array.from({ length: n }, (_, i) => ({ value: "v" + i, words: "w" })) });
    assert.strictEqual(form.kind(choices(2)), "switch");
    assert.strictEqual(form.kind(choices(3)), "switch");
    assert.strictEqual(form.kind(choices(4)), "list");
    assert.strictEqual(form.kind(choices(5)), "list");
    assert.strictEqual(form.LIST_FROM, 4);
    assert.strictEqual(form.SWITCH_UP_TO, 3);
});

// a switch says under it what the choice that is set means now - the board's line, one of them
const time = {
    name: "time", value: "auto",
    options: [
        { value: "auto", words: "auto", means: "the router, or the internet when it is silent" },
        { value: "internet", words: "internet", means: "the internet only (pool.ntp.org)" },
        { value: "server", words: "server", takes: "the server's name or address", means: "the server written below" },
    ],
};

test("a switch says what the choice that is set means, and only that one", () => {
    assert.strictEqual(form.means(time, "auto"), "the router, or the internet when it is silent");
    assert.strictEqual(form.means(time, "server"), "the server written below");
});

test("a choice without a meaning, or none of the choices, says nothing rather than a guess", () => {
    assert.strictEqual(form.means(setting("time"), "auto"), null, "the board of 20.09 sends no meaning");
    assert.strictEqual(form.means(time, "moon"), null);
});

test("a press on a switch sends at once, unless the choice needs a word written first", () => {
    assert.strictEqual(form.atOnce(time, "internet"), true);
    assert.strictEqual(form.atOnce(time, "server"), false);
});

test("a choice of a list waits for Save, as before", () => {
    const four = { name: "x", value: "a", options: ["a", "b", "c", "d"].map((v) => ({ value: v, words: v })) };
    assert.strictEqual(form.atOnce(four, "b"), false);
    assert.strictEqual(form.atOnce(setting("zone"), "utc"), false);
});

test("a setting with a span of numbers is a number, whatever the span is called", () => {
    // seconds, months and a plain whole number differ only in the words of the note; the field is one
    assert.strictEqual(form.kind(setting("igniter_limit")), "number");
    assert.strictEqual(form.kind(setting("poll_every")), "number");
    assert.strictEqual(form.kind(setting("ntc_address")), "number");
});

test("a value kept behind a prefix is read back as the choice and the word", () => {
    // the board keeps "own:buffer tank"; a person has to see "something else" chosen and "buffer tank" in the field
    assert.deepStrictEqual(form.picked(setting("role_ntc_3")), { value: "own", own: "buffer tank" });
    assert.deepStrictEqual(form.picked(setting("role_ntc_1")), { value: "water", own: "" });
    assert.deepStrictEqual(form.picked(setting("time")), { value: "auto", own: "" });
});

test("a word with a colon in it is not cut at the colon", () => {
    // only the prefix of a choice that takes a word counts; ":" inside the word is the word's
    const s = { name: "time", value: "server:time.google.com", options: setting("time").options };
    assert.deepStrictEqual(form.picked(s), { value: "server", own: "time.google.com" });
});

test("a value that is none of the choices is shown as itself, not silently dropped", () => {
    // a board upgraded under a page that has not reloaded: better a value a person can see than an empty field
    const s = { name: "zone", value: "riga", options: setting("zone").options };
    assert.deepStrictEqual(form.picked(s), { value: "riga", own: "" });
});

test("only the choice that takes a word has a field, and it carries the board's own words", () => {
    assert.strictEqual(form.takesOf(setting("role_ntc_1"), "own"), "what it measures, in your own words");
    assert.strictEqual(form.takesOf(setting("role_ntc_1"), "water"), null);
    assert.strictEqual(form.takesOf(setting("role_ntc_1"), "free"), null);
    assert.strictEqual(form.takesOf(setting("time"), "server"), "the server's name or address");
    assert.strictEqual(form.takesOf(setting("igniter_limit"), "300"), null);
});

test("a chosen word goes back behind its prefix, because that is how the board keeps it", () => {
    // settings::check refuses a bare "buffer tank" for a role: a misspelt choice would be taken for a name
    assert.strictEqual(form.sent(setting("role_ntc_3"), "own", "buffer tank"), "own:buffer tank");
    assert.strictEqual(form.sent(setting("time"), "server", "time.google.com"), "server:time.google.com");
});

test("a choice that takes no word goes back as itself", () => {
    assert.strictEqual(form.sent(setting("role_ntc_1"), "water", "anything typed earlier"), "water");
    assert.strictEqual(form.sent(setting("zone"), "kyiv", ""), "kyiv");
    assert.strictEqual(form.sent(setting("igniter_limit"), "120", ""), "120");
});

test("the word is not pasted twice onto a value that already carries it", () => {
    // picked() gives "own", the field gives the word; a value read back from the board must not become own:own:...
    assert.strictEqual(form.sent(setting("role_ntc_3"), "own:buffer tank", "buffer tank"), "own:buffer tank");
});

test("an empty word leaves the choice bare, and the board says what is wrong", () => {
    // the page does not invent a refusal of its own: settings::check names the setting and the choices
    assert.strictEqual(form.sent(setting("role_ntc_1"), "own", ""), "own:");
    assert.strictEqual(form.sent(setting("role_ntc_1"), "own", "   "), "own:");
});

test("the form is one line, escaped, and in the order it was given", () => {
    const written = form.body([["zone", "kyiv"], ["role_ntc_3", "own:buffer tank"]]);
    assert.strictEqual(written, "zone=kyiv&role_ntc_3=own%3Abuffer%20tank");
});

test("a name or a value with & or = in it does not become two settings", () => {
    const written = form.body([["role_ntc_1", "own:a&b=c"]]);
    assert.strictEqual(written.split("&").length, 1, written);
    assert.strictEqual(decodeURIComponent(written.split("=").slice(1).join("=")), "own:a&b=c");
});

test("every setting the board sent can be drawn and sent back", () => {
    // the sweep: no setting of the live board falls through the rules with an undefined
    for (const s of state.settings) {
        const how = form.kind(s);
        assert.ok(["switch", "list", "number", "text"].includes(how), `${s.name}: ${how}`);
        const { value, own } = form.picked(s);
        assert.strictEqual(typeof value, "string", s.name);
        assert.strictEqual(form.sent(s, value, own), s.value, `${s.name} does not come back as the board keeps it`);
    }
});

test("a control a person is holding shows their hand, not the board", () => {
    // The defect this was written from: the slider's value was bound to what the board last said, and the board
    // is asked every two seconds - so a handle held longer than that was thrown back to the old percent under
    // the finger. Settings in the same file already did it right; the slider did not.
    assert.strictEqual(form.handle(45, 30), 45);
});

test("a control nobody is holding shows the board", () => {
    assert.strictEqual(form.handle(undefined, 30), 30);
});

test("a hand holding zero is a hand, not an absence", () => {
    // `held || fromBoard` would read 0 as nothing and snap the handle back to the board's percent - exactly
    // where Stop leaves it, so dragging up from a stopped fan would fight the page.
    assert.strictEqual(form.handle(0, 30), 0);
});

// ---- a field a person is editing, and the board changed under it ----

test("a field held by a person that the board holds otherwise says what the board holds", () => {
    // the board went 51 -> 42 from the console while the person had typed 55 and not saved
    const auto_pa = { name: "auto_pa", value: "42", months: [0, 300] };
    assert.strictEqual(form.aside(auto_pa, { value: "55", own: "" }), "on the board: 42");
    // a choice says the words the page shows for it, not the value the board keeps
    const zone = setting("zone");
    const other = zone.options.find((o) => o.value !== zone.value);
    const kept = zone.options.find((o) => o.value === zone.value);
    assert.strictEqual(form.aside(zone, { value: other.value, own: "" }), `on the board: ${kept.words}`);
});

test("a field the board holds the same says nothing beside it", () => {
    // the pair: what the person holds is what the board holds - no line, whether or not they touched it
    const auto_pa = { name: "auto_pa", value: "42", months: [0, 300] };
    assert.strictEqual(form.aside(auto_pa, { value: "42", own: "" }), null);
    assert.strictEqual(form.aside(auto_pa, form.picked(auto_pa)), null);
    // a choice with a word of one's own is the same when both halves are
    const socket = setting("role_ntc_1");
    const own = socket.options.find((o) => o.takes);
    const held = { ...socket, value: `${own.value}:buffer tank` };
    assert.strictEqual(form.aside(held, { value: own.value, own: "buffer tank" }), null);
    assert.strictEqual(form.aside(held, { value: own.value, own: "boiler" }),
        `on the board: ${own.words} buffer tank`);
});


// ---- two numbers of one thing on one line: a setpoint and its hysteresis ----

const named = (name, value = "0") => ({ name, value, label: name, whole: [1, 85], unit: "UNIT" });

test("a pair of settings stands on one line at the place of the first, and the second is not drawn again", () => {
    const list = [named("mode"), named("hold"), named("drop"), named("step")];
    const rows = form.rows(list, [{ of: ["hold", "drop"], second: "HYST" }]);
    assert.deepStrictEqual(rows.map((r) => (r.pair ? ["pair", r.first.name, r.second.name, r.label] : ["one", r.setting.name])), [["one", "mode"], ["pair", "hold", "drop", "HYST"], ["one", "step"]]);
});

test("without a pair every setting is a line of its own, in the order of the list", () => {
    const list = [named("a"), named("b"), named("c")];
    assert.deepStrictEqual(form.rows(list, []).map((r) => r.setting.name), ["a", "b", "c"]);
    assert.deepStrictEqual(form.rows(list, undefined).map((r) => r.setting.name), ["a", "b", "c"]);
});

test("a pair whose second setting is not in the list is not a pair: the first stands alone and nothing is lost", () => {
    const rows = form.rows([named("hold"), named("step")], [{ of: ["hold", "drop"], second: "HYST" }]);
    assert.deepStrictEqual(rows.map((r) => (r.pair ? "pair" : r.setting.name)), ["hold", "step"]);
});

test("a pair says whether it makes a range and what its first is called, by what the menu says and not by the settings' names", () => {
    const list = [named("hold"), named("drop"), named("mid"), named("mid_c")];
    const rows = form.rows(list, [{ of: ["hold", "drop"], second: "HYST", range: true }, { of: ["mid", "mid_c"], first: "MIDDLE", second: "NEARER" }]);
    assert.deepStrictEqual(rows.map((r) => [r.range, r.firstLabel]), [[true, null], [false, "MIDDLE"]], "the range is the menu's say-so, the first's own label stands until the menu names another");
    assert.strictEqual(form.rows(list, [{ of: ["hold", "drop"], second: "HYST" }])[0].range, false, "a pair the menu gives no range makes none");
});

test("a single setting of a list that has pairs is called what the menu's `labels` call it, the others by their own", () => {
    const list = [named("hold"), named("drop"), named("step"), named("lock")];
    const rows = form.rows(list, [{ of: ["hold", "drop"], second: "HYST" }], { step: "BIG" });
    assert.deepStrictEqual(rows.filter((r) => !r.pair).map((r) => [r.setting.name, r.label]), [["step", "BIG"], ["lock", null]]);
});

test("the range of a pair is from the setpoint less its hysteresis up to the setpoint, in the unit of the setpoint", () => {
    assert.strictEqual(form.pairRange(named("hold", "45"), "4"), "41…45 UNIT");
    assert.strictEqual(form.pairRange(named("hold", "60"), "10"), "50…60 UNIT");
    assert.strictEqual(form.pairRange(named("pump", "35"), "4"), "31…35 UNIT");
});

test("the range says nothing while either number is not a number, and nothing for a hysteresis that takes the range under nought", () => {
    assert.strictEqual(form.pairRange(named("hold", ""), "4"), "");
    assert.strictEqual(form.pairRange(named("hold", "45"), ""), "");
    assert.strictEqual(form.pairRange(named("hold", "45"), "abc"), "");
    assert.strictEqual(form.pairRange(named("hold", "45"), "4.5"), "");
    assert.strictEqual(form.pairRange(named("hold", "3"), "5"), "");
});
