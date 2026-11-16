// The menu of the new burner is a tree: the main screen's daily settings, a page for every module - the hardware ones (fan, flame, draught, temperatures, flue, relays) and the logic (the
// automaton) - and the system tab, the board's own: its time, zone, restart and console. A module's page holds everything about it: its readings, its curves, its controls and its settings. Where a setting stands is the MENU's word
// (`menu.json`, `tree`) and never the board's: the dictionary says what a setting IS, so it carries no field of a section. What is held here is the place of every setting, every
// card and every control - numbers and codes, never the words of a title.
//
//     node --test tools/test_page_tree.js
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

// what the board of the new burner serves, made by its own API crate, and the page's menu
const DICT = JSON.parse(execSync("cargo +stable run -q -p burner_api --example served_json", { cwd: ROOT, encoding: "utf8", maxBuffer: 1 << 26 }));
const MENU = menu();
const TEXTS = { menu: MENU };

const pagesOf = (tree) => tree.modules.pages;
const sectionsOf = (tree) => [...tree.main.sections, ...pagesOf(tree).flatMap((p) => p.sections), ...tree.system.sections];
// the sections of the Service, on the system tab: the manual control and the numbers of the fire
const serviceOf = (tree) => tree.system.sections.filter((s) => s.group === "service");
const placedSections = (tree) => [...tree.main.sections, ...pagesOf(tree).flatMap((p) => p.sections), ...tree.system.sections];
const names = DICT.settings.map((s) => s.name);
// what a section holds of settings: its names without its actions, which are rows of a card (`card/row`)
const settingsIn = (section) => d.sectionNames(section).filter((n) => !n.includes("/"));
const HARDWARE = ["fan", "flame", "draught", "temperatures", "flue"];
const LOGIC = ["thermostat", "demand", "automaton"];

test("the menu has a tree: the main screen's sections, a page for each module in two groups, the hardware and the logic, and the system tab's", () => {
    assert.ok(MENU.tree, "menu.json has no `tree`");
    assert.ok(MENU.tree.main && Array.isArray(MENU.tree.main.sections), "the main screen's sections");
    assert.ok(!("user" in MENU.tree) && !("service" in MENU.tree), "the user's and the service's tabs are gone");
    const groups = MENU.tree.modules.groups;
    assert.deepStrictEqual(groups.map((g) => g.id), ["hardware", "logic"]);
    assert.deepStrictEqual(groups[0].modules, HARDWARE);
    assert.deepStrictEqual(groups[1].modules, LOGIC);
    assert.deepStrictEqual(pagesOf(MENU.tree).map((p) => p.id).sort(), [...HARDWARE, ...LOGIC].sort(), "a page for every module of the groups, and no page outside them");
    assert.ok(!pagesOf(MENU.tree).some((p) => p.id === "board"), "the board is no module of the burner: it has a tab of its own");
    assert.ok(MENU.tree.system.sections.length >= 2, "the system tab's sections");
});

test("every setting of the burner stands in exactly one place: the scheme's target, a module's page or the system tab", () => {
    const seen = new Map();
    for (const section of placedSections(MENU.tree)) for (const name of settingsIn(section)) seen.set(name, (seen.get(name) || 0) + 1);
    const lost = names.filter((n) => !seen.has(n));
    const twice = [...seen].filter(([, n]) => n > 1).map(([name]) => name);
    assert.deepStrictEqual(lost, [], "settings that stand on no module's page");
    assert.deepStrictEqual(twice, [], "settings that stand on two pages");
    assert.strictEqual(seen.size, names.length, "settings the board does not have");
});

test("the target of the room stands once, on the thermostat's page, drawn by the part of the panel and not as a list with a Save; the main screen has no settings of its own", () => {
    // the scheme draws the panel itself, from the board's settings, and asks the main screen for one section only: the soot's part (how clean the boiler is), which holds a reading and a button and no setting
    assert.deepStrictEqual(MENU.tree.main.sections.map((s) => [s.id, s.part, d.sectionNames(s)]), [["soot", "soot", []]]);
    const page = pagesOf(MENU.tree).find((p) => p.id === "demand");
    const target = page.sections.find((s) => s.id === "target");
    assert.ok(target, "the thermostat's page has no section of the target");
    assert.ok(target.part, "the section is drawn by a part (the panel), not as a list of settings with a Save");
    assert.deepStrictEqual(d.sectionNames(target), ["auto_room_c", "auto_room_eco_c", "auto_room_gap_c"], "the targets of the profiles and the hysteresis under them");
    assert.ok(target.title && target.title.en && target.title.ru, "the section has a title in both languages");
    const elsewhere = new Set(sectionsOf(MENU.tree).filter((s) => s !== target).flatMap((s) => d.sectionNames(s)));
    for (const name of d.sectionNames(target)) assert.ok(!elsewhere.has(name), `${name} stands in the target's section and in another`);
    const held = new Set(pagesOf(MENU.tree).find((p) => p.id === "thermostat").sections.flatMap((s) => d.sectionNames(s)));
    for (const name of ["auto_hold", "auto_hold_c", "auto_drop_c", "auto_step", "auto_lock_min", "auto_lock_c"]) assert.ok(held.has(name), `${name} stands no more on the boiler's page`);
    for (const name of d.sectionNames(target)) assert.ok(!held.has(name), `${name} stands on the boiler's page`);
});

