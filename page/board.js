// What the page knows of the board, kept OUTSIDE the components' reactivity and told to them by address.
//
// **WHY IT IS NOT A `useState`.** The board sends `[address, value]` and only what changed. Laying every value in its
// place and then making the whole picture again out of the dictionary, the texts and all the values - new objects for every row - and
// handing it to Owl's reactive state does not work: a component's reactive read keeps what it read until something it read CHANGES, and nothing ever
// wrote a row of an earlier picture again, so each child component keeps every row it has ever read, 62 of them an event, and the tab
// grows ~10 MB a minute until the browser takes it.
//
// **WHAT THIS DOES INSTEAD.** A row is made once, of the addresses it reads, and made again only when one of THEM changes: an event
// with one address changes the rows that read it and leaves every other row the same object. A component says what it read while
// it drew - `board.reader` collects the keys - and is told when one of them changes, and nothing else. What is kept is keys and
// callbacks, never a row.
//
// THE KEYS a component may depend on: an address (a row of a card), `settings` (all of them at once: a setting changes when someone
// saves it, and the components that draw them are drawn again whole), `head` (what the board says of itself: memory, uptime, clock)
// and `shape` (which rows there are at all: a row its texts hide is gone until they stop hiding it).
(function (root) {
    "use strict";

    // What says WHICH row or setting a thing is, not what it is worth now: reading it depends on nothing, so a component that picks its
    // rows by group is not told of every change of every row.
    const ROW_IS = new Set(["at", "group", "key"]);

    // Whether two things of the wire are the same: a number, a switch, a word, a list of words, or an object of one key.
    function same(a, b) {
        if (a === b) return true;
        if (a === null || b === null || typeof a !== "object" || typeof b !== "object") return false;
        if (Array.isArray(a) !== Array.isArray(b)) return false;
        const ka = Object.keys(a);
        return ka.length === Object.keys(b).length && ka.every((k) => same(a[k], b[k]));
    }

    // The values as a row reads them, saying which addresses it did
    function reading(all, seen) {
        return new Proxy(all, {
            get(target, key) {
                if (typeof key === "string" && key !== "length" && /^\d+$/.test(key)) seen.add(Number(key));
                return target[key];
            },
        });
    }

    class Board {
        constructor() {
            this.dict = null;
            this.texts = null;
            this.machine = null;
            this.values = [];
            this.all = [];
            this.rows = [];
            this.settings = [];
            this.head = null;
            this.since = 0;
            // the counts of seconds that go on by themselves, by address: the number the board said last and when, and which way it goes
            this.bases = new Map();
            // what a component is reading now, while it draws: a Set of keys, or null
            this.reader = null;
            this.watched = [];
            this.list = [];
            this.byName = new Map();
            this.readers = new Map();
            this.reads = new Map();
            this.watchers = new Map();
            this.subs = new Map();
        }

        // ---- what is known ----

        // The board's dictionary and the page's texts: every row is made of them and the values held so far.
        load(dict, texts, now = Date.now()) {
            this.dict = dict;
            this.texts = texts;
            // a count that came before the dictionary is counted from the moment the page can say what it is
            this.values.forEach((value, at) => this.count(at, value, now));
            // the automaton the board says it has, or the cycle the menu says a module runs, or none
            this.machine = dict.machine || root.dictionary.cycleOf(texts) || null;
            this.all = root.dictionary.held(dict, this.values);
            this.readers = new Map();
            this.reads = new Map();
            this.rows = new Array((dict.things || []).length).fill(null);
            this.watched = new Array(this.rows.length).fill(null);
            this.byName = new Map((dict.settings || []).map((s, i) => [s.name, i]));
            this.settings = new Array(this.byName.size).fill(null);
            for (let at = 0; at < this.rows.length; at++) this.make(at);
            for (const name of this.byName.keys()) this.make("s:" + name);
            this.make("head");
            this.reshape();
            for (const sub of [...this.subs.keys()]) sub.fn();
        }

        // RECORDS, [[address, value], ...]: each value in its place, the rows that read a changed address made again, the components
        // that read one of those told. Returns the keys that changed. What arrives before the dictionary is kept for it.
        take(records, now = Date.now()) {
            for (const [at, value] of records) this.count(at, value, now);
            return this.apply(records);
        }

        // WHAT A COUNT OF SECONDS IS NOW, counted on by the page from what the board last said of it (the board says it when it opens and when it jumps - a new stage, a new target -, not every second): up gains
        // the seconds that went, down loses them to nothing. The rows that read it are made again, as for an event. A number that is no count, and a count that has lost its number, are not moved by the clock.
        tick(now = Date.now()) {
            const records = [];
            for (const [at, base] of this.bases) {
                const gone = Math.floor((now - base.since) / 1000 + 0.5);
                const counted = base.way === "up" ? base.value + gone : Math.max(0, base.value - gone);
                if (counted !== this.values[at]) records.push([at, counted]);
            }
            return this.apply(records);
        }

        // the count at AT is said to be VALUE at NOW: counted on from here, or no count at all when it has no number now or is none
        count(at, value, now) {
            const thing = this.dict && this.dict.things[at];
            if (thing && thing.counts && typeof value === "number") this.bases.set(at, { value, since: now, way: thing.counts });
            else this.bases.delete(at);
        }

        apply(records) {
            const changed = [];
            for (const [at, value] of records) {
                if (same(this.values[at], value)) continue;
                this.values[at] = value;
                if (!this.dict) continue;
                const t = this.dict.things[at];
                if (t && "fixed" in t) continue;
                this.all[at] = value;
                changed.push(at);
            }
            if (!changed.length) return [];
            const due = new Set();
            for (const at of changed) for (const key of this.readers.get(at) || []) due.add(key);
            const made = [...due].filter((key) => this.make(key));
            const told = new Set(made.map((key) => (typeof key === "string" && key.startsWith("s:") ? "settings" : key)));
            if (this.reshape()) told.add("shape");
            this.tell(told);
            return made.concat(told.has("shape") ? ["shape"] : []);
        }

        // KEY made again of the addresses it reads; whether it changed. A row that comes out the same stays the same object.
        make(key) {
            const seen = new Set();
            const view = reading(this.all, seen);
            const { dict, texts } = this;
            let made;
            if (typeof key === "number") {
                made = root.dictionary.rowOf(dict, texts, view, key);
                this.follow(key, seen);
                if (same(made, this.rows[key])) return false;
                this.rows[key] = made;
                this.watched[key] = made && this.watch(made, ROW_IS, key);
            } else if (key === "head") {
                made = root.dictionary.headOf(dict, texts, view);
                this.follow(key, seen);
                if (same(made, this.head)) return false;
                if (!this.head || made.uptime !== this.head.uptime) this.since = Date.now();
                this.head = made;
            } else {
                const name = key.slice(2);
                const at = this.byName.get(name);
                made = root.dictionary.settingOf(dict, texts, view, dict.settings[at]);
                this.follow(key, seen);
                if (same(made, this.settings[at])) return false;
                this.settings[at] = made;
            }
            return true;
        }

        // KEY reads the addresses SEEN: found again by an event that changes one of them
        follow(key, seen) {
            for (const at of this.reads.get(key) || []) if (!seen.has(at)) this.readers.get(at).delete(key);
            for (const at of seen) {
                if (!this.readers.has(at)) this.readers.set(at, new Set());
                this.readers.get(at).add(key);
            }
            this.reads.set(key, seen);
        }

        // The same thing, saying to the component reading it that it did - a value it reads, not the name it is picked by
        watch(target, is, key) {
            return new Proxy(target, {
                get: (t, k) => {
                    if (this.reader && typeof k === "string" && !is.has(k)) this.reader.add(key);
                    return t[k];
                },
            });
        }

        // The rows there are, in the dictionary's order - each the latest of its address -; whether WHICH rows there are is another
        // answer than it was: a row made again is the same row of the list.
        reshape() {
            const now = this.watched.filter((w) => w !== null);
            const changed = now.length !== this.list.length || now.some((w, i) => w.at !== this.list[i].at);
            this.list = now;
            return changed;
        }

        // ---- what a component reads ----

        // The rows of the cards, drawn ones only, in the dictionary's order. Depends on which rows there are.
        get things() {
            if (this.reader) this.reader.add("shape");
            return this.list;
        }

        // The row at ADDRESS, or null when there is none now.
        row(at) {
            return this.watched[at] || null;
        }

        // The settings, in the dictionary's order. Depends on them: a setting changes when a person saves it or another page does,
        // and the component that draws them is drawn again with the whole list, the ones that came out the same being the same objects.
        get everySetting() {
            if (this.reader) this.reader.add("settings");
            return this.settings;
        }

        // The setting NAME, or null. Depends on the settings.
        setting(name) {
            const at = this.byName.get(name);
            return at === undefined ? null : this.everySetting[at];
        }

        // What the board says of itself. Depends on it.
        get vitals() {
            if (this.reader) this.reader.add("head");
            return this.head;
        }

        // ---- who is told ----

        // SUB - an object with `fn` - is to be called when any of KEYS changes, and of no other: what it watched before is let go
        follows(sub, keys) {
            const now = new Set(keys);
            const was = this.subs.get(sub) || new Set();
            for (const key of was) {
                if (now.has(key)) continue;
                const set = this.watchers.get(key);
                set.delete(sub);
                if (!set.size) this.watchers.delete(key);
            }
            for (const key of now) {
                if (!this.watchers.has(key)) this.watchers.set(key, new Set());
                this.watchers.get(key).add(sub);
            }
            this.subs.set(sub, now);
        }

        // SUB is told of nothing any more
        forgets(sub) {
            this.follows(sub, []);
            this.subs.delete(sub);
        }

        // Every component that reads one of KEYS is called, once
        tell(keys) {
            const due = new Set();
            for (const key of keys) for (const sub of this.watchers.get(key) || []) due.add(sub);
            for (const sub of due) sub.fn();
        }
    }

    root.boards = { Board, same };
})(typeof globalThis !== "undefined" ? globalThis : this);
