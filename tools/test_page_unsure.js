// A count that is not sure of its day: the board says `{"unknown": "day_unsure"}` for what it burned or used today and this month, and the page draws a
// dash - not the code, and not the kept number of a day it has not seen. The total is a number always. What is held here is what a person reads on the rows of
// the real dictionaries and the real menus of the burner and the Dual, with the words the MENU gives, not a dash the page draws by itself.
//
//     node --test tools/test_page_unsure.js
const test = require("node:test");
const assert = require("node:assert");
const { execSync } = require("node:child_process");

const { ROOT, load, menu, texts } = require("./page_sources");
const scope = {};
load(["form.js", "wire.js", "dictionary.js"], scope);
const d = scope.dictionary;

const served = (crate) => JSON.parse(execSync(`cargo +stable run -q -p ${crate} --example served_json`, { cwd: ROOT, encoding: "utf8", maxBuffer: 1 << 26 }));

// a device: its dictionary, its page's texts, and what the board says at each address
function device(crate, name) {
    const dict = served(crate);
    const all = texts(name);
    all.menu = menu(name);
    const at = (card, row, part) => dict.things.findIndex((t) => t.card === card && t.name === (part || row) && (part ? t.part_of === row : !t.part_of));
    const said = new Array(dict.things.length).fill(undefined);
    const words = (card, row) => d.render(dict, all, said, at(card, row)).words;
    return { dict, all, at, said, words };
}

const BURNER = device("burner_api", "burner");
const DUAL = device("bench_dualr3_api", "bench_dualr3");

test("the burner: today and the month of a count not sure of its day are a dash, and the total is its number", () => {
    const b = BURNER;
    b.said[b.at("pellets", "today")] = { unknown: "day_unsure" };
    b.said[b.at("pellets", "month")] = { unknown: "day_unsure" };
    b.said[b.at("pellets", "total")] = 5.0;
    assert.strictEqual(b.words("pellets", "today"), "-");
    assert.strictEqual(b.words("pellets", "month"), "-");
    assert.match(b.words("pellets", "total"), /5\.0/);
});

test("the burner: when the count is sure of its day, today and the month are the numbers", () => {
    const b = BURNER;
    b.said[b.at("pellets", "today")] = 0.0;
    b.said[b.at("pellets", "month")] = 3.2;
    assert.match(b.words("pellets", "today"), /0\.0/);
    assert.match(b.words("pellets", "month"), /3\.2/);
});

test("the Dual: the energy of today and the month, and each output of them, is a dash while the count is not sure of its day, and the total is the number it is", () => {
    const u = DUAL;
    for (const row of ["today", "month"]) {
        u.said[u.at("energy", row)] = { unknown: "day_unsure" };
        u.said[u.at("energy", row, "l1")] = { unknown: "day_unsure" };
        u.said[u.at("energy", row, "l2")] = { unknown: "day_unsure" };
    }
    u.said[u.at("energy", "total")] = 4.569;
    u.said[u.at("energy", "total", "l1")] = 2.485;
    u.said[u.at("energy", "total", "l2")] = 2.084;
    assert.strictEqual(u.words("energy", "today"), "-");
    assert.strictEqual(u.words("energy", "month"), "-");
    assert.match(u.words("energy", "total"), /4\.569/);
});

test("the Dual: a count that did not start says a dash, as one unsure of its day does: a row's value is a number or a word of state, and the label says what the row is", () => {
    const u = DUAL;
    u.said[u.at("energy", "today")] = { unknown: "not_counting" };
    assert.strictEqual(u.words("energy", "today"), "-");
});

test("the Dual: when the count is sure of its day, the energy of today and the month is a number again", () => {
    const u = DUAL;
    u.said[u.at("energy", "today")] = 0.122;
    assert.match(u.words("energy", "today"), /0\.122/);
});

test("every reason a row of the burner or of the Dual may give for an unknown value has words in the menu", () => {
    for (const { dict, all } of [BURNER, DUAL]) {
        const lost = [];
        for (const t of dict.things) {
            if (!(t.kinds || []).includes("unknown") || t.part_of) continue;
            for (const why of t.reasons || []) {
                const said = d.textsOf(all, t).says || [];
                if (!said.some(([key]) => key === `unknown:${why}` || key === "unknown")) lost.push(`${t.card}/${t.name}: ${why}`);
            }
        }
        assert.deepStrictEqual(lost.filter((l) => l.includes("day_unsure")), [], "a reason with no words is drawn as its code");
    }
});