test("the tree names no setting the board does not have", () => {
    const have = new Set(names);
    const unknown = sectionsOf(MENU.tree).flatMap((s) => settingsIn(s)).filter((n) => !have.has(n));
    assert.deepStrictEqual(unknown, []);
});

// what a module is: its cards' rows are its readings and its controls, a row's curves its curves; the page of a module holds them all, once
// the automaton's card is shown by its own part, which a section asks for with its `part` field (`burning`): its strip, its rows and the table of what it works by
const CYCLE = Object.entries(MENU.modules).find(([, m]) => m.cycle)[0];
const cardsOf = (page) => page.sections.flatMap((s) => [...(s.cards || []), ...(s.part === "burning" ? [CYCLE] : [])]);

test("every card of the board that is a module's stands on exactly one page, and no page names a card the board does not have", () => {
    const board = new Set(DICT.things.map((t) => t.card));
    const seen = new Map();
    for (const page of pagesOf(MENU.tree)) for (const card of new Set(cardsOf(page))) seen.set(card, (seen.get(card) || []).concat(page.id));
    // a card the Service shows (the relays') is shown there and on no module's page
    for (const section of serviceOf(MENU.tree)) for (const card of section.cards || []) seen.set(card, (seen.get(card) || []).concat(`service:${section.id}`));
    for (const [card, pages] of seen) {
        assert.ok(board.has(card), `${card} is no card of the board`);
        assert.strictEqual(pages.length, 1, `${card} stands on ${pages.join(", ")}`);
    }
    // the soot's card is read through the main screen's part (`"part": "soot"`) and its edges stand in the Service: no module page shows it
    const apart = new Set(["system", "settings", "other", "automaton_fixed", "soot"]);
    const missing = [...board].filter((c) => !apart.has(c) && !seen.has(c));
    assert.deepStrictEqual(missing, [], "cards of the board no module's page shows");
});

test("a hardware module's page holds its readings, its curves and its settings - its hands are in the Service", () => {
    const page = (id) => pagesOf(MENU.tree).find((p) => p.id === id);
    const rowsOf = (card) => DICT.things.filter((t) => t.card === card && !t.part_of);
    const settingsOf = (id) => page(id).sections.flatMap((s) => d.sectionNames(s));
    for (const id of HARDWARE) {
        const cards = cardsOf(page(id));
        assert.ok(cards.length >= 1, `${id} shows no card: no readings`);
        assert.ok(cards.every((c) => rowsOf(c).length >= 1), `${id}: a card with no rows`);
    }
    // the fan: the regulator is tuned here, so everything about it is on this one page
    const fan = rowsOf("fan");
    assert.ok(fan.some((t) => (t.curves || []).length >= 3), "the fan's three curves");
    assert.ok(fan.some((t) => (t.can || []).includes("hold")), "the board has the fan's sliders; the Service shows them");
    assert.ok(settingsOf("fan").includes("up_kp") && settingsOf("fan").includes("fan_band_share"), "the regulator's coefficients");
    // the others, each by what it is
    assert.ok(rowsOf("flame").some((t) => (t.curves || []).length), "the flame's curve");
    assert.ok(rowsOf("draught").some((t) => (t.curves || []).length), "the draught's curve");
    assert.ok(rowsOf("temperatures").some((t) => (t.curves || []).length), "the sensors' curves");
    assert.deepStrictEqual(settingsOf("flame").sort(), ["flame_lit", "flame_steady"]);
    assert.deepStrictEqual(settingsOf("draught").sort(), ["div_bottom", "div_top"]);
    assert.ok(["ntc_address", "poll_every", "role_ntc_1", "role_ntc_7"].every((n) => settingsOf("temperatures").includes(n)), "the sensors' roles, address and polling");
    assert.deepStrictEqual(settingsOf("flue"), [], "the flue has nothing to set");
});

