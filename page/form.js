// What a setting looks like and what goes back to the board - the part of the page that is judged by values, not by
// pixels, so it lives apart from the components and is tested on its own (tools/test_page_form.js).
//
// The board sends a setting as words and choices; it does not say how to draw it. The page decides, and these are
// the rules it decides by.
(function (root) {
    "use strict";

    // UP TO THIS MANY, A SWITCH: short words side by side, the one set lit, and a press sets it - no Save:
    // a list of sentences with a Save under it "is not how a page is made". The board holds
    // the same number and refuses a choice of that size worded as a sentence (crates/settings, switch_refusal).
    const SWITCH_UP_TO = 3;

    // ... AND FROM THE NEXT ONE A LIST: a row of four buttons wraps to a second line and reads as scattered buttons, not as one choice (REF_Device_Page_Look,
    // rule 6); seven sockets of seven choices each is the fifty-six lines this stage exists to remove. One border, said once.
    const LIST_FROM = SWITCH_UP_TO + 1;

    // How the setting is drawn. A span of numbers is a span whatever it counts - seconds, months, a plain whole -
    // so the three arrive at one field; the words of the span are the board's note, not a kind of their own.
    function kind(setting) {
        if (setting.seconds || setting.months || setting.whole) return "number";
        if (!setting.options || !setting.options.length) return "text";
        return setting.options.length <= SWITCH_UP_TO ? "switch" : "list";
    }

    // What the choice VALUE of a switch means now, in the board's one line; null when the board sends none.
    function means(setting, value) {
        const o = (setting.options || []).find((o) => o.value === value);
        return (o && o.means) || null;
    }

    // Whether a press on VALUE sends the setting at once: on a switch, unless the choice needs a word written first -
    // that one goes when the word is.
    function atOnce(setting, value) {
        return kind(setting) === "switch" && !takesOf(setting, value);
    }

    // The choice a person should see chosen, and the word beside it. The board keeps a written-in word behind the
    // prefix of its choice ("own:buffer tank", "server:time.google.com"), so the two are told apart here and
    // nowhere else on the page.
    function picked(setting) {
        const value = setting.value || "";
        for (const o of setting.options || []) {
            if (o.takes && value.startsWith(o.value + ":")) {
                return { value: o.value, own: value.slice(o.value.length + 1) };
            }
        }
        return { value, own: "" };
    }

    // What the field beside VALUE asks for, in the board's own words - and null for a choice that asks nothing.
    function takesOf(setting, value) {
        const o = (setting.options || []).find((o) => o.value === value);
        return (o && o.takes) || null;
    }

    // What goes back to the board for this setting. A choice that takes a word goes back behind its prefix: that is
    // how the board keeps it, and settings::check refuses a bare word, because a misspelt choice would otherwise be
    // taken for a name a person meant.
    function sent(setting, value, own) {
        if (picked({ options: setting.options, value }).own !== "") return value;
        const takes = takesOf(setting, value);
        return takes ? value + ":" + String(own || "").trim() : value;
    }

    // The whole tab as one form line. Every name and value is escaped: a word with & or = in it is one value, not
    // two settings.
    function body(pairs) {
        return pairs.map(([name, value]) => encodeURIComponent(name) + "=" + encodeURIComponent(value)).join("&");
    }

    // What a control shows when a person may be holding it: THEIR HAND WHILE THEY HOLD IT, the board's value
    // otherwise. The board is asked every couple of seconds, and a value bound straight to its answer throws a
    // held handle back to the old number under the finger.
    //
    // Written as a comparison with undefined and not as `held || fromBoard`, because 0 is a value a hand can
    // hold - it is where Stop leaves the fan, and dragging up from there would fight the page.
    function handle(held, fromBoard) {
        return held === undefined ? fromBoard : held;
    }

    // What the board holds for SETTING when the person holds HELD - the field they are editing and have not saved -
    // and the two differ: "on the board: 42"; null when they are the same. The field stays the person's; this line is
    // how they learn the board moved under their hand - from the console, another page, another browser.
    function aside(setting, held) {
        const board = picked(setting);
        if (!held || (held.value === board.value && (held.own || "") === board.own)) return null;
        const o = (setting.options || []).find((o) => o.value === board.value);
        const words = o ? o.words + (board.own ? " " + board.own : "") : board.value === "" ? "nothing" : board.value;
        return `on the board: ${words}`;
    }

    // THE LINES OF A LIST OF SETTINGS: each setting a line of its own, except the pairs the menu names - `{of: [first, second], first: label, second: label, range: true}` -, which stand on one line at the place of the first,
    // the second not drawn again. `labels` is what a single setting is called in a list that has pairs, where its own label is longer than the column the pairs keep their first label in. `first` is what the first is called in the pair, where its own label is too long for the line; `range` says the pair makes a range (a setpoint and its hysteresis), and no pair makes one otherwise. A pair whose second is not in the list is no pair: the first stands alone, so a setting is never lost for the menu naming it with another.
    function rows(list, pairs, labels) {
        const out = [];
        const taken = new Set();
        for (const setting of list) {
            if (taken.has(setting.name)) continue;
            const pair = (pairs || []).find((p) => p.of[0] === setting.name && list.some((s) => s.name === p.of[1]));
            if (!pair) {
                out.push({ setting, label: (labels && labels[setting.name]) || null });
                continue;
            }
            taken.add(pair.of[1]);
            out.push({ pair: true, first: setting, second: list.find((s) => s.name === pair.of[1]), label: pair.second, firstLabel: pair.first || null, range: !!pair.range });
        }
        return out;
    }

    // THE RANGE A PAIR OF A SETPOINT AND ITS HYSTERESIS MAKES: from the setpoint less the hysteresis up to the setpoint, in the setpoint's unit - "41…45 °C". Nothing while either is no whole number or the range
    // would go under nought: a half-typed number is not a range to read.
    function pairRange(first, second) {
        const whole = (text) => (/^\d+$/.test(String(text)) ? Number(text) : null);
        const top = whole(first.value);
        const down = whole(second);
        if (top === null || down === null || top - down < 0) return "";
        return `${top - down}…${top}${first.unit ? " " + first.unit : ""}`;
    }

    // A TEXT with its {places} filled from VARS; a place VARS do not name stays as it stands.
    function spell(text, vars) {
        return String(text === undefined ? "" : text).replace(/\{(\w+)\}/g, (all, name) => (vars[name] !== undefined ? String(vars[name]) : all));
    }

    // What a person reads of the value VALUE the board keeps for SETTING: the words of the choice - and the word of its own a choice took, behind them -, or a number with its unit.
    function valueWords(setting, value, ui) {
        if (setting && setting.form === "frame") return windowWords(value, ui);
        const options = (setting && setting.options) || [];
        const own = options.find((o) => o.takes && value.startsWith(o.value + ":"));
        if (own) return own.words + " " + value.slice(own.value.length + 1);
        const chosen = options.find((o) => o.value === value);
        if (chosen) return chosen.words;
        return setting && setting.unit ? `${value} ${setting.unit}` : value;
    }

    // WHAT A PERSON READS AFTER A SAVE, from the board's ANSWER and the page's own words. The board says what changed as the name and the value kept of each setting, and a refusal as a code with the field and the
    // numbers it stands on - never a sentence: the label of a setting, the words of a choice and the phrase of a refusal are UI's (the menu's `ui`), in the language the page speaks, and SETTINGS are the page's own list of
    // settings, which hold the labels and the words. An answer that is a text - what the other acts of the page still give - is said as it came.
    function saveSaid(answer, ui, settings) {
        const named = (name) => (settings || []).find((s) => s && s.name === name);
        const labelOf = (name) => (named(name) && named(name).label) || name;
        if (answer.error === undefined) {
            const lines = answer.changed.map((c) => `${labelOf(c.name)}: ${valueWords(named(c.name), String(c.value), ui)}`);
            return { words: lines.length ? ui.saved + " " + lines.join(". ") : ui.nothingChanged, bad: false };
        }
        if (typeof answer.error === "string") return { words: answer.error, bad: true };
        const why = answer.error;
        const refusals = ui.refusals || {};
        const setting = named(why.name);
        const options = (setting && setting.options) || [];
        const wordsOf = (value) => (options.find((o) => o.value === value) || { words: value }).words;
        const vars = { ...why, label: labelOf(why.name) };
        if (why.choices) vars.choices = why.choices.map(wordsOf).join(", ");
        if (why.role) vars.role = wordsOf(why.role);
        if (why.own) vars.takes = (options.find((o) => o.value === why.own) || {}).takes || "";
        const phrase = refusals[why.code];
        if (phrase === undefined) return { words: spell(refusals.unknown, vars), bad: true };
        const tail = why.own && refusals[why.code + "_own"] ? " " + spell(refusals[why.code + "_own"], vars) : "";
        return { words: spell(phrase, vars) + tail, bad: true };
    }

    // WHAT A PERSON READS OF A PRESS THE BOARD REFUSED: the board says a code and the numbers it stands on - `light_refused_room now=22.3 to=22.0` -, never a sentence, and the phrase is the page's own (the menu's `ui.refusals`, in the
    // language the page speaks), its {places} the numbers. An answer that is a sentence already - what the other presses give - and a code the table has no phrase for are said as they came.
    function pressSaid(error, ui) {
        const text = String(error === undefined ? "" : error);
        const parsed = /^(\w+)((?: \w+=\S+)*)$/.exec(text);
        const phrase = parsed && ((ui && ui.refusals) || {})[parsed[1]];
        if (phrase === undefined || phrase === null || !parsed) return text;
        const vars = {};
        for (const pair of parsed[2].trim().split(" ").filter(Boolean)) {
            const at = pair.indexOf("=");
            vars[pair.slice(0, at)] = pair.slice(at + 1);
        }
        return spell(phrase, vars);
    }

    // THE WINDOW OF THE SCHEDULE AS FIELDS. The board keeps a window as ONE line - `off`, or `set:` and `<days> <from>-<to> <profile>` / `<days> <from>+<N>h/<M>m <profile>` - and reads it with
    // `thermostat::Window::parse`; a person is never shown the line and never types it: the page keeps the window as FIELDS - the days (every day, working days, the weekend, days of one's own), the kind
    // (from-to, or every N hours), the start and the end or the hours between and the minutes each lasts, the profile - and builds the line from them and takes it apart into them again.
    // THE GRAMMAR IS THE BOARD'S, written twice: tools/test_page_window.js holds the two to each other - every window the fields can make is read by the Rust reader of the schedule crate (example `window_json`)
    // as the fields mean, and everything that reader refuses is refused here.
    const WINDOW_DAYS = ["daily", "week", "end", "custom"];
    const WINDOW_PROFILES = ["comfort", "eco", "off", "none"];
    const NAMES = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
    const WEEK = 0b0011111;
    const END = 0b1100000;
    const ALL = 0b1111111;

    // The window a person starts from where none is written: every day, from 22:00 to 06:00, and no profile - which keeps it off - and the hours and minutes an every starts from.
    function windowDefaults() {
        return { days: "daily", own: NAMES.map(() => false), kind: "between", from: "22:00", to: "06:00", every: 3, minutes: 20, profile: "none" };
    }

    // A time written HH:MM and no other way, as the board takes it, in minutes of the day; null where it is none.
    function minuteOf(text) {
        const m = /^(\d\d):(\d\d)$/.exec(String(text));
        if (!m || Number(m[1]) > 23 || Number(m[2]) > 59) return null;
        return Number(m[1]) * 60 + Number(m[2]);
    }

    // A whole number in FROM..TO from a number or the text of one, else null.
    function wholeIn(value, from, to) {
        const text = String(value).trim();
        if (!/^\d+$/.test(text)) return null;
        const n = Number(text);
        return n >= from && n <= to ? n : null;
    }

    // WHICH FIELD OF A WINDOW IS WRONG, by its name - `days`, `from`, `to`, `same`, `hours`, `minutes` -, or null where the fields make a window. A field the other kind keeps is not asked.
    function windowProblem(f) {
        if (f.days === "custom" && !(f.own || []).some(Boolean)) return "days";
        if (minuteOf(f.from) === null) return "from";
        if (f.kind === "every") {
            const hours = wholeIn(f.every, 1, 24);
            if (hours === null) return "hours";
            return wholeIn(f.minutes, 1, hours * 60) === null ? "minutes" : null;
        }
        if (minuteOf(f.to) === null) return "to";
        return minuteOf(f.from) === minuteOf(f.to) ? "same" : null;
    }

    // The days of the fields as the line writes them.
    function daysText(f) {
        if (f.days === "daily") return "daily";
        if (f.days === "week") return "mon-fri";
        if (f.days === "end") return "sat-sun";
        return NAMES.filter((_, i) => f.own[i]).join(",");
    }

    // THE LINE THE FIELDS MAKE - `mon-fri 22:00-06:00 eco` - or null where they make no window.
    function windowText(f) {
        if (windowProblem(f) !== null) return null;
        const span = f.kind === "every" ? `${f.from}+${Number(f.every)}h/${Number(f.minutes)}m` : `${f.from}-${f.to}`;
        return `${daysText(f)} ${span} ${f.profile}`;
    }

    // The bits of the days a line names, as the board reads them: `daily`, names, ranges (a range that runs past Sunday comes round) and lists of them; null where one is no day.
    function daysOf(text) {
        if (text === "daily") return ALL;
        let bits = 0;
        for (const part of text.split(",")) {
            const range = part.split("-");
            if (range.length > 2 || range.some((name) => !NAMES.includes(name))) return null;
            const from = NAMES.indexOf(range[0]);
            const to = NAMES.indexOf(range[range.length - 1]);
            for (let at = from; ; at = (at + 1) % 7) {
                bits |= 1 << at;
                if (at === to) break;
            }
        }
        return bits;
    }

    // THE FIELDS OF A LINE as the board reads it, or null where it is no window: the same words it refuses.
    function windowParse(text) {
        const words = String(text).toLowerCase().trim().split(/\s+/);
        if (words.length !== 3) return null;
        const [daysWord, spanWord, profileWord] = words;
        const mask = daysOf(daysWord);
        if (mask === null || !WINDOW_PROFILES.includes(profileWord)) return null;
        const fields = { ...windowDefaults(), profile: profileWord };
        fields.days = mask === ALL ? "daily" : mask === WEEK ? "week" : mask === END ? "end" : "custom";
        fields.own = NAMES.map((_, i) => fields.days === "custom" && (mask >> i & 1) === 1);
        const every = /^(\d\d:\d\d)\+(\d+)h\/(\d+)m$/.exec(spanWord);
        const between = /^(\d\d:\d\d)-(\d\d:\d\d)$/.exec(spanWord);
        if (every) Object.assign(fields, { kind: "every", from: every[1], every: Number(every[2]), minutes: Number(every[3]) });
        else if (between) Object.assign(fields, { kind: "between", from: between[1], to: between[2] });
        else return null;
        return windowProblem(fields) === null ? fields : null;
    }

    // THE FIELDS OF A SETTING'S VALUE: the window `set:<line>` holds, or the one to start from for `off` and for what no window reads.
    function windowOf(value) {
        const text = String(value || "");
        return (text.startsWith("set:") && windowParse(text.slice(4))) || windowDefaults();
    }

    // WHAT AN EDIT OF THE FIELDS PUTS IN THE PAGE'S HAND, as a choice that takes a word - {value, own}: `set` and the line it makes; `set` and no line where the fields make no window (the page does not send it); and `off`
    // where nothing was written and no profile picked: the window nobody wrote stays what the board kept.
    function windowEdit(f) {
        const text = windowText(f);
        if (text === null) return { value: "set", own: "" };
        return text === windowText(windowDefaults()) ? { value: "off", own: "" } : { value: "set", own: text };
    }

    // WHAT A PERSON READS OF A WINDOW the board keeps: words of the page's `ui.window`, never the line - "working days · 22:00-06:00 · eco".
    // THE STARTS OF AN EVERY IN A DAY, as times, in the order they come from the time it starts at: that time and every N hours after it, up to the day's own time again - the count starts anew every day,
    // so five hours from twenty hundred are 20:00, 01:00, 06:00, 11:00 and 16:00. Nothing for a from-to window or fields that make no window: a person is shown when it will start, never a guess.
    function windowStarts(f) {
        // a window with no profile is off and starts nothing, as the board reads it
        if (f.kind !== "every" || f.profile === "none" || windowProblem(f) !== null) return [];
        const from = minuteOf(f.from);
        const gap = Number(f.every) * 60;
        const out = [];
        for (let behind = 0; behind < 1440; behind += gap) {
            const at = (from + behind) % 1440;
            out.push(`${String(Math.floor(at / 60)).padStart(2, "0")}:${String(at % 60).padStart(2, "0")}`);
        }
        return out;
    }

    function windowWords(value, ui) {
        const words = (ui && ui.window) || {};
        const text = String(value || "");
        const f = text.startsWith("set:") ? windowParse(text.slice(4)) : null;
        if (!f) return words.unused !== undefined ? words.unused : text;
        const days = f.days === "custom" ? NAMES.map((_, i) => (f.own[i] ? (words.day || NAMES)[i] : null)).filter(Boolean).join(", ") : (words.days || {})[f.days];
        const span = f.kind === "every" ? spell(words.every_line, { from: f.from, every: f.every, minutes: f.minutes }) : spell(words.between, { from: f.from, to: f.to });
        return [days, span, (words.profile || {})[f.profile]].join(" · ");
    }

    root.form = { WINDOW_DAYS, WINDOW_PROFILES, windowDefaults, windowOf, windowParse, windowProblem, windowText, windowEdit, windowWords, windowStarts, LIST_FROM, SWITCH_UP_TO, means, atOnce, kind, picked, takesOf, sent, body, handle, aside, saveSaid, pressSaid, rows, pairRange };
})(typeof globalThis !== "undefined" ? globalThis : this);
