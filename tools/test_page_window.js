// The window of the schedule as the page writes it (after the first look): not a line of text a person types but FIELDS - the days (every day, working days, the weekend, days of one's own), the kind (from-to, or every N hours),
// the start and the end, or the hours between and the minutes each lasts, and the profile (comfort, eco, off, none). The board keeps the one line `window_N` and the page builds it from the fields and takes it apart into them again, so what is held here is
// that every kind of window survives the trip: BUILT, and the Rust reader of the board's own (`crates/schedule`, example `window_json`) reads from it what the fields mean; PARSED, a line the board keeps gives back the fields it was built from.
// The line itself is never shown to a person: what is shown is words (`windowWords`).
//
//     node --test tools/test_page_window.js
const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");
const { execSync } = require("node:child_process");

const ROOT = path.dirname(__dirname);
const { load, menu, texts } = require("./page_sources");
const scope = {};
load(["form.js", "wire.js", "burning.js", "dictionary.js", "board.js", "scheme.js"], scope);
const form = scope.form;
const MENU = menu();

const bits = (...days) => days.map((d) => ["mon", "tue", "wed", "thu", "fri", "sat", "sun"].indexOf(d)).reduce((m, i) => m | (1 << i), 0);
const own = (...days) => ["mon", "tue", "wed", "thu", "fri", "sat", "sun"].map((d) => days.includes(d));

/** The fields of a window, over the defaults. */
const fields = (over) => ({ ...form.windowDefaults(), ...over });

const DAYS = [
    { fields: { days: "daily" }, mask: 127 },
    { fields: { days: "week" }, mask: bits("mon", "tue", "wed", "thu", "fri") },
    { fields: { days: "end" }, mask: bits("sat", "sun") },
    { fields: { days: "custom", own: own("mon") }, mask: bits("mon") },
    { fields: { days: "custom", own: own("sun") }, mask: bits("sun") },
    { fields: { days: "custom", own: own("mon", "wed", "fri") }, mask: bits("mon", "wed", "fri") },
    { fields: { days: "custom", own: own("sat", "sun", "mon") }, mask: bits("sat", "sun", "mon") },
    { fields: { days: "custom", own: own("mon", "tue", "thu", "fri", "sat", "sun") }, mask: 127 & ~bits("wed") },
];
const BETWEEN = [
    { fields: { kind: "between", from: "00:00", to: "23:59" }, span: { between: [0, 1439] } },
    { fields: { kind: "between", from: "22:00", to: "06:00" }, span: { between: [1320, 360] } },
    { fields: { kind: "between", from: "06:30", to: "08:00" }, span: { between: [390, 480] } },
];
const EVERY = [
    { fields: { kind: "every", from: "07:00", every: 3, minutes: 20 }, span: { every: [420, 3, 20] } },
    { fields: { kind: "every", from: "00:00", every: 1, minutes: 60 }, span: { every: [0, 1, 60] } },
    { fields: { kind: "every", from: "23:30", every: 24, minutes: 60 }, span: { every: [1410, 24, 60] } },
    { fields: { kind: "every", from: "13:00", every: 24, minutes: 1440 }, span: { every: [780, 24, 1440] } },
    { fields: { kind: "every", from: "06:15", every: 2, minutes: 1 }, span: { every: [375, 2, 1] } },
];
const PROFILES = ["comfort", "eco", "off", "none"];

/** Every window the fields can make, each with what it means. */
function cases() {
    const out = [];
    for (const d of DAYS) for (const s of [...BETWEEN, ...EVERY]) for (const profile of PROFILES) out.push({ fields: fields({ ...d.fields, ...s.fields, profile }), mask: d.mask, span: s.span, profile: profile === "none" ? null : profile });
    return out;
}

