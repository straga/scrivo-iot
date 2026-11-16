// The page speaks Russian: every word of the menu has its Russian, every item of the menu has its line in the reference tab, the words of the interface stand in the menu and not in the code of the page, and the
// language is chosen and remembered. What is held here is the SHAPE of the words - present, in both languages, the places in them kept - and never the words: a person at the boiler edits them, and a row that fails on a changed
// word says nothing of the burner.
//
//     node --test tools/test_page_russian.js
const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");

const ROOT = path.dirname(__dirname);
const { load, menu, texts, read } = require("./page_sources");
const scope = {};
load(["form.js", "wire.js", "burning.js", "dictionary.js", "board.js", "scheme.js"], scope);
const d = scope.dictionary;

const MENU = menu();
const TEXTS = texts();

// every {en, ru} word of a node, with where it stands
function words(node, where = "", out = []) {
    if (Array.isArray(node)) node.forEach((x, i) => words(x, `${where}[${i}]`, out));
    else if (node && typeof node === "object") {
        const keys = Object.keys(node);
        if (keys.length && keys.every((k) => k === "en" || k === "ru")) out.push([where, node]);
        else for (const k of keys) words(node[k], `${where}/${k}`, out);
    }
    return out;
}
const places = (text) => (text.match(/\{[^}]*\}/g) || []).sort();
const hasLetters = (text) => /[A-Za-zЀ-ӿ]{2,}/.test(text.replace(/\{[^}]*\}/g, ""));
const ALL = words(MENU);

test("every word of the menu has its Russian, and the places in it - what the page fills in - are the English one's", () => {
    const missing = ALL.filter(([, w]) => hasLetters(w.en) && (!w.ru || !w.ru.trim())).map(([where]) => where);
    assert.deepStrictEqual(missing.slice(0, 8), [], `${missing.length} words have no Russian`);
    const broken = ALL.filter(([, w]) => w.ru && JSON.stringify(places(w.ru)) !== JSON.stringify(places(w.en))).map(([where]) => where);
    assert.deepStrictEqual(broken, [], "words whose places differ between the languages");
});

test("a word with letters in it says itself in Cyrillic in Russian - a word left as it is in English is a word not translated, except a unit or a code the menu keeps", () => {
    const KEPT = /^(UTC|kW|Pa|mV|P|I|[-{}:.0-9 %°CsmVΩ·]+)$/;
    const same = ALL.filter(([, w]) => w.ru && w.ru === w.en && hasLetters(w.en) && !KEPT.test(w.en)).map(([where, w]) => `${where}=${w.en}`);
    assert.deepStrictEqual(same.slice(0, 8), [], `${same.length} words are the same in both languages`);
});

// ---- the reference tab ----

const keysToExplain = () => {
    const out = [];
    // the main screen's sections are told once, with the settings they hold: the scheme's target of the room
    for (const section of MENU.tree.main.sections) {
        out.push(`section:${section.id}`);
        for (const name of d.sectionNames(section)) out.push(name);
    }
    for (const section of [...d.pagesOf(MENU.tree).flatMap((page) => page.sections), ...MENU.tree.system.sections]) {
        out.push(`section:${section.id}`);
        for (const name of d.sectionNames(section)) out.push(name);
    }
    for (const section of MENU.modules.automaton.cycle.sections) out.push(`stage:${section.name}`);
    for (const [card, module] of Object.entries(MENU.modules)) {
        for (const key of Object.keys(module.rows || {})) if (!key.includes(".") && !/^ntc_[2-7]$/.test(key)) out.push(`row:${card}/${key}`);
    }
    return out;
};

test("the menu has a reference: a name and a line of what it is and what it is for, in both languages, for every item of the menu", () => {
    const entries = (MENU.help && MENU.help.entries) || {};
    const missing = keysToExplain().filter((k) => !entries[k] || !entries[k].name || !entries[k].text || !entries[k].name.ru || !entries[k].text.ru || !entries[k].name.en || !entries[k].text.en);
    assert.deepStrictEqual(missing.slice(0, 10), [], `${missing.length} items have no line in the reference`);
});

test("the reference names nothing the menu has not: a line with no item is a line nobody reads", () => {
    const have = new Set(keysToExplain());
    const stray = Object.keys((MENU.help && MENU.help.entries) || {}).filter((k) => !have.has(k));
    assert.deepStrictEqual(stray, []);
});

