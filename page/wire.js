// What the page knows of the board it speaks to, apart from the components, so that it is tested on its own
// (tools/test_page_burning.js): which board the address names and where its sockets are, the contract the page reads, the pace of
// the stream and of Live, what a thing's button sends, which settings stand with the choices made, and the rows of the card Now.
// Every device's page has these; nothing here names a thing, a card or a setting - the board's dictionary does.
(function (root) {
    "use strict";

    // THE SETTINGS IN FORCE, for the choices the person has picked (PICKED, over what the board holds): which of a section's SETTINGS stand, and what each offers. WHAT MATTERS
    // WITH WHICH CHOICE IS THE BOARD'S WORD, in its dictionary: a choice lists the settings it shows (`shows`) and the settings that must hold a value for it to be offered
    // (`needs`) - the page names no setting. A setting some choice lists is shown while a choice that lists it is the one set; one that none lists is always shown. A choice
    // needing a sensor that no socket plays is not offered, the one the board holds staying on the list so a person sees what is set and not an empty choice. ALL is every
    // setting of the board, where `needs` looks. Where a setting stands is the menu's tree (dictionary.sectionSettings), never its level.
    function inForce(settings, picked, all) {
        const held = new Map((all || settings).map((s) => [s.name, s]));
        const shown = governed(settings, picked);
        const offered = (s) => {
            if (!(s.options || []).some((o) => o.needs)) return s;
            const played = (o) => o.needs.among.some((name) => held.has(name) && held.get(name).value === o.needs.holds);
            return { ...s, options: s.options.filter((o) => !o.needs || o.value === s.value || played(o)) };
        };
        return settings.filter(shown).map(offered);
    }

    // WHETHER A SETTING IS IN FORCE with the choices made (PICKED, over what the board holds): one that some choice of another setting lists
    // is in force while a choice that lists it is the one set - and only then, whichever setting is asked -, one that none lists always is.
    function governed(settings, picked) {
        const now = (s) => {
            const p = (picked || {})[s.name];
            return p && p.value !== undefined ? p.value : s.value;
        };
        const governors = new Map();
        for (const g of settings) {
            for (const o of g.options || []) {
                for (const name of o.shows || []) {
                    if (!governors.has(name)) governors.set(name, new Set());
                    governors.get(name).add(g);
                }
            }
        }
        return (s) => [...(governors.get(s.name) || [])].every((g) => {
            const chosen = g.options.find((o) => o.value === now(g));
            return Boolean(chosen) && (chosen.shows || []).includes(s.name);
        });
    }

    // What a relay's button offers: the other of what its value is.
    function relayButton(thing) {
        return thing && thing.value && thing.value.on === true ? "off" : "on";
    }

    // WHAT A THING'S BUTTON SENDS, AND WHAT IT SAYS: the state it offers - on for a relay that is
    // off, off for one that is on - or a press of a button; never "switch over", which sent twice undoes itself. null
    // for a thing that answers to neither.
    function control(thing) {
        const can = (thing && thing.can) || [];
        // BY ADDRESS, NEVER BY NAME: the name is the page's word for it, the address the board's
        if (!thing || thing.at === undefined || thing.at === null) return null;
        const path = (way) => `/${way}/${thing.at}`;
        // WHAT CANNOT BE TAKEN BACK IS ASKED ABOUT FIRST: the board says it in the dictionary, and the page - not the device - asks the person
        const ask = thing.irreversible ? { ask: true } : {};
        if (can.includes("switch")) {
            const on = Boolean(thing.value && thing.value.on === true);
            return { path: path(on ? "off" : "on"), words: relayButton(thing), ...ask };
        }
        if (can.includes("press")) {
            return { path: path("press"), words: thing.name.charAt(0).toUpperCase() + thing.name.slice(1), ...ask };
        }
        return null;
    }

    // THE CONTRACT THIS PAGE READS (docs/CONTRACT_Device_Api.md). A board that answers another one
    // may carry the same names with other meanings - or, an older one, say nothing of what this page draws from the dictionary: the automaton,
    // its clocks, what a choice bears on -, and a page drawing it by its own rules shows a person what is not there, or hides what is. So it
    // draws nothing and says why, and what to do.
    const CONTRACT = 10;

    // Why the answer SAID is not to be drawn by this page, as a code the page says in its words (the menu's `ui`): {kind, number, mine}; null when it is of this page's contract. `number` is none for a board that names none.
    function foreign(said) {
        const number = said && said.contract;
        if (number === CONTRACT) return null;
        const none = number === undefined || number === null;
        return { kind: none ? "noContract" : "contractOther", number: none ? "" : number, mine: CONTRACT };
    }

    // THE ROWS OF THE CARD NOW, the page's own summary of what the burner is doing: THINGS are the rows of the cards the board names for it, and
    // each says where the board puts it (`now`, its place in the board's list - `machine.now` - which the dictionary makes a row carry), what to
    // call it (`as`, the page's word) and the number that stands beside its word (`alongside`, when the board says one and the row has a
    // number). A row the board does not put in the card is not drawn; a place several rows answer to is drawn once.
    function now(things) {
        return (things || [])
            .filter((t) => t.now !== undefined)
            .sort((a, b) => a.now - b.now)
            // where several rows are the one the board means (two sensors in the same role) the first stands for it
            .filter((t, i, all) => all.findIndex((other) => other.now === t.now) === i)
            .map((t) => ({
                name: t.as !== undefined ? t.as : t.name,
                value: t.alongside !== undefined ? `${t.state} \u00b7 ${t.alongside}` : t.state,
                level: t.level,
            }));
    }

    // WHICH BOARD THIS PAGE SPEAKS TO: the one named by ?board=<host>[:port] in the page's address,
    // so the same page runs on a computer; without it, the board that served the page. SEARCH is location.search.
    // Only a host and a port are taken - a scheme, a path or a user in front of it would send a person's commands
    // somewhere the address does not show.
    const HOST = /^[A-Za-z0-9](?:[A-Za-z0-9.-]*[A-Za-z0-9])?(?::[0-9]{1,5})?$/;

    function boardAt(search) {
        const named = new URLSearchParams(search || "").get("board");
        if (!named) return { base: "", error: null };
        if (!HOST.test(named)) {
            return { base: null, error: { kind: "badBoard", named } };
        }
        return { base: `http://${named}`, error: null };
    }

    // PATH on the board at BASE ("" - the board that served the page).
    function at(base, path) {
        return base + path;
    }

    // The console's socket at PATH on the board at BASE; PAGE is the page's own address, for the board that served it.
    function socketAt(base, path, page) {
        const where = new URL(path, base || page);
        where.protocol = where.protocol === "https:" ? "wss:" : "ws:";
        return where.href;
    }

    // LIVE: a page hears the board at the pace of everyone, and a person who wants every change presses Live. The page asks for it in the request of the stream, and the BOARD ends it after the time its dictionary
    // says - so a forgotten tab does not keep the board at a full run -; the button goes back to the period when it is over. UNTIL is the moment Live ends, in ms; 0 - not pressed.
    function streamPath(until, now) {
        return until > now ? "/events?every=0" : "/events";
    }

    // the period the board sends at when Live is not pressed, in whole seconds, from its dictionary: 0 for a board that says none. The button of Live shows it.
    function livePeriod(dict) {
        return (dict && dict.stream && dict.stream.every) || 0;
    }

    // WHAT THE BUTTON SAYS: the period the board sends at - "5s", the unit being UNIT's, the page's - and, while Live lasts, the word Live and nothing counted: the board ends Live by itself and the
    // button goes back to the period when it does. "" for a board that names no period - there is nothing to put on the button.
    function liveButton(dict, live, word, unit) {
        const every = livePeriod(dict);
        if (!every) return "";
        return live ? word : `${every}${unit}`;
    }

    // how long the board's dictionary says Live lasts, in seconds; 0 for a board that says nothing of it - it has no Live to press
    function liveSeconds(dict) {
        return (dict && dict.stream && dict.stream.live_s) || 0;
    }

    // A TABLE OF THE KIT (the menu, the table of texts): inside the page, where tools/page_kit.py puts it - a data element `kit-<name>` -, else asked for by name
    // with GET, for a page served as separate files. The page is one file on the board, so that opening it costs the board one connection and not six.
    async function kitFile(name, doc, get) {
        const element = doc && doc.getElementById("kit-" + name);
        return element ? JSON.parse(element.textContent) : get(name);
    }

    root.wire = { kitFile, streamPath, livePeriod, liveButton, liveSeconds, now, boardAt, at, socketAt, CONTRACT, foreign, inForce, governed, relayButton, control };
})(typeof globalThis !== "undefined" ? globalThis : this);
