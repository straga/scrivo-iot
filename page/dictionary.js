// Words and levels out of a board's values of contract 3: the board sends what each address IS, by code
// and number, and this page keeps its own table of texts (texts.json) saying how a person reads each of them.
//
// A TEMPLATE is text with places in braces: `{}` is the row's own value, `{spread}` a part of the row - said by its own
// words when named bare, as a number with `{spread:.0}` -, `{unit}` the row's unit. A place's spec: `.N` - N places,
// `span` - seconds as "4 min 22 s", `clock` - seconds as "4:22", and "1:23:10" from an hour, `daytime` - minutes since midnight as "06:00", `sig` - a small quantity as two figures from 1 up and one under it.
//
// A CASE is what a value answers to: `on`, `off`, the code itself, `unknown:<reason>`, `number`, `at_least`,
// `seconds`, `words`, `text`, `none`, `0` for a number or a count that is nought, `*` for anything. A key may hang on a
// neighbour - a part, or a row of the same card - as `on&feed=off`. The words and the level are those of THE FIRST KEY,
// IN THE ORDER WRITTEN, that holds: which of two true things to say is the table's to choose, so the cases are pairs in
// an array and never keys of an object, whose order JavaScript does not keep for a key like "0".
(function (root) {
    "use strict";

    // the kind a value is on the wire, the thing's KINDS saying which of a number's kinds it is
    function kindOf(value, kinds) {
        if (value === null || value === undefined) return "none";
        if (typeof value === "boolean") return "on";
        if (typeof value === "number") return kinds.includes("number") ? "number" : "seconds";
        if (typeof value === "string") return "word";
        if (Array.isArray(value)) return "words";
        if ("at_least" in value) return "at_least";
        if ("unknown" in value) return "unknown";
        if ("text" in value) return "text";
        return "none";
    }

    // the cases VALUE answers to, first what says the most
    function cases(value, kinds) {
        const kind = kindOf(value, kinds);
        const own = kind === "on" ? (value ? "on" : "off")
            : kind === "word" ? value
            : kind === "unknown" ? "unknown:" + value.unknown
            : kind;
        const out = [own];
        if ((kind === "number" || kind === "seconds") && value === 0) out.unshift("0");
        if (kind === "unknown") out.push("unknown");
        out.push("*");
        return out;
    }

    function thing(dict, at) {
        return (dict.things || [])[at];
    }

    function valueAt(values, at) {
        return at === null || at === undefined || values[at] === undefined ? null : values[at];
    }

    // the address of the row NAME of CARD
    function address(dict, card, name) {
        const i = (dict.things || []).findIndex((t) => t.card === card && t.name === name && !t.part_of);
        return i < 0 ? null : i;
    }

    // the address of the part NAME of the thing at AT
    function part(dict, at, name) {
        const row = thing(dict, at);
        if (!row) return null;
        const i = dict.things.findIndex((t) => t.card === row.card && t.part_of === row.name && t.name === name);
        return i < 0 ? null : i;
    }

    // what the template of the thing at AT calls NAME: a part of its own, or - for a part - a part beside it
    function called(dict, at, name) {
        const own = part(dict, at, name);
        if (own !== null) return own;
        const t = thing(dict, at);
        if (!t || !t.part_of) return null;
        const row = address(dict, t.card, t.part_of);
        return row === null ? null : part(dict, row, name);
    }

    // a neighbour a key may hang on: what the template could name, or a row of the same card
    function neighbour(dict, at, name) {
        const n = called(dict, at, name);
        if (n !== null) return n;
        const t = thing(dict, at);
        return t ? address(dict, t.card, name) : null;
    }

    // the key of a table of texts that names the thing T: `card/name`, `card/row.part`
    function keyOf(t) {
        return t.part_of ? `${t.card}/${t.part_of}.${t.name}` : `${t.card}/${t.name}`;
    }

    // ---- the menu ----
    //
    // THE MENU SAYS WHERE AND HOW A MODULE IS SHOWN, and the board says only what the module IS. It lies in the page's kit (menu.json) beside the table of
    // texts, keyed by the board's own names: `modules.<card>` holds its `card` (title, and `after` - the card it stands behind), its `rows` (`<row>` and
    // `<row>.<part>`: label, says, level, `now` - where it stands in the card Now, one entry or a list of them under conditions (`when`, `rank`) -, `scene` - where it stands on the drawing: one entry or a list under conditions like `now`, `place` and, for the pot, `number`, the row its number is read from -, `curves`: title and
    // the words and span of each line), its `settings` (label, unit, level `work` or `tuning`, note) and the `notes` of its settings' parts; beside `modules` the menu's `scheme` holds the words the drawing says (scheme.js, WORDS_NEEDED). Every text is
    // `{en, ru}`; the page reads `texts.lang` and falls back to en for an empty ru. A module the menu does not name is drawn by the table of texts and
    // by what the dictionary says, as before: the two forms live side by side until the last module has moved.

    // NODE with every `{en, ru}` text in it put as the string of LANG
    function localized(node, lang) {
        if (Array.isArray(node)) return node.map((x) => localized(x, lang));
        if (node === null || typeof node !== "object") return node;
        const keys = Object.keys(node);
        if (keys.length && keys.every((k) => k === "en" || k === "ru")) {
            const said = node[lang];
            return said !== undefined && said !== "" ? said : node.en;
        }
        return Object.fromEntries(keys.map((k) => [k, localized(node[k], lang)]));
    }

    const localizedMenus = new WeakMap();

    // the page's menu in the page's language, or null where the page has none; made once for a menu and a language
    function menuOf(texts) {
        if (!texts || !texts.menu) return null;
        const lang = texts.lang || "en";
        let byLang = localizedMenus.get(texts.menu);
        if (!byLang) localizedMenus.set(texts.menu, (byLang = new Map()));
        if (!byLang.has(lang)) byLang.set(lang, localized(texts.menu, lang));
        return byLang.get(lang);
    }

    // TEXT with its {places} filled from VARS: a place the vars do not name is left as it stands
    function spell(text, vars) {
        return String(text === undefined ? "" : text).replace(/\{(\w+)\}/g, (all, name) => (vars && vars[name] !== undefined ? String(vars[name]) : all));
    }

    // THE LANGUAGE OF THE PAGE: what the address asks for (`?lang=`), else what a person chose and the browser kept, else Russian - and only a language the page has. ASKED and STORED are the two texts, empty where there is none.
    const LANGS = ["ru", "en"];
    function langOf({ asked, stored }) {
        for (const one of [asked, stored]) if (LANGS.includes(one)) return one;
        return "ru";
    }

    // THE TABLE OF TEXTS in LANG: the table as it is for English, and for another language the table with its `ru` laid over it - objects merged, anything else (a list of choices, a word) replaced.
    function speak(texts, lang) {
        const over = (base, top) => {
            if (top === undefined) return base;
            if (base === null || typeof base !== "object" || Array.isArray(base) || top === null || typeof top !== "object" || Array.isArray(top)) return top;
            return { ...base, ...Object.fromEntries(Object.keys(top).map((k) => [k, over(base[k], top[k])])) };
        };
        if (lang === "en" || !texts[lang]) return texts;
        const { [lang]: laid, ...rest } = texts;
        return over(rest, laid);
    }

    // THE UNITS of the menu in the page's language: {code: how a person reads it}; {} for a menu that has none
    function menuUnits(texts) {
        const menu = menuOf(texts);
        return (menu && menu.units) || {};
    }

    // THE WORDS OF THE INTERFACE in the page's language, from the menu's `ui`: the buttons, the tabs, the headings and what the page says of itself. {} for a page whose menu has none.
    function uiOf(texts) {
        const menu = menuOf(texts);
        return (menu && menu.ui) || {};
    }

    // THE REFERENCE: groups of items the menu explains, in the order of the menu's tree - the main screen, then each module's page, a section with the settings it holds -, then the stages of the cycle, then the cards of the modules.
    // Each item is its name and its line, said by the menu's `help`. An item the menu has no line for is left out; the rows of the same kind (the seven sockets) are one.
    function helpOf(texts) {
        const menu = menuOf(texts);
        if (!menu || !menu.help) return [];
        const entries = menu.help.entries || {};
        const item = (key) => (entries[key] ? { key, name: entries[key].name, text: entries[key].text } : null);
        const groups = [];
        const tree = menu.tree || {};
        // the main screen shows settings that stand on the automaton's page: it is told once, there
        for (const section of (tree.main && tree.main.sections) || []) groups.push({ title: section.title, items: [`section:${section.id}`, ...sectionNames(section)].map(item).filter(Boolean) });
        for (const page of pagesOf(tree)) {
            for (const section of page.sections || []) {
                const keys = [`section:${section.id}`, ...sectionNames(section)];
                groups.push({ title: section.title || page.title, items: keys.map(item).filter(Boolean) });
            }
        }
        for (const section of (tree.system && tree.system.sections) || []) groups.push({ title: section.title, items: [`section:${section.id}`, ...sectionNames(section)].map(item).filter(Boolean) });
        const stages = Object.values(menu.modules || {}).flatMap((m) => ((m.cycle && m.cycle.sections) || []).map((s) => `stage:${s.name}`));
        if (stages.length) groups.push({ title: (menu.help.groups || {}).stages, items: stages.map(item).filter(Boolean) });
        for (const [card, module] of Object.entries(menu.modules || {})) {
            const keys = Object.keys(module.rows || {}).filter((k) => !k.includes(".") && !/^ntc_[2-7]$/.test(k)).map((k) => `row:${card}/${k}`);
            const items = keys.map(item).filter(Boolean);
            if (items.length) groups.push({ title: (module.card && module.card.title) || card, items });
        }
        return groups.filter((g) => g.items.length);
    }

    // the module of the menu whose settings hold NAME, and what the menu says of it: {card, entry}, or null
    function menuSetting(texts, name) {
        const menu = menuOf(texts);
        for (const [card, module] of Object.entries((menu && menu.modules) || {})) {
            const entry = (module.settings || {})[name];
            if (entry) return { card, entry, module };
        }
        return null;
    }

    // what the menu says of the row or part T, or null: a row of its module, or the words of a setting's part
    function menuRow(texts, t) {
        const menu = menuOf(texts);
        if (!menu) return null;
        if (t.card === "settings") {
            if (!t.part_of) return null;
            const owner = menuSetting(texts, t.part_of);
            return (owner && (owner.module.notes || {})[`${t.part_of}.${t.name}`]) || null;
        }
        const module = (menu.modules || {})[t.card];
        return (module && (module.rows || {})[t.part_of ? `${t.part_of}.${t.name}` : t.name]) || null;
    }

    function textsOf(texts, t) {
        return menuRow(texts, t) || ((texts && texts.rows) || {})[keyOf(t)] || {};
    }

    // CARDS in the order the menu puts them: a card the menu says stands after another goes behind it, the rest keep theirs
    function cardOrder(cards, texts) {
        const out = [...cards];
        const menu = menuOf(texts);
        for (const [name, module] of Object.entries((menu && menu.modules) || {})) {
            const after = module.card && module.card.after;
            if (!after || !out.includes(name) || !out.includes(after)) continue;
            out.splice(out.indexOf(name), 1);
            out.splice(out.indexOf(after) + 1, 0, name);
        }
        return out;
    }

    // what the card CARD is headed by: the menu's title, or its own name
    function cardTitle(texts, card) {
        const menu = menuOf(texts);
        const module = menu && (menu.modules || {})[card];
        return (module && module.card && module.card.title) || card;
    }

    // The entries of a menu row's `now` or `scene`: one, or a list. A list is for a row that stands in the card Now as several things by what a part of it says - the sensor
    // whose role is the boiler's, the one whose role is the supply's -: each entry carries the condition it stands there under (`when`, the keys `says` and `level` use)
    // and its `rank` among the entries that stand behind the same row.
    function nowEntries(now) {
        return !now ? [] : Array.isArray(now) ? now : [now];
    }

    // the entry of NOW that holds for the row at AT as it stands now, or null
    function nowEntry(dict, all, at, now) {
        return nowEntries(now).find((e) => e.when === undefined || looked(dict, all, at, [[e.when, true]]) === true) || null;
    }

    // where the menu's ENTRY stands in the card Now, as a place among the board's list `machine.now`: just behind the row it is to stand after, and the
    // entries behind the same one by their rank, then in the order the menu writes them
    function nowAfter(dict, texts, entry) {
        const menu = menuOf(texts);
        const list = (dict.machine && dict.machine.now) || [];
        const same = [];
        for (const module of Object.values((menu && menu.modules) || {})) {
            for (const said of Object.values(module.rows || {})) {
                for (const e of nowEntries(said.now)) same.push({ e, after: e.after, rank: e.rank || 0, written: same.length });
            }
        }
        const mine = same.find((x) => x.e === entry);
        const behind = mine ? mine.after : undefined;
        const at = list.findIndex((n) => `${n.card}/${n.row}` === behind);
        const k = same.filter((x) => x.after === behind).sort((a, b) => a.rank - b.rank || a.written - b.written).findIndex((x) => x.e === entry);
        return (at < 0 ? list.length : at) + 0.5 + k / 1000;
    }

    // THE CYCLE THE MENU SAYS A MODULE RUNS: which row of a card is the stage, which says what it does now, which says where it broke, which are pressed, which stand
    // apart from the card's list, and the numbers it goes by stage by stage - the page's own words for each. The shape the board's dictionary once carried as `machine`, said by
    // the page: the board names the entities, the menu names the cycle. null where no module of the menu has one.
    function cycleOf(texts) {
        const menu = menuOf(texts);
        for (const [card, module] of Object.entries((menu && menu.modules) || {})) {
            const cycle = module.cycle;
            if (!cycle) continue;
            const sections = cycle.sections || [];
            const words = {};
            for (const section of sections) {
                const items = {};
                const specs = {};
                for (const item of section.items || []) {
                    items[item.of[0]] = item.label;
                    if (item.spec) specs[item.of[0]] = item.spec;
                }
                words[section.name] = { title: section.title, items, specs, notes: section.notes || [] };
            }
            return {
                card,
                fixed: card,
                stage: cycle.stage,
                says: cycle.says,
                cause: cycle.cause,
                // the buttons stand on the card the menu names - the thermostat's, for the automaton's cycle - and on the cycle's own when it names none
                pressCard: cycle.press_card || card,
                press: cycle.press || [],
                aside: cycle.aside || [],
                sections: sections.map((section) => ({ name: section.name, by: section.by || "", items: (section.items || []).map((item) => item.of) })),
                words,
                now: [],
            };
        }
        return null;
    }

    // the rows of the cards the card Now is made of: those of the board's list and those the menu puts in it
    function nowRows(things, texts, machine) {
        const cards = new Set(((machine && machine.now) || []).map((n) => n.card));
        const menu = menuOf(texts);
        for (const [card, module] of Object.entries((menu && menu.modules) || {})) {
            if (Object.values(module.rows || {}).some((row) => row.now)) cards.add(card);
        }
        return (things || []).filter((t) => cards.has(t.group));
    }

    // THE MENU'S TREE, in the page's language: `main` - the sections of the main screen, the daily settings - and `modules`: its `groups` (the hardware, the logic), each the ids of its modules in
    // the order they are listed, and its `pages`, one for each module, holding everything about it in sections - the cards whose rows are its readings, its curves and its controls (`cards`), its
    // settings in the order they stand, a table, the automaton's own (`burning`) and, for the board's, a mark. null for a page whose menu has no tree.
    function treeOf(texts) {
        const menu = menuOf(texts);
        return (menu && menu.tree) || null;
    }

    // THE PAGES OF THE MODULES, in the order the groups list them: [{id, title, group, sections}]. [] for a page whose menu has no tree.
    function pagesOf(tree) {
        const modules = (tree && tree.modules) || {};
        const pages = modules.pages || [];
        return (modules.groups || []).flatMap((group) => (group.modules || []).map((id) => pages.find((p) => p.id === id)).filter(Boolean).map((page) => ({ ...page, group: group.id })));
    }

    // EVERY SECTION OF THE TREE: the main screen's, then the modules' pages' one by one, then the system tab's.
    function sectionsOf(tree) {
        return [...((tree && tree.main && tree.main.sections) || []), ...((tree && tree.modules && tree.modules.pages) || []).flatMap((page) => page.sections || []), ...((tree && tree.system && tree.system.sections) || [])];
    }

    // THE NAMES OF WHAT A SECTION HOLDS, in the order the menu lists them: its settings, then the numbers of its table row by row, then its actions - the one answer to "what stands in this section"
    function sectionNames(section) {
        const own = (section && section.settings) || [];
        const cells = (((section && section.table) || {}).rows || []).flatMap((row) => row.settings || []);
        // the windows of a schedule are settings the page draws as forms, listed apart from the lines
        return [...own, ...cells, ...((section && section.windows) || []), ...((section && section.actions) || [])];
    }

    // WHERE A SETTING STANDS: {tab: "modules", module, section} - the page of the module it belongs to; a setting the main screen shows too stands on its module's page first -, {tab: "system", section}
    // for the board's own, {tab: "main", section} for one the main screen alone holds, or null: a setting the tree does not hold is a gap in the menu, never a guess
    function placeOf(tree, name) {
        for (const page of (tree && tree.modules && tree.modules.pages) || []) {
            const section = (page.sections || []).find((s) => sectionNames(s).includes(name));
            if (section) return { tab: "modules", module: page.id, section: section.id };
        }
        for (const tab of ["system", "main"]) {
            const section = ((tree && tree[tab] && tree[tab].sections) || []).find((s) => sectionNames(s).includes(name));
            if (section) return { tab, section: section.id };
        }
        return null;
    }

    // THE ROWS OF A SECTION as the page draws them. `shown`: the rows of the cards the menu puts here, in the order it names the cards and the board's within each - except a row some section of the tree
    // names in its `actions`: a row the menu says is drawn apart (`apart`) is in none of the lists; a command is drawn where the menu places it, and nowhere else, so the card of a module on the main screen shows the readings and the module's own page holds
    // the command beside its settings. `actions`: the rows this section names, in the order it names them; a name the board has no row for is left out.
    function sectionRows(tree, section, things) {
        const cards = (section && section.cards) || [];
        const placed = new Set(sectionsOf(tree).flatMap((s) => s.actions || []));
        const all = things || [];
        const shown = all.filter((t) => cards.includes(t.group) && !t.apart && !placed.has(actionName(t))).sort((x, y) => cards.indexOf(x.group) - cards.indexOf(y.group));
        const actions = ((section && section.actions) || []).map((name) => all.find((t) => actionName(t) === name)).filter(Boolean);
        return { shown, actions };
    }

    // THE NAME THE MENU CALLS A ROW BY WHEN IT PLACES IT: its card and its name, as the help calls it (`row:card/name`) - a name alone is the same in two cards
    function actionName(t) {
        return `${t.group}/${t.key}`;
    }

    // THE TABLE OF A SECTION, or null for a section that holds none: its columns, and a row a step with the board's setting in each cell - null where the board has none - and the mark of the row the burner burns on.
    // THAT ROW IS THE ONE THE BOARD HOLDS, never the one a person has picked and not saved: the burner burns on what the board keeps. A level the table has no row for marks none.
    function tableOf(section, every) {
        const table = section && section.table;
        if (!table) return null;
        const held = new Map((every || []).map((s) => [s.name, s]));
        const chosen = held.get(table.chosen);
        return {
            columns: table.columns || [],
            rows: (table.rows || []).map((row) => ({
                step: row.step,
                title: row.title,
                chosen: Boolean(chosen) && chosen.value === row.step,
                cells: (row.settings || []).map((name) => held.get(name) || null),
            })),
        };
    }

    // THE NUMBERS OF THE ROW THE BURNER BURNS ON, as settings called by the column they stand in - `step.pa`, `step.on_s` -: what the automaton's table of the stages says of the draught and the feed, whichever row is
    // chosen. Nothing where the board names no row, so a stage says the number is not sent and does not make one up.
    function chosenStep(tree, every) {
        for (const section of sectionsOf(tree)) {
            const table = tableOf(section, every);
            if (!table) continue;
            const row = table.rows.find((r) => r.chosen);
            if (!row) return [];
            return row.cells.map((cell, i) => cell && { ...cell, name: `step.${table.columns[i].key}` }).filter(Boolean);
        }
        return [];
    }

    // THE SETTINGS OF A SECTION as the page draws them: the board's own settings (EVERY), in the order the section lists them; one the board has not is left out,
    // and so is one a choice governs while that choice is not the one set (PICKED over what the board holds) - burning.inForce
    // THE WINDOWS OF A SECTION: the board's settings the menu draws as forms and not as lines - `windows` in the section -, in the order it lists them, the ones the board has
    function windowsOf(section, every, picked) {
        const held = new Map((every || []).map((s) => [s.name, s]));
        const mine = ((section && section.windows) || []).filter((name) => held.has(name)).map((name) => held.get(name));
        return root.wire.inForce(mine, picked, every);
    }

    function sectionSettings(section, every, picked) {
        const held = new Map((every || []).map((s) => [s.name, s]));
        const mine = (section.settings || []).filter((name) => held.has(name)).map((name) => held.get(name));
        return root.wire.inForce(mine, picked, every);
    }

    // the words the drawing says, in the page's language: the menu's `scheme`, or none for a page whose menu has none
    function schemeWords(texts) {
        const menu = menuOf(texts);
        return (menu && menu.scheme) || {};
    }

    // the lines of a curve as the board answered them, with the words and the span the menu says for the line of the same place
    function curveLines(lines, curve) {
        if (!curve || !curve.lines) return lines;
        return lines.map((line, i) => {
            const said = curve.lines[i];
            if (!said) return line;
            const out = { ...line };
            if (said.words !== undefined && said.words !== "") out.words = said.words;
            if (said.span !== undefined) out.span = said.span;
            return out;
        });
    }

    // what PAIRS - [[key, said], ...] - say for the thing at AT: the first key that holds
    function looked(dict, values, at, pairs) {
        if (!pairs) return null;
        const t = thing(dict, at);
        const own = cases(valueAt(values, at), t.kinds || []);
        for (const [key, said] of pairs) {
            const [first, ...conds] = key.split("&");
            if (!own.includes(first)) continue;
            const hold = conds.every((c) => {
                const [name, wanted] = c.split("=");
                const n = neighbour(dict, at, name);
                if (n === null) return false;
                return cases(valueAt(values, n), thing(dict, n).kinds || []).includes(wanted);
            });
            if (hold) return said;
        }
        return null;
    }

    // The words of the button that folds a card's settings, from the page's texts (`fold`: `show` with {n} for how many, and `hide`): none made up where the texts have none.
    function foldWords(texts, open, n) {
        const said = (texts && texts.fold && texts.fold[open ? "hide" : "show"]) || "";
        return said.replace("{n}", String(n));
    }

    // THE UNITS A PERSON READS: the menu's `units` in the page's language, laid in once as the page loads; a unit the menu does not name is its own code
    let UNITS = {};
    function useUnits(map) {
        UNITS = map || {};
    }
    function unit(code) {
        return UNITS[code] !== undefined && UNITS[code] !== "" ? UNITS[code] : code;
    }

    // seconds as a span: "45 s", "5 min", "4 min 22 s" - in the units of the language
    function span(seconds) {
        const m = Math.floor(seconds / 60), s = seconds % 60;
        if (m === 0) return `${s} ${unit("s")}`;
        return s === 0 ? `${m} ${unit("min")}` : `${m} ${unit("min")} ${s} ${unit("s")}`;
    }

    // WHAT A CURVE SAYS OF ITS PACE: the span of the points it holds and the step between two of them, in the units and the words of the language - "15 min · 1000 ms a point"; the words are the menu's `ui.pace`, with the places {span} and {ms}
    function paceWords(points, stepMs, ui) {
        const seconds = Math.round(points * stepMs / 1000);
        const held = seconds >= 120 ? Math.round(seconds / 60) + " " + unit("min") : seconds + " " + unit("s");
        return spell(ui.pace, { span: held, ms: stepMs + " " + unit("ms") });
    }

    // N as the board's Rust wrote it with PLACES places: the f32 it held, a tie to the even digit
    function fixed(n, places) {
        const sign = n < 0 ? "-" : "";
        const v = Math.abs(Math.fround(n));
        const scaled = v * Math.pow(10, places);
        const floor = Math.floor(scaled);
        const text = scaled - floor === 0.5
            ? ((floor % 2 === 0 ? floor : floor + 1) / Math.pow(10, places)).toFixed(places)
            : v.toFixed(places);
        return (Number(text) === 0 ? "" : sign) + text;
    }

    // N as a person reads a small quantity: two figures from 1 up, one under it - 14.4 is 14, 0.59 is 0.6, 0.0147 is 0.01
    function significant(n) {
        return String(Number(n.toPrecision(n >= 1 ? 2 : 1)));
    }

    // VALUE as it stands in words, SPEC saying how
    function shown(value, spec) {
        const number = typeof value === "number" ? value
            : value && typeof value === "object" && "at_least" in value ? value.at_least : null;
        if (number !== null) {
            if (spec === "volts") return `${(number / 1000).toFixed(2)} ${unit("V")}`;
            if (spec === "span") return span(Math.max(0, Math.round(number)));
            if (spec === "clock") {
                const s = Math.max(0, Math.round(number));
                const two = (n) => String(n).padStart(2, "0");
                // under an hour m:ss, from it h:mm:ss: a wait of two hours reads as two hours and not as a hundred and twenty minutes
                return s >= 3600 ? `${Math.floor(s / 3600)}:${two(Math.floor(s / 60) % 60)}:${two(s % 60)}` : `${Math.floor(s / 60)}:${two(s % 60)}`;
            }
            if (spec && spec.startsWith(".")) return fixed(number, Number(spec.slice(1)) || 0);
            if (spec === "sig") return significant(number);
            // minutes since midnight as the time of day, HH:MM
            if (spec === "daytime") {
                const minutes = ((Math.round(number) % 1440) + 1440) % 1440;
                return `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`;
            }
            return String(number);
        }
        // a setting keeps its number as text; a spec asks for it as the number it is
        if (spec && value && typeof value === "object" && "text" in value && value.text.trim() !== "" && !isNaN(Number(value.text))) {
            return shown(Number(value.text), spec);
        }
        if (value === null || value === undefined) return "-";
        if (typeof value === "boolean") return value ? "on" : "off";
        if (typeof value === "string") return value;
        if (Array.isArray(value)) return value.join(" ");
        if ("unknown" in value) return value.unknown;
        if ("text" in value) return value.text;
        return "-";
    }

    // TEMPLATE filled in for the thing at AT: its own value, its unit, its parts - and a row of its card by name
    function fill(dict, texts, values, at, template) {
        const t = thing(dict, at);
        const own = valueAt(values, at);
        return template.replace(/\{([^{}]*)\}/g, (_, inside) => {
            const [name, spec] = inside.split(":");
            if (name === "") return shown(own, spec);
            if (name === "unit") return unit(t.unit || "");
            const n = called(dict, at, name) ?? address(dict, t.card, name);
            if (n === null) return "-";
            return spec === undefined ? render(dict, texts, values, n).words : shown(valueAt(values, n), spec);
        });
    }

    // what the thing at AT says to a person, and its level
    function render(dict, texts, values, at) {
        const t = thing(dict, at);
        const said = textsOf(texts, t);
        const own = valueAt(values, at);
        const kind = kindOf(own, t.kinds || []);
        const counts = !t.part_of && (kind === "number" || kind === "at_least") && t.unit;
        const template = looked(dict, values, at, said.says) ?? (counts ? "{} {unit}" : "{}");
        return { words: fill(dict, texts, values, at, template), level: looked(dict, values, at, said.level) };
    }

    // The rows of the system card that are what the board IS rather than rows of it: a page shows them apart.
    const BOARD_ROWS = ["free", "used", "uptime", "low_memory", "time", "core_0", "core_1"];

    // VALUE as a page of contract 1 reads a thing's value, UNIT beside a number
    function drawnValue(t, value, unit) {
        const kinds = t.kinds || [];
        if (kinds.includes("button")) return { button: {} };
        switch (kindOf(value, kinds)) {
            case "on": return { on: value };
            case "number": return { number: { value, unit } };
            case "seconds": return { seconds: value };
            case "word": return { word: value };
            case "words": return { words: value };
            case "at_least": return { at_least: { value: value.at_least, unit } };
            case "text": return { text: value.text };
            default: return { unknown: {} };
        }
    }

    // the text a setting is kept as
    function kept(value) {
        return value && typeof value === "object" && "text" in value ? value.text : value === null || value === undefined ? "" : String(value);
    }

    // Where the board puts the row at AT in the card that sums up what it is doing (`machine.now`, in the order it says): the place in its list, or -1.
    // A row is meant by its name, or - where which row it is depends on what the board is set to - by a part of it holding a code: the sensor whose
    // role is the boiler's. That part is read like any other value, so the row is told when the code changes.
    function nowPlace(dict, all, at) {
        const t = dict.things[at];
        const list = (dict.machine && dict.machine.now) || [];
        return list.findIndex((n) => {
            if (n.card !== t.card) return false;
            if (n.row !== undefined) return n.row === t.name;
            const where = part(dict, at, n.part);
            return where !== null && valueAt(all, where) === n.is;
        });
    }

    // What the page's table says of a row of the card Now: its word, and how to show the number beside it. The key is the row the board
    // names, or the card, the part and the code that find it.
    function nowWords(texts, entry) {
        const key = entry.row !== undefined ? `${entry.card}/${entry.row}` : `${entry.card}/${entry.part}=${entry.is}`;
        return ((texts && texts.now) || {})[key] || {};
    }

    // One row of what a page draws, made of the board's DICT, the page's TEXTS and the VALUES by address (a number fixed in the
    // image is the dictionary's own value: ALL is what `held` makes of them): what the thing at AT is called, says, is worth,
    // may be commanded with, how it slides and what it signals. Null for what is not a row of a card - a part, a setting, a
    // row of the board itself - and for a row its own texts hide now.
    function rowOf(dict, texts, all, at) {
        const things = dict.things || [];
        const t = things[at];
        if (t.part_of || t.card === "settings" || (t.card === "system" && BOARD_ROWS.includes(t.name))) return null;
        const said = textsOf(texts, t);
        if ((said.hide || []).length && looked(dict, all, at, said.hide.map((k) => [k, true]))) return null;
        const r = render(dict, texts, all, at);
        const unitCode = said.unit || t.unit || "";
        const one = {
            name: fill(dict, texts, all, at, said.label || t.name),
            // what the board calls it, which the name above - what a person reads - is not
            key: t.name,
            // where a command to it goes
            at,
            group: t.card,
            state: r.words,
            level: r.level,
            value: drawnValue(t, all[at], unitCode),
            can: t.can || [],
        };
        // the few words the drawing says of it, where the menu words them: filled like the label - its own value, its parts, a row of its card by name
        if (typeof said.short === "string") one.short = fill(dict, texts, all, at, said.short);
        // what its command does cannot be taken back, the board says: the button asks before it sends
        if (t.irreversible) one.irreversible = true;
        // it counts down to something that is about to happen, the board says
        if (t.clock) one.clock = true;
        const words = partsOf(dict, t, all);
        if (Object.keys(words).length) one.parts = words;
        // a number the image fixes is read where the numbers a card goes by are listed, and is no row of the card
        if ("fixed" in t) one.fixed = true;
        // the menu says a part of the page draws it - the button that runs the burner is the scheme's -: a row of the board that stands in no card's list
        if (said.apart) one.apart = true;
        // where it stands on the drawing, by the code its part says now (the sensor whose role is the boiler's stands at the boiler), and the key of the row beside it its number is read from
        const stands = nowEntry(dict, all, at, said.scene);
        if (stands) {
            one.place = stands.place;
            if (stands.number !== undefined) one.placeNumber = stands.number;
        }
        const place = nowPlace(dict, all, at);
        const mine = nowEntry(dict, all, at, said.now);
        if (place >= 0) {
            const entry = dict.machine.now[place];
            // a row the menu words is worded by it, whoever lists it: the board of an older image still does
            const words = mine ? { as: mine.as, spec: mine.spec } : nowWords(texts, entry);
            one.now = place;
            one.as = words.as !== undefined ? words.as : one.name;
            const other = entry.with ? address(dict, t.card, entry.with) : null;
            const number = other === null ? undefined : valueAt(all, other);
            if (typeof number === "number") one.alongside = shown(number, words.spec);
        } else if (mine) {
            // a row the menu puts in the card Now: its place, its word and the number beside it are the menu's
            one.now = nowAfter(dict, texts, mine);
            one.as = mine.as !== undefined ? mine.as : one.name;
            const other = mine.beside ? address(dict, t.card, mine.beside) : null;
            const number = other === null ? undefined : valueAt(all, other);
            if (typeof number === "number") one.alongside = shown(number, mine.spec);
        }
        if (t.range) {
            const where = t.range.at;
            const unitThere = said.unit || (things[where] && things[where].unit) || t.unit || "";
            one.range = { min: t.range.min, max: t.range.max, step: t.range.step, value: all[where] ?? t.range.min, unit: unitThere };
        }
        const signal = part(dict, at, "signal");
        if (signal !== null) one.signal = all[signal] ?? null;
        if ((t.curves || []).length) {
            // a curve is worded by its title - the menu's has the lines of the picture besides - or by the table's word, or by its name
            one.curves = t.curves.map((name) => {
                const said_of = (said.curves || {})[name];
                const curve = { name, words: (said_of && typeof said_of === "object" ? said_of.title : said_of) || name };
                if (said_of && typeof said_of === "object" && said_of.lines) {
                    curve.lines = said_of.lines.map((line) => ({ words: line.legend, span: line.span }));
                }
                return curve;
            });
        }
        return one;
    }

    // THE PARTS OF A ROW THAT ARE WORDS: `why` of the automaton's `doing` is the CODE of the cause, and the list of what asks for a person reads the code and never the row's words. The addresses of a row's parts are found once per dictionary.
    const PARTS = new WeakMap();
    function partsOf(dict, t, all) {
        let by = PARTS.get(dict);
        if (!by) {
            by = {};
            (dict.things || []).forEach((p, at) => {
                if (p.part_of) (by[`${p.card}/${p.part_of}`] = by[`${p.card}/${p.part_of}`] || []).push([p.name, at]);
            });
            PARTS.set(dict, by);
        }
        const found = {};
        for (const [name, at] of by[`${t.card}/${t.name}`] || []) if (typeof all[at] === "string") found[name] = all[at];
        return found;
    }

    // The values by address as a page reads them: a number fixed in the image is the dictionary's own value.
    function held(dict, values) {
        return (dict.things || []).map((t, at) => ("fixed" in t ? t.fixed : valueAt(values, at)));
    }

    // One setting as a form draws it: its name and label, what the board holds, what it takes, the words of its choices and
    // what they mean, and its note - the words of a part the board says of it, numbers the board worked out and this page only words.
    function settingOf(dict, texts, all, s) {
        const placed = menuSetting(texts, s.name);
        const words = (placed && placed.entry) || ((texts && texts.settings) || {})[s.name] || {};
        const at = address(dict, "settings", s.name);
        const one = { name: s.name, label: words.label || s.name, value: at === null ? s.default : kept(all[at]) || s.default };
        // a setting the menu words as a form (a window of the schedule) is drawn by its own component, and said in words wherever it is said
        if (words.form) one.form = words.form;
        if (s.kind === "choice") {
            one.options = (s.choices || []).map((c) => {
                const w = (words.choices || []).find((x) => x.value === c.value) || {};
                const option = { value: c.value, words: w.words || c.value };
                // a choice that takes a word of its own keeps it behind its prefix whether or not the menu says what the word is (a window of the schedule is a form: no hint of a line, and the line still goes back behind `set:`)
                if (w.takes) option.takes = w.takes;
                else if (c.takes) option.takes = true;
                if (w.means) option.means = w.means;
                // what the choice bears on, as the board says it: the settings that matter with it, the role a socket must play (board.js draws none of it)
                if (c.shows) option.shows = c.shows;
                if (c.needs) option.needs = c.needs;
                return option;
            });
        }
        // a plain whole number goes out as months: the form draws that field as a number from and to
        if (s.kind === "months" || s.kind === "whole") one.months = [s.from, s.to];
        if (s.kind === "seconds") one.seconds = [s.from, s.to];
        // tenths of a second: the board's span is in tenths, a person types seconds with a tenth, and the field says so to the keyboard
        // a target of the room in degrees is a number with a tenth too, and no seconds: the panel steps through its `range` by 0.1
        if (s.kind === "tenths") {
            one.tenths = true;
            if (!s.unit || s.unit === "s") one.seconds = [s.from / 10, s.to / 10];
            else one.range = [s.from / 10, s.to / 10];
        }
        // the card it stands on: the menu's for a module it places, else the board's
        const card = placed ? placed.card : s.card;
        if (card) one.card = card;
        // what stands beside its number: the unit the board says, or seconds
        if (s.unit) one.unit = unit(s.unit);
        else if (s.kind === "seconds" || s.kind === "tenths") one.unit = unit("s");
        // folded under the card's everyday settings: the board says so, the page only draws it
        // and the menu's level for one it places - `work` is what a setting is without a level -, else the board's
        const level = placed && words.level !== undefined ? (words.level === "tuning" ? "tuning" : undefined) : s.level;
        if (level) one.level = level;
        if (at !== null && words.note !== undefined) {
            const where = words.note === "" ? at : part(dict, at, words.note);
            const said = where === null ? "-" : render(dict, texts, all, where).words;
            // a note the menu words with nothing says nothing: the setting is in order, and "as it should be" tells no one anything
            if (said !== "-" && said !== "") one.note = said;
        }
        return one;
    }

    // What the board says of itself, apart from its cards: memory, the time it has run, whether it is short of memory, its clock.
    function headOf(dict, texts, all) {
        const top = (name) => {
            const at = address(dict, "system", name);
            return at === null ? null : all[at];
        };
        const time = address(dict, "system", "time");
        // how busy each core is: a number a core, none for one not measured yet, and no list at all for a board that has no rows for them
        const cores = ["core_0", "core_1"].map((name) => address(dict, "system", name));
        return {
            cores: cores.every((at) => at === null) ? null : cores.map((at) => (at === null || !Number.isFinite(all[at]) ? null : all[at])),
            free: top("free"),
            used: top("used"),
            uptime: top("uptime"),
            low_memory: top("low_memory"),
            time: time === null ? null : render(dict, texts, all, time).words,
        };
    }

    // What a page draws, all at once: the rows and the settings and the head, made of the board's DICT, the page's TEXTS and
    // the VALUES by address. The page itself draws by `rowOf`, `settingOf` and `headOf`, one at a time (board.js); this
    // is what a tool that wants the whole picture asks for.
    function state(dict, texts, values) {
        const all = held(dict, values);
        const drawn = [];
        (dict.things || []).forEach((_, at) => {
            const one = rowOf(dict, texts, all, at);
            if (one) drawn.push(one);
        });
        const time = headOf(dict, texts, all);
        return {
            contract: dict.contract,
            all: true,
            device: dict.device,
            board: dict.board,
            kit: String(dict.version || "").split("+")[0],
            terminal: dict.terminal,
            ...time,
            things: drawn,
            settings: (dict.settings || []).map((s) => settingOf(dict, texts, all, s)),
        };
    }

    // Whether the board's dictionary NOW is still the one HELD, the page drawing by it: the words to say when it is
    // not, as a code the page says in the menu's words - an image written since the page opened, whose addresses mean other things - and null when it is.
    function stale(held, now) {
        if (now && now.version === held.version) return null;
        return { kind: now && now.version ? "speaksDictionary" : "namesNoDictionary", version: now && now.version ? now.version : "", mine: held.version };
    }

    root.dictionary = {
        render, shown, address, part, cases, keyOf, state, rowOf, settingOf, headOf, held, stale,
        cardOrder, cardTitle, cycleOf, textsOf, nowRows, schemeWords, treeOf, pagesOf, sectionsOf, placeOf, unit, useUnits, menuUnits, paceWords, spell, langOf, speak, uiOf, helpOf, sectionNames, sectionRows, tableOf, chosenStep, sectionSettings, windowsOf, curveLines, foldWords,
    };
})(typeof globalThis !== "undefined" ? globalThis : this);