test("a line of the reference says more than its name: it is not the name said again", () => {
    const entries = (MENU.help && MENU.help.entries) || {};
    const thin = Object.entries(entries).filter(([, e]) => e.text && e.name && (e.text.ru || "").trim().length < (e.name.ru || "").trim().length + 25).map(([k]) => k);
    assert.deepStrictEqual(thin.slice(0, 10), [], `${thin.length} lines are hardly longer than their names`);
});

test("the page makes the reference out of the menu, in the tree's order, in the language asked", () => {
    const ru = d.helpOf({ menu: MENU, lang: "ru" });
    const en = d.helpOf({ menu: MENU, lang: "en" });
    assert.ok(Array.isArray(ru) && ru.length >= 5, "groups of the reference");
    const count = (groups) => groups.reduce((n, g) => n + g.items.length, 0);
    assert.strictEqual(count(ru), keysToExplain().length, "an item for every item of the menu");
    assert.strictEqual(count(en), count(ru));
    assert.ok(ru.every((g) => g.title && g.items.every((i) => i.name && i.text)), "a title for a group, a name and a line for an item");
    assert.notStrictEqual(ru[0].items[0].text, en[0].items[0].text, "Russian is not English");
});

test("the language of the page: Russian unless a person chose another, and what is stored or asked for is taken only if it is one the page has", () => {
    assert.strictEqual(d.langOf({ asked: "", stored: null }), "ru");
    assert.strictEqual(d.langOf({ asked: "", stored: "en" }), "en");
    assert.strictEqual(d.langOf({ asked: "en", stored: "ru" }), "en", "what the address asks wins over what is stored");
    assert.strictEqual(d.langOf({ asked: "", stored: "xx" }), "ru");
    assert.strictEqual(d.langOf({ asked: "klingon", stored: "en" }), "en");
});

test("the table of texts has a Russian for every row and setting it words, and the page overlays it by language", () => {
    const spoken = d.speak(TEXTS, "ru");
    assert.notDeepStrictEqual(spoken.rows, TEXTS.rows, "the rows are said in Russian");
    for (const key of Object.keys(TEXTS.rows)) assert.ok(spoken.rows[key], key);
    assert.deepStrictEqual(d.speak(TEXTS, "en").rows, TEXTS.rows, "English is the table as it is");
    const zone = spoken.settings.zone.choices.map((c) => c.words);
    assert.ok(zone.every((w) => w.split(/\s+/).length <= 2), `a zone is one or two words: ${zone.join(" | ")}`);
});

// ---- time and zone by the rules 6 and 7 ----

test("a choice of the time says no sentence under it while all is well, and the time's note speaks only when something is not right", () => {
    for (const lang of ["en", "ru"]) {
        const time = d.speak(TEXTS, lang).settings.time;
        for (const choice of time.choices) assert.ok(!choice.means, `${lang}: ${choice.value} explains itself under the switch`);
        // the states in which the time came from somewhere are the board's words for "all is well": the table words them with nothing, and the page says nothing - a state the table does not word would be drawn as its code
        const says = d.speak(TEXTS, lang).rows["settings/time.note"].says;
        for (const kind of ["came_from", "came_from_at"]) {
            const one = says.find(([when]) => when === kind);
            assert.ok(one, `${lang}: the table does not word ${kind}, so the page would draw the code`);
            assert.strictEqual(one[1], "", `${lang}: the note says something of ${kind} while the time is fine`);
        }
    }
});

// ---- the words of the interface stand in the menu ----

test("the menu holds the words of the interface: the tabs, the buttons, the headings the code used to say", () => {
    const ui = MENU.ui || {};
    for (const key of ["save", "stop", "send", "terminal", "now", "howItRuns", "modules", "noModules", "about", "restart", "goDown", "lightDark", "reference", "scheme", "burner"]) {
        assert.ok(ui[key] && ui[key].en && ui[key].ru, `ui.${key}`);
    }
});