test("the boiler's page (the thermostat's card, the water layer) holds what the water says, the switch, the level and the distance, the setpoint of the water, the two buttons - and the mode, the setpoint, the hysteresis, the levels and the wait before a light", () => {
    const page = pagesOf(MENU.tree).find((p) => p.id === "thermostat");
    assert.deepStrictEqual(page.sections.flatMap((s) => s.cards || []), ["thermostat"], "its own card, and no other");
    const settings = page.sections.flatMap((s) => d.sectionNames(s)).sort();
    assert.deepStrictEqual(settings, ["auto_drop_c", "auto_hold", "auto_hold_c", "auto_step", "auto_step_mid", "auto_step_small", "auto_mid_c", "auto_small_c", "auto_lock_min", "auto_lock_c"].sort());
    const automaton = pagesOf(MENU.tree).find((p) => p.id === "automaton");
    const kept = automaton.sections.flatMap((s) => d.sectionNames(s));
    assert.ok(!kept.some((n) => [...settings, "auto_asked_by", "auto_profile", "window_1"].includes(n)), "the automaton's page still holds a setting of the thermostat");
    assert.ok(!MENU.modules.automaton.rows.start && !MENU.modules.automaton.rows.stop && !MENU.modules.automaton.rows.wanted, "the automaton's menu still words the buttons or the switch");
    for (const name of ["switch", "heat", "rung", "left", "setpoint", "lock", "start", "stop", "light"]) assert.ok(MENU.modules.thermostat.rows[name], `the boiler's menu has no words for ${name}`);
    for (const name of ["silent_room", "profile", "source"]) assert.ok(!MENU.modules.thermostat.rows[name], `the boiler's menu still words ${name}, which is the thermostat's`);
});

test("the thermostat's page (the demand's card) holds who asks, why, the profile and the target - and who asks, the targets of the room, the profiles with the six windows", () => {
    const page = pagesOf(MENU.tree).find((p) => p.id === "demand");
    assert.deepStrictEqual(page.sections.flatMap((s) => s.cards || []), ["demand"], "its own card, and no other");
    const settings = page.sections.flatMap((s) => d.sectionNames(s)).sort();
    assert.deepStrictEqual(settings, ["auto_asked_by", "auto_room_c", "auto_room_eco_c", "auto_room_gap_c", "auto_profile", "window_1", "window_2", "window_3", "window_4", "window_5", "window_6"].sort());
    for (const name of ["asked", "why", "silent_room", "profile", "source", "target"]) assert.ok(MENU.modules.demand.rows[name], `the thermostat's menu has no words for ${name}`);
    assert.ok(!MENU.modules.thermostat.settings.auto_eco_c && !MENU.modules.demand.settings.auto_eco_c, "the setpoint of the water by profile is no more");
});

test("the schedule's settings stand in a section of their own on the thermostat's page, apart from the boiler's setpoint", () => {
    const schedule = ["auto_profile", "window_1", "window_2", "window_3", "window_4", "window_5", "window_6"];
    const page = pagesOf(MENU.tree).find((p) => p.id === "demand");
    const own = page.sections.find((s) => d.sectionNames(s).includes("window_1"));
    assert.ok(own, "no section holds the windows");
    assert.deepStrictEqual(d.sectionNames(own), schedule, "the profile, then the windows in the order the first that is open stands");
    assert.ok(own.title && own.title.en && own.title.ru, "the section has a title in both languages");
});

test("who asks stands on the thermostat's page alone: the targets of the room and their hysteresis moved to the scheme", () => {
    const own = pagesOf(MENU.tree).find((p) => p.id === "demand").sections.find((s) => d.sectionNames(s).includes("auto_asked_by"));
    assert.ok(own, "no section holds who asks");
    assert.deepStrictEqual(d.sectionNames(own), ["auto_asked_by"]);
    assert.ok(!(own.pairs || []).length, "no pair of the room's target and its hysteresis is left on the page");
});

test("the automaton's page holds what it does, the pellets and what it works by - and none of the numbers of the fire, which are the Service's", () => {
    const page = pagesOf(MENU.tree).find((p) => p.id === "automaton");
    assert.deepStrictEqual(page.sections.filter((s) => s.part === "burning").map((s) => s.given), ["readings", "works"], "its readings, and the table of what it works by");
    assert.deepStrictEqual(page.sections.filter((s) => s.burning !== undefined), [], "a section asks for a part by `part` and nothing else");
    const kept = page.sections.flatMap((s) => d.sectionNames(s));
    assert.deepStrictEqual(kept, ["pellet_rate"], "the pellet counter's rate is the only setting left on the page");
});

const FIRE = ["steps", "lighting", "working", "smouldering", "pump", "protection"];

