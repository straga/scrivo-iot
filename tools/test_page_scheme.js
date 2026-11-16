// devices/burner/page/scheme.js: the burner drawn, judged by values and not by pixels - what each organ does now and where it stands, which
// number stands where it is measured, the flame in the pot, the fan that turns, the feed and the mode on the drawing.
// It is the one part of the page that names the plant, and it names it by the board's KEYS: a row's label is the menu's to word and a person's to
// change, so every row below carries a label that says nothing of what it is, and every word the drawing says comes from the table WORDS.
//
//     node --test tools/test_page_scheme.js
const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");

const ROOT = path.dirname(__dirname);
const { load, menu: mergedMenu } = require("./page_sources");
const scope = {};
load(["press.js", "wire.js", "dictionary.js", "scheme.js"], scope);
const scheme = scope.scheme;

// the drawing's words as the menu carries them, made up here: what is tested is where a word comes from, never the word
const WORDS = {
    title: "T0",
    parts: { hopper: "H0", pot: "P0", fan: "F0", flue: "U0", water: "W0", flame: "L0" },
    places: { flue: "n-flue", water_out: "n-out", water_back: "n-back", draught: "n-draught", boiler: "n-boiler", room: "n-room" },
    hands: { take: "h-take", give: "h-give", take_text: "h-take-text", give_text: "h-give-text" },
    feed: { pause: "f-pause", banned: "f-banned", allow: "f-allow", ban: "f-ban" },
    actions: { feeding: "a-feeding", pause: "a-pause", feed_banned: "a-feed-banned", burning: "a-burning", holds: "a-holds {}", full: "a-full", by_hand: "a-by-hand" },
    why: { water: "y-water {c} {unit}", overheated: "y-hot", no_number: "y-none" },
    gap: "g-nothing of {organ}",
    pot: { step: "p-step {kw} {next} {unit}", step_last: "p-last {kw}" },
    hold: { power: "w-power", boiler: "w-boiler", return: "w-return" },
    ask: { off: "k-stopped", unasked: "k-unasked", cold: "k-heating {now} > {to} {unit}", warm: "k-no-heat {now} / {to} {unit}", waits: "k-waits {now} / {to} {unit}", wait: "k-wait", pause: "k-pause {left}",
           manual: { room: "k-man-room {now} > {to} {unit}", water: "k-man-water {now} > {to} {unit}" },
           why: { profile_off: "k-profile", scheduled_on: "k-on", scheduled_off: "k-outside", source_silent: "k-silent", not_read: "k-not-read", starting: "k-starting" } },
    badge: { temp: "bd {what} {to} {unit} {drop}", plain: "bd {what} {to} {unit}", power: "bd {what} {kw}" },
    pellets: { today: "pl-today", month: "pl-month", total: "pl-total" },
    attention: { title: "at-title", ack: "at-ack", other: "at-other", relief: { waits: "rl-waits {n}", counting: "rl-counting {n}", spent: "rl-spent {n} {limit}" }, why: { no_fuel: "at-fuel", never_lit: "at-lit", no_fronts: "at-fronts" }, soot: { words: "at-soot {n}", action: "at-soot-do" }, rows: { "thermostat/silent_return": "at-return", "automaton/alarm": "at-alarm", "flue/flue_temp": "at-flue", "draught/blast_pressure": "at-draught" } },
    heat: { smoulder: "h-s {elapsed} {total} {on} {off} {left}" },
};

// a row as the page builds it (dictionary.js, rowOf): its card, its KEY, a label that is nobody's business, the words a person reads, its value
const LABEL = "label no code may read";
const WORDS_SAID = "words a person reads, which the page must not draw by";
const row = (group, key, value, more = {}) => ({ group, key, name: `${LABEL}: ${key}`, state: WORDS_SAID, value, ...more });
const on_ = (on) => ({ on });
const secs = (seconds) => ({ seconds });
const word = (w) => ({ word: w });
const deg = (value) => ({ number: { value, unit: "°C" } });
const pa = (value) => ({ number: { value, unit: "Pa" } });

test("the flame goes on the scheme as the word of its value, coloured by its level, in the words the menu says it", () => {
    assert.deepStrictEqual(scheme.flameOnScheme({ value: word("steady"), state: "s-words", level: "ok" }), { word: "steady", words: "s-words", level: "ok" });
    assert.deepStrictEqual(scheme.flameOnScheme({ value: word("weak"), state: "w-words", level: "warn" }), { word: "weak", words: "w-words", level: "warn" });
    assert.deepStrictEqual(scheme.flameOnScheme({ value: word("out"), state: "o-words", level: "bad" }), { word: "out", words: "o-words", level: "bad" });
});

test("a flame not read yet, or no flame row at all, puts nothing on the scheme - whatever its words say", () => {
    assert.strictEqual(scheme.flameOnScheme({ state: "steady", value: { unknown: {} } }), null);
    assert.strictEqual(scheme.flameOnScheme({ state: "steady" }), null);
    assert.strictEqual(scheme.flameOnScheme(undefined), null);
    assert.strictEqual(scheme.flameOnScheme({ value: word("sideways"), level: "ok" }), null);
});

// the relays' dots: a relay by its key, closed by its value and never by its words
test("a relay's dot is lit by its value, found by the relay's key, whatever its label and its words", () => {
    const dots = scheme.dots([row("relays", "igniter", on_(true)), row("relays", "pump", on_(false), { state: "on" })]);
    assert.deepStrictEqual(dots, [{ key: "igniter", words: `${LABEL}: igniter`, on: true }, { key: "pump", words: `${LABEL}: pump`, on: false }]);
});

test("a row of another card with a relay's key is no relay", () => {
    assert.deepStrictEqual(scheme.dots([row("automaton", "igniter", on_(true))]), []);
});

test("a number goes on the scheme from its value, and at the top of the scale as at least", () => {
    assert.strictEqual(scheme.schemeNumber({ value: deg(61.2) }), "61.2 °C");
    assert.strictEqual(scheme.schemeNumber({ value: pa(49) }), "49 Pa");
    assert.strictEqual(scheme.schemeNumber({ value: { at_least: { value: 300, unit: "Pa" } } }), "≥ 300 Pa");
});

test("degrees keep one decimal whatever their size", () => {
    assert.strictEqual(scheme.schemeNumber({ value: deg(100) }), "100.0 °C");
    assert.strictEqual(scheme.schemeNumber({ value: deg(9) }), "9.0 °C");
});

test("a sensor that reads nothing keeps its place with a dash; a sensor that is not there puts nothing", () => {
    assert.strictEqual(scheme.schemeNumber({ value: { unknown: {} } }), "-");
    assert.strictEqual(scheme.schemeNumber({ state: "under 500 mV, its own zero" }), null);
});

test("the draught stands by its right edge, with room on its left for the longest it says", () => {
    assert.strictEqual(scheme.NUMBERS.find((n) => n.place === "draught").side, "end", "the pascals grow to the right and the digits walk");
});

// WHERE A NUMBER STANDS IS THE PLACE THE MENU GAVE ITS ROW (`place` of the row, dictionary.js), not what the row is called
test("every number of the scheme stands where the menu put its row, with the menu's word for the place", () => {
    const rows = [
        row("flue", "flue_temp", deg(143), { place: "flue" }),
        row("temperatures", "ntc_2", deg(61.2), { place: "water_out" }),
        row("temperatures", "ntc_3", deg(54), { place: "water_back" }),
        row("temperatures", "ntc_1", deg(63.4), { place: "boiler" }),
        row("temperatures", "ntc_4", deg(19.8), { place: "room" }),
        row("draught", "blast_pressure", pa(49), { place: "draught" }),
    ];
    assert.deepStrictEqual(
        scheme.schemeNumbers(rows, WORDS).map((n) => `${n.words} ${n.value}`),
        ["n-flue 143.0 °C", "n-out 61.2 °C", "n-back 54.0 °C", "n-draught 49 Pa", "n-boiler 63.4 °C", "n-room 19.8 °C"],
    );
});

test("a row stands where the menu says and nowhere else: moved to another place it moves, put nowhere it is not drawn", () => {
    const at = (place) => scheme.schemeNumbers([row("temperatures", "ntc_7", deg(40), { place })], WORDS).map((n) => n.words);
    assert.deepStrictEqual(at("room"), ["n-room"]);
    assert.deepStrictEqual(at("boiler"), ["n-boiler"]);
    assert.deepStrictEqual(at(undefined), [], "no place, no number");
    assert.deepStrictEqual(at("the moon"), [], "a place the drawing does not have");
});

test("a row called what a place used to be called is not drawn there unless the menu put it there", () => {
    const rows = [
        { group: "flue", key: "x", name: "flue gas", state: "143 °C", value: deg(143) },
        { group: "draught", key: "y", name: "blast pressure", state: "49 Pa", value: pa(49) },
    ];
    assert.deepStrictEqual(scheme.schemeNumbers(rows, WORDS), []);
});

test("the pressure stands on the scheme whatever the fan is asked for, and a sensor that cannot give it keeps its place with a dash", () => {
    const shown = (fan, pressure) =>
        scheme.schemeNumbers([row("automaton", "fan", fan), row("draught", "blast_pressure", pressure, { place: "draught" })], WORDS).find((n) => n.words === "n-draught");
    for (const fan of [word("off"), word("by_hand"), word("full"), pa(51)]) assert.strictEqual(shown(fan, pa(38)).value, "38 Pa", JSON.stringify(fan));
    assert.strictEqual(shown(word("full"), { at_least: { value: 300, unit: "Pa" } }).value, "≥ 300 Pa");
    assert.strictEqual(shown(word("off"), { unknown: {} }).value, "-");
    assert.strictEqual(shown(word("off"), undefined), undefined, "read off the words");
});

// what each organ does now, from the VALUES of the automaton's card found by the board's keys. The words are the table's.
const card = (rows, more = []) => [
    row("automaton", "organs", { words: ["auger", "igniter", "pump", "fan"] }),
    ...Object.entries(rows).map(([key, value]) => (typeof value === "string" ? row("automaton", key, undefined, { state: value }) : row("automaton", key, value))),
    // the relays of the same keys, open: the scheme's dots, and not the automaton's rows
    row("relays", "auger", on_(false)),
    row("relays", "igniter", on_(false)),
    ...more,
];
const on = (organ, things, settings) => scheme.onScheme(organ, things, WORDS, settings);

test("a working auger in its dose says feeding and the seconds left", () => {
    assert.deepStrictEqual(on("auger", card({ stage: word("working"), auger: on_(true), auger_switches_in: secs(5) })), { words: "a-feeding", value: "5 s" });
});

test("a working auger in its pause says pause and the seconds left", () => {
    assert.deepStrictEqual(on("auger", card({ stage: word("working"), auger: on_(false), auger_switches_in: secs(15) })), { words: "a-pause", value: "15 s" });
});

test("the dose of an attempt at lighting is counted down on the scheme too", () => {
    assert.deepStrictEqual(on("auger", card({ stage: word("kindling"), auger: on_(true), dose_ends_in: secs(22), igniter_off_in: secs(62) })), { words: "a-feeding", value: "22 s" });
});

test("an attempt past its dose puts nothing by the auger: it is not pausing, it is done", () => {
    assert.strictEqual(on("auger", card({ stage: word("kindling"), auger: on_(false), dose_ends_in: secs(0), igniter_off_in: secs(12) })), null);
});

test("a banned feed is said by the auger, in the middle of a dose too", () => {
    const lit = card({ stage: word("kindling"), auger: on_(false), feed: on_(false), dose_ends_in: secs(22) });
    assert.deepStrictEqual(on("auger", lit), { words: "a-feed-banned", value: null });
    const working = card({ stage: word("working"), auger: on_(false), feed: on_(false), auger_switches_in: secs(9) });
    assert.deepStrictEqual(on("auger", working), { words: "a-feed-banned", value: null });
});

test("an allowed feed leaves the auger's words as they were", () => {
    assert.deepStrictEqual(on("auger", card({ stage: word("kindling"), auger: on_(true), feed: on_(true), dose_ends_in: secs(22) })), { words: "a-feeding", value: "22 s" });
});

test("an igniter burning with the fan's air says so and counts to its end", () => {
    assert.deepStrictEqual(on("igniter", card({ stage: word("kindling"), igniter: on_(true), igniter_off_in: secs(83) })), { words: "a-burning", value: "1 min 23 s" });
});

test("the fan says what it is asked for: a setpoint, full air, a hand on it", () => {
    assert.deepStrictEqual(on("fan", card({ stage: word("kindling"), fan: pa(51) })), { words: "a-holds 51 Pa", value: null });
    assert.deepStrictEqual(on("fan", card({ stage: word("purge"), fan: word("full") })), { words: "a-full", value: null });
    assert.deepStrictEqual(on("fan", card({ stage: word("stop"), fan: word("by_hand") })), { words: "a-by-hand", value: null });
});

test("a fan asked for nothing, or for a word the page does not know, says nothing", () => {
    assert.strictEqual(on("fan", card({ stage: word("stop"), fan: word("off") })), null);
    assert.strictEqual(on("fan", card({ stage: word("stop"), fan: word("sideways") })), null);
    assert.strictEqual(on("fan", card({ stage: word("stop"), fan: "full" })), null, "read off the words");
});

test("a running pump says why and nothing else - the water from the number of its setting, the overheating, the boiler giving no number - and it does not say that it runs: the pump on the drawing is green", () => {
    const from = (value) => [kept("auto_pump_on_c", value)];
    const why = (w, settings = from("35")) => on("pump", card({ stage: word("stop"), pump: on_(true), pump_by: word(w) }), settings);
    assert.deepStrictEqual(why("water"), { words: "y-water 35 °C", value: null });
    assert.deepStrictEqual(why("water", from("40")), { words: "y-water 40 °C", value: null }, "the number is the setting's, whatever it is set to");
    assert.deepStrictEqual(why("overheated"), { words: "y-hot", value: null });
    assert.deepStrictEqual(why("no_number"), { words: "y-none", value: null });
});

test("a running pump with no reason to say - the water without its setting said, a reason as words only, or one the page does not know - says nothing rather than a number made up", () => {
    const run = (by, settings) => on("pump", card({ stage: word("stop"), pump: on_(true), pump_by: by }), settings);
    assert.strictEqual(run(word("water"), []), null, "no setting said");
    assert.strictEqual(run(word("water"), [kept("auto_pump_on_c", "not a number")]), null, "a setting that is no number");
    assert.strictEqual(run("water 47 °C", [kept("auto_pump_on_c", "35")]), null, "read off the words");
    assert.strictEqual(run(word("moonlight"), [kept("auto_pump_on_c", "35")]), null);
    assert.strictEqual(on("pump", card({ stage: word("stop"), pump: on_(false), pump_by: word("water") }), [kept("auto_pump_on_c", "35")]), null, "a pump that stands says nothing");
});

test("a stopped burner puts no action by any organ", () => {
    const things = card({ stage: word("stop"), auger: on_(false), igniter: on_(false), pump: on_(false), fan: word("off"), pump_by: "water 19 °C" });
    for (const organ of ["auger", "igniter", "pump", "fan"]) assert.strictEqual(on(organ, things), null, organ);
    assert.strictEqual(on("auger", []), null, "no card at all");
});