/** What the board's own reader makes of each text, by the example of the thermostat crate. */
function rust(texts) {
    const out = execSync("cargo +stable run -q -p schedule --example window_json", { cwd: ROOT, input: texts.join("\n") + "\n", encoding: "utf8", maxBuffer: 1 << 24 });
    return out.trim().split("\n").map((line) => JSON.parse(line));
}

/** The fields a kind keeps: what the other kind keeps is not part of the window. */
function kept(f) {
    const base = { days: f.days, own: f.days === "custom" ? f.own : [], kind: f.kind, profile: f.profile };
    return f.kind === "between" ? { ...base, from: f.from, to: f.to } : { ...base, from: f.from, every: f.every, minutes: f.minutes };
}

test("every window the fields can make is built into a line the board's own reader takes, and reads as the fields mean", () => {
    const all = cases();
    const texts = all.map((c) => form.windowText(c.fields));
    assert.ok(texts.every((t) => typeof t === "string"), "a window the fields make has no line: " + texts.findIndex((t) => typeof t !== "string"));
    const read = rust(texts);
    all.forEach((c, i) => {
        const { opens, ...meaning } = read[i];
        assert.deepStrictEqual(meaning, { ok: true, days: c.mask, span: c.span, profile: c.profile }, `${texts[i]}`);
    });
    assert.strictEqual(all.length, 8 * 8 * 4, "8 sets of days, 3 from-to and 5 every, 4 profiles");
});

test("a line the board keeps is taken apart into the fields it was built from, for every kind of window", () => {
    for (const c of cases()) {
        const text = form.windowText(c.fields);
        assert.deepStrictEqual(kept(form.windowParse(text)), kept(c.fields), text);
        assert.deepStrictEqual(kept(form.windowOf("set:" + text)), kept(c.fields), "set:" + text);
    }
});

test("a line is the same line again after the trip through the fields, so a save that changed nothing changes nothing", () => {
    for (const c of cases()) {
        const text = form.windowText(c.fields);
        assert.strictEqual(form.windowText(form.windowParse(text)), text);
    }
});

test("the days of the choice are the choice's own however the line writes them: every day, working days, the weekend, and any other set is days of one's own", () => {
    assert.strictEqual(form.windowParse("daily 22:00-06:00 eco").days, "daily");
    assert.strictEqual(form.windowParse("mon-sun 22:00-06:00 eco").days, "daily");
    assert.strictEqual(form.windowParse("mon-fri 22:00-06:00 eco").days, "week");
    assert.strictEqual(form.windowParse("mon,tue,wed,thu,fri 22:00-06:00 eco").days, "week");
    assert.strictEqual(form.windowParse("sat-sun 22:00-06:00 eco").days, "end");
    assert.strictEqual(form.windowParse("sat,sun 22:00-06:00 eco").days, "end");
    const mine = form.windowParse("sat-mon 22:00-06:00 eco");
    assert.deepStrictEqual([mine.days, mine.own], ["custom", own("sat", "sun", "mon")], "a range that runs past Sunday comes round");
    assert.deepStrictEqual(form.windowParse("mon-wed,sat 07:00-09:00 eco").own, own("mon", "tue", "wed", "sat"));
});

