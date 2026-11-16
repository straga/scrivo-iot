// The profile by the clock as a page says it (moved to the demand's card in 6.19): the card's rows of what stands - the profile, what chose it with the window and when it ends - and the time of day a number of minutes is. What is held
// here is what a person is TOLD, said in the one place the board does not say it: the number of the window, the end as HH:MM, a board that does not know the time saying so, and each profile by words of its own. The words themselves are the
// menu's and a person at the boiler edits them, so a row here never holds a phrase.
//
//     node --test tools/test_page_schedule.js
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

const DICT = JSON.parse(execSync("cargo +stable run -q -p burner_api --example served_json", { cwd: ROOT, encoding: "utf8", maxBuffer: 1 << 26 }));
const MENU = menu();
const LANGS = ["en", "ru"];

// the values of the table, nothing said, and the demand's card said as asked
function values(said) {
    const all = DICT.things.map(() => null);
    for (const [name, value, part] of said) {
        const at = part ? d.part(DICT, d.address(DICT, "demand", part), name) : d.address(DICT, "demand", name);
        assert.notStrictEqual(at, null, `the card has no ${part ? "part" : "row"} ${name}`);
        all[at] = value;
    }
    return all;
}

function line(lang, said) {
    d.useUnits(MENU.units, lang);
    const texts = { menu: MENU, lang };
    const v = values(said);
    return d.render(DICT, texts, v, d.address(DICT, "demand", "source")).words;
}

function profileWords(lang, profile) {
    d.useUnits(MENU.units, lang);
    return d.render(DICT, { menu: MENU, lang }, values([["profile", profile]]), d.address(DICT, "demand", "profile")).words;
}

test("a minute of the day is HH:MM, from midnight to the last minute", () => {
    assert.strictEqual(d.shown(0, "daytime"), "00:00");
    assert.strictEqual(d.shown(371, "daytime"), "06:11");
    assert.strictEqual(d.shown(1320, "daytime"), "22:00");
    assert.strictEqual(d.shown(1439, "daytime"), "23:59");
});

test("each profile has words of its own, and a profile that stands for nothing says none", () => {
    for (const lang of LANGS) {
        const said = ["comfort", "eco", "off"].map((p) => profileWords(lang, p));
        assert.strictEqual(new Set(said).size, 3, `${lang}: three profiles, three words: ${said}`);
        assert.ok(said.every((w) => w.length > 1 && !w.includes("{")), `${lang}: ${said}`);
        assert.ok(profileWords(lang, { unknown: "none" }).length <= 3, `${lang}: a profile that is none says a dash`);
    }
});

test("a window that stands is said as its number and the minute it ends at, in both languages", () => {
    for (const lang of LANGS) {
        const said = line(lang, [["profile", "eco"], ["source", "window"], ["window", 2, "source"], ["until", 360, "source"]]);
        assert.ok(said.includes("2"), `${lang}: the number of the window: ${said}`);
        assert.ok(said.includes("06:00"), `${lang}: the minute the window ends at: ${said}`);
        assert.ok(!said.includes("{") && !said.includes("}"), `${lang}: a place left unfilled: ${said}`);
    }
});

test("the main profile names no window and no end", () => {
    for (const lang of LANGS) {
        const said = line(lang, [["profile", "comfort"], ["source", "main"]]);
        assert.ok(said.length > 1, `${lang}: the main profile says something`);
        assert.ok(!/\d/.test(said), `${lang}: a number in the line of the main profile: ${said}`);
        assert.ok(!said.includes("{"), `${lang}: ${said}`);
    }
});

test("a board that does not know the time says so in a line of its own", () => {
    for (const lang of LANGS) {
        const unknown = line(lang, [["profile", "comfort"], ["source", "no_clock"]]);
        const main = line(lang, [["profile", "comfort"], ["source", "main"]]);
        assert.notStrictEqual(unknown, main, `${lang}: the time the board does not know is said as the main profile's line`);
        assert.ok(unknown.length > 1 && !unknown.includes("{"), `${lang}: ${unknown}`);
    }
});

test("where nothing stands - nobody asks - the line says nothing of a profile", () => {
    for (const lang of LANGS) {
        const said = line(lang, [["source", { unknown: "none" }]]);
        assert.ok(said.length <= 3 && !said.includes("{"), `${lang}: ${said}`);
    }
});

test("each window is kept by the board as a choice between not used and a window that takes a line, and the page words it as a form, with no hint of a line to type", () => {
    for (const n of [1, 2, 3, 4, 5, 6]) {
        const setting = DICT.settings.find((s) => s.name === `window_${n}`);
        assert.ok(setting, `no setting window_${n}`);
        assert.deepStrictEqual(setting.choices.map((c) => [c.value, Boolean(c.takes)]), [["off", false], ["set", true]]);
        const words = MENU.modules.demand.settings[`window_${n}`];
        assert.ok(words && words.label && words.label.en && words.label.ru, `the menu has no label for window_${n}`);
        assert.strictEqual(words.form, "frame", "drawn as a form");
        assert.ok(!words.choices, "a form has no choices to word: a person is not shown the line");
    }
});
