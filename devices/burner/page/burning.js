// The burner's automaton, judged by values and not by pixels, so it lives apart from the components and is tested
// on its own (tools/test_page_burning.js): which stage of the strip is lit, and which number of the
// automaton a person can change from the page and which is fixed in the image.
//
// The board sends the automaton's stage as a word (crates/automaton, `stage`), its card's settings, and its fixed
// numbers as rows of the card automaton_fixed; the page decides how they read, and these are the rules.
//
// The part of the page that is the burner's: the strip of stages, the table of what the automaton goes by, the clocks and what
// comes next. What every board's page needs of its board - the address, the stream, the buttons - is the engine's (page/wire.js).
(function (root) {
    "use strict";

    // THE STRIP OF STAGES, read off the row that says the stage: its cells, in the order a fire goes through them, are the page's own table
    // (texts.json, the stage's `strip`), and which stage the board is in and where a fault happened are the board's words (the rows the
    // dictionary's `machine` names as `stage` and `cause`).
    //
    // ONE WORD A CELL, SO THE STRIP IS A LINE AND NOT A BLOCK: a two-word stage breaks in two on a phone and makes every cell twice as tall.
    // The line under the strip names the stage in full.
    //
    // A STAGE THAT IS THE SAME FIRE CARRIED LOWER LIGHTS THE CELL OF THE OTHER (`as`): a cell for it would read as a stage the burner goes
    // through on the way out. A FAULT LIGHTS THE CELL OF THE STAGE IT HAPPENED ON (`cause`), IN THE LEVEL THE TABLE GIVES IT (`marks`): the
    // burner has stopped, and what a person needs from the strip is where. A burning off after a failure is lit as itself, with the cell of
    // the stage it happened on marked beside it. A board that does not say where - after a restart, say - marks nothing rather than a guess.
    function strip(spec, stage, cause) {
        const cells = (spec && spec.cells) || [];
        const as = (spec && spec.as) || {};
        const cell = (code) => as[code] || code;
        const broke = cell(cause);
        const at = ((spec && spec.cause) || []).includes(stage) ? broke : cell(stage);
        const mark = ((spec && spec.marks) || {})[stage] || null;
        return cells.map(([key, words]) => ({ key, words, now: key === at, level: key === broke ? mark : null }));
    }

    // THE TABLE OF WHAT THE AUTOMATON GOES BY, stage by stage, made of the board's word for what stands in each stage (`machine.sections`:
    // names of settings, and of numbers the image fixes) and the page's for how it reads (WORDS, texts.json `sections`: a stage's title, the
    // words of each item, the reading of a number and the lines a stage says besides). SETTINGS are every setting of the board, FIXED the rows
    // of the card of the image's numbers. A setting is changed on this page and is drawn as one; a number of the image only a new image
    // changes. An item some choice of a setting lists stands only while that choice is the one held - the working row of a burner that
    // holds no temperature does not smoulder -, and one the board did not send says so, and not a number made up for it.
    function parameters(machine, words, settings, fixed) {
        const byName = new Map(settings.map((s) => [s.name, s]));
        const kept = new Map(fixed.map((r) => [r.key, r]));
        // whether a setting stands in THIS stage: where the board names the setting that decides the stage's numbers, one its choices list stands
        // while a choice that lists it is the one held
        const stands = (section) => {
            const by = section.by && byName.get(section.by);
            if (!by) return () => true;
            const chosen = (by.options || []).find((o) => o.value === by.value);
            return (s) => !(by.options || []).some((o) => (o.shows || []).includes(s.name)) || Boolean(chosen && (chosen.shows || []).includes(s.name));
        };
        const said = (s, spec) => {
            if (s.value === undefined || s.value === "") return null;
            const chosen = (s.options || []).find((o) => o.value === s.value);
            if (chosen) return chosen.words;
            if (spec) return root.dictionary.shown(Number(s.value), spec);
            return s.unit ? `${s.value} ${s.unit}` : s.value;
        };
        return machine.sections.map((section) => {
            const tx = (words || {})[section.name] || {};
            const parts = [];
            const in_force = stands(section);
            for (const item of section.items) {
                const first = item[0];
                const s = byName.get(first);
                if (s && !in_force(s)) continue;
                const label = ((tx.items || {})[first]) ?? first;
                if (item.length > 1) {
                    // several settings of one kind in a line, "20 · 10 · 10 s"; one the board did not send makes the whole line missing
                    const got = item.map((name) => (byName.has(name) && byName.get(name).value !== undefined && byName.get(name).value !== "" ? byName.get(name).value : null));
                    const unit = s && s.unit ? ` ${s.unit}` : "";
                    parts.push({ name: first, words: label, value: got.includes(null) ? null : `${got.join(" · ")}${unit}`, set: true, missing: got.includes(null) });
                } else if (s) {
                    const value = said(s, (tx.specs || {})[first]);
                    parts.push({ name: first, words: label, value: value === null ? null : value, set: true, missing: value === null });
                } else {
                    // a fixed number is read off its value; words without one are not a number the page can stand behind
                    const n = kept.has(first) && kept.get(first).value && kept.get(first).value.number;
                    const sent = Boolean(n && typeof n.value === "number");
                    parts.push({ name: first, words: label, value: sent ? `${n.value} ${root.dictionary.unit(n.unit)}` : null, set: false, missing: !sent });
                }
            }
            for (const [label, value] of tx.notes || []) parts.push({ words: label, value, set: false, missing: false });
            // THE STAGE IS KNOWN BY ITS NAME - the board's code - and said by its title, which is the page's word
            return { name: section.name, stage: tx.title || section.name, parts };
        });
    }

    // A CLOCK COUNTS DOWN TO SOMETHING, and the board says which of its rows do (`clock` in the dictionary): the time a stage has lasted and
    // the silence of a sensor count up and are none. A clock with nothing to say in a stage carries no seconds - only a dash - so it
    // is known by the board's word and not by its value.
    function isClock(thing) {
        return thing.clock === true;
    }

    // WHAT COMES NEXT, in one row: of the clocks, the nearest one still running, said by its own
    // word and the time left - "igniter-off \u00b7 40 s". The board keeps sending every clock, since a record of a run wants them
    // all; only the page says one. No clock running - a dash, and the row stays.

    function next(things) {
        const running = things
            .filter((t) => isClock(t) && t.value && typeof t.value.seconds === "number" && t.value.seconds > 0)
            .sort((a, b) => a.value.seconds - b.value.seconds);
        if (!running.length) return { value: "-", level: "dim" };
        // ONE WORD AND A TIME (REF_Device_Page_Look, rule 7): the clock's own word, then what is left
        return { value: `${running[0].name} \u00b7 ${running[0].state}`, level: null };
    }

    root.burning = { next, isClock, strip, parameters };
})(typeof globalThis !== "undefined" ? globalThis : this);
