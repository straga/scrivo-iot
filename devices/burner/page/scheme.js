// THE ONE PART OF THIS PAGE THAT KNOWS THE PLANT: the burner drawn - hopper, auger, pot, flue and the water loop - with the board's own
// numbers standing where they are measured. Where each organ's action and each number stands on the drawing, which rows say
// what each organ does, and how the flame, the fan, the pump and the feed read on it are the drawing's knowledge, and it names the rows
// it draws by their card and their KEY - what the board calls them -, never by their label: a label is the menu's to word and a person's to change.
// A number whose row means different things by how the board is set up (a temperature socket by its role) stands where the MENU puts it, `place`
// of the row (dictionary.js). Every word the drawing says is the menu's (`scheme`, WORDS_NEEDED below), handed in as WORDS.
// The rest of the page names nothing of the board: it draws by what the board's dictionary says.
// Judged by values and not by pixels, so it lives apart from the component (`Scheme` of app.js) and is tested on its own
// (tools/test_page_scheme.js).
(function (root) {
    "use strict";

    // THE FLAME ON THE SCHEME: the one word of its state - out, weak, steady, by the CODE of its value; said in the words the menu gives that code - in the pot, coloured by its level. The
    // scheme draws numbers, and the flame is not one: it is three states through two thresholds (crates/flame), so
    // it goes on the drawing as its word. Not read yet is not a state of the fire and puts nothing there.
    const FLAME_WORDS = ["out", "weak", "steady", "unconfigured"];

    function flameOnScheme(row) {
        const word = row && row.value && row.value.word;
        return FLAME_WORDS.includes(word) ? { word, words: row.state, level: row.level } : null;
    }

    // A ROW IS ITS KEY AND ITS CARD, NEVER ITS KEY ALONE. The automaton's card carries auger, igniter and fan - what it switches - beside the
    // relays of the same keys, and the draught and the flame both have a pin. Found by key alone, the scheme drew the automaton's "auger on" as a
    // relay closed while every relay was open (23.09).
    function find(things, group, key) {
        return (things || []).find((t) => t.group === group && t.key === key) || null;
    }

    // the card whose rows say what each organ does: the automaton's
    const AUTOMATON = "automaton";
    const THERMOSTAT = "thermostat";
    // the card of what the house asks of the boiler and why: the thermostat of the page, the floor above the boiler's water
    const DEMAND = "demand";

    // WHAT ASKS FOR A PERSON: the causes the automaton names (the board's own list, the parts of `automaton/doing.why`) and the rows - by their card and key - that stand at the level bad when something is wrong with the plant. Each has an action in the menu;
    // the test of the table holds the menu against these lists, and the menu's rows against the second, so a row that can go bad and has no action is found before a person is.
    const ATTENTION_WHY = { never_lit: 1, flame_lost: 1, flame_not_back: 1, no_fuel: 1, no_draught: 1, low_pressure: 1, overheated: 1, unconfigured: 1, draught_silent: 1, flame_silent: 1, water_silent: 1, return_silent: 1, no_fronts: 1 };
    const ATTENTION_ROWS = {
        "thermostat/silent_return": 1, "demand/silent_room": 1,
        "automaton/silent_draught": 1, "automaton/silent_flame": 1, "automaton/silent_flue": 1, "automaton/silent_water": 1, "automaton/silent_fronts": 1, "automaton/alarm": 1,
        "temperatures/bus": 1, "flue/flue_temp": 1, "draught/blast_pressure": 1, "flame/flame": 1,
        "fan/holding": 1,
    };

    // The words the drawing says, by where they stand in the menu's `scheme`: every leaf is a word the menu owes the drawing (the test of the table
    // holds the menu against this list, so a drawing never says a word nobody worded).
    const WORDS_NEEDED = {
        title: 1,
        parts: { hopper: 1, pot: 1, fan: 1, flue: 1, water: 1, flame: 1 },
        places: { flue: 1, water_out: 1, water_back: 1, draught: 1, boiler: 1, room: 1 },
        hands: { take: 1, give: 1, take_text: 1, give_text: 1 },
        feed: { pause: 1, banned: 1, allow: 1, ban: 1 },
        run: { start: 1, stop: 1, burning_off: 1, light_now: 1 },
        actions: { feeding: 1, pause: 1, feed_banned: 1, burning: 1, holds: 1, full: 1, by_hand: 1 },
        why: { water: 1, overheated: 1, no_number: 1 },
        gap: 1,
        heat: { smoulder: 1 },
        pot: { step: 1, step_last: 1 },
        hold: { power: 1, boiler: 1, return: 1 },
        ask: { off: 1, unasked: 1, cold: 1, warm: 1, waits: 1, wait: 1, pause: 1, manual: { room: 1, water: 1 }, why: { profile_off: 1, scheduled_on: 1, scheduled_off: 1, source_silent: 1, not_read: 1, starting: 1 } },
        target: { comfort: 1, eco: 1, lower: 1, raise: 1, gap_lower: 1, gap_raise: 1, gap_aria: 1, pending: 1 },
        badge: { temp: 1, plain: 1, power: 1 },
        pellets: { today: 1, yesterday: 1, month: 1, total: 1 },
        attention: { title: 1, ack: 1, other: 1, relief: { waits: 1, counting: 1, spent: 1 }, soot: { words: 1, action: 1 }, why: ATTENTION_WHY, rows: ATTENTION_ROWS },
        soot: { title: 1, clean: 1, since: 1, cleaned: 1, button: 1, none: 1 },
    };

    // WHERE EACH ORGAN'S ACTION STANDS ON THE SCHEME: beside the organ and crossing no pipe. One
    // place an organ the automaton commands, named by the key of its row; `schemeGaps` holds this list against the board's.
    const PLACES = {
        auger: { x: 48, y: 138 },
        // FROM THE LEFT, just inside the water's loop: its words centred on the plug ran over the loop's left line;
        // from x 110 the longest, "burning 59 min 59 s", ends short of the right one
        igniter: { x: 110, y: 170, side: "start" },
        // LEFT OF THE PUMP, over its label: the pump stands on the return branch, the right side of the loop, and the room to its right is 21 units - its reason, "from 35 °C", is 45 wide
        pump: { x: 281, y: 158, side: "end" },
        fan: { x: 126, y: 58, side: "start" },
    };

    // THE ACTION OF AN ORGAN, A WORD AND WHAT IS LEFT OR WHAT IT CAME TO, from the VALUES of the automaton's card and
    // nothing else: its rows, found by their keys, say what each organ does, its clocks what is left (the same clocks the card
    // counts down - no second source). The words are WORDS'. None - the organ does nothing now, and nothing is said.
    function onScheme(organ, things, words, settings) {
        const said = (words && words.actions) || {};
        const value = (key) => {
            const one = find(things, AUTOMATON, key);
            return one && one.value ? one.value : null;
        };
        const isOn = (key) => value(key) !== null && value(key).on === true;
        // a clock still running, in words; null when it has run out or is not on the card
        const running = (key) => {
            const v = value(key);
            return v && typeof v.seconds === "number" && v.seconds > 0 ? seconds(v.seconds) : null;
        };
        if (organ === "auger") {
            // a person banned the feed from the main screen: the auger stands whatever the stage, and says why
            if (value("feed") !== null && value("feed").on === false) return { words: said.feed_banned, value: null };
            const cycle = value("auger_switches_in");
            if (cycle && typeof cycle.seconds === "number") return { words: isOn("auger") ? said.feeding : said.pause, value: seconds(cycle.seconds) };
            if (isOn("auger") && running("dose_ends_in")) return { words: said.feeding, value: running("dose_ends_in") };
            return null;
        }
        if (organ === "igniter") {
            // the plug burns only with air: on is burning, counted to its end
            if (!isOn("igniter")) return null;
            return { words: said.burning, value: running("igniter_off_in") };
        }
        // what the fan is ASKED for; what the sensor reads stands on the scheme as a number of its own
        if (organ === "fan") {
            const asked = value("fan");
            if (asked && asked.number) return { words: (said.holds || "").replace("{}", number(asked.number)), value: null };
            const by = asked && FAN_ASKED[asked.word];
            return by ? { words: said[by], value: null } : null;
        }
        // THE PUMP SAYS WHY IT RUNS AND NOT THAT IT RUNS: the pump on the drawing is green while it runs, and "on" beside a green circle says it twice. The reason by the number a person set - "from 35 °C" is the
        // setting `auto_pump_on_c` -, or the overheating, or the boiler giving no number. No reason the page can say, no line: a number made up for it would be a lie on the boiler.
        if (organ === "pump") {
            if (!isOn("pump")) return null;
            const why = value("pump_by");
            const key = why && PUMP_BY[why.word];
            const template = key && ((words && words.why) || {})[key];
            if (!template) return null;
            if (key !== "water") return { words: template, value: null };
            const from = settingNumber(settings, "auto_pump_on_c");
            return from === null ? null : { words: spell(template, { c: degrees(from), unit: root.dictionary.unit("\u00b0C") }), value: null };
        }
        return null;
    }

    // What the fan is asked for, by the word of the automaton's row `fan`, as the key of the word the menu says it by; off is not an action, and a word the
    // page does not know says nothing rather than a guess.
    const FAN_ASKED = { full: "full", by_hand: "by_hand" };

    // A setting of the board as a number by its name: null while it is not said or is no number.
    function settingNumber(settings, name) {
        const one = (settings || []).find((x) => x && x.name === name);
        const n = one ? Number(one.value) : NaN;
        return Number.isFinite(n) ? n : null;
    }

    // Why the pump runs, by the word of the row `pump_by` (automaton::pump_word), as the key of the menu's word. "water" is the boiler's water past the
    // pump's threshold; the word alone read on the drawing as the name of the loop beside it.
    const PUMP_BY = { water: "water", overheated: "overheated", no_number: "no_number" };

    // Whole seconds as a person reads them on the scheme: "7 s", "1 min 23 s".
    function seconds(s) {
        const u = (code) => root.dictionary.unit(code);
        return s < 60 ? `${s} ${u("s")}` : `${Math.floor(s / 60)} ${u("min")} ${s % 60} ${u("s")}`;
    }

    // A number of a value with its unit: degrees to the tenth at any size - a hundred must not change the figure's width
    // -, the rest whole.
    function number(n) {
        const digits = n.unit === "\u00b0C" ? 1 : 0;
        return `${n.value.toFixed(digits)} ${root.dictionary.unit(n.unit)}`;
    }

    // WHAT A THING READS, FOR THE SCHEME, FROM ITS VALUE: a number with its unit, or at the top of its scale "at least"
    // it; a dash when the sensor is there and reads nothing this second, so its place does not go empty;
    // nothing when there is no value at all - the words may carry digits, and they are not read.
    function schemeNumber(thing) {
        const v = thing && thing.value;
        if (!v) return null;
        if (v.number) return number(v.number);
        if (v.at_least) return `\u2265 ${number(v.at_least)}`;
        if (v.unknown) return "-";
        return null;
    }

    // THE DOTS OF THE RELAYS, lit by the relay's value and never by its words: each by the key of its row, with the row's own label
    // - the menu's word for the relay - to name it on the drawing.
    function dots(things) {
        return (things || [])
            .filter((t) => t.group === "relays")
            .map((t) => ({ key: t.key, words: t.name, on: Boolean(t.value && t.value.on === true) }));
    }

    // THE FAN ON THE SCHEME, turning by its card's value - the regulator's mode - and never by its words.
    function fanOnScheme(things) {
        const row = find(things, "fan", "holding");
        const mode = row && row.value && row.value.word;
        return { turning: Boolean(mode) && mode !== "off" };
    }

    // THE WORDS OF AN ACTION ON THE SCHEME: what is happening, what is left of it, and why - joined by what each part
    // of the label is, never by what its text looks like.
    function schemeText(label) {
        const left = label.value ? `${label.words} ${label.value}` : label.words;
        return label.by ? `${left} \u00b7 ${label.by}` : left;
    }

    // WHERE EACH NUMBER STANDS ON THE DRAWING: touching what it is about and crossing nothing - the flue's degrees
    // sit beside the flue's own line, not on it, and the draught's pascals clear of the igniter. The water in the boiler
    // stands at the head of the thermometer put into the boiler from above - a line up from the pot - as it is put in
    // on the boiler itself; the flue's degrees on the far side of the flue, clear of that line; the
    // boiler room clear of the drawing, under the modules' power. A number is named by its PLACE - the one the menu gave its row - and its word is the menu's.
    //
    // A NUMBER STANDS BY AN EDGE THAT DOES NOT MOVE: the draught by its right edge, with the room
    // on its left for "\u2265 300 Pa" - on its left edge it walked right when it passed the top of its scale.
    const NUMBERS = [
        { place: "flue", x: 252, y: 40, side: "start" },
        // THE SUPPLY STANDS ON THE RETURN'S HEIGHT: it stood 8 units higher only because the pump's label crowded under it
        { place: "water_out", x: 109, y: 196, side: "start" },
        { place: "water_back", x: 254, y: 196 },
        { place: "draught", x: 182, y: 74, side: "end" },
        { place: "boiler", x: 194, y: 26, side: "end" },
        { place: "room", x: 44, y: 226 },
    ];

    // the row the menu put at PLACE of the drawing, or null
    function standing(things, place) {
        return (things || []).find((t) => t.place === place) || null;
    }

    // The numbers of the scheme, each where it is measured, from the VALUES of the rows the menu put at their places: only what the board reads now
    // goes on the picture - a socket nobody assigned, or a sensor without a number this second, puts nothing there, because an empty label on a boiler
    // reads as a sensor that has failed. The pressure among them stands whatever the fan is asked for.
    function schemeNumbers(things, words) {
        const named = (words && words.places) || {};
        return NUMBERS.map((one) => ({ ...one, words: named[one.place], value: schemeNumber(standing(things, one.place)) })).filter((one) => one.value);
    }

    // The organs the board says the automaton commands (its card's row `organs`) that have no place on the scheme, in the menu's words - nothing when every one has.
    function schemeGaps(things, words, places = PLACES) {
        const listed = find(things, AUTOMATON, "organs");
        const organs = listed && listed.value && Array.isArray(listed.value.words) ? listed.value.words : [];
        return organs.filter((organ) => !places[organ]).map((organ) => ((words && words.gap) || "").replace("{organ}", organ));
    }

    // THE FEED UNDER THE SCHEME: allowed (true), banned (false) or not said yet (null) by the value of the card's row `feed`; the words on
    // the hopper's button and what a press of it would do are the menu's.
    function feedLine(things, words) {
        const said = (words && words.feed) || {};
        const thing = find(things, AUTOMATON, "feed");
        if (!thing || !thing.value || typeof thing.value.on !== "boolean") return { allowed: null, words: "-", press: said.ban };
        return thing.value.on ? { allowed: true, words: said.pause, press: said.ban } : { allowed: false, words: said.banned, press: said.allow };
    }

    // THE HANDS' BUTTON IN THE SERVICE: the switch `semi` of the automaton's card is on while the hands are taken (the automaton keeps running and lights the fire as before; a part the hand sets stays as set until the automaton
    // decides it again) and off while the automaton holds every part. The button says the press it makes - Take by hand where the switch is off, Give back where it is on - and under it stands what that press does, in the menu's words;
    // a switch the board has not said leaves the button idle with no text. The page names a row by the board's key, never by its label.
    function handsLine(things, words) {
        const said = (words && words.hands) || {};
        const thing = find(things, AUTOMATON, "semi");
        if (!thing || !thing.value || typeof thing.value.on !== "boolean") return { semi: null, button: said.take, text: "" };
        return thing.value.on ? { semi: true, button: said.give, text: said.give_text } : { semi: false, button: said.take, text: said.take_text };
    }

    // A TEXT with its {places} filled from VARS; a place VARS do not name stays as it stands.
    function spell(text, vars) {
        return String(text === undefined ? "" : text).replace(/\{(\w+)\}/g, (all, name) => (vars[name] !== undefined ? String(vars[name]) : all));
    }

    // THE STEP THE LINE SAYS IS THE ONE THE THERMOSTAT ORDERED: the card's `rung` names which of the three the fire is on - big, mid, small -, and HOLD says what each is in kilowatts. A card that says no rung, or none, leaves the line on the chosen step.
    function kwOf(things, hold) {
        const rung = find(things, THERMOSTAT, "rung");
        const word = rung && rung.value && rung.value.word;
        if (word === "mid" && hold.mid) return hold.mid;
        if (word === "small" && hold.small) return hold.small;
        return hold.kw;
    }

    // THE NEXT THRESHOLD OF THE STEP: the water the step burns until - the setpoint TO less the distance the next DIFFERENT step begins at (the middle one at `auto_mid_c`, the small one at `auto_small_c`), counted from the step
    // the thermostat's `rung` says it is on. A middle step that is the big one is passed over: the water at which the fire really changes. None where the distances are not said, the hold has no steps down from this one, or this is the last.
    function nextThreshold(things, hold, stairs, to) {
        if (!stairs) return null;
        const rung = find(things, THERMOSTAT, "rung");
        const word = rung && rung.value && rung.value.word;
        const steps = [{ kw: hold.kw, far: Infinity }, { kw: hold.mid || hold.kw, far: stairs.mid_c }, { kw: hold.small || hold.kw, far: stairs.small_c }];
        const here = word === "small" ? 2 : word === "mid" ? 1 : 0;
        for (let i = here + 1; i < steps.length; i++) if (steps[i].kw !== steps[here].kw) return to - steps[i].far;
        return null;
    }

    // THE DISTANCES THE STEPS OF THE WAY UP BEGIN AT, from the settings by the board's names: the middle step and the small one, in degrees under the setpoint; null while either is not there or is no number.
    function stairsOf(settings) {
        const number = (name) => {
            const one = (settings || []).find((s) => s && s.name === name);
            return one ? Number(one.value) : NaN;
        };
        const [mid_c, small_c] = [number("auto_mid_c"), number("auto_small_c")];
        return [mid_c, small_c].every(Number.isFinite) ? { mid_c, small_c } : null;
    }

    // WHAT THE BURNER HOLDS, from the settings by the board's names - the mode, the setpoint in degrees and the step in kilowatts -, for the line under the drawing; null while any of them is not there or the setpoint is no number.
    function holdOf(settings) {
        const value = (name) => {
            const one = (settings || []).find((s) => s && s.name === name);
            return one ? one.value : null;
        };
        const to = Number(value("auto_hold_c"));
        const mode = value("auto_hold");
        const kw = value("auto_step");
        if (mode === null || kw === null || value("auto_hold_c") === null || !Number.isFinite(to)) return null;
        // the middle and the small step: a row of the table, and `chosen` - or a board that has no such setting - is the chosen step
        const level = (name) => {
            const kept = value(name);
            return kept === null || kept === "chosen" ? kw : kept;
        };
        return { mode, to, kw, mid: level("auto_step_mid"), small: level("auto_step_small") };
    }

    // HOW THE BURNER SMOULDERS, from the settings: how long it lasts, and the dose and the pause of its feed, in seconds; null while any is not there or is no number.
    function smoulderOf(settings) {
        const number = (name) => {
            const one = (settings || []).find((s) => s && s.name === name);
            return one ? Number(one.value) : NaN;
        };
        const [total_s, on_s, off_s] = [number("auto_smoulder_s"), number("auto_smould_on"), number("auto_smould_off")];
        return [total_s, on_s, off_s].every(Number.isFinite) ? { total_s, on_s, off_s } : null;
    }

    // Seconds as minutes and two figures of seconds: 219 is 3:39, 3600 is 60:00.
    function clock(s) {
        return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
    }

    // SMOULDERING IN NUMBERS: the time it has lasted out of the whole of it, the feed - a dose and a pause -, and what is left until it goes out; from the card's `for` - how long the stage has lasted - and the
    // settings it smoulders by, TUNING {total_s, on_s, off_s}. Said in smouldering and not before the board has said how long the stage has lasted.
    function smoulderLine(things, tuning, words) {
        const stage = (find(things, AUTOMATON, "doing") || {}).value;
        const lasted = (find(things, AUTOMATON, "for") || {}).value;
        if (!stage || stage.word !== "smouldering" || !lasted || typeof lasted.seconds !== "number") return "";
        const said = (words && words.heat) || {};
        return spell(said.smoulder, {
            elapsed: clock(lasted.seconds),
            total: clock(tuning.total_s),
            on: seconds(tuning.on_s),
            off: seconds(tuning.off_s),
            left: clock(Math.max(0, tuning.total_s - lasted.seconds)),
        });
    }

    // THE FLAME IN VOLTS TO THE HUNDREDTH: "3.15 V" reads at a glance where
    // "3153 mV" does not. The board speaks millivolts; the page says volts, the same way for the pin and the thresholds.
    function volts(mv) {
        return `${(mv / 1000).toFixed(2)} ${root.dictionary.unit("V")}`;
    }

    // what the flame's pin holds, beside the word the thresholds make of it, so a person sees how far from the next
    // word the fire is. Read off the pin's value - a pin that reads nothing is a dash, and never 0, which would read as
    // the brightest flame there is. WHERE is the row the number is read from, {group, key}, as the menu names it (`placeNumber` of the row in the pot); none - the pin of the flame.
    function flameVolts(things, where) {
        const from = where || { group: "flame", key: "pin" };
        const pin = find(things, from.group, from.key);
        const n = pin && pin.value && pin.value.number;
        return n && typeof n.value === "number" && n.unit === "mV" ? volts(n.value) : "-";
    }

    // THE HYSTERESIS the thermostat lights below, in degrees, from the settings; null while it is not there or is no number.
    function dropOf(settings) {
        const one = (settings || []).find((s) => s && s.name === "auto_drop_c");
        const n = one ? Number(one.value) : NaN;
        return Number.isFinite(n) ? n : null;
    }

    // a number of degrees as a person writes it: whole where it is whole, one decimal otherwise
    function degrees(n) {
        return Number.isInteger(n) ? String(n) : String(Math.round(n * 10) / 10);
    }

    // THE SETPOINT THE THERMOSTAT HOLDS NOW - a profile's, a window's - from the card's `setpoint`, and the setting only while the card says none
    function setpointNow(things, hold) {
        const setpoint = find(things, THERMOSTAT, "setpoint");
        return setpoint && setpoint.value && setpoint.value.number && typeof setpoint.value.number.value === "number" ? setpoint.value.number.value : hold.to;
    }

    // the stages in which the automaton serves a demand for heat: the purge before a light, the lighting, the way up, the work and the smouldering (REF_Device_Page_Look, rule 5)
    const HEATING = ["purge", "kindling", "rising", "working", "smouldering"];

    // THE PLACE OF THE SENSOR THE BURNER HOLDS, by the VALUE of the setting `auto_hold` the board keeps: the boiler's water or the return's. The mode of power holds no sensor, and a board that has not said the setting says none:
    // a number is never taken from the boiler because the water "is the boiler".
    const HELD_PLACE = { boiler: "boiler", return: "water_back" };
    function heldPlace(settings) {
        const one = (settings || []).find((s) => s && s.name === "auto_hold");
        return one && HELD_PLACE[one.value] ? HELD_PLACE[one.value] : null;
    }

    // WHY THE BURNER STANDS OR BURNS, said as a thermostat's screen says it, by the VALUES of the thermostat's switch and lock, the demand's rows and the room's sensor - never by the words a row says:
    //   a burner switched off by Stop says "stop"; one nobody has switched on since the board came up asks for the button;
    //   a switched-on one says who holds the target, the reading and the target - an arrow where heat is served, a slash where it is not -, and the pause that is left before a light;
    //   a reason that has no numbers (the profile off, the schedule, a silent room) stands in the place of the numbers.
    // Heat asked while the automaton stands is `waits`: the burner does not heat, and the arrow would say it does. A board of an older image that has no switch is told by the demand's rows and the stage alone.
    // Said in the menu's `ask` words, the signs among them; a word the menu has not got leaves the line out rather than says it without its words.
    function askOf(things, said, unit, settings) {
        const word = (row) => (row && row.value && typeof row.value.word === "string" ? row.value.word : null);
        const words = said.ask;
        if (!words) return null;
        // a reading of a place as the line writes it: whole where it is whole, a dash where the place has no number - a sensor that is silent, a place nobody holds - and never the number of another place
        const reading = (place) => {
            const n = place === null ? null : numberOf(standing(things, place), unit);
            return n === null ? "-" : degrees(n);
        };
        // A LIGHT'S CYCLE IS THE PERSON'S WORD AND HOLDS OVER THE HOUSE'S: while the thermostat carries the fire to a nominal - the room's target or the water's setpoint - that is what the burner burns for, whatever the switch and the house say
        const kind = word(find(things, THERMOSTAT, "manual"));
        const end = find(things, THERMOSTAT, "manual_c");
        const ends = end && end.value && end.value.number;
        if (words.manual && words.manual[kind] && ends && typeof ends.value === "number") {
            return { word: "manual", text: spell(words.manual[kind], { now: reading(kind === "water" ? heldPlace(settings) : "room"), to: degrees(ends.value), unit }) };
        }
        const switched = word(find(things, THERMOSTAT, "switch"));
        if (switched === "off" || switched === "unasked") return words[switched] ? { word: switched, text: words[switched] } : null;
        const asked = word(find(things, DEMAND, "asked"));
        if (asked !== "cold" && asked !== "warm") return null;
        const lock = find(things, THERMOSTAT, "lock");
        const left = lock && lock.value && typeof lock.value.seconds === "number" && lock.value.seconds > 0 ? lock.value.seconds : 0;
        // what the burner DOES is the verdict, the house's ask is the reason: heat asked says "heating" only where the automaton is on its way up or burning, and "waits" where it stands - in a pause, in a stop, burning off, in a fault -
        // and where the board has not said what the automaton does
        const stage = word(find(things, AUTOMATON, "doing"));
        const verdict = asked === "cold" && !HEATING.includes(stage) ? "waits" : asked;
        if (!words[verdict]) return null;
        const why = word(find(things, DEMAND, "why"));
        const target = find(things, DEMAND, "target");
        const n = target && target.value && target.value.number;
        // THE ROOM'S NUMBERS stand only where the board says a target of the room (the row `target` of the demand is a number: the house asks by a room); a house that asks by the schedule has none, and says its reason in words
        const roomAsks = n && typeof n.value === "number";
        const reason = why && words.why ? words.why[why] : undefined;
        if (!reason && !roomAsks) return null;
        const parts = [spell(reason || words[verdict], { now: reading("room"), to: roomAsks ? degrees(n.value) : "-", unit })];
        // the pause counts down in the page's clock; a stand without a pause that is not heating says so in a word
        if (left > 0 && words.pause) parts.push(spell(words.pause, { left: root.dictionary.shown(left, "clock") }));
        else if (!reason && verdict === "waits" && words.wait) parts.push(words.wait);
        return { word: verdict, text: parts.join(" \u00b7 ") };
    }

    // THE ONE BUTTON THAT RUNS THE BURNER, by the VALUES of the thermostat's switch and of the automaton's stage: `press` is the row it presses - "stop" where the switch is on, "start" where it is off or nobody has pressed
    // since the board came up -, null where the board has not said the switch. A fault is Stop whatever the switch says, because Stop is the one press that acknowledges it. `cannot` is the code of the reason a press of Start
    // would be taken and not carried out - the fire burns out, and a burner in that stage does not start -, null otherwise; the table words it, and the button stands on the page with the reason beside it.
    const BURNING_OUT = ["burning_off", "burning_off_because"];

    function runButton(things) {
        const switched = find(things, THERMOSTAT, "switch");
        const stands = switched && switched.value && typeof switched.value.word === "string" ? switched.value.word : null;
        const stage = (find(things, AUTOMATON, "doing") || {}).value;
        const at = stage && typeof stage.word === "string" ? stage.word : null;
        if (at === "fault") return { press: "stop", cannot: null };
        if (stands === "on") return { press: "stop", cannot: null };
        if (stands === "off" || stands === "unasked") return { press: "start", cannot: BURNING_OUT.includes(at) ? "burning_off" : null };
        return { press: null, cannot: null };
    }

    // WHETHER THE PAGE OFFERS "LIGHT NOW": a switched-on burner that stands in a stop with no fire - in the pause after the fire went out, and where heat is not needed -, and a press of Light lights it at once, past the thermostat.
    // In a fire that burns, burns out or faults the automaton does not hear a Light (a fault answers Stop alone), and the button is not there. Decided by the VALUES of the switch and the stage.
    function lightNow(things) {
        const word = (row) => (row && row.value && typeof row.value.word === "string" ? row.value.word : null);
        return word(find(things, THERMOSTAT, "switch")) === "on" && word(find(things, AUTOMATON, "doing")) === "stop";
    }

    // ONE PRESS AT A TIME (page/press.js holds the lock and its clock; the limit of 8 s and the moment of 600 ms after the board's word are there). WHAT THE BURNER'S PRESSES WAIT FOR is here: the board has said what was
    // pressed when the rows say it, by the VALUES - the switch at the word pressed, the pause gone, the hands taken or given, the feed at the state pressed.
    const pressLock = {
        get LIMIT_MS() { return root.press.LIMIT_MS; },
        get COOL_MS() { return root.press.COOL_MS; },
        take: (kind, want, now, held) => root.press.take(kind, want, now, held),
        held: (lock) => root.press.held(lock),
        phase: (lock) => root.press.phase(lock),
        settled(lock, things) {
            if (!lock) return false;
            if (lock.kind === "run") {
                const row = find(things, THERMOSTAT, "switch");
                return Boolean(row && row.value && row.value.word === lock.want);
            }
            if (lock.kind === "light") return !lightNow(things);
            if (lock.kind === "hands") return handsLine(things, {}).semi === lock.want;
            if (lock.kind === "feed") return feedLine(things, {}).allowed === lock.want;
            return false;
        },
        ended: (lock, things, now, ok) => root.press.ended(lock, pressLock.settled(lock, things), now, ok),
        tick: (lock, things, now, ok) => root.press.tick(lock, pressLock.settled(lock, things), now, ok),
    };

    // THE THERMOSTAT ON THE SCHEME: what it holds stands as one BADGE in the title row of the scheme's card - the thing it holds and to how much -, and its other numbers stand in the drawing by the things they are about. Three things:
    //   BADGE  in the title row of the card, between the word of the card and the buttons of the mode, always: "boiler 44 °C −4", "return 40 °C −2", "power 6 kW" - the mode by its word, the setpoint the thermostat holds now and the distance it lights below; none where the board has not said what it holds. It is not in the SVG: close over the boiler's number it read as the number's caption;
    //   STEP   at the pot, while the fire works: the step the thermostat ordered and, where the next step is another one, the water it burns until - the last step says itself alone;
    //   NOTES  small, at the boiler, only while they act: a window of the schedule that is not the main profile, and the wait before a light. Each is the `short` form the menu gives its row.
    //   ASK    under the title row, above the drawing: why the burner stands or burns - "switched off by Stop · Start turns it on", "no heat needed · room 23.8 °C, target 22 °C · pause 1:58 left", "heating · room 19.6 °C, target 22 °C" -, from the switch and the lock of the thermostat, the demand's rows and the room's sensor; none where nobody asks.
    // What is DECIDED is read off the rows' VALUES - the stage, the word of the source, the seconds of the wait -, never off a word; what is SHOWN is WORDS' `badge`, `hold` and `pot` and the rows' short forms. STAIRS is `stairsOf` of the settings.
    function drawn(things, settings, words, stairs) {
        const said = words || {};
        const hold = holdOf(settings);
        const unit = root.dictionary.unit("\u00b0C");
        const now = hold ? setpointNow(things, hold) : null;
        let badge = null;
        const what = hold && said.hold ? said.hold[hold.mode] : undefined;
        if (what && said.badge) {
            if (hold.mode === "power") badge = spell(said.badge.power, { what, kw: kwOf(things, hold) });
            else {
                const drop = dropOf(settings);
                badge = drop === null ? spell(said.badge.plain, { what, to: degrees(now), unit }) : spell(said.badge.temp, { what, to: degrees(now), unit, drop: degrees(drop) });
            }
        }
        const stage = (find(things, AUTOMATON, "doing") || {}).value;
        let step = "";
        if (hold && said.pot && stage && stage.word === "working") {
            const kw = kwOf(things, hold);
            const next = hold.mode === "power" ? null : nextThreshold(things, hold, stairs, now);
            step = next !== null ? spell(said.pot.step, { kw, next: degrees(next), unit }) : spell(said.pot.step_last, { kw });
        }
        const notes = [];
        const source = find(things, DEMAND, "source");
        if (source && source.value && source.value.word === "window" && source.short) notes.push(source.short);
        const lock = find(things, THERMOSTAT, "lock");
        if (lock && lock.value && typeof lock.value.seconds === "number" && lock.value.seconds > 0 && lock.short) notes.push(lock.short);
        return { badge: badge === null ? null : { mode: hold.mode, text: badge }, step, notes, ask: askOf(things, said, unit, settings) };
    }

    // THE PELLETS AT THE HOPPER: burned today, yesterday, this month and in all, in kilos, from the VALUES of the card `pellets` by their keys - all four to a tenth: the four stand in a column and the card says
    // tenths, and a total of 0.7 kg drawn as "1" beside a "today" of 0.7 reads as a bigger number than the day's. Null where the board has no such card (an older image); a number not read is a dash - a row an older image does not carry too, the place stays -, and a nought is a nought.
    const PELLETS = ["today", "yesterday", "month", "total"];

    function pelletsOnScheme(things) {
        const rows = PELLETS.map((key) => find(things, "pellets", key));
        if (rows.every((one) => one === null)) return null;
        return PELLETS.map((key, at) => {
            const n = rows[at] && rows[at].value && rows[at].value.number;
            const ok = n && typeof n.value === "number";
            return { key, text: !ok ? "-" : n.value.toFixed(1) };
        });
    }

    // THE ZONE OF A MAIN NUMBER: where the value stands on a short scale, and where the tolerance is on it - all as shares of the scale, 0 to 1, the mark clamped to it so it never leaves the bar:
    //   DRAUGHT  the pascals the automaton asks the fan for, give or take the band the fan holds them within (`fan_band`); the scale is three bands each way. A full purge, a hand on the damper or an off fan ask for no number and there is no zone;
    //   WATER    the boiler's water, only while the thermostat holds the boiler: the setpoint held now less the distance it lights below, up to the setpoint; the scale is four distances under and two over;
    //   FLAME    a steady flame - at or under the board's `flame_steady` millivolts, for lower is lighter; the scale is the pin's own, nought to FLAME_TOP.
    // WARN is the mark out of its zone while the fire works - and only then: a dark pot in a stop, a flame still weak as it rises, and the water that passes through its band at every cycle are not deviations, and colour is for deviations (rule 14).
    const FLAME_TOP = 3300;
    const WORKS = ["working", "smouldering"];
    const BANDS_EACH_WAY = 3;

    // WHERE THE BARS STAND on the drawing (viewBox units), each under the number it is about and ending where that number ends or standing in the middle of the pot: the draught's under its name, the water's under the boiler's name, the flame's under its name in the pot.
    const ZONE_PLACES = { draught: { x: 142, y: 90.2, w: 40 }, water: { x: 154, y: 41.3, w: 40 }, flame: { x: 146, y: 149.3, w: 40 } };

    function zone(value, min, max, from, to, warns) {
        const share = (v) => Math.min(1, Math.max(0, (v - min) / (max - min)));
        const inside = value >= from && value <= to;
        return { from: share(from), to: share(to), at: share(value), inside, warn: warns && !inside };
    }

    // A MARK IS COLOURED ONLY AFTER IT HAS STOOD OUT FOR A WHILE: on the boiler's records (02.10, 04.10, 27.09) the draught came back within the fan's band 0 to 5 seconds after the setpoint was changed at work - nine changes, 5 s the longest -, and a colour at the first second out
    // would flash at every step. SINCE is the page's memory, {zone: the millisecond it first stood out}, kept by the caller between its draws; ZONES are `zonesOf`'s; NOW is a millisecond. A zone that stands in, asks for nothing or is gone is forgotten. -> the zones, `warn` gated; the zones handed in are not changed.
    const DWELL_MS = 10000;

    function settle(since, zones, now) {
        const out = {};
        for (const name of Object.keys(zones)) {
            const one = zones[name];
            if (!one || !one.warn) {
                delete since[name];
                out[name] = one;
                continue;
            }
            if (since[name] === undefined) since[name] = now;
            out[name] = { ...one, warn: now - since[name] >= DWELL_MS };
        }
        return out;
    }

    // THE MAINS AT THE POWER'S DOT: the voltage and the power of the socket the burner is plugged into, "227 V \u00b7 4 W", by the VALUES of the rows `voltage` and `power` of the card `mains` and by their units, never by their words.
    // Both or nothing: a socket that is silent, not set or no meter puts nothing on the drawing, because a dash beside the power's dot reads as a power that has failed; the card says why. Nought is a reading.
    function mainsOnScheme(things) {
        const volts = numberOf(find(things, "mains", "voltage"), "V");
        const watts = numberOf(find(things, "mains", "power"), "W");
        if (volts === null || watts === null) return null;
        return `${number({ value: volts, unit: "V" })} \u00b7 ${number({ value: watts, unit: "W" })}`;
    }

    function numberOf(thing, unit) {
        const v = thing && thing.value;
        if (v && v.number && typeof v.number.value === "number" && v.number.unit === unit) return v.number.value;
        if (v && v.at_least && typeof v.at_least.value === "number" && v.at_least.unit === unit) return v.at_least.value;
        return null;
    }

    function zonesOf(things, settings) {
        const stage = (find(things, AUTOMATON, "doing") || {}).value;
        const works = Boolean(stage) && WORKS.includes(stage.word);
        const setting = (name) => {
            const one = (settings || []).find((s) => s && s.name === name);
            const n = one ? Number(one.value) : NaN;
            return Number.isFinite(n) ? n : null;
        };
        const out = { draught: null, water: null, flame: null };

        const pa = numberOf(standing(things, "draught"), "Pa");
        const asked = numberOf(find(things, AUTOMATON, "fan"), "Pa");
        const band = setting("fan_band");
        if (pa !== null && asked !== null && band !== null && band > 0) out.draught = zone(pa, asked - BANDS_EACH_WAY * band, asked + BANDS_EACH_WAY * band, asked - band, asked + band, works);

        const hold = holdOf(settings);
        const water = numberOf(standing(things, "boiler"), "\u00b0C");
        const drop = dropOf(settings);
        if (water !== null && hold && hold.mode === "boiler" && drop !== null && drop > 0) {
            const to = setpointNow(things, hold);
            out.water = zone(water, to - 4 * drop, to + 2 * drop, to - drop, to, false);
        }

        const mv = numberOf(find(things, "flame", "pin"), "mV");
        const steady = setting("flame_steady");
        if (mv !== null && steady !== null) out.flame = zone(mv, 0, FLAME_TOP, 0, steady, works);
        return out;
    }

    // WHAT ASKS FOR A PERSON: the active failures in one list, each with the one thing to do, in the menu's words (WORDS' `attention`).
    //   The automaton's `doing` while it is a failure: a fault, or a burning off that has a reason. The action is the menu's word for the cause's CODE (the part `why` of the row, handed by the dictionary as `parts`), and a fault - which waits for a person - adds the word that
    //   says a press of Stop acknowledges it; a cause the menu has no action for says the menu's general one.
    //   Any row at the level bad that the menu names (ATTENTION_ROWS) with the action it gives it; the key is its card's AND its own, so a same-named row of another card is no such row. A reading over the top of a sensor's scale (`at_least`) is not asked about.
    // Decided from the level the dictionary gave the row and the codes, never from its words. A healthy board has none.
    //   The fault for want of fronts (`no_fronts`) lets go of itself, and says where that stands in the board's own numbers: the seconds it still owes after the pin is back (`relief_in`, whole while the pin is away - `silent_fronts` says it is), or, when the
    //   day's let-goes are spent (`relief_today` of `relief_limit`), that a person is wanted. Without those numbers - an older board, a time of day the board does not believe - it says the general word of its cause and the Stop, as every fault does.
    function reliefAction(things, said) {
        const seconds = (key) => {
            const row = find(things, AUTOMATON, key);
            return row && row.value && typeof row.value.seconds === "number" ? row.value.seconds : null;
        };
        const count = (key) => {
            const row = find(things, AUTOMATON, key);
            return row && row.value && row.value.number && typeof row.value.number.value === "number" ? row.value.number.value : null;
        };
        const left = seconds("relief_in");
        if (left !== null) return spell(seconds("silent_fronts") !== null ? said.waits : said.counting, { n: left });
        const today = count("relief_today");
        const limit = count("relief_limit");
        if (today !== null && limit !== null && today >= limit) return spell(said.spent, { n: today, limit });
        return null;
    }

    // THE TARGET OF THE ROOM SET ON THE SCHEME (9.32) AND ON THE THERMOSTAT'S PAGE (9.35): the panel under the line of the reason. It stands in any stage of the automaton and in a Light's cycle too: the thermostat takes the target again every second and a cycle burns to the one that stands now. One table says which settings it changes and in which case; a next mode of the thermostat is a row of it and no second panel.
    //   TARGETS  who asks (the value of the setting `auto_asked_by`) -> the profile that stands (the value of the demand's row `profile`) -> the setting of its target. A profile with no setting (off) and a giver with no row have no panel.
    // The limits are the settings' own, as the page holds them (`months`); the gap stands under the lowest target by the board's own span. Nothing is guessed: a setting the board has not said leaves the line as it was.
    const TARGETS = { room: { comfort: "auto_room_c", eco: "auto_room_eco_c" } };
    const GAP = "auto_room_gap_c";

    function panelOf(things, settings) {
        const kept = (name) => (settings || []).find((s) => s && s.name === name) || null;
        // a number of a setting the page holds as whole (`months`) or as tenths of its unit (`range`, `tenths`): its value, its span, the step a press makes
        const whole = (name) => {
            const one = kept(name);
            const n = one ? Number(one.value) : NaN;
            const span = one && (one.range || one.months);
            if (!one || !span || !Number.isFinite(n) || one.value === "") return null;
            const tenths = Boolean(one.tenths && one.range);
            return { setting: name, value: n, from: span[0], to: span[1], unit: one.unit || "", step: tenths ? 0.1 : 1, tenths };
        };
        const by = kept("auto_asked_by");
        const table = by ? TARGETS[by.value] : undefined;
        const standing_ = find(things, DEMAND, "profile");
        const profile = standing_ && standing_.value && typeof standing_.value.word === "string" ? standing_.value.word : null;
        if (!table || !profile || !table[profile]) return null;
        const target = whole(table[profile]);
        const gap = whole(GAP);
        if (!target || !gap) return null;
        const room = numberOf(standing(things, "room"), "\u00b0C");
        return { profile, setting: target.setting, value: target.value, from: target.from, to: target.to, unit: target.unit, step: target.step, tenths: target.tenths, gap: { setting: gap.setting, value: gap.value, from: gap.from, to: gap.to, step: gap.step, tenths: gap.tenths }, room, fire: toTenth(target.value - gap.value) };
    }

    // A number to the nearest tenth: the sum of two numbers with a tenth leaves dust in a float (22.1 - 2 is 20.1000000000000014 in some sums), and a number said or sent carries none.
    function toTenth(x) {
        return Math.round(x * 10) / 10;
    }

    // ONE STEP of a number by WAY (1 or -1), or null where the setting's limit is reached: a button at its limit sends nothing. A step is the setting's own (`step`: 0.1 for a target by tenths, 1 where it is not said); it is made in whole
    // tenths so that 22.1 and 0.1 are 22.2 and ten steps from 22 are 23, whatever a float says of the sum.
    function stepped(one, way) {
        const step = one.step || 1;
        const scale = step < 1 ? Math.round(1 / step) : 1;
        const next = (Math.round(one.value * scale) + way * Math.round(step * scale)) / scale;
        return next < one.from || next > one.to ? null : next;
    }

    // A number of the panel as it is said: one digit after the point where the setting is by tenths (22.0, 22.1), as it is where it is whole.
    function valueText(one) {
        return one.tenths ? Number(one.value).toFixed(1) : String(one.value);
    }

    // A button HELD goes on by itself: nothing for HOLD_FIRST_MS, then a step at that moment and one every HOLD_EVERY_MS. The steps a hold of MS has made.
    const HOLD_FIRST_MS = 400;
    const HOLD_EVERY_MS = 100;
    function holdSteps(ms) {
        return ms < HOLD_FIRST_MS ? 0 : 1 + Math.floor((ms - HOLD_FIRST_MS) / HOLD_EVERY_MS);
    }

    // THE PANEL AS THE HAND HOLDS IT: HELD(name) is the number a press has set and the board has not yet been told, undefined when no hand is on it. The fire's line moves with the numbers shown, and a button is off where its setting has no more room.
    function handled(panel, held) {
        const shown = (one) => {
            const hand = held(one.setting);
            const value = typeof hand === "number" ? hand : one.value;
            return { ...one, value, canUp: stepped({ ...one, value }, 1) !== null, canDown: stepped({ ...one, value }, -1) !== null };
        };
        const target = shown(panel);
        const gap = shown(panel.gap);
        return { ...target, gap, fire: toTenth(target.value - gap.value) };
    }

    // THE STRIP of the panel: the band the room falls through before the fire is lit (the fire's line up to the target), the target and the room, each as a share of the scale, and the scale's ticks. The scale is whole degrees, from two under the lower of
    // the fire's line and the room to two over the target, at least eight wide; its ticks stand at every 2 degrees where it is narrow, 5 where it is wider, 10 beyond that.
    function stripOf(p) {
        const lo = Math.floor(Math.min(p.fire, p.room === null || p.room === undefined ? p.fire : p.room) - 2);
        const hi = Math.max(Math.ceil(p.value + 2), lo + 8);
        const span = hi - lo;
        const share = (x) => Math.max(0, Math.min(100, ((x - lo) / span) * 100));
        const every = span <= 12 ? 2 : span <= 30 ? 5 : 10;
        const ticks = [];
        for (let t = Math.ceil(lo / every) * every; t <= hi; t += every) ticks.push({ at: share(t), label: String(t) });
        return { from: share(p.fire), to: share(p.value), target: share(p.value), room: p.room === null || p.room === undefined ? null : share(p.room), ticks };
    }

    // THE HAND ON THE PANEL, SENT ONCE: a press puts the number under the setting's name and restarts the wait; when the hand has been off for WAIT ms every number held goes to the board by SEND(name, value, keep), once.
    // SEND answers whatever it answers, and the hand is let go either way: the panel then shows what the board holds. TIMER is {set, clear} of the clock.
    // THE LAST WRITE WINS. Nothing is asked of the board before a write and nothing is compared with what the page knows of it: what the board holds by then is written over, and the person sees the number on the scheme and presses
    // again ("no check before the write").
    // FLUSH() sends what waits at once, with the request that outlives the page: a page put away. A setting has one write on its way at a time; a press made meanwhile goes right after it has landed.
    function targetSender({ send, wait, timer }) {
        const hand = new Map();     // name -> the number pressed last, until it has gone
        const waits = new Map();    // name -> the timer of its wait: each setting waits its own 2 s
        const flight = new Map();   // name -> whether the next write goes with the request that outlives the page, while one is on its way
        let flying = Promise.resolve();
        const go = (name, keep) => {
            const value = hand.get(name);
            flight.set(name, keep);
            const job = (async () => {
                try { await send(name, value, keep); } finally {
                    const next = flight.get(name);
                    flight.delete(name);
                    if (hand.get(name) === value) hand.delete(name);
                    else if (hand.has(name)) go(name, next);
                }
            })();
            flying = Promise.all([flying, job.catch(() => {})]);
        };
        const fire = (name, keep) => {
            if (waits.has(name)) timer.clear(waits.get(name));
            waits.delete(name);
            if (!hand.has(name)) return;
            if (!flight.has(name)) go(name, keep);
            else if (keep) flight.set(name, true);
        };
        return {
            press(name, value) {
                hand.set(name, value);
                if (waits.has(name)) timer.clear(waits.get(name));
                waits.set(name, timer.set(() => fire(name, false), wait));
            },
            held: (name) => hand.get(name),
            pending: () => hand.size > 0,
            flush: () => { for (const name of [...hand.keys()]) fire(name, true); },
            flushed: () => flying,
        };
    }

    // WHAT THE BOARD SAID OF THE LAST WRITE OF EACH SETTING, while it refused it: a refusal belongs to its setting, whatever the board said of the other settings of the panel. It goes when that setting is written and taken, or
    // when the hand presses that setting again; the press of another setting and the board taking another setting leave it. OF(names) is what is said of the settings a panel shows, one after another.
    function refusals() {
        let said = {};
        const without = (name) => { const next = { ...said }; delete next[name]; return next; };
        return {
            wrote(name, ok, words) { said = ok || !words ? without(name) : { ...said, [name]: words }; },
            pressed(name) { if (name in said) said = without(name); },
            of: (names) => names.map((name) => said[name]).filter(Boolean).join(" "),
        };
    }

    // HOW CLEAN THE BOILER IS: the board sends the median of the flue minus the water over the last burns (`soot/soot_c`, degrees), how many burns it is made of, the kilograms burned since the cleaning and the day of it - numbers only. The percent
    // is the page's: the way from the dirty edge to the clean one, as the two settings a person set put them, held between nought and a hundred. Where an edge is not known the percent is null: nothing is guessed. A board with no soot card has no soot.
    const SOOT = "soot";
    const SOOT_ASKS_BELOW = 25;
    function sootOf(things, settings) {
        if (!find(things, SOOT, "soot_c")) return null;
        const edge = (name) => {
            const one = (settings || []).find((s) => s && s.name === name);
            const n = one && one.value !== "" ? Number(one.value) : NaN;
            return Number.isFinite(n) ? n : null;
        };
        const degrees = numberOf(find(things, SOOT, "soot_c"), "\u00b0C");
        const clean = edge("soot_clean_c");
        const dirty = edge("soot_dirty_c");
        const percent = degrees !== null && clean !== null && dirty !== null && dirty > clean ? Math.max(0, Math.min(100, ((dirty - degrees) / (dirty - clean)) * 100)) : null;
        return {
            degrees,
            percent: percent === null ? null : Math.round(percent),
            raw: percent,
            clean,
            burns: numberOf(find(things, SOOT, "soot_burns"), "burns"),
            kg: numberOf(find(things, SOOT, "kg_since_clean"), "kg"),
            day: numberOf(find(things, SOOT, "cleaned_day"), "day"),
        };
    }

    // The day of a cleaning as day and month, from the days since 1970 the board sends.
    function dayText(days) {
        const d = new Date(days * 864e5);
        const two = (n) => String(n).padStart(2, "0");
        return `${two(d.getUTCDate())}.${two(d.getUTCMonth() + 1)}`;
    }

    function attention(things, words, settings) {
        const said = (words && words.attention) || {};
        const out = [];
        const doingRow = find(things, AUTOMATON, "doing");
        const stage = doingRow && doingRow.value && doingRow.value.word;
        if (stage === "fault" || (stage === "burning_off_because")) {
            const why = doingRow.parts && doingRow.parts.why;
            const action = (said.why && said.why[why]) || said.other || "";
            const relief = stage === "fault" && why === "no_fronts" && said.relief ? reliefAction(things, said.relief) : null;
            out.push({
                key: "automaton/doing",
                words: doingRow.state,
                action: relief !== null ? relief : stage === "fault" && said.ack ? `${action} ${said.ack}` : action,
                level: stage === "fault" ? "bad" : "warn",
            });
        }
        for (const one of things || []) {
            const key = `${one.group}/${one.key}`;
            if (one.level !== "bad" || !said.rows || !(key in said.rows)) continue;
            // a reading over the top of the sensor's scale is a number the sensor cannot say, not a failure: the fan blows full on a purge and the draught reads over, by design
            if (one.value && one.value.at_least) continue;
            out.push({ key, words: `${one.name} ${one.state}`, action: said.rows[key], level: "bad" });
        }
        // the boiler wants cleaning: below a quarter clean, by the number and the edges a person set; the degrees over the clean edge are what the words say
        const soot = said.soot ? sootOf(things, settings) : null;
        if (soot && soot.raw !== null && soot.raw < SOOT_ASKS_BELOW) {
            out.push({ key: "soot/soot_c", words: spell(said.soot.words, { n: soot.percent }), action: said.soot.action, level: "warn" });
        }
        return out;
    }

    root.scheme = { panelOf, stepped, handled, stripOf, targetSender, refusals, runButton, lightNow, pressLock, drawn, pelletsOnScheme, zonesOf, settle, attention, sootOf, dayText, ZONE_PLACES, NUMBERS, PLACES, WORDS_NEEDED, smoulderLine, holdOf, stairsOf, smoulderOf, clock, flameVolts, feedLine, handsLine, schemeNumbers, mainsOnScheme, valueText, holdSteps, HOLD_FIRST_MS, HOLD_EVERY_MS, flameOnScheme, standing, onScheme, schemeGaps, schemeText, dots, schemeNumber, fanOnScheme };
})(typeof globalThis !== "undefined" ? globalThis : this);