test("everything the board's reader refuses is no window to the page either, and everything it takes is one", () => {
    const ok = ["daily 22:00-06:00 eco", "mon-fri 06:30-08:00 comfort", "sat-sun 08:00-10:00 off", "mon,wed,fri 07:00-09:00 eco", "mon-wed,sat 07:00-09:00 eco", "sat-mon 07:00-09:00 eco", "daily 07:00+3h/20m comfort", "mon-fri 06:15+24h/60m eco", "daily 00:00+1h/60m eco", "daily 22:00-06:00 none", "  Mon-Fri   06:30-08:00   COMFORT "];
    const no = ["", "daily", "daily 22:00-06:00", "daily 22:00-06:00 eco extra", "daily 22:00-22:00 eco", "daily 24:00-06:00 eco", "daily 22:60-06:00 eco", "daily 7:00-9:00 eco", "daily 22:00-6:00 eco", "someday 22:00-06:00 eco", "mon- 22:00-06:00 eco", "-fri 22:00-06:00 eco", "mon,,fri 22:00-06:00 eco", "daily 22:00-06:00 warm", "daily 07:00+0h/20m comfort", "daily 07:00+25h/20m comfort", "daily 07:00+3h/0m comfort", "daily 07:00+3h/181m comfort", "daily 07:00+3h/20 comfort", "daily 07:00+3/20m comfort", "daily 07:00+h/20m comfort"];
    const read = rust([...ok, ...no]);
    ok.forEach((t, i) => {
        assert.strictEqual(read[i].ok, true, t);
        assert.notStrictEqual(form.windowParse(t), null, `${t} is a window`);
    });
    no.forEach((t, i) => {
        assert.strictEqual(read[ok.length + i].ok, false, t);
        assert.strictEqual(form.windowParse(t), null, `${t} is no window`);
    });
});

test("fields that make no window say which field is wrong, and have no line", () => {
    const bad = [
        [fields({ days: "custom", own: own() }), "days"],
        [fields({ kind: "between", from: "", to: "06:00" }), "from"],
        [fields({ kind: "between", from: "22:00", to: "" }), "to"],
        [fields({ kind: "between", from: "22:00", to: "22:00" }), "same"],
        [fields({ kind: "every", from: "07:00", every: 0, minutes: 20 }), "hours"],
        [fields({ kind: "every", from: "07:00", every: 25, minutes: 20 }), "hours"],
        [fields({ kind: "every", from: "07:00", every: "x", minutes: 20 }), "hours"],
        [fields({ kind: "every", from: "07:00", every: 3, minutes: 0 }), "minutes"],
        [fields({ kind: "every", from: "07:00", every: 3, minutes: 181 }), "minutes"],
        [fields({ kind: "every", from: "07:00", every: 3, minutes: 1.5 }), "minutes"],
        [fields({ kind: "every", from: "", every: 3, minutes: 20 }), "from"],
    ];
    for (const [f, why] of bad) {
        assert.strictEqual(form.windowProblem(f), why, JSON.stringify(f));
        assert.strictEqual(form.windowText(f), null, JSON.stringify(f));
    }
    for (const c of cases()) assert.strictEqual(form.windowProblem(c.fields), null, form.windowText(c.fields));
});

test("a field that the other kind keeps is not asked: the hours of an every do not make a from-to wrong, and the end of a from-to does not make an every wrong", () => {
    assert.strictEqual(form.windowProblem(fields({ kind: "between", from: "22:00", to: "06:00", every: 99, minutes: 0 })), null);
    assert.strictEqual(form.windowProblem(fields({ kind: "every", from: "07:00", every: 3, minutes: 20, to: "" })), null);
});

test("a window nobody wrote is shown with a window to start from and no profile, and left as it was until a profile is picked", () => {
    const fresh = form.windowOf("off");
    assert.strictEqual(fresh.profile, "none");
    assert.notStrictEqual(form.windowText(fresh), null, "the fields it starts from make a window, so picking a profile is all it takes");
    assert.deepStrictEqual(form.windowEdit(fresh), { value: "off", own: "" }, "nothing changed: the board keeps what it kept");
    const picked = form.windowEdit({ ...fresh, profile: "eco" });
    assert.strictEqual(picked.value, "set");
    assert.strictEqual(form.windowParse(picked.own).profile, "eco");
    const written = form.windowEdit({ ...fresh, days: "week" });
    assert.strictEqual(written.value, "set", "a window with its days changed is written, and off while its profile is none");
    assert.strictEqual(form.windowParse(written.own).profile, "none");
});

test("fields that make no window are an edit with no line, so the page can tell them from one it may send", () => {
    assert.deepStrictEqual(form.windowEdit(fields({ days: "custom", own: own(), profile: "eco" })), { value: "set", own: "" });
});

