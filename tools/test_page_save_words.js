// devices/burner/page/form.js, `saveSaid`: what a person reads after a save. The board answers with data - the name and the value kept of each setting that changed, and a refusal as a code with the
// field and the numbers it stands on - and the page says it in its own words: the label of a setting, the words of a choice, the phrase of a refusal come from the page's table, in the language the page speaks.
// The ui table here is made up, with every place of a phrase marked: what is held is the MECHANISM - which place is filled from which field, never a phrase and never a name of the burner.
//
//     node --test ./tools/test_page_save_words.js
const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");

const ROOT = path.dirname(__dirname);
const { load, menu, texts, read } = require("./page_sources");
const scope = {};
load(["form.js"], scope);
const form = scope.form;

const settings = [
    { name: "hold_c", label: "LABEL-HOLD", value: "60", months: [5, 85], unit: "UNIT-C" },
    { name: "pause", label: "LABEL-PAUSE", value: "30", seconds: [1, 600], unit: "UNIT-S" },
    {
        name: "zone",
        label: "LABEL-ZONE",
        value: "utc",
        options: [
            { value: "utc", words: "WORDS-UTC" },
            { value: "gmt+2", words: "WORDS-GMT2" },
        ],
    },
    {
        name: "time",
        label: "LABEL-TIME",
        value: "auto",
        options: [
            { value: "auto", words: "WORDS-AUTO" },
            { value: "server", words: "WORDS-SERVER", takes: "TAKES-HOST" },
        ],
    },
];

const ui = {
    saved: "SAVED",
    nothingChanged: "NOTHING",
    refusals: {
        not_whole: "WHOLE {label} {text} {from} {to}",
        not_tenths: "TENTHS {label} {text} {from} {to} {unit}",
        none_of: "NONE {label} {text} {choices}",
        none_of_own: "OWN {own} {takes}",
        no_socket: "SOCKET {label} {role}",
        value_too_long: "LONG {label} {max} {got}",
        owner_silent: "SILENT {within_s}",
        not_written: "UNWRITTEN",
        unknown: "UNKNOWN {code}",
    },
};

const said = (answer) => form.saveSaid(answer, ui, settings);

test("what changed is said by the label of each setting and the value in the page's words, the numbers with their unit", () => {
    const got = said({
        changed: [
            { name: "hold_c", value: "45" },
            { name: "zone", value: "gmt+2" },
            { name: "pause", value: "90" },
        ],
    });
    assert.deepStrictEqual(got, { words: "SAVED LABEL-HOLD: 45 UNIT-C. LABEL-ZONE: WORDS-GMT2. LABEL-PAUSE: 90 UNIT-S", bad: false });
});

test("a choice that took a word of its own is said with that word behind the words of the choice", () => {
    const got = said({ changed: [{ name: "time", value: "server:time.example.org" }] });
    assert.deepStrictEqual(got, { words: "SAVED LABEL-TIME: WORDS-SERVER time.example.org", bad: false });
});

test("a save that changed nothing says so, and a setting the page does not know is said by its name", () => {
    assert.deepStrictEqual(said({ changed: [] }), { words: "NOTHING", bad: false });
    assert.deepStrictEqual(said({ changed: [{ name: "mystery", value: "7" }] }), { words: "SAVED mystery: 7", bad: false });
});

test("a refusal is the phrase of its code with the places filled from the fields of the answer, and the label for the name", () => {
    const got = said({ error: { code: "not_whole", name: "hold_c", text: "abc", from: 5, to: 85 } });
    assert.deepStrictEqual(got, { words: "WHOLE LABEL-HOLD abc 5 85", bad: true });
});

test("the choices a refusal lists are said by the words of the choices and not by the values the board keeps", () => {
    const got = said({ error: { code: "none_of", name: "zone", text: "mars", choices: ["utc", "gmt+2"] } });
    assert.strictEqual(got.words, "NONE LABEL-ZONE mars WORDS-UTC, WORDS-GMT2");
    assert.ok(!got.words.includes("gmt+2"), got.words);
});