// What a static word in the code is: a text between tags, a title or a placeholder, or a string of words handed to the page to say. A class, an event, a key, a code is no word.
function interfaceWords(source) {
    const clean = source.replace(/\/\*[\s\S]*?\*\//g, "").replace(/<!--[\s\S]*?-->/g, "");
    const found = [];
    for (const line of clean.split("\n")) {
        const code = line.replace(/(^|[^:"'`])\/\/.*$/, "$1");
        for (const m of code.matchAll(/>\s*([A-Za-z][^<>{}$`]*?[A-Za-z.])\s*</g)) found.push(m[1]);
        // a name or a label handed to a component as a quoted word: name="'board'"
        for (const m of code.matchAll(/\b(?:name|label|heading)="'([A-Za-z][^']*)'"/g)) found.push(m[1]);
        for (const m of code.matchAll(/(?<![\w-])(?:title|placeholder|aria-label)="([A-Za-z][^"]+)"/g)) if (/[ A-Z]/.test(m[1])) found.push(m[1]);
        // a string handed back or kept: a code (`none`, `at_least`) and a list of classes are no words
        const codeOrClass = (t) => /^[a-z_]+$/.test(t) || /\w-\w/.test(t);
        for (const m of code.matchAll(/\breturn\s+"([A-Za-z][^"]{2,})"/g)) if (!codeOrClass(m[1])) found.push(m[1]);
        for (const m of code.matchAll(/\b(?:const|let)\s+[A-Z_]+\s*=\s*"([A-Za-z][^"]{2,})"/g)) if (!codeOrClass(m[1])) found.push(m[1]);
        for (const m of code.matchAll(/[?:]\s*"([A-Z][a-z]{1,}(?: [a-z]+)*)"/g)) found.push(m[1]);
        for (const m of code.matchAll(/\b(?:this\.)?say\(\s*["`]([^"`$]{3,})/g)) found.push(m[1]);
        for (const m of code.matchAll(/\.note\s*=\s*"([^"]{3,})"/g)) found.push(m[1]);
        for (const m of code.matchAll(/\bnote:\s*"([^"]{3,})"/g)) found.push(m[1]);
        // a phrase of plain words in quotes - three or more of them, no dash, digit or bracket in the string: not a class, not a code
        for (const m of code.matchAll(/["'`]([a-z]{2,}(?: [a-z,]{2,}){2,})["'`]/g)) found.push(m[1]);
    }
    return found;
}

for (const file of ["page.js", "parts.js", "wire.js", "burning.js", "scheme.js", "board.js", "dictionary.js"]) {
    test(`${file} says no word to a person: they are the menu's`, () => {
        const said = interfaceWords(read(file));
        assert.deepStrictEqual(said, [], `words in the code of ${file}`);
    });
}

test("the page fills the places of a word - {name} - from what it is given, and leaves a place nobody names as it stands", () => {
    assert.strictEqual(d.spell("{s} s in {board}", { s: 20, board: "b1" }), "20 s in b1");
    assert.strictEqual(d.spell("{a} and {b}", { a: 1 }), "1 and {b}");
    assert.strictEqual(d.spell(undefined, {}), "");
    // the ones the menu says with places in them come out whole for the numbers the page gives
    const ui = d.uiOf({ menu: MENU, lang: "ru" });
    for (const [key, vars] of [["boardRestarts", { s: 20 }], ["memory", { free: 1, used: 2 }], ["typeToDown", { board: "b1" }]]) {
        assert.ok(!/\{\w+\}/.test(d.spell(ui[key], vars)), `${key} left a place unfilled`);
    }
});


// ---- the pace of a curve ----

test("the pace a curve says - the span and the step between two points - has no English word in Russian, for a span of seconds and for one of minutes", () => {
    d.useUnits(d.menuUnits({ menu: MENU, lang: "ru" }));
    const ui = d.uiOf({ menu: MENU, lang: "ru" });
    for (const points of [30, 600]) {
        const said = d.paceWords(points, 1000, ui);
        assert.ok(/\d/.test(said), `${said}: the numbers are in it`);
        assert.deepStrictEqual(said.match(/[A-Za-z]{2,}/g), null, `${said}: English words on a Russian page`);
    }
});

test("the pace a curve says in English keeps its places filled: the step and the span, the same numbers as in Russian", () => {
    d.useUnits(d.menuUnits({ menu: MENU, lang: "en" }));
    const said = d.paceWords(600, 1000, d.uiOf({ menu: MENU, lang: "en" }));
    assert.match(said, /^10 min\b.*\b1000 ms\b/, said);
    assert.ok(!/[{}]/.test(said), `${said}: a place left unfilled`);
});