test("what a person reads of a window the board keeps is words, never the line", () => {
    const ui = { window: { unused: "not used", days: { daily: "every day", week: "working days", end: "weekend" }, day: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"], profile: { comfort: "comfort", eco: "eco", off: "off", none: "none" }, between: "{from}-{to}", every_line: "{from}, every {every} h for {minutes} min" } };
    assert.strictEqual(form.windowWords("off", ui), "not used");
    const said = form.windowWords("set:mon-fri 22:00-06:00 eco", ui);
    for (const piece of ["working days", "22:00", "06:00", "eco"]) assert.ok(said.includes(piece), `${piece} in ${said}`);
    assert.ok(!said.includes("mon-fri") && !said.includes("set:"), said);
    const every = form.windowWords("set:mon,wed 07:00+3h/20m comfort", ui);
    for (const piece of ["Mon", "Wed", "07:00", "3", "20", "comfort"]) assert.ok(every.includes(piece), `${piece} in ${every}`);
    assert.ok(!every.includes("+3h/20m"), every);
});

test("the menu words every field of the form in both languages, and the windows stand in their sections as forms and not as lines", () => {
    const words = MENU.ui.window;
    assert.ok(words, "the menu's ui has no `window` words");
    for (const key of ["unused", "days", "day", "kind", "from", "to", "every", "hours", "minutes", "profile", "bad", "between", "every_line"]) assert.ok(words[key] !== undefined, `no ui.window.${key}`);
    for (const section of [MENU.tree.main.sections, MENU.tree.modules.pages.find((p) => p.id === "demand").sections].flat().filter((s) => s.windows)) {
        assert.deepStrictEqual(section.windows, ["window_1", "window_2", "window_3", "window_4", "window_5", "window_6"]);
        assert.ok(!(section.settings || []).some((n) => n.startsWith("window_")), "a window is in `windows` and not in `settings`: a setting is drawn as a line");
    }
    for (const n of [1, 2, 3, 4, 5, 6]) assert.strictEqual(MENU.modules.demand.settings[`window_${n}`].form, "frame", `window_${n} is not worded as a form`);
});

// THE BOARD'S SETTING AS THE PAGE HOLDS IT, made from the real dictionary and the real menu: what the page sends back for a window is what the form built, and what it takes from the board is what the board kept.
const DICT = JSON.parse(execSync("cargo +stable run -q -p burner_api --example served_json", { cwd: ROOT, encoding: "utf8", maxBuffer: 1 << 26 }));

function held(name, text) {
    const all = DICT.things.map(() => null);
    all[scope.dictionary.address(DICT, "settings", name)] = { text };
    const texts = { menu: MENU, lang: "en" };
    return scope.dictionary.settingOf(DICT, texts, all, DICT.settings.find((s) => s.name === name));
}

test("a window the board keeps is held by the page as the choice `set` and the line behind it, and what the form builds goes back as `set:` and the line", () => {
    const setting = held("window_2", "set:mon-fri 22:00-06:00 eco");
    assert.deepStrictEqual(form.picked(setting), { value: "set", own: "mon-fri 22:00-06:00 eco" });
    const edit = form.windowEdit({ ...form.windowOf(setting.value), profile: "off" });
    assert.deepStrictEqual(edit, { value: "set", own: "mon-fri 22:00-06:00 off" });
    assert.strictEqual(form.sent(setting, edit.value, edit.own), "set:mon-fri 22:00-06:00 off", "the line the board takes");
    assert.strictEqual(setting.form, "frame");
});

test("a window nobody wrote is held as `off`, goes back as `off` while the fields are the ones it started from, and as `set:` and the line once a profile is picked", () => {
    const setting = held("window_3", "off");
    assert.deepStrictEqual(form.picked(setting), { value: "off", own: "" });
    const fresh = form.windowOf(setting.value);
    assert.strictEqual(form.sent(setting, form.windowEdit(fresh).value, form.windowEdit(fresh).own), "off");
    const picked = form.windowEdit({ ...fresh, profile: "comfort" });
    assert.strictEqual(form.sent(setting, picked.value, picked.own), "set:daily 22:00-06:00 comfort");
});

test("what the board answers to a save of a window is said in words, not as the line", () => {
    const setting = held("window_1", "off");
    const said = form.saveSaid({ changed: [{ name: "window_1", value: "set:mon-fri 22:00-06:00 eco" }] }, { saved: "Saved:", nothingChanged: "-", ...scope.dictionary.uiOf({ menu: MENU, lang: "en" }) }, [setting]);
    assert.ok(!said.words.includes("mon-fri") && !said.words.includes("set:"), said.words);
    for (const piece of ["Window 1", "22:00", "06:00"]) assert.ok(said.words.includes(piece), `${piece} in ${said.words}`);
});

// ---- a person sees when an every will start ----

const HM = (minute) => `${String(Math.floor(minute / 60)).padStart(2, "0")}:${String(minute % 60).padStart(2, "0")}`;

test("the starts of an every in a day are the time it starts at and every N hours after it, up to the day's own time again - five hours from twenty hundred are twenty, one, six, eleven and sixteen", () => {
    const starts = (from, every) => form.windowStarts(fields({ kind: "every", from, every, minutes: 20, profile: "eco" }));
    assert.deepStrictEqual(starts("20:00", 5), ["20:00", "01:00", "06:00", "11:00", "16:00"]);
    assert.deepStrictEqual(starts("07:00", 3), ["07:00", "10:00", "13:00", "16:00", "19:00", "22:00", "01:00", "04:00"]);
    assert.deepStrictEqual(starts("06:00", 7), ["06:00", "13:00", "20:00", "03:00"], "the ten o'clock a count that went on would give is not there");
    assert.deepStrictEqual(starts("23:30", 24), ["23:30"]);
    assert.strictEqual(starts("00:00", 1).length, 24);
});

test("a window that is from-to, or whose fields are wrong, has no starts to list", () => {
    assert.deepStrictEqual(form.windowStarts(fields({ kind: "between", from: "22:00", to: "06:00" })), []);
    assert.deepStrictEqual(form.windowStarts(fields({ kind: "every", from: "", every: 3, minutes: 20 })), []);
    assert.deepStrictEqual(form.windowStarts(fields({ kind: "every", from: "07:00", every: 0, minutes: 20 })), []);
    assert.deepStrictEqual(form.windowStarts(fields({ kind: "every", from: "07:00", every: 3, minutes: 999 })), []);
    assert.deepStrictEqual(form.windowStarts(fields({ kind: "every", from: "07:00", every: 3, minutes: 20, profile: "none" })), [], "a window with no profile is off and starts nothing");
});

test("the starts the page lists are the minutes at which the board's own reader opens the window, on every every the fields can make", () => {
    const every = cases().filter((c) => c.fields.kind === "every" && c.fields.days === "daily" && Number(c.fields.minutes) < Number(c.fields.every) * 60);
    assert.ok(every.length >= 8, `${every.length} windows tried`);
    const read = rust(every.map((c) => form.windowText(c.fields)));
    every.forEach((c, i) => {
        const listed = form.windowStarts(c.fields).map((t) => Number(t.slice(0, 2)) * 60 + Number(t.slice(3))).sort((a, b) => a - b);
        assert.deepStrictEqual(listed, read[i].opens, `${form.windowText(c.fields)}: the page says ${form.windowStarts(c.fields).map(String)}`);
    });
});

test("the menu words the list of starts, with the list's place, in both languages", () => {
    const words = MENU.ui.window.starts;
    assert.ok(words && words.en.includes("{list}") && words.ru.includes("{list}"), JSON.stringify(words));
});