test("the Service holds the numbers of the fire, section for section as the automaton's page held them", () => {
    const sections = Object.fromEntries(serviceOf(MENU.tree).map((s) => [s.id, s]));
    for (const id of FIRE) assert.ok(sections[id], `the Service has no section ${id}`);
    assert.ok(sections.steps.table, "the power steps");
    assert.ok(d.sectionNames(sections.lighting).includes("auto_igniter"), "lighting");
    assert.ok(d.sectionNames(sections.working).includes("auto_burn_off_s"), "burning off");
    assert.ok(d.sectionNames(sections.pump).includes("auto_pump_on_c"), "the pump");
    const automaton = new Set(DICT.settings.filter((s) => s.name.startsWith("auto_") && !["auto_hold", "auto_hold_c", "auto_drop_c", "auto_step", "auto_step_mid", "auto_step_small", "auto_mid_c", "auto_small_c", "auto_lock_min", "auto_lock_c", "auto_asked_by", "auto_room_c", "auto_room_eco_c", "auto_room_gap_c", "auto_profile"].includes(s.name)).map((s) => s.name));
    const placed = new Set(FIRE.flatMap((id) => d.sectionNames(sections[id])));
    assert.deepStrictEqual([...automaton].filter((n) => !placed.has(n)), [], "a number of the fire the Service does not hold");
});

test("the system tab holds the time and the zone, the three numbers of the way back into the network, the Service and the board's own: what it is, restart, the way down, the console", () => {
    const outside = MENU.tree.system.sections.filter((s) => s.group !== "service");
    assert.deepStrictEqual(outside.flatMap((s) => settingsIn(s)).sort(), ["net_point_tries", "net_reboot_laps", "net_reset_laps", "time", "zone"]);
    assert.ok(outside.some((s) => s.board === true), "the board's own");
    assert.deepStrictEqual(serviceOf(MENU.tree).map((s) => s.id), ["hands", "hands_relays", "hands_fan", ...FIRE, "restart", "mains", "soot_scale"], "the manual control, its relays and its fan, then the numbers of the fire, then the hour of the planned restart, then the socket the burner is plugged into, then the edges of the soot scale");
});

test("the manual control is in the Service: the one button of the hands, the relays and the fan's hands - and no module's page holds a control", () => {
    const service = Object.fromEntries(serviceOf(MENU.tree).map((s) => [s.id, s]));
    assert.strictEqual(service.hands.part, "hands", "the button of the hands is a part of the device's page");
    assert.deepStrictEqual(service.hands_relays.cards, ["relays"]);
    assert.deepStrictEqual(d.sectionNames(service.hands_relays), ["auger_limit"], "the limit of the manual auger stands with the relays it limits");
    assert.deepStrictEqual([...service.hands_fan.actions].sort(), ["fan/draught", "fan/gate", "fan/purge"]);
    for (const page of pagesOf(MENU.tree)) assert.ok(!page.sections.some((s) => s.actions && s.actions.length), `the page ${page.id} places a command`);
    assert.ok(!pagesOf(MENU.tree).some((p) => p.id === "relays"), "the relays are no module's page: their rows are the hands");
});

test("every control of the board has one home: the Service, the scheme's one button or its \"light now\", the hopper or the hands' button - and no section lists controls apart from its card", () => {
    for (const section of sectionsOf(MENU.tree)) assert.ok(!("controls" in section), `section ${section.id} lists controls apart from its card`);
    const controls = DICT.things.filter((t) => !t.part_of && (t.can || []).some((c) => ["switch", "toggle", "hold", "press"].includes(c)) && t.card !== "settings");
    const home = (t) => {
        const name = `${t.card}/${t.name}`;
        if (["thermostat/start", "thermostat/stop"].includes(name)) return "run";
        if (name === "thermostat/light") return "light_now";
        if (name === "automaton/feed") return "hopper";
        if (name === "automaton/semi") return "hands";
        if (name === "soot/clean") return "soot_part";
        const inService = serviceOf(MENU.tree).some((s) => (s.cards || []).includes(t.card) || (s.actions || []).includes(name));
        return inService ? "service" : null;
    };
    assert.deepStrictEqual(controls.filter((t) => home(t) === null).map((t) => `${t.card}/${t.name}`), [], "a control that has no home");
    assert.deepStrictEqual(controls.filter((t) => home(t) === "service").map((t) => `${t.card}/${t.name}`).sort(), ["fan/draught", "fan/gate", "fan/purge", "relays/auger", "relays/aux_power", "relays/igniter", "relays/pump"]);
});