test("a choice that takes a word of its own adds the value to write it behind and what it takes, in the page's words, to the refusal", () => {
    const got = said({ error: { code: "none_of", name: "time", text: "x y", choices: ["auto", "server"], own: "server" } });
    assert.strictEqual(got.words, "NONE LABEL-TIME x y WORDS-AUTO, WORDS-SERVER OWN server TAKES-HOST");
});

test("a refusal of a mode held by a sensor names the label of the setting and the role it lacks", () => {
    assert.strictEqual(said({ error: { code: "no_socket", name: "zone", role: "utc" } }).words, "SOCKET LABEL-ZONE WORDS-UTC");
});

test("a refusal of the page's own, and one of the flash, fill their places from their own fields", () => {
    assert.strictEqual(said({ error: { code: "value_too_long", name: "zone", max: 128, got: 200 } }).words, "LONG LABEL-ZONE 128 200");
    assert.strictEqual(said({ error: { code: "owner_silent", within_s: 3 } }).words, "SILENT 3");
    assert.strictEqual(said({ error: { code: "not_written" } }).words, "UNWRITTEN");
});

test("a code the page has no phrase for is said as unknown with the code - never as an object", () => {
    const got = said({ error: { code: "from_a_newer_board", name: "zone" } });
    assert.deepStrictEqual(got, { words: "UNKNOWN from_a_newer_board", bad: true });
    assert.ok(!got.words.includes("[object"), got.words);
});

test("an answer that is a text, as the other acts of the page still give, is said as it came", () => {
    assert.deepStrictEqual(said({ error: "pump cannot be switched" }), { words: "pump cannot be switched", bad: true });
});

// ---- the words of a refusal of a press: the board says a code and two numbers, the page says the sentence ----

test("a Light the board refused is said from its code and its numbers, in the page's table, and not as the code", () => {
    const table = { refusals: { light_refused_room: "ROOM {now} of {to}", light_refused_water: "WATER {now} of {to}", unknown: "UNKNOWN" } };
    assert.strictEqual(form.pressSaid("light_refused_room now=22.3 to=22.0", table), "ROOM 22.3 of 22.0");
    assert.strictEqual(form.pressSaid("light_refused_water now=61.0 to=60.0", table), "WATER 61.0 of 60.0");
});

test("a refusal of a press that the page has no words for, and an answer that is a sentence already, are said as they came", () => {
    const table = { refusals: { light_refused_room: "ROOM {now} of {to}" } };
    assert.strictEqual(form.pressSaid("light_refused_other now=1 to=2", table), "light_refused_other now=1 to=2");
    assert.strictEqual(form.pressSaid("the thermostat has no button push", table), "the thermostat has no button push");
    assert.strictEqual(form.pressSaid("", table), "");
});

test("a refusal of a number by tenths says the span and the unit the board sent with it - degrees for a target, seconds for a time of the auger", () => {
    assert.strictEqual(said({ error: { code: "not_tenths", name: "hold_c", text: "5.9", from: "6", to: "35", unit: "\u00b0C" } }).words, "TENTHS LABEL-HOLD 5.9 6 35 \u00b0C");
    assert.strictEqual(said({ error: { code: "not_tenths", name: "pause", text: "0", from: "0.1", to: "120", unit: "s" } }).words, "TENTHS LABEL-PAUSE 0 0.1 120 s");
});

// ---- the cleaning the flash did not take: the board says a code, the page has the phrase in both languages ----

test("a cleaning the board could not save is said by the page's own phrase, in both languages, and not as the code", () => {
    const refusals = menu().ui.refusals;
    const one = refusals.clean_not_written;
    assert.ok(one && one.en && one.ru, "the menu has no phrase for clean_not_written in both languages");
    for (const lang of ["en", "ru"]) {
        const said = form.pressSaid("clean_not_written", { refusals: { clean_not_written: one[lang] } });
        assert.strictEqual(said, one[lang]);
        assert.ok(!said.includes("clean_not_written") && !said.includes("{"), `${lang}: ${said}`);
    }
});