test("an organ's action is never read off its words", () => {
    const things = card({ stage: word("working"), auger: { on: false } });
    things.find((t) => t.key === "auger" && t.group === "automaton").state = "on";
    assert.strictEqual(on("auger", things), null);
});

test("an organ is found by its key and by nothing the row is called", () => {
    // the clock of the dose under the old label of it: no key, no countdown
    const old = [{ group: "automaton", key: "label", name: "dose ends in", value: secs(22) }, row("automaton", "auger", on_(true))];
    assert.strictEqual(on("auger", old), null, "a row called `dose ends in` is not the dose's clock");
    // and the clock under its key whatever it is called
    const kept = [row("automaton", "dose_ends_in", secs(22), { name: "dose-ends" }), row("automaton", "auger", on_(true))];
    assert.deepStrictEqual(on("auger", kept), { words: "a-feeding", value: "22 s" });
});

test("the automaton's rows are the card's: a row of another card with the same key does not move an organ", () => {
    const things = [row("relays", "auger", on_(true)), row("relays", "dose_ends_in", secs(9))];
    assert.strictEqual(on("auger", things), null);
});

// the drawing says nothing of its own: every action's words are the table's, and a table with other words says other words
test("the words of an action are the table's: replace the table and the drawing says the replacement", () => {
    const other = JSON.parse(JSON.stringify(WORDS).replace(/"a-/g, '"zz-'));
    const things = card({ stage: word("working"), auger: on_(true), auger_switches_in: secs(5) });
    assert.strictEqual(scheme.onScheme("auger", things, other).words, "zz-feeding");
});

test("every organ the automaton commands has its place on the scheme", () => {
    assert.deepStrictEqual(scheme.schemeGaps(card({ stage: word("stop") }), WORDS), []);
});

test("a scheme without the igniter's place is refused, in the table's words naming the igniter", () => {
    const { igniter, ...without } = scheme.PLACES;
    assert.deepStrictEqual(scheme.schemeGaps(card({ stage: word("stop") }), WORDS, without), ["g-nothing of igniter"]);
});

test("the organs are taken as the list the board sends and never cut out of words", () => {
    const things = card({ stage: word("stop") });
    things[0].value = word("auger igniter pump fan");
    const { igniter, ...without } = scheme.PLACES;
    assert.deepStrictEqual(scheme.schemeGaps(things, WORDS, without), [], "the words were split into organs");
});

test("the fan on the scheme turns when its mode is anything but off, by the key of the fan's row", () => {
    const fan = (w, state = WORDS_SAID) => [row("fan", "holding", word(w), { state })];
    assert.deepStrictEqual(scheme.fanOnScheme(fan("hold")), { turning: true });
    assert.deepStrictEqual(scheme.fanOnScheme(fan("full")), { turning: true });
    assert.deepStrictEqual(scheme.fanOnScheme(fan("off", "purging at 100 %")), { turning: false });
    assert.deepStrictEqual(scheme.fanOnScheme([]), { turning: false }, "a board without a fan card");
    assert.deepStrictEqual(scheme.fanOnScheme([{ group: "fan", key: "gate", name: "holding", value: word("hold") }]), { turning: false }, "a row called holding is not the fan's mode");
});

test("an action on the scheme is said as it is: the word, what is left, and why", () => {
    assert.strictEqual(scheme.schemeText({ words: "feeding", value: "5 s" }), "feeding 5 s");
    assert.strictEqual(scheme.schemeText({ words: "on", value: null, by: "too hot" }), "on · too hot");
    assert.strictEqual(scheme.schemeText({ words: "full", value: null }), "full");
    assert.strictEqual(scheme.schemeText({ words: "on", value: null, by: "5 s" }), "on · 5 s");
});

// THE FEED UNDER THE SCHEME AND THE MODE: one flag by the key and the value, the words the table's
const feed = (on, state) => row("automaton", "feed", on === undefined ? undefined : { on }, { state });
const semi = (on) => row("automaton", "semi", on === undefined ? undefined : { on });

test("the feed under the scheme is allowed or banned by its value, in the table's words", () => {
    assert.deepStrictEqual(scheme.feedLine([feed(true, "any words")], WORDS), { allowed: true, words: "f-pause", press: "f-ban" });
    assert.deepStrictEqual(scheme.feedLine([feed(false, "any other words")], WORDS), { allowed: false, words: "f-banned", press: "f-allow" });
});

test("the hands' button says Give back where the switch of the hands is on and Take where it is off, each with what its press does, in the table's words", () => {
    assert.deepStrictEqual(scheme.handsLine([semi(true)], WORDS), { semi: true, button: "h-give", text: "h-give-text" });
    assert.deepStrictEqual(scheme.handsLine([semi(false)], WORDS), { semi: false, button: "h-take", text: "h-take-text" });
});

test("hands the board has not said are no hands, and the button idle with no text; a feed likewise", () => {
    assert.deepStrictEqual(scheme.handsLine([], WORDS), { semi: null, button: "h-take", text: "" });
    assert.strictEqual(scheme.handsLine([semi(undefined)], WORDS).semi, null);
    assert.strictEqual(scheme.handsLine([row("relays", "semi", on_(true))], WORDS).semi, null, "another card's row");
    assert.strictEqual(scheme.handsLine([{ group: "automaton", key: "x", name: "semi", value: on_(true) }], WORDS).semi, null, "a row called semi is not the switch");
    assert.strictEqual(scheme.feedLine([], WORDS).allowed, null);
    assert.strictEqual(scheme.feedLine([feed(undefined, "allowed")], WORDS).allowed, null);
    assert.strictEqual(scheme.feedLine([row("relays", "feed", on_(true))], WORDS).allowed, null);
});

// THE FLAME IN VOLTS TO THE HUNDREDTH: the number on the flame's pin beside its word, read off the pin's value
const pinOf = (value, group = "flame", key = "pin") => row(group, key, value);

test("the flame's number is the volts on its pin to the hundredth, by the value", () => {
    assert.strictEqual(scheme.flameVolts([pinOf({ number: { value: 900, unit: "mV" } })]), "0.90 V");
    assert.strictEqual(scheme.flameVolts([pinOf({ number: { value: 3153, unit: "mV" } })]), "3.15 V");
    assert.strictEqual(scheme.flameVolts([pinOf({ number: { value: 0, unit: "mV" } })]), "0.00 V");
});

test("a pin that reads nothing, another card's pin, or a number of another unit is a dash, not 0", () => {
    assert.strictEqual(scheme.flameVolts([pinOf({ unknown: {} })]), "-");
    assert.strictEqual(scheme.flameVolts([]), "-");
    assert.strictEqual(scheme.flameVolts([pinOf({ number: { value: 156, unit: "mV" } }, "draught")]), "-");
    assert.strictEqual(scheme.flameVolts([pinOf({ number: { value: 2, unit: "V" } })]), "-");
});

// COMPLETENESS OF THE TABLE: every word the drawing may say is a key of the menu's `scheme`, so a drawing never says a word nobody worded
test("the menu words everything the drawing says, in both languages' shape", () => {
    const menu = mergedMenu();
    const said = menu.scheme;
    assert.ok(said, "the menu has a `scheme`");
    const leaves = (node, at = "") =>
        node && typeof node === "object" && !("en" in node) ? Object.entries(node).flatMap(([k, v]) => leaves(v, `${at}/${k}`)) : [[at, node]];
    const need = leaves(scheme.WORDS_NEEDED);
    assert.ok(need.length > 20, "the drawing names the words it needs");
    const have = new Map(leaves(said));
    for (const [at] of need) {
        assert.ok(have.has(at), `the menu says nothing at scheme${at}`);
        assert.ok(typeof have.get(at).en === "string" && have.get(at).en !== "", `the menu has no English at scheme${at}`);
    }
});


// ---- the line under the drawing: on what the burner burns and why, and the smouldering in numbers ----
// Held on what the numbers and the order are, from the rows' VALUES and the settings; the words are the table's, made up above.

const doing = (w) => row("automaton", "doing", word(w));
// whether a person asked for heat: the thermostat's switch, by its code - not pressed since the board came up, on, off
const asked = (state) => row("thermostat", "switch", word(state));
const lasted = (seconds) => row("automaton", "for", secs(seconds));
const at = (place, value) => ({ ...row("temperatures", `ntc_${place}`, deg(value)), place });
const HOLD = { mode: "boiler", to: 45, kw: "6" };
// the step the thermostat's order is on: the card's `rung`, by its code - big, mid, small
const rung = (state) => row("thermostat", "rung", word(state));
const STAIRS = { ...HOLD, mid: "5", small: "3" };

test("smouldering in numbers: the time gone and the whole of it, the feed, and what is left until it goes out", () => {
    assert.strictEqual(scheme.smoulderLine([doing("smouldering"), lasted(219)], SMOULDER, WORDS), "h-s 3:39 15:00 1 s 50 s 11:21");
});

test("the time of smouldering is counted in whole minutes and two figures of seconds, and never goes under nought", () => {
    assert.strictEqual(scheme.smoulderLine([doing("smouldering"), lasted(0)], SMOULDER, WORDS), "h-s 0:00 15:00 1 s 50 s 15:00");
    assert.strictEqual(scheme.smoulderLine([doing("smouldering"), lasted(900)], SMOULDER, WORDS), "h-s 15:00 15:00 1 s 50 s 0:00");
    assert.strictEqual(scheme.smoulderLine([doing("smouldering"), lasted(1000)], SMOULDER, WORDS), "h-s 16:40 15:00 1 s 50 s 0:00");
    assert.strictEqual(scheme.smoulderLine([doing("smouldering"), lasted(65)], { ...SMOULDER, total_s: 3600 }, WORDS), "h-s 1:05 60:00 1 s 50 s 58:55");
});

test("smouldering in numbers is said in smouldering only, and not before the seconds of the stage are known", () => {
    for (const stage of ["working", "stop", "purge", "burning_off", "fault"]) assert.strictEqual(scheme.smoulderLine([doing(stage), lasted(100)], SMOULDER, WORDS), "", stage);
    assert.strictEqual(scheme.smoulderLine([doing("smouldering")], SMOULDER, WORDS), "");
    assert.strictEqual(scheme.smoulderLine([doing("smouldering"), { ...lasted(0), value: { unknown: {} } }], SMOULDER, WORDS), "");
});

const SMOULDER = { total_s: 900, on_s: 1, off_s: 50 };

const kept = (name, value) => ({ name, value, label: `a label that says nothing of ${name}` });

test("what the burner holds is read from the settings by the board's names: the mode, the setpoint as a number and the step", () => {
    const list = [kept("auto_hold", "return"), kept("auto_hold_c", "55"), kept("auto_step", "8"), kept("auto_drop_c", "4")];
    assert.deepStrictEqual(scheme.holdOf(list), { mode: "return", to: 55, kw: "8", mid: "8", small: "8" });
});

test("a setting not there yet, or a setpoint that is no number, is no hold: the line says nothing rather than a guess", () => {
    assert.strictEqual(scheme.holdOf([kept("auto_hold", "boiler"), kept("auto_step", "6")]), null);
    assert.strictEqual(scheme.holdOf([kept("auto_hold", "boiler"), kept("auto_hold_c", "x"), kept("auto_step", "6")]), null);
    assert.strictEqual(scheme.holdOf([null, kept("auto_hold", "boiler")]), null, "a list the page has not filled yet");
    assert.strictEqual(scheme.holdOf([]), null);
});

test("how the burner smoulders is read from the settings: how long, and the dose and the pause of the feed, as numbers", () => {
    const list = [kept("auto_smoulder_s", "900"), kept("auto_smould_on", "1"), kept("auto_smould_off", "50"), kept("auto_smould_pa", "0")];
    assert.deepStrictEqual(scheme.smoulderOf(list), { total_s: 900, on_s: 1, off_s: 50 });
    assert.strictEqual(scheme.smoulderOf([kept("auto_smoulder_s", "900")]), null);
    assert.strictEqual(scheme.smoulderOf([kept("auto_smoulder_s", "x"), kept("auto_smould_on", "1"), kept("auto_smould_off", "50")]), null);
});

// ---- the line says the step the thermostat ordered, not the one chosen ----

test("the distances of the steps are read from the settings by the board's names, as numbers", () => {
    assert.deepStrictEqual(scheme.stairsOf([kept("auto_mid_c", "8"), kept("auto_small_c", "3")]), { mid_c: 8, small_c: 3 });
    assert.strictEqual(scheme.stairsOf([kept("auto_mid_c", "8")]), null);
    assert.strictEqual(scheme.stairsOf([kept("auto_mid_c", "x"), kept("auto_small_c", "3")]), null);
    assert.strictEqual(scheme.stairsOf([null]), null);
});


// ---- the thermostat on the drawing: its numbers stand by the things they are about, and no line of text stands under the drawing ----
// The setpoint at the boiler, above its temperature; the step of the fire and the next threshold at the pot, while it burns; what is small and acts only now - a window of the schedule that is not the main profile, the wait before a light - at the boiler too.
// What is DECIDED is read off the rows' VALUES; what is SHOWN is the table's words made up above, and the `short` form the menu gives a row.

const STAGES = ["stop", "purge", "kindling", "rising", "working", "smouldering", "burning_off", "fault"];
const SETTINGS = [kept("auto_hold", "boiler"), kept("auto_hold_c", "44"), kept("auto_step", "6"), kept("auto_drop_c", "4"), kept("auto_step_mid", "5"), kept("auto_step_small", "3"), kept("auto_mid_c", "8"), kept("auto_small_c", "3")];
const drawn = (rows, settings = SETTINGS) => scheme.drawn(rows, settings, WORDS, scheme.stairsOf(settings));
const sp = (value) => row("thermostat", "setpoint", deg(value));
const source = (w, short) => ({ ...row("demand", "source", word(w)), short });
const lock = (seconds, short) => ({ ...row("thermostat", "lock", seconds === null ? { none: {} } : secs(seconds)), short });

const power = (settings = SETTINGS) => settings.map((s) => (s.name === "auto_hold" ? kept("auto_hold", "power") : s));
const holdBy = (mode) => SETTINGS.map((s) => (s.name === "auto_hold" ? kept("auto_hold", mode) : s));

test("the badge names what the boiler holds and to how much: the boiler and the return by the setpoint and the distance, the power by its step - one badge, whatever the stage and the switch", () => {
    for (const stage of STAGES) for (const sw of ["on", "off", "unasked"]) assert.deepStrictEqual(drawn([doing(stage), asked(sw)]).badge, { mode: "boiler", text: "bd w-boiler 44 °C 4" }, `${stage} ${sw}`);
    assert.deepStrictEqual(drawn([doing("stop")], holdBy("return")).badge, { mode: "return", text: "bd w-return 44 °C 4" });
    assert.strictEqual(drawn([doing("stop")], holdBy("room")).badge, null, "the room is no mode of the water since 6.19: a kept `room` has no word and no badge");
    assert.deepStrictEqual(drawn([doing("stop")], power()).badge, { mode: "power", text: "bd w-power 6" });
});

test("the badge says the setpoint the thermostat holds NOW - a profile's - and not the setting, and without the setting of the distance it is the setpoint alone", () => {
    assert.strictEqual(drawn([doing("stop"), sp(30)]).badge.text, "bd w-boiler 30 °C 4");
    assert.strictEqual(drawn([doing("stop")], SETTINGS.filter((s) => s.name !== "auto_drop_c")).badge.text, "bd w-boiler 44 °C");
});

test("a hold by power says the step the thermostat ordered, and a board that has not said what it holds - or the word of it - puts no badge", () => {
    assert.strictEqual(drawn([doing("working"), rung("mid")], power()).badge.text, "bd w-power 5");
    assert.strictEqual(drawn([doing("stop")], []).badge, null);
    assert.strictEqual(scheme.drawn([doing("stop")], SETTINGS, { ...WORDS, hold: {} }, null).badge, null, "a mode the menu has no word for is no badge rather than a badge without its word");
});

test("the boiler carries no label of what is set: the badge is the thermostat's one place, and nothing of it is left at the boiler", () => {
    for (const mode of ["boiler", "return", "power"]) assert.ok(!("set" in drawn([doing("stop")], holdBy(mode))), mode);
});

test("the step stands at the pot while the fire works, with the water the step burns until where the next step is another one", () => {
    assert.strictEqual(drawn([doing("working"), rung("big")]).step, "p-step 6 36 °C");
    assert.strictEqual(drawn([doing("working"), rung("mid")]).step, "p-step 5 41 °C");
    assert.strictEqual(drawn([doing("working")]).step, "p-step 6 36 °C", "a card that says no step is on the chosen one");
});

test("the last step has no threshold to say, nor has a hold by power or steps that are all one", () => {
    assert.strictEqual(drawn([doing("working"), rung("small")]).step, "p-last 3");
    assert.strictEqual(drawn([doing("working")], power()).step, "p-last 6");
    assert.strictEqual(drawn([doing("working")], SETTINGS.map((s) => (s.name === "auto_step_mid" || s.name === "auto_step_small" ? kept(s.name, "chosen") : s))).step, "p-last 6");
    assert.strictEqual(drawn([doing("working")], SETTINGS.filter((s) => s.name !== "auto_mid_c")).step, "p-last 6", "a board that has not said the distances");
});

test("the threshold is counted from the setpoint the thermostat holds now", () => {
    assert.strictEqual(drawn([doing("working"), rung("big"), sp(30)]).step, "p-step 6 22 °C");
});

test("the step stands at the pot only while it burns: in no other stage, and with no order of the thermostat said anywhere", () => {
    for (const stage of STAGES.filter((s) => s !== "working")) for (const sw of ["on", "off", "unasked"]) assert.strictEqual(drawn([doing(stage), asked(sw), row("thermostat", "heat", word("burn"))]).step, "", `${stage} ${sw}`);
    assert.deepStrictEqual(Object.keys(drawn([doing("stop"), asked("off")])).sort(), ["ask", "badge", "notes", "step"], "the drawing is told these four and no more");
});

// ---- the one button that runs the burner: Stop where the switch is on, Start where it is off ----
// Decided by the VALUES of the thermostat's switch and of the automaton's stage; the button's words are the table's. The result names the row the press goes to (`press`) and, where
// the press would be taken and not carried out, the reason (`cannot`) - a code the table words, never a sentence.

const runOf = (rows) => scheme.runButton(rows);

test("one button for the switch: Stop where it is on, Start where it is off or nobody has pressed since the board came up", () => {
    assert.deepStrictEqual(runOf([doing("working"), asked("on")]), { press: "stop", cannot: null });
    assert.deepStrictEqual(runOf([doing("stop"), asked("off")]), { press: "start", cannot: null });
    assert.deepStrictEqual(runOf([doing("stop"), asked("unasked")]), { press: "start", cannot: null });
});

test("in a fault the button is Stop whatever the switch says: Stop is the only press that acknowledges it", () => {
    for (const sw of ["on", "off", "unasked"]) assert.deepStrictEqual(runOf([doing("fault"), asked(sw)]), { press: "stop", cannot: null }, sw);
});

test("Start is refused with its reason while the fire burns out - the board takes the press and does not carry it out - and Stop never is", () => {
    for (const stage of ["burning_off", "burning_off_because"]) {
        assert.deepStrictEqual(runOf([doing(stage), asked("off")]), { press: "start", cannot: "burning_off" }, stage);
        assert.deepStrictEqual(runOf([doing(stage), asked("on")]), { press: "stop", cannot: null }, stage);
    }
});

test("Start is not refused in any other stage of a switched-off burner", () => {
    for (const stage of STAGES.filter((s) => s !== "burning_off" && s !== "fault")) assert.strictEqual(runOf([doing(stage), asked("off")]).cannot, null, stage);
});

test("a board that has not said the switch, or says it as unknown, gives no button; a stage not said does not refuse Start", () => {
    assert.deepStrictEqual(runOf([doing("stop")]), { press: null, cannot: null }, "an older image has no switch");
    assert.deepStrictEqual(runOf([doing("stop"), row("thermostat", "switch", { unknown: {} })]), { press: null, cannot: null });
    assert.deepStrictEqual(runOf([asked("off")]), { press: "start", cannot: null });
});

test("the button is decided by the VALUES of the switch and the stage and never by the words their rows say", () => {
    const liar = { ...asked("off"), state: "on, switched on" };
    const lying = { ...doing("stop"), state: "fault, burning off" };
    assert.deepStrictEqual(runOf([lying, liar]), { press: "start", cannot: null });
});

// ---- one press at a time: a button that sends an absolute command by the state it sees is locked from its press until the board says it took it ----
// Without it a second click, arriving after the board's answer has turned the button into its opposite, sends the opposite and takes the first press back. The lock is a record of the press and the state it waits for;
// time and the board's rows are handed in, so the lock is held here on values and on the clock, never on a timer.

const L = scheme.pressLock;
const T0 = 1000;

test("a press takes the lock and holds it while the board still says what it said before the press", () => {
    const lock = L.take("run", "on", T0);
    assert.deepStrictEqual(L.tick(lock, [asked("off")], T0 + 50), lock, "the switch is still off");
    assert.strictEqual(L.held(lock), true);
    assert.strictEqual(L.held(null), false);
});

test("the lock stays for a moment after the board says it took the press, so a second click aimed at the old word is not taken for a new press, and is gone after it", () => {
    const lock = L.take("run", "on", T0);
    const taken = L.tick(lock, [asked("on")], T0 + 100);
    assert.strictEqual(L.held(taken), true, "the board has said it: the button has just changed its word, and nobody has read it yet");
    assert.strictEqual(L.held(L.tick(taken, [asked("on")], T0 + 100 + L.COOL_MS - 1)), true);
    assert.strictEqual(L.tick(taken, [asked("on")], T0 + 100 + L.COOL_MS), null);
});

test("a press the board refused or did not answer is not waited for: the lock is gone at once", () => {
    const lock = L.take("run", "on", T0);
    assert.strictEqual(L.tick(lock, [asked("off")], T0 + 10, false), null);
});

test("a press the board has not taken by the end of the wait lets the button go: the limit is the longest the page waits for the board to say it", () => {
    const lock = L.take("run", "on", T0);
    assert.strictEqual(L.held(L.tick(lock, [asked("off")], T0 + L.LIMIT_MS - 1)), true);
    assert.strictEqual(L.tick(lock, [asked("off")], T0 + L.LIMIT_MS), null);
});

test("what each press waits for: the switch at the word pressed, the pause gone, the hands taken or given, the feed at the state pressed", () => {
    assert.strictEqual(L.settled(L.take("run", "off", T0), [asked("off")]), true);
    assert.strictEqual(L.settled(L.take("run", "off", T0), [asked("on")]), false);
    assert.strictEqual(L.settled(L.take("light", true, T0), [doing("stop"), asked("on"), lock(118)]), false, "the pause still counts");
    assert.strictEqual(L.settled(L.take("light", true, T0), [doing("working"), asked("on"), lock(null)]), true);
    assert.strictEqual(L.settled(L.take("hands", true, T0), [semi(true)]), true);
    assert.strictEqual(L.settled(L.take("hands", true, T0), [semi(false)]), false);
    assert.strictEqual(L.settled(L.take("feed", false, T0), [feed(false, "x")]), true);
    assert.strictEqual(L.settled(L.take("feed", false, T0), [feed(true, "x")]), false);
});

test("a press that settles on rows the board has not said yet is not settled: nothing is taken for the board's word", () => {
    assert.strictEqual(L.settled(L.take("run", "on", T0), []), false);
    assert.strictEqual(L.settled(L.take("hands", true, T0), [semi(undefined)]), false);
    assert.strictEqual(L.settled(L.take("feed", true, T0), []), false);
});

test("the lock is decided by the VALUES of the rows and never by the words they say", () => {
    const liar = { ...asked("off"), state: "on, switched on" };
    assert.strictEqual(L.settled(L.take("run", "on", T0), [liar]), false);
});

test("the second of two presses in a row is not sent: the first holds the lock, and a lock held is a press refused", () => {
    const first = L.take("run", "on", T0);
    assert.strictEqual(L.held(first), true);
    assert.strictEqual(L.take("run", "off", T0 + 5, first), first, "a press taken while another is held leaves the first as it was");
});

// ---- what a press shows while it waits: "accepted" from the press, until the board has said it, and why the lock was let go ----
// The button shows the phase of the lock, and the page says the end that is not a success (refused, silent). Both are decided on the lock, the rows and the clock handed in.

test("a press is `accepted` from the moment it is taken until the board has said it, then `answered`; no lock, no phase", () => {
    const lock = L.take("run", "on", T0);
    assert.strictEqual(L.phase(lock), "accepted", "the instant of the press");
    const waiting = L.tick(lock, [asked("off")], T0 + 3000);
    assert.strictEqual(L.phase(waiting), "accepted", "three seconds on, the board still says what it said");
    const taken = L.tick(lock, [asked("on")], T0 + 3000);
    assert.strictEqual(L.phase(taken), "answered", "the board has said it: the button has its new word");
    assert.strictEqual(L.phase(null), null);
});

test("the end of a lock is told: refused where the board refused, silent where the limit came with no word, and nothing where the board said it or the lock is still held", () => {
    const lock = L.take("run", "on", T0);
    assert.strictEqual(L.ended(lock, [asked("off")], T0 + 10, false), "refused");
    assert.strictEqual(L.ended(lock, [asked("off")], T0 + L.LIMIT_MS), "silent");
    assert.strictEqual(L.ended(lock, [asked("off")], T0 + L.LIMIT_MS - 1), null, "still waiting");
    assert.strictEqual(L.ended(lock, [asked("on")], T0 + L.LIMIT_MS), null, "the board has said it at the limit: no silence to report");
    const taken = L.tick(lock, [asked("on")], T0 + 100);
    assert.strictEqual(L.ended(taken, [asked("on")], T0 + 100 + L.COOL_MS), null, "the moment after the word has passed: an end that is no complaint");
    assert.strictEqual(L.ended(null, [], T0, false), null);
});

// ---- why the burner stands or burns: one line over the drawing ----
// What it says is decided by the VALUES of the thermostat's switch and lock and of the demand's rows and the room's sensor; the words are the table's, made up above.

const demand = (key, value) => row("demand", key, value);
const askedBy = (word_, why, target) => [demand("asked", word(word_)), demand("why", word(why)), demand("target", target === undefined ? { unknown: {} } : deg(target))];
const room = (value) => at("room", value);
const said = (things) => drawn([doing("stop"), ...things]).ask;
// the stage of the fire is what the burner DOES; the house's ask is only the reason: heat asked says "heating" where the fire is on its way up or burning, and "waits" where the automaton stands
const saidAt = (stage, things) => drawn([doing(stage), ...things]).ask;
const HEATING = ["purge", "kindling", "rising", "working", "smouldering"];

test("a burner switched off by Stop says so and what turns it on, whatever the house asks", () => {
    assert.deepStrictEqual(said([asked("off")]), { word: "off", text: "k-stopped" });
    assert.deepStrictEqual(said([asked("off"), ...askedBy("cold", "below_target", 22), room(19.6)]), { word: "off", text: "k-stopped" }, "cold house, switch off: the switch is the reason");
    assert.deepStrictEqual(said([asked("off"), lock(118)]), { word: "off", text: "k-stopped" }, "a pause is not said of a burner that is off");
});

test("a burner nobody has switched on since the board came up says it waits for the button", () => {
    assert.deepStrictEqual(said([asked("unasked"), ...askedBy("cold", "below_target", 22), room(19.6)]), { word: "unasked", text: "k-unasked" });
});

test("a switched-on burner says the reading and the target - an arrow where heat is served, a slash where it is not - and the pause that is left", () => {
    assert.deepStrictEqual(said([asked("on"), ...askedBy("warm", "target_reached", 22), room(23.8), lock(118)]), { word: "warm", text: "k-no-heat 23.8 / 22 °C \u00b7 k-pause 1:58" });
    assert.deepStrictEqual(saidAt("working", [asked("on"), ...askedBy("cold", "below_target", 22), room(19.6)]), { word: "cold", text: "k-heating 19.6 > 22 °C" });
    assert.deepStrictEqual(said([asked("on"), ...askedBy("cold", "below_target", 22), room(19.6)]), { word: "waits", text: "k-waits 19.6 / 22 °C \u00b7 k-wait" }, "heat asked, the automaton stands and no pause is counted: it waits, it does not heat");
    assert.deepStrictEqual(said([asked("on"), ...askedBy("cold", "below_target", 22), room(19.6), lock(118)]), { word: "waits", text: "k-waits 19.6 / 22 °C \u00b7 k-pause 1:58" }, "heat asked and a pause to wait out: the count says it waits");
    assert.deepStrictEqual(said([asked("on"), ...askedBy("warm", "target_reached", 22), room(23.8), lock(0)]), { word: "warm", text: "k-no-heat 23.8 / 22 °C" }, "no pause left, none said");
});

test("what the burner does is the verdict and a pause counted after a stop is added to it, whatever stage the fire is in", () => {
    for (const stage of STAGES) assert.strictEqual(saidAt(stage, [asked("on"), ...askedBy("warm", "target_reached", 22), room(23.8), lock(118)]).text, "k-no-heat 23.8 / 22 °C \u00b7 k-pause 1:58", stage);
    for (const stage of STAGES.filter((one) => !HEATING.includes(one))) assert.strictEqual(saidAt(stage, [asked("on"), ...askedBy("cold", "below_target", 22), room(19.6), lock(118)]).word, "waits", stage);
    for (const stage of HEATING) assert.strictEqual(saidAt(stage, [asked("on"), ...askedBy("cold", "below_target", 22), room(19.6), lock(118)]).word, "cold", `${stage}: a fire that burns heats, the pause is no longer the reason`);
});

// ---- the pause: the press that ends it now ----
// The pause after the fire went out ends by itself, by a press of Start, or when the boiler's water is below `auto_lock_c`; the line counts it down, and the page has "light now" for the second. What the water must be below stands in the settings.

test("the water the pause ends below is not said in the line, whatever the setting holds", () => {
    const withLockC = (c) => [...SETTINGS, kept("auto_lock_c", String(c))];
    const things = [asked("on"), ...askedBy("cold", "below_target", 22), room(19.6), lock(118)];
    assert.strictEqual(drawn([doing("stop"), ...things], withLockC(30)).ask.text, drawn([doing("stop"), ...things], SETTINGS).ask.text);
    assert.strictEqual(drawn([doing("stop"), ...things], withLockC(30)).ask.text, "k-waits 19.6 / 22 °C \u00b7 k-pause 1:58");
});

test("light now: while a switched-on burner stands in a stop with no fire - in a pause and where heat is not needed - and in no other stage", () => {
    assert.strictEqual(scheme.lightNow([doing("stop"), asked("on"), lock(118)]), true, "in a pause");
    assert.strictEqual(scheme.lightNow([doing("stop"), asked("on"), lock(0)]), true, "the pause is over and the water does not ask");
    assert.strictEqual(scheme.lightNow([doing("stop"), asked("on"), lock(null)]), true, "no pause");
    assert.strictEqual(scheme.lightNow([doing("stop"), asked("on")]), true, "the board says no pause row");
    for (const stage of STAGES.filter((one) => one !== "stop")) assert.strictEqual(scheme.lightNow([doing(stage), asked("on"), lock(118)]), false, `${stage}: a fire that burns, burns out or faults is not lit by hand`);
    assert.strictEqual(scheme.lightNow([asked("on"), lock(118)]), false, "the board has not said the stage");
    assert.strictEqual(scheme.lightNow([doing("stop"), asked("off"), lock(118)]), false, "a burner that is off has Start, which switches it on");
    assert.strictEqual(scheme.lightNow([doing("stop"), asked("unasked"), lock(118)]), false);
});

test("light now is decided by the values of the switch and the stage and never by the words their rows say", () => {
    const liar = { ...asked("off"), state: "on, switched on" };
    const lying = { ...doing("working"), state: "stop, no fire" };
    assert.strictEqual(scheme.lightNow([doing("stop"), liar, lock(118)]), false);
    assert.strictEqual(scheme.lightNow([lying, asked("on"), lock(118)]), false);
});

test("where the board has not said what the automaton does, heat asked is not said to be heated", () => {
    assert.strictEqual(drawn([asked("on"), ...askedBy("cold", "below_target", 22), room(19.6)]).ask.word, "waits");
    assert.strictEqual(drawn([asked("on"), ...askedBy("warm", "target_reached", 22), room(23.8)]).ask.word, "warm", "no heat needed is the house's word and stands");
});

test("the reasons that are not numbers say their own words in the place of the numbers", () => {
    assert.strictEqual(saidAt("working", [asked("on"), ...askedBy("cold", "scheduled_on"), room(19.6)]).text, "k-on", "the schedule asks and no room does: the room's reading stands nowhere in the line");
    assert.strictEqual(said([asked("on"), ...askedBy("warm", "scheduled_off")]).text, "k-outside");
    assert.strictEqual(said([asked("on"), ...askedBy("warm", "profile_off")]).text, "k-profile");
    assert.strictEqual(saidAt("working", [asked("on"), ...askedBy("cold", "source_silent", 22)]).text, "k-silent");
    assert.strictEqual(said([asked("on"), ...askedBy("warm", "not_read")]).text, "k-not-read");
    assert.strictEqual(said([asked("on"), ...askedBy("warm", "starting"), lock(118)]).text, "k-starting \u00b7 k-pause 1:58");
});

test("a room that reads nothing is a dash in the line, never a number made up", () => {
    assert.strictEqual(saidAt("working", [asked("on"), ...askedBy("cold", "below_target", 22)]).text, "k-heating - > 22 °C");
});

test("every reason the board says has words of its own, and the line stands in every stage of the fire", () => {
    for (const why of ["profile_off", "scheduled_on", "scheduled_off", "source_silent", "not_read", "starting"]) assert.ok(WORDS.ask.why[why], why);
    for (const stage of STAGES) assert.strictEqual(saidAt(stage, [asked("on"), ...askedBy("cold", "below_target", 22), room(19.6)]).word, HEATING.includes(stage) ? "cold" : "waits", stage);
});

test("where nobody asks of a switched-on burner, or the board says a word that is neither, or the menu has no words for it, the line says nothing", () => {
    assert.strictEqual(said([asked("on"), demand("asked", { unknown: {} }), demand("why", { unknown: {} })]), null, "nobody asks");
    assert.strictEqual(said([]), null, "a board of an older image has no demand's card and no switch");
    assert.strictEqual(said([asked("on"), demand("asked", word("lukewarm")), demand("why", word("below_target"))]), null, "a word nobody knows is no ask");
    assert.strictEqual(scheme.drawn([doing("stop"), asked("off")], SETTINGS, { ...WORDS, ask: undefined }, null).ask, null, "no words, no line rather than a line without its words");
    assert.strictEqual(saidAt("working", [asked("on"), ...askedBy("cold", "a_reason_not_worded", 21)]).text, "k-heating - > 21 °C", "a reason with no words of its own is the numbers where a room asks");
    assert.strictEqual(saidAt("working", [asked("on"), ...askedBy("cold", "a_reason_not_worded"), room(19.6)]), null, "and nothing where no room asks: the room's number is not made the house's");
});

test("a board of an older image, with no switch among its rows, is told by what the house asks alone", () => {
    assert.deepStrictEqual(saidAt("working", [...askedBy("cold", "below_target", 22), room(19.6)]), { word: "cold", text: "k-heating 19.6 > 22 °C" });
});

test("what is asked is read off the VALUE of the row and never off the words the board's row says", () => {
    const liar = { ...demand("asked", word("warm")), state: "cold, heat asked" };
    assert.strictEqual(said([asked("on"), liar, demand("why", word("target_reached")), demand("target", deg(22))]).word, "warm");
    const liarSwitch = { ...asked("off"), state: "on, burner switched on" };
    assert.strictEqual(said([liarSwitch]).word, "off");
});

test("a window of the schedule that is not the main profile is said small at the boiler, in the row's own short form, and the main profile says nothing", () => {
    assert.deepStrictEqual(drawn([doing("stop"), source("window", "eco until 06:00")]).notes, ["eco until 06:00"]);
    assert.deepStrictEqual(drawn([doing("stop"), source("main", "main-short")]).notes, []);
    assert.deepStrictEqual(drawn([doing("stop"), source("no_clock", "no-clock-short")]).notes, []);
    assert.deepStrictEqual(drawn([doing("stop"), { ...row("demand", "source", { unknown: {} }), short: "x" }]).notes, []);
});

test("the wait before a light is said small at the boiler while it runs, and not when the card says there is none", () => {
    assert.deepStrictEqual(drawn([doing("stop"), lock(83, "wait 1:23")]).notes, ["wait 1:23"]);
    assert.deepStrictEqual(drawn([doing("stop"), lock(null, "wait -")]).notes, []);
    assert.deepStrictEqual(drawn([doing("stop"), lock(0, "wait 0:00")]).notes, [], "a wait that has run out is no wait");
});

test("a window and a wait together stand in that order, and what the rows' words say decides nothing", () => {
    assert.deepStrictEqual(drawn([doing("stop"), lock(83, "wait 1:23"), source("window", "eco until 06:00")]).notes, ["eco until 06:00", "wait 1:23"]);
    const liar = { ...row("demand", "source", word("main")), state: "window 1 until 06:00", short: "main-short" };
    assert.deepStrictEqual(drawn([doing("stop"), liar]).notes, []);
});

test("a row without a short form says nothing even when it is what is decided on", () => {
    assert.deepStrictEqual(drawn([doing("stop"), row("demand", "source", word("window"))]).notes, []);
    assert.deepStrictEqual(drawn([doing("stop"), row("thermostat", "lock", secs(83))]).notes, []);
});


// ---- the pellets at the hopper: four numbers - burned today, yesterday, this month, in all -, read from the VALUES of the card `pellets`, never from its words ----

const pellet = (key, kg) => row("pellets", key, kg === null ? { unknown: {} } : { number: { value: kg, unit: "kg" } });

test("the pellets at the hopper are today, yesterday, the month and the total, each to a tenth of a kilo, in that order, read by the rows' keys and not by their words", () => {
    const got = scheme.pelletsOnScheme([pellet("total", 412.4), pellet("yesterday", 3.31), pellet("today", 1.234), pellet("month", 37.6)]);
    assert.deepStrictEqual(got, [{ key: "today", text: "1.2" }, { key: "yesterday", text: "3.3" }, { key: "month", text: "37.6" }, { key: "total", text: "412.4" }]);
});

test("the numbers are never in two kinds of figures: a total under a kilo is not '1' beside a 'today' of '0.7'", () => {
    const got = scheme.pelletsOnScheme([pellet("today", 0.04), pellet("yesterday", 0.3), pellet("month", 0.7), pellet("total", 0.696)]);
    assert.deepStrictEqual(got.map((p) => p.text), ["0.0", "0.3", "0.7", "0.7"]);
});

test("a board that has no card of pellets puts nothing at the hopper, and a number it has not read is a dash and never a nought", () => {
    assert.strictEqual(scheme.pelletsOnScheme([]), null);
    assert.strictEqual(scheme.pelletsOnScheme([row("automaton", "today", { number: { value: 3, unit: "kg" } })]), null, "a row of another card of the same key is not the pellets'");
    assert.deepStrictEqual(scheme.pelletsOnScheme([pellet("today", null), pellet("yesterday", 3.3), pellet("month", 38), pellet("total", 412)]).map((p) => p.text), ["-", "3.3", "38.0", "412.0"]);
    assert.deepStrictEqual(scheme.pelletsOnScheme([pellet("today", 0), pellet("yesterday", 0), pellet("month", 0), pellet("total", 0)]).map((p) => p.text), ["0.0", "0.0", "0.0", "0.0"], "nothing burned yet is a nought to a tenth, the one thing that did happen")
});

test("yesterday the board does not know is a dash in its place, and a board of an older image without the row at all has a dash there too, never a missing row", () => {
    const unknown = scheme.pelletsOnScheme([pellet("today", 1.2), pellet("yesterday", null), pellet("month", 38), pellet("total", 412)]);
    assert.deepStrictEqual(unknown.map((p) => [p.key, p.text]), [["today", "1.2"], ["yesterday", "-"], ["month", "38.0"], ["total", "412.0"]]);
    const older = scheme.pelletsOnScheme([pellet("today", 1.2), pellet("month", 38), pellet("total", 412)]);
    assert.deepStrictEqual(older.map((p) => [p.key, p.text]), [["today", "1.2"], ["yesterday", "-"], ["month", "38.0"], ["total", "412.0"]]);
});

test("the words the scheme needs of the pellets name yesterday too, so a menu without its word is named by the gap check", () => {
    assert.ok(scheme.WORDS_NEEDED.pellets.yesterday, "the scheme draws yesterday and holds the menu to a word for it");
});


// ---- the zone of the three main numbers: the draught, the boiler's water and the flame, each on a short scale with its tolerance from the board's settings, the value as a mark ----

const draughtRow = (pa) => ({ ...row("draught", "blast_pressure", pa === null ? { unknown: {} } : pa === "over" ? { at_least: { value: 300, unit: "Pa" } } : { number: { value: pa, unit: "Pa" } }), place: "draught" });
const askedPa = (pa) => row("automaton", "fan", { number: { value: pa, unit: "Pa" } });
const boilerRow = (c) => ({ ...row("temperatures", "ntc_1", deg(c)), place: "boiler" });
const pin = (mv) => row("flame", "pin", { number: { value: mv, unit: "mV" } });
const BANDS = [kept("fan_band", "10"), kept("flame_steady", "1200"), kept("flame_lit", "2000")];
const near = (got, want) => assert.ok(Math.abs(got - want) < 1e-9, `${got} is not ${want}`);

test("the draught's zone is the pascals the fan is asked for, give or take the band the board holds it within; the mark is what the sensor reads, inside or out", () => {
    const inside = scheme.zonesOf([doing("working"), askedPa(50), draughtRow(55)], BANDS).draught;
    assert.strictEqual(inside.inside, true);
    near(inside.from, 1 / 3); near(inside.to, 2 / 3); near(inside.at, 0.5 + 5 / 60);
    const out = scheme.zonesOf([doing("working"), askedPa(50), draughtRow(75)], BANDS).draught;
    assert.strictEqual(out.inside, false);
    assert.ok(out.at > out.to, "the mark stands over the zone's end");
});

test("the mark never leaves the scale: a value far over it stands at its end, a value 'at least' the top of the sensor is over", () => {
    const far = scheme.zonesOf([doing("working"), askedPa(50), draughtRow(900)], BANDS).draught;
    assert.strictEqual(far.at, 1);
    assert.strictEqual(scheme.zonesOf([doing("working"), askedPa(50), draughtRow("over")], BANDS).draught.at, 1);
    assert.strictEqual(scheme.zonesOf([doing("working"), askedPa(50), draughtRow(-400)], BANDS).draught.at, 0);
});

test("the draught has no zone where the fan is asked for no number, the band is not said, or the sensor reads nothing", () => {
    assert.strictEqual(scheme.zonesOf([doing("working"), row("automaton", "fan", word("full")), draughtRow(55)], BANDS).draught, null, "a full purge holds no pascals");
    assert.strictEqual(scheme.zonesOf([doing("working"), askedPa(50), draughtRow(55)], []).draught, null);
    assert.strictEqual(scheme.zonesOf([doing("working"), askedPa(50), draughtRow(null)], BANDS).draught, null);
    assert.strictEqual(scheme.zonesOf([doing("working"), draughtRow(55)], BANDS).draught, null, "the automaton has not said what it asks");
});

test("the boiler's water has a zone only while the thermostat holds the boiler: the setpoint less the distance up to the setpoint - the one held now", () => {
    const inside = scheme.zonesOf([doing("working"), boilerRow(42)], SETTINGS).water;
    assert.strictEqual(inside.inside, true);
    const below = scheme.zonesOf([doing("working"), boilerRow(30)], SETTINGS).water;
    assert.strictEqual(below.inside, false);
    assert.ok(below.at < below.from);
    const profile = scheme.zonesOf([doing("working"), boilerRow(42), sp(30)], SETTINGS).water;
    assert.strictEqual(profile.inside, false, "the setpoint held now is 30, and 42 is over it");
    for (const mode of ["return", "room", "power"]) assert.strictEqual(scheme.zonesOf([doing("working"), boilerRow(42)], holdBy(mode)).water, null, mode);
    assert.strictEqual(scheme.zonesOf([doing("working"), boilerRow(42)], SETTINGS.filter((s) => s.name !== "auto_drop_c")).water, null, "no distance, no zone");
});

test("the flame's zone is a steady flame - at or under the board's steady threshold, lower millivolts are more light; a pin that reads nothing has no mark", () => {
    const steady = scheme.zonesOf([doing("working"), pin(900)], BANDS).flame;
    assert.strictEqual(steady.inside, true);
    const weak = scheme.zonesOf([doing("working"), pin(1800)], BANDS).flame;
    assert.strictEqual(weak.inside, false);
    assert.ok(weak.at > weak.to);
    assert.strictEqual(scheme.zonesOf([doing("working"), { ...pin(0), value: { unknown: {} } }], BANDS).flame, null);
    assert.strictEqual(scheme.zonesOf([doing("working"), pin(900)], [kept("flame_lit", "2000")]).flame, null, "no steady threshold, no zone");
});

test("a mark is flagged only where it is out of its zone while the fire works - a dark pot in a stop and a flame still weak while it rises are no deviation", () => {
    for (const stage of ["working", "smouldering"]) {
        assert.strictEqual(scheme.zonesOf([doing(stage), pin(1800)], BANDS).flame.warn, true, stage);
        assert.strictEqual(scheme.zonesOf([doing(stage), askedPa(50), draughtRow(75)], BANDS).draught.warn, true, stage);
        assert.strictEqual(scheme.zonesOf([doing(stage), pin(900)], BANDS).flame.warn, false, `${stage} inside`);
    }
    for (const stage of ["stop", "purge", "kindling", "rising", "burning_off", "fault"]) {
        assert.strictEqual(scheme.zonesOf([doing(stage), pin(3150)], BANDS).flame.warn, false, stage);
        assert.strictEqual(scheme.zonesOf([doing(stage), askedPa(50), draughtRow(75)], BANDS).draught.warn, false, stage);
    }
    assert.strictEqual(scheme.zonesOf([doing("working"), boilerRow(30)], SETTINGS).water.warn, false, "the water passes through its band at every cycle: out of it is no deviation");
});


// ---- what asks for a person: the active faults and warnings in one list, each with its action in the menu's words ----

const failing = (stage, why, state) => ({ ...row("automaton", "doing", word(stage)), state, parts: { why }, level: stage === "fault" ? "bad" : "warn" });
const bad = (group, key, name, state) => ({ ...row(group, key, secs(12)), name, state, level: "bad" });

test("a fault asks for its action and for the Stop that acknowledges it; the action is the menu's word for the cause's CODE, and what the words say decides nothing", () => {
    const got = scheme.attention([failing("fault", "no_fuel", "fault · liar")], WORDS);
    assert.deepStrictEqual(got, [{ key: "automaton/doing", words: "fault · liar", action: "at-fuel at-ack", level: "bad" }]);
});

// ---- the fault for want of fronts says where it stands: the seconds to the let-go, or that the day's are spent ----

const relief = ({ left = null, today = null, limit = 3, silent = null } = {}) => [
    row("automaton", "silent_fronts", silent === null ? { none: true } : secs(silent)),
    row("automaton", "relief_in", left === null ? { none: true } : secs(left)),
    row("automaton", "relief_today", today === null ? { none: true } : { number: { value: today, unit: "times" } }),
    row("automaton", "relief_limit", { number: { value: limit, unit: "times" } }),
];
const actionOfDoing = (things) => scheme.attention(things, WORDS).find((i) => i.key === "automaton/doing").action;

test("a fault for want of fronts whose pin has not come back says how long it will wait after it does, and does not ask for the Stop", () => {
    assert.strictEqual(actionOfDoing([failing("fault", "no_fronts", "fault · pin"), ...relief({ left: 60, today: 1, silent: 300 })]), "rl-waits 60");
});

test("once the pin is back the same fault says the seconds that are left of the run, counting down with the board's number", () => {
    assert.strictEqual(actionOfDoing([failing("fault", "no_fronts", "fault · pin"), ...relief({ left: 37, today: 1 })]), "rl-counting 37");
    assert.strictEqual(actionOfDoing([failing("fault", "no_fronts", "fault · pin"), ...relief({ left: 1, today: 2 })]), "rl-counting 1");
});

test("a fault whose let-goes of the day are spent says how many and of how many, and that a person is wanted", () => {
    assert.strictEqual(actionOfDoing([failing("fault", "no_fronts", "fault · pin"), ...relief({ left: null, today: 3, limit: 3 })]), "rl-spent 3 3");
});

test("a fault for want of fronts the board will not let go by itself - the time of day not believed, so no seconds and no count - says the general word of its cause and the Stop", () => {
    assert.strictEqual(actionOfDoing([failing("fault", "no_fronts", "fault · pin"), ...relief({ left: null, today: null })]), "at-fronts at-ack");
    assert.strictEqual(actionOfDoing([failing("fault", "no_fronts", "fault · pin")]), "at-fronts at-ack", "an older board that has none of these rows");
});

test("the seconds of the let-go are said only for the fault for want of fronts, and a burning off for it is not yet a fault that waits", () => {
    assert.strictEqual(actionOfDoing([failing("fault", "no_fuel", "fault · liar"), ...relief({ left: 37, today: 1 })]), "at-fuel at-ack");
    assert.strictEqual(actionOfDoing([failing("burning_off_because", "no_fronts", "burning off · pin"), ...relief({ left: 60, today: 1 })]), "at-fronts");
});

test("a burning off that has a reason is a warning with its action and without the Stop - it is not a fault yet", () => {
    assert.deepStrictEqual(scheme.attention([failing("burning_off_because", "never_lit", "burning-off · no-light")], WORDS), [{ key: "automaton/doing", words: "burning-off · no-light", action: "at-lit", level: "warn" }]);
});

test("a cause the menu has no action for still asks, with the menu's general action, and a stage that is no failure asks for nothing", () => {
    assert.deepStrictEqual(scheme.attention([failing("fault", "something_new", "fault · new")], WORDS).map((i) => i.action), ["at-other at-ack"]);
    for (const stage of STAGES.filter((s) => s !== "fault")) assert.deepStrictEqual(scheme.attention([{ ...doing(stage), level: "dim" }], WORDS), [], stage);
    assert.deepStrictEqual(scheme.attention([{ ...doing("burning_off"), level: "dim", parts: { why: "no_fuel" } }], WORDS), [], "a plain burning off, with no reason, is not asked about");
});

test("a row at the level bad that the menu gives an action asks for it, named by the row's name and its state; a row not at that level, or one with no action in the menu, does not", () => {
    const silent = bad("thermostat", "silent_return", "return sensor silent for", "12 s");
    assert.deepStrictEqual(scheme.attention([silent], WORDS), [{ key: "thermostat/silent_return", words: "return sensor silent for 12 s", action: "at-return", level: "bad" }]);
    assert.deepStrictEqual(scheme.attention([{ ...silent, level: "dim" }], WORDS), [], "silent for none is no row at all, and a quiet one is not asked about");
    assert.deepStrictEqual(scheme.attention([bad("automaton", "refused", "refused", "x: not a number")], WORDS), [], "a refused setting is the form's to say");
    assert.deepStrictEqual(scheme.attention([bad("thermostat", "silent_return", "same key", "other card")].map((r) => ({ ...r, group: "automaton" })), WORDS), [], "a key is its card's and its own");
});

test("a pressure read over the top of the sensor's scale is no failure - the fan blows full on a purge and the sensor is meant to read over - and a draught the sensor does not give at all is", () => {
    const over = { ...row("draught", "blast_pressure", { at_least: { value: 300, unit: "Pa" } }), name: "blast pressure", state: "over", level: "bad" };
    assert.deepStrictEqual(scheme.attention([over], WORDS), []);
    const missing = { ...row("draught", "blast_pressure", { unknown: {} }), name: "blast pressure", state: "missing", level: "bad" };
    assert.deepStrictEqual(scheme.attention([missing], WORDS).map((i) => i.action), ["at-draught"]);
});

test("a healthy board asks for nothing and several causes ask in the order the board lists them, the fault first", () => {
    assert.deepStrictEqual(scheme.attention([doing("working"), { ...row("flame", "flame", word("steady")), level: "ok" }], WORDS), []);
    const got = scheme.attention([bad("flue", "flue_temp", "flue gas", "silent"), failing("fault", "no_fuel", "fault · liar"), bad("automaton", "alarm", "alarm", "on")], WORDS);
    assert.deepStrictEqual(got.map((i) => i.key), ["automaton/doing", "flue/flue_temp", "automaton/alarm"]);
});

test("every row of the menu that can stand at the level bad has an action for the list of what asks for a person, bar the two the list leaves to others - and every cause the board names has one", () => {
    const menu = mergedMenu();
    // the fault and the burning off are the list's own, by the cause; a refused setting is said by the form it was sent from; the zero crossings and the gate of the fan are judged only after the first seconds and stand at bad on the first second of every run
    // (tools/test_attention_on_frames.py), and what they settle on is said by the fan's `holding`
    const apart = ["automaton/doing", "automaton/refused", "fan/zero_crossings", "fan/gate"];
    const canGoBad = Object.entries(menu.modules).flatMap(([card, mod]) => Object.entries(mod.rows || {}).filter(([, r]) => (r.level || []).some(([, level]) => level === "bad")).map(([name]) => `${card}/${name}`)).filter((k) => !apart.includes(k));
    const listed = Object.keys(scheme.WORDS_NEEDED.attention.rows);
    assert.ok(canGoBad.length >= 10, "the menu names rows that can go bad");
    assert.deepStrictEqual(canGoBad.filter((k) => !listed.includes(k)), [], "rows that can stand at bad and ask for nothing");
    assert.deepStrictEqual(listed.filter((k) => !canGoBad.includes(k)), [], "rows the list names that cannot stand at bad");
    const causes = menu.modules.automaton.rows["doing.why"].says.map(([code]) => code);
    assert.deepStrictEqual(Object.keys(scheme.WORDS_NEEDED.attention.why).sort(), causes.sort(), "the causes the menu words and the causes the list holds an action for");
});

test("every cause the board's dictionary names for a failure is worded by the menu, and the seconds of a silent pin are a row the menu words and asks a person about", () => {
    const menu = mergedMenu();
    const served = JSON.parse(fs.readFileSync(path.join(ROOT, "devices/burner/api/tests/golden/dictionary.json"), "utf8"));
    const why = served.things.find((t) => t.card === "automaton" && t.part_of === "doing" && t.name === "why");
    const worded = menu.modules.automaton.rows["doing.why"].says.map(([code]) => code);
    assert.deepStrictEqual([...why.words].sort(), [...worded].sort(), "the causes the board says and the causes the menu has a word for");
    // the pin that gives no fronts: its seconds are a row of the card at the level bad, and the list of what asks for a person holds it
    assert.ok(served.things.some((t) => t.card === "automaton" && t.name === "silent_fronts"), "the dictionary has the row");
    assert.deepStrictEqual(menu.modules.automaton.rows.silent_fronts.level, [["*", "bad"]]);
    assert.ok(Object.keys(scheme.WORDS_NEEDED.attention.rows).includes("automaton/silent_fronts"));
});


// ---- the mark is in the warning colour only after it has stood out of its zone for a while ----
// On the boiler's records (02.10, 04.10, 27.09) the draught came back within the fan's band 0 to 5 seconds after the setpoint was changed at work (nine changes, 5 s the longest): a mark coloured at the first second out would flash at every step.

const outOf = (warn) => ({ draught: { from: 0.4, to: 0.6, at: 0.9, inside: false, warn }, water: null, flame: null });

test("a mark out of its zone is in the warning colour only after it has stood out for ten seconds, counted from the first second out", () => {
    const since = {};
    assert.strictEqual(scheme.settle(since, outOf(true), 100000).draught.warn, false, "the first second out");
    assert.strictEqual(scheme.settle(since, outOf(true), 105000).draught.warn, false, "five seconds out - the longest the boiler took to come back");
    assert.strictEqual(scheme.settle(since, outOf(true), 109999).draught.warn, false);
    assert.strictEqual(scheme.settle(since, outOf(true), 110000).draught.warn, true, "ten seconds out");
    assert.strictEqual(scheme.settle(since, outOf(true), 200000).draught.warn, true, "and it stays");
});

test("a mark back in its zone, or out of it where nothing asks, starts the count again", () => {
    const since = {};
    scheme.settle(since, outOf(true), 100000);
    scheme.settle(since, { ...outOf(false), draught: { ...outOf(false).draught, inside: true } }, 106000);
    assert.strictEqual(scheme.settle(since, outOf(true), 108000).draught.warn, false, "it was in between: the count is from 108");
    assert.strictEqual(scheme.settle(since, outOf(true), 117999).draught.warn, false);
    assert.strictEqual(scheme.settle(since, outOf(true), 118000).draught.warn, true);
    scheme.settle(since, outOf(false), 119000);
    assert.strictEqual(scheme.settle(since, outOf(true), 120000).draught.warn, false, "out where nothing asks cleared it");
});

test("a zone that is gone loses its count, and what settling does not touch is as it was", () => {
    const since = {};
    scheme.settle(since, outOf(true), 100000);
    const gone = scheme.settle(since, { draught: null, water: null, flame: null }, 105000);
    assert.deepStrictEqual(gone, { draught: null, water: null, flame: null });
    assert.strictEqual(scheme.settle(since, outOf(true), 112000).draught.warn, false, "the sensor was silent in between: counted again from 112");
    const zones = outOf(true);
    const got = scheme.settle({}, zones, 0);
    assert.deepStrictEqual({ ...got.draught, warn: true }, zones.draught, "the shares and the verdict are the zone's own");
    assert.deepStrictEqual(zones, outOf(true), "the zones handed in are not changed");
});

// ---- a Light's cycle: what the burner burns for while it goes ----
// The thermostat's rows `manual` (the kind: room or water) and `manual_c` (where it ends) say it; the line over the drawing says it before anything the house asks, because a Light is the person's word and it holds over the house's.

const manualRow = (kind) => row("thermostat", "manual", kind === null ? { unknown: {} } : word(kind));
const manualAt = (c) => row("thermostat", "manual_c", c === null ? { unknown: {} } : deg(c));

test("a cycle of the room says what the burner burns for - the room's target and where the room stands - before anything the house asks", () => {
    const line = drawn([doing("working"), asked("on"), ...askedBy("warm", "target_reached", 22), room(21.1), manualRow("room"), manualAt(22)]).ask;
    assert.strictEqual(line.word, "manual");
    assert.strictEqual(line.text, "k-man-room 21.1 > 22 °C", "the room's reading, then the target");
    assert.ok(!line.text.includes("k-no-heat"), "the house's warm is not the reason of a cycle");
});

test("a cycle of the water says the setpoint it burns to, and it is said with the switch off too - a Light is not the switch", () => {
    for (const sw of ["on", "off", "unasked"]) {
        const line = drawn([doing("working"), asked(sw), manualRow("water"), manualAt(60), at("boiler", 52)]).ask;
        assert.strictEqual(line.word, "manual", sw);
        assert.strictEqual(line.text, "k-man-water 52 > 60 °C", "the boiler's reading, then the setpoint");
    }
});

// WHICH SENSOR "NOW" IS READ OFF in a cycle of the water: the one the thermostat holds, by the VALUE of the setting `auto_hold` - the boiler's water or the return's -, and none where it holds none
const withHold = (mode) => SETTINGS.filter((s) => s.name !== "auto_hold").concat(mode === null ? [] : [kept("auto_hold", mode)]);
const silent = (place) => ({ ...row("temperatures", `ntc_${place}`, { unknown: {} }), place });
const waterLine = (mode, readings) => drawn([doing("working"), asked("on"), manualRow("water"), manualAt(60), ...readings], withHold(mode)).ask.text;

test("a cycle of the water says the reading of the sensor the thermostat holds: the boiler where it holds the boiler, the return where it holds the return", () => {
    assert.strictEqual(waterLine("boiler", [at("boiler", 52), at("water_back", 31)]), "k-man-water 52 > 60 °C");
    assert.strictEqual(waterLine("return", [at("boiler", 52), at("water_back", 31)]), "k-man-water 31 > 60 °C", "the return's reading and not the boiler's");
});

test("a held sensor that is silent is a dash, never the number of the other one; the mode of power and a board that has not said the mode hold no sensor", () => {
    assert.strictEqual(waterLine("return", [at("boiler", 52), silent("water_back")]), "k-man-water - > 60 °C");
    assert.strictEqual(waterLine("boiler", [silent("boiler"), at("water_back", 31)]), "k-man-water - > 60 °C");
    assert.strictEqual(waterLine("power", [at("boiler", 52), at("water_back", 31)]), "k-man-water - > 60 °C");
    assert.strictEqual(waterLine(null, [at("boiler", 52), at("water_back", 31)]), "k-man-water - > 60 °C", "no setting, no sensor");
    assert.strictEqual(waterLine("return", [at("boiler", 52)]), "k-man-water - > 60 °C", "a return nobody has put on a socket");
});

test("a cycle of the room says the room's reading whatever the thermostat holds, and a room that is silent is a dash", () => {
    const roomLine = (mode, readings) => drawn([doing("working"), asked("on"), manualRow("room"), manualAt(22), ...readings], withHold(mode)).ask.text;
    for (const mode of ["boiler", "return", "power"]) assert.strictEqual(roomLine(mode, [room(21.1), at("boiler", 52), at("water_back", 31)]), "k-man-room 21.1 > 22 °C", mode);
    assert.strictEqual(roomLine("boiler", [silent("room"), at("boiler", 52)]), "k-man-room - > 22 °C");
});

test("with no cycle the line is what it always was, and a cycle whose end the board has not said says nothing of it", () => {
    assert.deepStrictEqual(drawn([doing("stop"), asked("off"), manualRow(null), manualAt(null)]).ask, { word: "off", text: "k-stopped" });
    assert.deepStrictEqual(drawn([doing("working"), asked("off"), manualRow("room"), manualAt(null)]).ask, { word: "off", text: "k-stopped" }, "a cycle without its number is not a line the page can say");
});

// ---- the target of the room is set on the scheme (9.32) ----
// The panel under the line of the reason changes the target of the profile that stands and the gap below it. WHICH setting it changes is decided by the VALUES - who asks (the setting `auto_asked_by`),
// the profile that stands (the demand's row `profile`), whether a Light's cycle holds the fire (the thermostat's `manual`) -, never by a word; the limits are the settings' own.

const setting = (name, value, from, to) => ({ ...kept(name, value), months: [from, to], unit: "°C" });
const TARGETS = (by = "room", c = "21", eco = "17", gap = "2") => [kept("auto_asked_by", by), setting("auto_room_c", c, 6, 35), setting("auto_room_eco_c", eco, 6, 35), setting("auto_room_gap_c", gap, 1, 5)];
const profile = (w) => demand("profile", w === null ? { unknown: {} } : word(w));
const panel = (things, settings = TARGETS()) => scheme.panelOf(things, settings);

test("the panel changes the comfort target while the comfort profile stands, and the eco target while eco stands", () => {
    const comfort = panel([profile("comfort"), ...askedBy("cold", "below_target", 21), room(19.6)]);
    assert.deepStrictEqual({ setting: comfort.setting, profile: comfort.profile, value: comfort.value, from: comfort.from, to: comfort.to }, { setting: "auto_room_c", profile: "comfort", value: 21, from: 6, to: 35 });
    const eco = panel([profile("eco"), ...askedBy("warm", "target_reached", 17), room(18)], TARGETS("room", "21", "16"));
    assert.deepStrictEqual({ setting: eco.setting, profile: eco.profile, value: eco.value }, { setting: "auto_room_eco_c", profile: "eco", value: 16 });
});

test("the panel carries the gap, the room and the line the fire is lit below", () => {
    const p = panel([profile("comfort"), ...askedBy("cold", "below_target", 21), room(19.6)]);
    assert.deepStrictEqual(p.gap, { setting: "auto_room_gap_c", value: 2, from: 1, to: 5, step: 1, tenths: false });
    assert.strictEqual(p.room, 19.6);
    assert.strictEqual(p.fire, 19, "the target less the gap");
    assert.strictEqual(p.unit, "°C");
});

test("a silent room leaves the panel open with no reading", () => {
    const p = panel([profile("comfort"), ...askedBy("cold", "source_silent", 21)]);
    assert.strictEqual(p.setting, "auto_room_c");
    assert.strictEqual(p.room, null);
});

test("no panel where there is no target of the room to change", () => {
    const base = [...askedBy("cold", "below_target", 21), room(19.6)];
    assert.strictEqual(panel([profile("off"), ...base]), null, "the profile off has no target");
    assert.strictEqual(panel([profile(null), ...base]), null, "no profile said");
    assert.strictEqual(panel([profile("comfort"), ...base], TARGETS("schedule")), null, "the schedule asks");
    assert.strictEqual(panel([profile("comfort"), ...base], TARGETS("none")), null, "nobody asks");
    assert.strictEqual(panel([profile("comfort"), ...base], [kept("auto_asked_by", "room")]), null, "the settings the panel changes are not on the board");
    assert.strictEqual(panel([profile("comfort"), ...base], []), null, "the settings are not filled yet");
});

// 9.35: the target of the room is the person's to change in any minute. A Light's cycle burns to the target that stands now (the thermostat takes it again every second), so the cycle shuts nothing.
const EVERY_STAGE = ["stop", "purge", "kindling", "rising", "working", "smouldering", "burning_off", "burning_off_because", "fault"];

test("the panel stands while a Light's cycle goes, of the room or of the water: the cycle burns to the target that stands now", () => {
    const base = [profile("comfort"), ...askedBy("cold", "below_target", 21), room(19.6)];
    for (const kind of ["room", "water"]) {
        const p = panel([...base, manualRow(kind), manualAt(kind === "room" ? 22 : 60)]);
        assert.ok(p, `no panel in a cycle of the ${kind}`);
        assert.deepStrictEqual({ setting: p.setting, value: p.value, profile: p.profile }, { setting: "auto_room_c", value: 21, profile: "comfort" }, kind);
    }
});

test("the panel stands in every stage of the automaton, with or without a cycle", () => {
    for (const stage of EVERY_STAGE) for (const cycle of [[], [manualRow("room"), manualAt(22)]]) {
        const p = panel([doing(stage), profile("comfort"), ...askedBy("cold", "below_target", 21), room(19.6), ...cycle]);
        assert.ok(p, `no panel in ${stage}${cycle.length ? " in a cycle" : ""}`);
        assert.strictEqual(p.setting, "auto_room_c", stage);
    }
});

test("the panel stands where the board has not yet read the demand: no row of it, a row it could not read, or no target in it", () => {
    for (const rows of [[], [demand("asked", { unknown: {} })], askedBy("cold", "below_target")]) {
        const p = panel([profile("comfort"), room(19.6), ...rows]);
        assert.ok(p, JSON.stringify(rows));
        assert.strictEqual(p.setting, "auto_room_c");
    }
});

test("a cycle gives no panel where there is no target of the room to change", () => {
    const cycle = [manualRow("room"), manualAt(22)];
    assert.strictEqual(panel([profile("off"), ...askedBy("cold", "below_target", 21), ...cycle]), null, "the profile off has no target, cycle or not");
    assert.strictEqual(panel([profile("comfort"), ...askedBy("cold", "below_target", 21), ...cycle], TARGETS("schedule")), null, "the schedule asks, cycle or not");
});

test("the panel is read from the rows of the demand's card, not from a row of another card with the same key", () => {
    assert.strictEqual(panel([row("automaton", "profile", word("comfort")), ...askedBy("cold", "below_target", 21)]), null);
});

test("a step moves the number by one and stops at the limit of the setting", () => {
    assert.strictEqual(scheme.stepped({ value: 21, from: 6, to: 35 }, 1), 22);
    assert.strictEqual(scheme.stepped({ value: 21, from: 6, to: 35 }, -1), 20);
    assert.strictEqual(scheme.stepped({ value: 35, from: 6, to: 35 }, 1), null, "no more above the top");
    assert.strictEqual(scheme.stepped({ value: 6, from: 6, to: 35 }, -1), null, "no more under the bottom");
    assert.strictEqual(scheme.stepped({ value: 5, from: 1, to: 5 }, 1), null, "the gap's top");
    assert.strictEqual(scheme.stepped({ value: 1, from: 1, to: 5 }, -1), null, "the gap's bottom");
});

// the page sends once the hand has stayed off the button for the wait: a clock made up here, and a send that records what it was handed
function clock() {
    const timers = new Map();
    let next = 1;
    return {
        set: (fn, ms) => { timers.set(next, { fn, ms }); return next++; },
        clear: (id) => timers.delete(id),
        fire: () => { const all = [...timers.values()]; timers.clear(); all.forEach((t) => t.fn()); },
        // the timer that was set first runs out, and no other
        fireOne: () => { const first = [...timers.entries()][0]; if (first) { timers.delete(first[0]); first[1].fn(); } },
        count: () => timers.size,
    };
}

test("five quick presses send one setting, with the last number", async () => {
    const sent = [];
    const timer = clock();
    const sender = scheme.targetSender({ send: async (name, value) => { sent.push([name, value]); return true; }, wait: 2000, timer });
    for (const v of [22, 23, 24, 25, 26]) sender.press("auto_room_c", v);
    assert.strictEqual(timer.count(), 1, "one timer, restarted by each press");
    assert.deepStrictEqual(sent, [], "nothing goes before the wait is over");
    assert.strictEqual(sender.held("auto_room_c"), 26, "the panel shows the hand's number meanwhile");
    timer.fire();
    await sender.flushed();
    assert.deepStrictEqual(sent, [["auto_room_c", 26]]);
});

test("the target and the gap pressed together go as two settings, once each", async () => {
    const sent = [];
    const timer = clock();
    const sender = scheme.targetSender({ send: async (name, value) => { sent.push([name, value]); return true; }, wait: 2000, timer });
    sender.press("auto_room_c", 22);
    sender.press("auto_room_gap_c", 3);
    sender.press("auto_room_c", 23);
    timer.fire();
    await sender.flushed();
    assert.deepStrictEqual(sent.slice().sort(), [["auto_room_c", 23], ["auto_room_gap_c", 3]]);
});

test("once it is sent the hand is let go, and the panel shows what the board holds", async () => {
    const timer = clock();
    const sender = scheme.targetSender({ send: async () => true, wait: 2000, timer });
    sender.press("auto_room_c", 22);
    timer.fire();
    await sender.flushed();
    assert.strictEqual(sender.held("auto_room_c"), undefined);
});

test("a refusal, or a board that did not answer, lets the hand go too: the number goes back to the board's", async () => {
    for (const answer of [false, null]) {
        const timer = clock();
        const sender = scheme.targetSender({ send: async () => answer, wait: 2000, timer });
        sender.press("auto_room_c", 22);
        timer.fire();
        await sender.flushed();
        assert.strictEqual(sender.held("auto_room_c"), undefined, String(answer));
    }
});

test("a press after a send is a new send", async () => {
    const sent = [];
    const timer = clock();
    const sender = scheme.targetSender({ send: async (name, value) => { sent.push(value); return true; }, wait: 2000, timer });
    sender.press("auto_room_c", 22);
    timer.fire();
    await sender.flushed();
    sender.press("auto_room_c", 23);
    timer.fire();
    await sender.flushed();
    assert.deepStrictEqual(sent, [22, 23]);
});

test("the hand's number stands on the panel in place of the board's while the hand is on it, and the fire's line moves with it", () => {
    const p = panel([profile("comfort"), ...askedBy("cold", "below_target", 21), room(19.6)]);
    const held = scheme.handled(p, (name) => ({ auto_room_c: 24, auto_room_gap_c: 3 })[name]);
    assert.deepStrictEqual({ value: held.value, gap: held.gap.value, fire: held.fire }, { value: 24, gap: 3, fire: 21 });
    const same = scheme.handled(p, () => undefined);
    assert.deepStrictEqual({ value: same.value, gap: same.gap.value, fire: same.fire }, { value: 21, gap: 2, fire: 19 });
});

test("a button is off at the limit of its setting", () => {
    const top = scheme.handled(panel([profile("comfort"), ...askedBy("cold", "below_target", 35)], TARGETS("room", "35", "17", "5")), () => undefined);
    assert.deepStrictEqual({ up: top.canUp, down: top.canDown, gapUp: top.gap.canUp, gapDown: top.gap.canDown }, { up: false, down: true, gapUp: false, gapDown: true });
    const bottom = scheme.handled(panel([profile("comfort"), ...askedBy("cold", "below_target", 6)], TARGETS("room", "6", "17", "1")), () => undefined);
    assert.deepStrictEqual({ up: bottom.canUp, down: bottom.canDown, gapUp: bottom.gap.canUp, gapDown: bottom.gap.canDown }, { up: true, down: false, gapUp: true, gapDown: false });
});

test("the strip puts the band, the target and the room on a scale of whole degrees around them", () => {
    const strip = scheme.stripOf({ value: 22, fire: 20, room: 21.7 });
    assert.deepStrictEqual(strip.ticks.map((t) => t.label), ["18", "20", "22", "24", "26"]);
    assert.deepStrictEqual({ from: strip.from, to: strip.to, target: strip.target }, { from: 25, to: 50, target: 50 });
    assert.ok(Math.abs(strip.room - 46.25) < 1e-9, "the room is 3.7 of 8 degrees up the scale");
});

test("the strip of a silent room has no mark of the room, and a room off the scale stands at its edge", () => {
    assert.strictEqual(scheme.stripOf({ value: 22, fire: 20, room: null }).room, null);
    assert.strictEqual(scheme.stripOf({ value: 22, fire: 20, room: 5 }).room > 0, true, "the scale reaches down to the room");
    const far = scheme.stripOf({ value: 35, fire: 30, room: 6 });
    assert.ok(far.room >= 0 && far.room <= 100 && far.target >= 0 && far.target <= 100);
});

// ---- what happens to a number while the hand is off the panel (9.32) ----
// THE QUEUE IS SIMPLE ON PURPOSE: a press puts the number under the setting's name, the wait runs out, the number goes; a page put away sends what waits at once; a press made while the write before it is on its way goes after it.
// The board is NOT asked before a write and nothing is compared with what the page knows of it: the last write wins, and what the board holds by then is written over - on purpose ("no check before the write").

// a send that answers at once and writes down what it was given
const recording = (sent) => async (name, value, keep) => { sent.push([name, value, !!keep]); return true; };

// a send that does not answer until it is told to, and when it does the board holds the number written, as the page's `put` leaves it
function board(start) {
    const now = { ...start };
    const sent = [];
    const waiting = [];
    return {
        now, sent, waiting,
        send: (name, value, keep) => new Promise((resolve) => { sent.push([name, value, Boolean(keep)]); waiting.push(() => { now[name] = value; resolve(true); }); }),
        land: () => (waiting.shift() || (() => {}))(),
    };
}
// the microtasks a write goes through run out: the writes still waiting to land stay waiting
const settle = async () => { for (let i = 0; i < 30; i++) await Promise.resolve(); };
const numbers = (b) => b.sent.map((one) => one[1]);

test("a number another hand set on the board while the hand was off is written over: the last write wins, on purpose", async () => {
    // the queue is given nothing of the board but the way to send: what the board holds - 30 from another hand, say - is not asked and not compared with
    const sent = [];
    const timer = clock();
    const sender = scheme.targetSender({ send: recording(sent), wait: 2000, timer });
    sender.press("auto_room_c", 22);
    assert.strictEqual(sender.held("auto_room_c"), 22, "the hand's number stands on the panel");
    timer.fire();
    await sender.flushed();
    assert.deepStrictEqual(sent, [["auto_room_c", 22, false]], "the 22 goes whatever the board holds by now");
});

test("while a number waits the page says so, whichever setting it waits for - the panel of another profile has the same wait", () => {
    const sender = scheme.targetSender({ send: recording([]), wait: 2000, timer: clock() });
    assert.strictEqual(sender.pending(), false);
    sender.press("auto_room_c", 22);
    assert.strictEqual(sender.pending(), true);
});

test("a profile that changed while the hand was off leaves the pressed setting to go, and the panel of the new profile shows its own number", async () => {
    const sent = [];
    const timer = clock();
    const sender = scheme.targetSender({ send: recording(sent), wait: 2000, timer });
    sender.press("auto_room_c", 24);
    const eco = panel([profile("eco"), ...askedBy("warm", "target_reached", 17), room(18)], TARGETS("room", "21", "17"));
    const shown = scheme.handled(eco, (name) => sender.held(name));
    assert.deepStrictEqual({ setting: shown.setting, value: shown.value }, { setting: "auto_room_eco_c", value: 17 }, "the eco panel does not show the comfort hand's 24");
    assert.strictEqual(sender.pending(), true, "the wait is still said");
    timer.fire();
    await sender.flushed();
    assert.deepStrictEqual(sent, [["auto_room_c", 24, false]], "the comfort setting goes, not the eco one");
});

test("the window of the schedule that changes the profile moves the panel to the target of the new profile, and back", () => {
    const rows = (w) => [profile(w), demand("source", word("window")), ...askedBy("cold", "below_target", 21), room(19.6)];
    assert.strictEqual(panel(rows("comfort")).setting, "auto_room_c");
    assert.strictEqual(panel(rows("eco")).setting, "auto_room_eco_c");
    assert.strictEqual(panel(rows("comfort")).setting, "auto_room_c", "and back when the window ends");
});

test("a page that is put away sends what waits, once, with the request that outlives the page; a second leaving sends nothing more", async () => {
    const sent = [];
    const timer = clock();
    const sender = scheme.targetSender({ send: recording(sent), wait: 2000, timer });
    sender.press("auto_room_c", 22);
    sender.flush(true);
    sender.flush(true);
    await sender.flushed();
    assert.deepStrictEqual(sent, [["auto_room_c", 22, true]]);
    assert.strictEqual(timer.count(), 0, "the timer is not left to send it again");
});

test("nothing waits - a page put away sends nothing", async () => {
    const sent = [];
    const sender = scheme.targetSender({ send: recording(sent), wait: 2000, timer: clock() });
    sender.flush(true);
    await sender.flushed();
    assert.deepStrictEqual(sent, []);
});

// ---- every press reaches the board: the sequences the reviews found, and every order of a model ----

test("R2: a page put away, back, pressed again and put away while the first write is on its way: the second press goes after the first has landed", async () => {
    const timer = clock();
    const b = board({ auto_room_c: 21 });
    const sender = scheme.targetSender({ send: b.send, wait: 2000, timer });
    sender.press("auto_room_c", 22);
    sender.flush(true);
    sender.press("auto_room_c", 23);
    sender.flush(true);
    await settle();
    assert.deepStrictEqual(numbers(b), [22], "one write on its way for the setting");
    b.land();
    await settle();
    assert.deepStrictEqual(b.sent, [["auto_room_c", 22, true], ["auto_room_c", 23, true]]);
    b.land();
    await settle();
    assert.strictEqual(sender.pending(), false, "nothing is left saying a number waits");
    assert.strictEqual(b.now.auto_room_c, 23);
});

test("R3: the answer of the first write and a frame of the stream come late, after the page is back, and a press made meanwhile is not lost", async () => {
    const timer = clock();
    const b = board({ auto_room_c: 21 });
    const sender = scheme.targetSender({ send: b.send, wait: 2000, timer });
    sender.press("auto_room_c", 22);
    timer.fire();
    sender.flush();
    await settle();
    sender.press("auto_room_c", 23);
    b.land();
    // a frame of the stream, taken before the write, reaches the page after it and puts the old number back into what the page knows: the queue is not asked about it
    b.now.auto_room_c = 21;
    await settle();
    assert.deepStrictEqual(numbers(b), [22, 23], "the late answer and the late frame take no press away: the 23 goes after the 22 has landed");
    b.land();
    await settle();
    assert.strictEqual(sender.pending(), false);
});

test("a press made while the write before it is on its way goes when that has landed, though its own wait ran out meanwhile or has not", async () => {
    for (const expired of [true, false]) {
        const timer = clock();
        const b = board({ auto_room_c: 21 });
        const sender = scheme.targetSender({ send: b.send, wait: 2000, timer });
        sender.press("auto_room_c", 22);
        timer.fire();
        await settle();
        sender.press("auto_room_c", 23);
        if (expired) timer.fire();
        await settle();
        assert.deepStrictEqual(numbers(b), [22], "not before the first has landed");
        b.land();
        await settle();
        assert.deepStrictEqual(numbers(b), [22, 23], String(expired));
        b.land();
        await settle();
        assert.strictEqual(sender.pending(), false);
    }
});

test("presses made while a write is on its way go as one write, the last number", async () => {
    const timer = clock();
    const b = board({ auto_room_c: 21 });
    const sender = scheme.targetSender({ send: b.send, wait: 2000, timer });
    sender.press("auto_room_c", 22);
    timer.fire();
    await settle();
    for (const v of [23, 24, 25]) sender.press("auto_room_c", v);
    b.land();
    await settle();
    assert.deepStrictEqual(numbers(b), [22, 25]);
});

test("a refusal or silence of the first write does not take away the press made after it", async () => {
    const sent = [];
    let first = true;
    const waiting = [];
    const timer = clock();
    const sender = scheme.targetSender({ send: (name, value, keep) => new Promise((resolve) => { sent.push(value); waiting.push(() => resolve(first ? ((first = false), false) : true)); }), wait: 2000, timer });
    sender.press("auto_room_c", 22);
    timer.fire();
    await settle();
    sender.press("auto_room_c", 23);
    waiting.shift()();
    await settle();
    assert.deepStrictEqual(sent, [22, 23]);
});

// every order of presses, timers running out, pages put away and writes landing: the last number pressed is the last sent, and nothing is left waiting
function sequences(seed, count, length) {
    let state = seed;
    const next = (n) => { state = (state * 1103515245 + 12345) & 0x7fffffff; return state % n; };
    return Array.from({ length: count }, () => Array.from({ length }, () => next(6)));
}

test("whatever the order of presses, timers, pages put away and writes landing (one setting, writes that answer late), the last number pressed is the last sent and nothing is left waiting", async () => {
    const failed = [];
    for (const [at, ops] of sequences(9, 400, 14).entries()) {
        const timer = clock();
        const b = board({ auto_room_c: 21 });
        const sender = scheme.targetSender({ send: b.send, wait: 2000, timer });
        let last = null;
        let value = 21;
        for (const op of ops) {
            if (op === 0 || op === 1) { value += 1; last = value; sender.press("auto_room_c", value); }
            else if (op === 2) timer.fire();
            else if (op === 3) sender.flush(true);
            else if (op === 4) b.land();
            else { b.land(); b.land(); }
            await Promise.resolve();
            await Promise.resolve();
        }
        for (let i = 0; i < 12; i++) { b.land(); timer.fire(); await settle(); }
        const lastSent = b.sent.length ? b.sent[b.sent.length - 1][1] : null;
        if (last !== null && (lastSent !== last || sender.pending() || b.now.auto_room_c !== last)) failed.push([at, ops.join(""), last, lastSent, sender.pending()]);
    }
    assert.deepStrictEqual(failed.slice(0, 5), [], `${failed.length} of 400 orders lose a press`);
});

test("the same orders with two settings: each setting's last number is the last sent", async () => {
    const failed = [];
    for (const [at, ops] of sequences(31, 300, 16).entries()) {
        const timer = clock();
        const b = board({ auto_room_c: 21, auto_room_gap_c: 1 });
        const sender = scheme.targetSender({ send: b.send, wait: 2000, timer });
        const last = {};
        let value = 0;
        for (const op of ops) {
            const name = op === 1 ? "auto_room_gap_c" : "auto_room_c";
            if (op === 0 || op === 1) { value += 1; last[name] = value; sender.press(name, value); }
            else if (op === 2) timer.fire();
            else if (op === 3) sender.flush(true);
            else b.land();
            await Promise.resolve();
            await Promise.resolve();
        }
        for (let i = 0; i < 12; i++) { b.land(); timer.fire(); await settle(); }
        for (const name of Object.keys(last)) {
            const lastSent = [...b.sent].reverse().find((one) => one[0] === name);
            if (!lastSent || lastSent[1] !== last[name] || b.now[name] !== last[name]) failed.push([at, ops.join(""), name, last[name], lastSent && lastSent[1]]);
        }
        if (sender.pending()) failed.push([at, ops.join(""), "pending"]);
    }
    assert.deepStrictEqual(failed.slice(0, 5), [], `${failed.length} of 300 orders lose a press`);
});

// ---- a refusal belongs to its setting, and so does the wait (9.32, review R4) ----
// The target and the hysteresis are two settings under one panel: what the board said of one is not undone by what it said of the other.

// the panel's wiring as parts.js makes it: a write that lands says its answer to the refusals of ITS setting
function wiredPanel() {
    const timer = clock();
    const refused = scheme.refusals();
    const answers = new Map();
    const sender = scheme.targetSender({
        wait: 2000,
        timer,
        send: (name) => new Promise((resolve) => { answers.set(name, (ok, words) => { refused.wrote(name, ok, words); resolve(ok); }); }),
    });
    const NAMES = ["auto_room_c", "auto_room_gap_c"];
    return { timer, refused, sender, answers, said: () => refused.of(NAMES) };
}

test("the board refuses one setting and takes the other, in either order of the answers: the refusal stands", async () => {
    for (const order of [["auto_room_c", "auto_room_gap_c"], ["auto_room_gap_c", "auto_room_c"]]) {
        const w = wiredPanel();
        w.sender.press("auto_room_c", 36);
        w.sender.press("auto_room_gap_c", 2);
        w.timer.fire();
        await settle();
        for (const name of order) w.answers.get(name)(name === "auto_room_gap_c", name === "auto_room_gap_c" ? "" : "target refused");
        await settle();
        assert.strictEqual(w.said(), "target refused", order.join(" then "));
    }
});

test("the board refuses the hysteresis and takes the target, in either order: the refusal of the hysteresis stands", async () => {
    for (const order of [["auto_room_c", "auto_room_gap_c"], ["auto_room_gap_c", "auto_room_c"]]) {
        const w = wiredPanel();
        w.sender.press("auto_room_c", 22);
        w.sender.press("auto_room_gap_c", 9);
        w.timer.fire();
        await settle();
        for (const name of order) w.answers.get(name)(name === "auto_room_c", name === "auto_room_c" ? "" : "hysteresis refused");
        await settle();
        assert.strictEqual(w.said(), "hysteresis refused", order.join(" then "));
    }
});

test("the board takes both: nothing is said; and both refused: both are said", async () => {
    const both = wiredPanel();
    both.sender.press("auto_room_c", 22);
    both.sender.press("auto_room_gap_c", 2);
    both.timer.fire();
    await settle();
    both.answers.get("auto_room_gap_c")(true, "");
    both.answers.get("auto_room_c")(true, "");
    await settle();
    assert.strictEqual(both.said(), "");
    const neither = wiredPanel();
    neither.sender.press("auto_room_c", 36);
    neither.sender.press("auto_room_gap_c", 9);
    neither.timer.fire();
    await settle();
    neither.answers.get("auto_room_c")(false, "target refused");
    neither.answers.get("auto_room_gap_c")(false, "hysteresis refused");
    await settle();
    assert.strictEqual(neither.said(), "target refused hysteresis refused");
});

test("a refusal goes when its own setting is taken, or when the hand presses that setting again - and not when the other one is pressed", () => {
    const refused = scheme.refusals();
    const NAMES = ["auto_room_c", "auto_room_gap_c"];
    refused.wrote("auto_room_c", false, "target refused");
    refused.wrote("auto_room_gap_c", false, "hysteresis refused");
    refused.pressed("auto_room_gap_c");
    assert.strictEqual(refused.of(NAMES), "target refused", "a press of the hysteresis does not undo the target's refusal");
    refused.wrote("auto_room_gap_c", true, "");
    assert.strictEqual(refused.of(NAMES), "target refused", "nor does the hysteresis being taken");
    refused.pressed("auto_room_c");
    assert.strictEqual(refused.of(NAMES), "", "the press of the target itself does");
    refused.wrote("auto_room_c", false, "target refused");
    refused.wrote("auto_room_c", true, "");
    assert.strictEqual(refused.of(NAMES), "", "and so does its own write being taken");
    refused.wrote("auto_room_c", false, "");
    assert.strictEqual(refused.of(NAMES), "", "a refusal with no words says nothing");
});

test("a refusal of a setting the panel does not show is kept for it and not said under another", () => {
    const refused = scheme.refusals();
    refused.wrote("auto_room_eco_c", false, "eco refused");
    assert.strictEqual(refused.of(["auto_room_c", "auto_room_gap_c"]), "");
    assert.strictEqual(refused.of(["auto_room_eco_c", "auto_room_gap_c"]), "eco refused");
});

test("each number waits its own 2 s: a press of the hysteresis does not postpone the target's write", async () => {
    const sent = [];
    const timer = clock();
    const sender = scheme.targetSender({ send: recording(sent), wait: 2000, timer });
    sender.press("auto_room_c", 22);
    sender.press("auto_room_gap_c", 2);
    assert.strictEqual(timer.count(), 2, "a timer for each setting");
    timer.fireOne();
    await sender.flushed();
    assert.deepStrictEqual(sent, [["auto_room_c", 22, false]], "the target's 2 s are out: it goes, and the hysteresis waits its own");
    timer.fireOne();
    await sender.flushed();
    assert.deepStrictEqual(sent.map((one) => one[0]), ["auto_room_c", "auto_room_gap_c"]);
});

test("a press of one setting restarts only its own wait", async () => {
    const sent = [];
    const timer = clock();
    const sender = scheme.targetSender({ send: recording(sent), wait: 2000, timer });
    sender.press("auto_room_c", 22);
    sender.press("auto_room_c", 23);
    assert.strictEqual(timer.count(), 1, "the same setting pressed again has one wait");
    timer.fire();
    await sender.flushed();
    assert.deepStrictEqual(sent, [["auto_room_c", 23, false]]);
});

// ---- the mains at the power's dot ----

const volts = (value) => ({ number: { value, unit: "V" } });
const watts = (value) => ({ number: { value, unit: "W" } });
const reason = (why) => ({ unknown: why });

test("the mains stand at the power's dot as the voltage and the power of the socket the burner is plugged into, in one line", () => {
    assert.strictEqual(scheme.mainsOnScheme([row("mains", "voltage", volts(227)), row("mains", "power", watts(4))]), "227 V \u00b7 4 W");
    // whole numbers, as the socket says them and the dictionary's decimals: 0
    assert.strictEqual(scheme.mainsOnScheme([row("mains", "voltage", volts(226.6)), row("mains", "power", watts(62.4))]), "227 V \u00b7 62 W");
    // nought is a reading: the socket is there and the burner draws nothing
    assert.strictEqual(scheme.mainsOnScheme([row("mains", "voltage", volts(0)), row("mains", "power", watts(0))]), "0 V \u00b7 0 W");
});

test("nothing stands at the power's dot where either number is missing, whatever the reason: a dash would read as a socket that has failed", () => {
    for (const why of ["off", "not_read", "silent", "unreadable"]) {
        assert.strictEqual(scheme.mainsOnScheme([row("mains", "voltage", reason(why)), row("mains", "power", reason(why))]), null, why);
    }
    assert.strictEqual(scheme.mainsOnScheme([row("mains", "voltage", volts(227))]), null, "no power row");
    assert.strictEqual(scheme.mainsOnScheme([row("mains", "power", watts(4))]), null, "no voltage row");
    assert.strictEqual(scheme.mainsOnScheme([row("mains", "voltage", volts(227)), row("mains", "power", reason("silent"))]), null, "power silent");
    assert.strictEqual(scheme.mainsOnScheme([]), null);
    assert.strictEqual(scheme.mainsOnScheme(undefined), null);
});

test("a number of another unit in the mains' rows is not read as volts or watts", () => {
    assert.strictEqual(scheme.mainsOnScheme([row("mains", "voltage", { number: { value: 227, unit: "A" } }), row("mains", "power", watts(4))]), null);
});

test("the mains are looked for by the rows of the card mains and not by what the rows are called", () => {
    const other = (key, value) => row("temperatures", key, value);
    assert.strictEqual(scheme.mainsOnScheme([other("voltage", volts(227)), other("power", watts(4))]), null);
});

test("the mains line is worded in the units of the page's language: volts and watts in Russian are \u0412 and \u0412\u0442, and in English V and W", () => {
    const things = [row("mains", "voltage", volts(227)), row("mains", "power", watts(4))];
    try {
        scope.dictionary.useUnits(scope.dictionary.menuUnits({ menu: mergedMenu(), lang: "ru" }));
        assert.strictEqual(scheme.mainsOnScheme(things), "227 \u0412 \u00b7 4 \u0412\u0442");
        scope.dictionary.useUnits(scope.dictionary.menuUnits({ menu: mergedMenu(), lang: "en" }));
        assert.strictEqual(scheme.mainsOnScheme(things), "227 V \u00b7 4 W");
    } finally {
        scope.dictionary.useUnits({});
    }
});

// ---- the target goes by tenths of a degree (9.33) ----
// A target is 22.1 and not 22: the setting says it is tenths with its span (`range`, `tenths`, the unit), the panel steps by 0.1 and says one digit after the point, a button held goes on by itself, and a series of presses is one write.
// The sum of the steps is exact: 22.1 + 0.1 is 22.2, ten steps of 0.1 from 22 are 23.

const tenthsSetting = (name, value, from, to) => ({ ...kept(name, value), range: [from, to], tenths: true, unit: "°C" });
const BY_TENTHS = (by = "room", c = "21", eco = "17", gap = "2") => [kept("auto_asked_by", by), tenthsSetting("auto_room_c", c, 6, 35), tenthsSetting("auto_room_eco_c", eco, 6, 35), setting("auto_room_gap_c", gap, 1, 5)];
const comfort_ = [profile("comfort"), ...askedBy("cold", "below_target", 21), room(19.6)];

test("the panel of a target by tenths carries the number with its tenth, a step of 0.1 and the span of the setting; the gap stays whole with a step of 1", () => {
    const p = panel(comfort_, BY_TENTHS("room", "22.1"));
    assert.deepStrictEqual({ setting: p.setting, value: p.value, from: p.from, to: p.to, step: p.step, tenths: p.tenths, unit: p.unit }, { setting: "auto_room_c", value: 22.1, from: 6, to: 35, step: 0.1, tenths: true, unit: "°C" });
    assert.deepStrictEqual({ value: p.gap.value, step: p.gap.step, tenths: p.gap.tenths }, { value: 2, step: 1, tenths: false });
    assert.strictEqual(p.fire, 20.1, "the target less the gap, to the tenth and without the dust of the sum");
});

test("a target kept whole by an image before is the same number with a nought tenth", () => {
    const p = panel(comfort_, BY_TENTHS("room", "22"));
    assert.strictEqual(p.value, 22);
    assert.strictEqual(scheme.valueText(p), "22.0");
});

test("a step is a tenth, exact: 22.1 up is 22.2, 22.1 down is 22.0, and ten steps from 22 are 23", () => {
    const one = (value) => ({ value, from: 6, to: 35, step: 0.1 });
    assert.strictEqual(scheme.stepped(one(22.1), 1), 22.2);
    assert.strictEqual(scheme.stepped(one(22.1), -1), 22);
    let value = 22;
    for (let i = 0; i < 10; i++) value = scheme.stepped(one(value), 1);
    assert.strictEqual(value, 23);
    value = 23;
    for (let i = 0; i < 10; i++) value = scheme.stepped(one(value), -1);
    assert.strictEqual(value, 22);
    // the sums that go wrong in a float: 0.1 + 0.2, 5.1 + 0.1
    assert.strictEqual(scheme.stepped(one(6.2), 1), 6.3);
    assert.strictEqual(scheme.stepped(one(5.1 + 1), 1), 6.2);
});

test("at the span's ends a button sends nothing: 35 does not go up, 6 does not go down, 34.9 goes to 35 and 6.1 to 6", () => {
    const one = (value) => ({ value, from: 6, to: 35, step: 0.1 });
    assert.strictEqual(scheme.stepped(one(35), 1), null);
    assert.strictEqual(scheme.stepped(one(6), -1), null);
    assert.strictEqual(scheme.stepped(one(34.9), 1), 35);
    assert.strictEqual(scheme.stepped(one(6.1), -1), 6);
});

test("a whole number's step is still 1 where the setting has no tenths", () => {
    assert.strictEqual(scheme.stepped({ value: 21, from: 6, to: 35 }, 1), 22);
    assert.strictEqual(scheme.stepped({ value: 35, from: 6, to: 35 }, 1), null);
});

test("the hand's number shows and the buttons know the limits by tenths", () => {
    const p = panel(comfort_, BY_TENTHS("room", "22.1"));
    const held = scheme.handled(p, (name) => (name === "auto_room_c" ? 35 : undefined));
    assert.deepStrictEqual({ value: held.value, canUp: held.canUp, canDown: held.canDown }, { value: 35, canUp: false, canDown: true });
    const lower = scheme.handled(p, (name) => (name === "auto_room_c" ? 6.1 : undefined));
    assert.strictEqual(lower.canDown, true);
    const floor = scheme.handled(p, (name) => (name === "auto_room_c" ? 6 : undefined));
    assert.strictEqual(floor.canDown, false);
});

test("the number is said with one digit after the point where the setting is tenths and whole where it is whole", () => {
    assert.strictEqual(scheme.valueText({ value: 22, tenths: true }), "22.0");
    assert.strictEqual(scheme.valueText({ value: 22.1, tenths: true }), "22.1");
    assert.strictEqual(scheme.valueText({ value: 35, tenths: true }), "35.0");
    assert.strictEqual(scheme.valueText({ value: 21, tenths: false }), "21");
    assert.strictEqual(scheme.valueText({ value: 2 }), "2");
});

test("a button held goes on by itself: nothing for 400 ms, then a step at 400 ms and one every 100 ms", () => {
    const at = (ms) => scheme.holdSteps(ms);
    assert.deepStrictEqual([at(0), at(399), at(400), at(499), at(500), at(1000)], [0, 0, 1, 1, 2, 7]);
    assert.strictEqual(scheme.holdSteps(-5), 0);
});

test("thirty presses are one write of the last number, with one timer restarted by each press", async () => {
    const sent = [];
    const timer = clock();
    const sender = scheme.targetSender({ send: async (name, value) => { sent.push([name, value]); return true; }, wait: 2000, timer });
    let value = 22;
    for (let i = 0; i < 30; i++) {
        value = scheme.stepped({ value, from: 6, to: 35, step: 0.1 }, 1);
        sender.press("auto_room_c", value);
    }
    assert.strictEqual(timer.count(), 1, "one timer, restarted by each press");
    assert.deepStrictEqual(sent, [], "nothing goes while the hand is on");
    timer.fire();
    await sender.flushed();
    assert.deepStrictEqual(sent, [["auto_room_c", 25]], "thirty tenths up from 22 is 25, without the dust of a float");
});

test("the page makes a setting of tenths of a degree a range with a tenth and its unit, and a time of the auger still seconds", () => {
    const d = scope.dictionary;
    const dict = { things: [], settings: [] };
    const target = d.settingOf(dict, {}, [], { name: "auto_room_c", kind: "tenths", from: 60, to: 350, unit: "°C", default: "21" });
    assert.deepStrictEqual({ tenths: target.tenths, range: target.range, seconds: target.seconds, unit: target.unit, value: target.value }, { tenths: true, range: [6, 35], seconds: undefined, unit: "°C", value: "21" });
    const dose = d.settingOf(dict, {}, [], { name: "auto_dose", kind: "tenths", from: 1, to: 1200, unit: "s", default: "2" });
    assert.deepStrictEqual({ tenths: dose.tenths, range: dose.range, seconds: dose.seconds, unit: dose.unit }, { tenths: true, range: undefined, seconds: [0.1, 120], unit: "s" });
    const before = d.settingOf(dict, {}, [], { name: "auto_dose", kind: "tenths", from: 1, to: 1200, default: "2" });
    assert.deepStrictEqual({ seconds: before.seconds, unit: before.unit }, { seconds: [0.1, 120], unit: "s" }, "a board that does not say the unit of a tenths setting says seconds, as the boards before did");
});


// ---- the soot: how clean the boiler is, from the number the board sends and the two edges a person set; the percent is the page's and never the board's ----

const sootRows = ({ c = 50, kg = 12.5, burns = 6, day = null } = {}) => [
    row("soot", "soot_c", c === null ? { unknown: { reason: "no_burns" } } : deg(c)),
    row("soot", "soot_burns", { number: { value: burns, unit: "burns" } }),
    row("soot", "kg_since_clean", { number: { value: kg, unit: "kg" } }),
    row("soot", "cleaned_day", day === null ? { unknown: { reason: "none_yet" } } : { number: { value: day, unit: "day" } }),
];
const EDGES = [{ name: "soot_clean_c", value: "40" }, { name: "soot_dirty_c", value: "60" }];

test("the percent of a clean boiler is the way from the dirty edge to the clean one, held between nought and a hundred", () => {
    const percent = (c, edges = EDGES) => scheme.sootOf(sootRows({ c }), edges).percent;
    assert.strictEqual(percent(40), 100, "at the clean edge");
    assert.strictEqual(percent(60), 0, "at the dirty edge");
    assert.strictEqual(percent(50), 50);
    assert.strictEqual(percent(44), 80);
    assert.strictEqual(percent(35), 100, "cleaner than the clean edge is not more than all");
    assert.strictEqual(percent(75), 0, "dirtier than the dirty edge is not less than nothing");
    assert.strictEqual(percent(50, [{ name: "soot_clean_c", value: "38" }, { name: "soot_dirty_c", value: "58" }]), 40, "the edges are the ones a person set: eight of twenty degrees to the dirty one");
});

test("the soot says the number, the burns, the kilograms and the day as they are; the percent only where both edges are known", () => {
    const got = scheme.sootOf(sootRows({ c: 50, kg: 12.5, burns: 6, day: 20400 }), EDGES);
    assert.deepStrictEqual({ degrees: got.degrees, burns: got.burns, kg: got.kg, day: got.day }, { degrees: 50, burns: 6, kg: 12.5, day: 20400 });
    for (const edges of [[], [EDGES[0]], [EDGES[1]], [{ name: "soot_clean_c", value: "" }, EDGES[1]], undefined]) {
        const no = scheme.sootOf(sootRows(), edges);
        assert.strictEqual(no.percent, null, JSON.stringify(edges));
        assert.strictEqual(no.degrees, 50, "the number does not wait for the edges");
    }
});

test("before the first burn there is no number and no percent, and a board with no soot card has no soot", () => {
    const got = scheme.sootOf(sootRows({ c: null, day: null }), EDGES);
    assert.deepStrictEqual({ degrees: got.degrees, percent: got.percent, day: got.day, kg: got.kg }, { degrees: null, percent: null, day: null, kg: 12.5 });
    assert.strictEqual(scheme.sootOf([row("pellets", "total", { number: { value: 1, unit: "kg" } })], EDGES), null);
    assert.strictEqual(scheme.sootOf(undefined, EDGES), null);
});

test("the day of the cleaning is said as day and month from the days since 1970", () => {
    assert.strictEqual(scheme.dayText(Date.UTC(2026, 10, 8) / 864e5), "08.11");
    assert.strictEqual(scheme.dayText(Date.UTC(2027, 0, 3) / 864e5), "03.01");
});

test("the boiler asks for cleaning below a quarter, in the menu's words, with the percent it stands at; at a quarter and above it asks for nothing", () => {
    const asks = (c, edges = EDGES) => scheme.attention(sootRows({ c }), WORDS, edges).filter((i) => i.key === "soot/soot_c");
    assert.deepStrictEqual(asks(56), [{ key: "soot/soot_c", words: "at-soot 20", action: "at-soot-do", level: "warn" }], "20 % is below a quarter: it says 20");
    assert.deepStrictEqual(asks(55), [], "exactly a quarter is not below it");
    assert.deepStrictEqual(asks(45), []);
    assert.strictEqual(asks(80).length, 1, "past the dirty edge it asks, once");
    assert.deepStrictEqual(asks(null), [], "no number is no ask");
    assert.deepStrictEqual(scheme.attention(sootRows({ c: 56 }), WORDS, undefined).filter((i) => i.key === "soot/soot_c"), [], "no edges: nothing to judge by");
    assert.deepStrictEqual(asks(56, [EDGES[0]]), []);
});

test("the soot asks after the faults of the plant and never instead of them", () => {
    const got = scheme.attention([failing("fault", "no_fuel", "fault"), ...sootRows({ c: 60 })], WORDS, EDGES);
    assert.deepStrictEqual(got.map((i) => i.key), ["automaton/doing", "soot/soot_c"]);
});