test("a section is named by a code, once, and every title that is given is worded in English", () => {
    const ids = [...sectionsOf(MENU.tree).map((s) => s.id), ...pagesOf(MENU.tree).map((p) => `page:${p.id}`)];
    assert.strictEqual(new Set(ids).size, ids.length, "an id twice");
    for (const page of pagesOf(MENU.tree)) assert.ok(page.title && typeof page.title.en === "string" && page.title.en !== "", page.id);
    for (const section of sectionsOf(MENU.tree)) assert.ok(!section.title || (typeof section.title.en === "string" && section.title.en !== ""), section.id);
    for (const group of MENU.tree.modules.groups) assert.ok(group.title && group.title.en, group.id);
});

test("the board's dictionary says nothing of where a setting stands", () => {
    const keys = new Set(DICT.settings.flatMap((s) => Object.keys(s)));
    for (const field of ["section", "tab", "tree", "place", "group", "menu", "module"]) assert.ok(!keys.has(field), `the dictionary's settings carry '${field}'`);
});

test("the menu says a setting's level nowhere: the tree is where it stands", () => {
    const withLevel = Object.entries(MENU.modules).flatMap(([card, m]) => Object.entries(m.settings || {}).filter(([, s]) => "level" in s).map(([name]) => `${card}/${name}`));
    assert.deepStrictEqual(withLevel, []);
});

// the page reads the tree in its language, as it reads the rest of the menu
test("the tree is read in the page's language, and en where the menu says none", () => {
    const en = d.treeOf(TEXTS);
    assert.ok(en && en.modules.pages.length >= 7);
    const first = MENU.tree.modules.pages[0];
    assert.strictEqual(en.modules.pages[0].title, first.title.en);
    const other = JSON.parse(JSON.stringify(MENU));
    other.tree.modules.pages[0].title.ru = "ру";
    other.tree.modules.pages[1].title.ru = "";
    assert.strictEqual(d.treeOf({ menu: other, lang: "ru" }).modules.pages[0].title, "ру");
    assert.strictEqual(d.treeOf({ menu: other, lang: "ru" }).modules.pages[1].title, MENU.tree.modules.pages[1].title.en, "an empty ru falls back to en");
    assert.strictEqual(d.treeOf({}), null, "no menu, no tree");
});

// the settings of a section as the page draws them: the dictionary's own, in the tree's order, the ones a choice governs left out while it is not the one set
test("the settings of a section come in the tree's order, whatever the order of the dictionary", () => {
    const every = [{ name: "b", value: "x" }, { name: "a", value: "y" }, { name: "c", value: "z" }];
    assert.deepStrictEqual(d.sectionSettings({ settings: ["a", "b"] }, every, {}).map((s) => s.name), ["a", "b"]);
});

test("a setting the board does not have is not drawn, and one a choice governs stands only while that choice is set", () => {
    const mode = { name: "mode", value: "power", options: [{ value: "power", shows: ["step"] }, { value: "boiler", shows: [] }] };
    const step = { name: "step", value: "14" };
    const section = { settings: ["mode", "step", "ghost"] };
    assert.deepStrictEqual(d.sectionSettings(section, [mode, step], {}).map((s) => s.name), ["mode", "step"]);
    assert.deepStrictEqual(d.sectionSettings(section, [{ ...mode, value: "boiler" }, step], {}).map((s) => s.name), ["mode"]);
    assert.deepStrictEqual(d.sectionSettings(section, [mode, step], { mode: { value: "boiler" } }).map((s) => s.name), ["mode"], "what a person picked and has not saved counts");
});

test("a person finds a setting through the tree: its place says which tab, which module's page and which section", () => {
    const tree = d.treeOf(TEXTS);
    const hold = d.placeOf(tree, "auto_hold_c");
    assert.deepStrictEqual(hold, { tab: "modules", module: "thermostat", section: "thermostat" }, "a daily setting is shown on the main screen too, and its place is its module's page");
    assert.deepStrictEqual(d.placeOf(tree, "role_ntc_1"), { tab: "modules", module: "temperatures", section: "temperatures" });
    assert.deepStrictEqual(d.placeOf(tree, "up_kp"), { tab: "modules", module: "fan", section: "fan" });
    assert.deepStrictEqual(d.placeOf(tree, "zone"), { tab: "system", section: "time" });
    assert.strictEqual(d.placeOf(tree, "nothing"), null);
});

test("the page lists every module's sections whatever the page they are on", () => {
    const tree = d.treeOf(TEXTS);
    assert.deepStrictEqual(d.sectionsOf(tree).map((s) => s.id), sectionsOf(MENU.tree).map((s) => s.id));
    assert.deepStrictEqual(d.sectionsOf(null), []);
});
