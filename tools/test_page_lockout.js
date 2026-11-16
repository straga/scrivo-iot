// The wait before a light as a page says it: what is left of it, as hours, minutes and seconds - "1:23:10" -, said as what a person can do ("can be lit in ..."), and nothing where there is no wait. What is held here is the SHAPE
// of what is said - the time, in both languages, and no place left unfilled; the words are the menu's and a person at the boiler edits them.
//
//     node --test tools/test_page_lockout.js
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

function line(lang, value) {
    d.useUnits(MENU.units, lang);
    const all = DICT.things.map(() => null);
    const at = d.address(DICT, "thermostat", "lock");
    all[at] = value;
    return d.render(DICT, { menu: MENU, lang }, all, at).words;
}

test("seconds as a clock are m:ss under an hour and h:mm:ss from it, so the wait of two hours reads as two hours and not as a hundred and twenty minutes", () => {
    assert.strictEqual(d.shown(0, "clock"), "0:00");
    assert.strictEqual(d.shown(59, "clock"), "0:59");
    assert.strictEqual(d.shown(263, "clock"), "4:23");
    assert.strictEqual(d.shown(3599, "clock"), "59:59");
    assert.strictEqual(d.shown(3600, "clock"), "1:00:00");
    assert.strictEqual(d.shown(4990, "clock"), "1:23:10");
    assert.strictEqual(d.shown(7200, "clock"), "2:00:00");
    assert.strictEqual(d.shown(43200, "clock"), "12:00:00");
});

test("what is left of the wait is said as the time alone, in both languages: the label of the row says what it is the time to", () => {
    for (const lang of ["en", "ru"]) {
        assert.strictEqual(line(lang, 4990), "1:23:10", lang);
        assert.strictEqual(line(lang, 59), "0:59", lang);
    }
});

test("where there is no wait the row says a dash and no time", () => {
    for (const lang of ["en", "ru"]) {
        const said = line(lang, null);
        assert.ok(!/\d/.test(said) && said.length <= 3, `${lang}: ${said}`);
    }
});

test("the two settings stand on the thermostat's page, in the section of the setpoint, and are worded in both languages", () => {
    const page = MENU.tree.modules.pages.find((p) => p.id === "thermostat");
    const sections = [...page.sections, ...MENU.tree.main.sections];
    for (const name of ["auto_lock_min", "auto_lock_c"]) {
        const holding = sections.filter((s) => d.sectionNames(s).includes(name));
        assert.strictEqual(holding.length, 1, `${name}: the page's section, and no section of the scheme`);
        const words = MENU.modules.thermostat.settings[name];
        assert.ok(words && words.label.en && words.label.ru, `${name} has no label`);
        assert.ok(MENU.help.entries[name] && MENU.help.entries[name].text.en && MENU.help.entries[name].text.ru, `${name} has no line in the reference`);
    }
    assert.ok(MENU.modules.thermostat.rows.lock, "the row has no words");
    assert.ok(MENU.help.entries["row:thermostat/lock"], "the row has no line in the reference");
});
