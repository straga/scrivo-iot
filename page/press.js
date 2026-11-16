// ONE PRESS AT A TIME, as a value: the lock a press takes on its button, and what it says while it is held. A button that sends an absolute command by the state it sees - Start where the switch is off, Stop where it is on -
// turns into its opposite the moment the board's answer comes in, and a second click on it, arriving after that, sends the opposite and takes the first press back. So a press LOCKS the button, and the lock is let go
// when the board has said what was pressed (`settled`), a moment after it (COOL_MS: the button has just changed its word and nobody has read it, so a click aimed at the old word is not a new press), when the board
// refuses or does not answer (`ok` false), or at LIMIT_MS, whichever comes first.
//
// LIMIT_MS is 8 s: the board answers a command in the same request - tens of milliseconds, a second on a bad network -, and what it did not say there it says in its stream, whose period is 5 s at the slowest the page asks
// for (the Live button); one period and the time of a request and a margin. A press the board took and did not tell in that time is a button locked for 8 s, which is the price of not sending a second command blind.
// COOL_MS is 600 ms: longer than the double click of a hand (under 500 ms), shorter than a person reads the new word of the button and means a second press.
//
// The lock is a record - {kind, want, at, settledAt} - and the clock is handed in, so it is held on values and tested without a timer. WHAT SETTLES A PRESS is not here: it is the device's (scheme.js, pressLock.settled, by
// the kind of the press and the board's rows) and is handed in as a yes or a no, ISSETTLED. The engine's own buttons, which know nothing of the plant, are settled by the board's answer to the request (page.js).
//
//     node --test tools/test_page_press.js
(function (root) {
    "use strict";

    const LIMIT_MS = 8000;
    const COOL_MS = 600;

    // a press takes the lock; a press taken while a lock is held leaves the held one as it was
    function take(kind, want, now, held) {
        return held ? held : { kind, want, at: now, settledAt: null };
    }

    function held(lock) {
        return lock !== null && lock !== undefined;
    }

    // what the button shows of the lock: `accepted` while the board has not said the press - the page has taken it and waits -, `answered` from the moment it has, until the lock goes; none without a lock
    function phase(lock) {
        if (!lock) return null;
        return lock.settledAt === null ? "accepted" : "answered";
    }

    // WHY a lock is let go when that is a thing to say: `refused` where the board refused (OK false), `silent` where the limit came and the board had not said the press; none where it said it or the lock is still held
    function ended(lock, isSettled, now, ok) {
        if (!lock) return null;
        if (ok === false) return "refused";
        if (lock.settledAt === null && now - lock.at >= LIMIT_MS && !isSettled) return "silent";
        return null;
    }

    // the lock after a look at the board at NOW: gone where the board refused (OK false), where the moment after it has said the press has passed, or where the limit is reached; the moment of its saying noted
    function tick(lock, isSettled, now, ok) {
        if (!lock) return null;
        if (ok === false) return null;
        if (now - lock.at >= LIMIT_MS) return null;
        if (lock.settledAt !== null) return now - lock.settledAt >= COOL_MS ? null : lock;
        return isSettled ? { ...lock, settledAt: now } : lock;
    }

    // WHAT SETTLES A PRESS OF THE ENGINE'S OWN BUTTONS (a row of a card): the board's answer to the request, which `once` notes on the lock as `answered`.
    const answered = (lock) => Boolean(lock && lock.answered);
    const rowLock = {
        take: (kind, want, now, held_) => take(kind, want, now, held_),
        held: (lock) => held(lock),
        ended: (lock, _things, now, ok) => ended(lock, answered(lock), now, ok),
        tick: (lock, _things, now, ok) => tick(lock, answered(lock), now, ok),
    };

    // THE PRESS OF A BUTTON, from the hand to the end of the lock (page.js, pressOnce, hands it the page's own clock, timers, lock store and words). ENV is {locks, now, every, clear, began, ended}.
    //
    // AN ANSWER BELONGS TO THE PRESS THAT ASKED IT. The board may be silent for longer than the limit; the lock is let go, the person presses again, and then the old request answers, or fails. What the old press did is the old
    // press's: its answer is read against the lock IT took (`id`, which every lock this press makes by noting its answer or its moment carries on), never against whatever stands on the channel now, and its watch is its own timer.
    // A request that fails - an answer that never comes, a send that throws - is a refusal of ITS press and of no other.
    let counter = 0;
    function once(env, channel, engine, kind, want, things, send) {
        const how = engine || rowLock, locks = env.locks;
        if (how.held(locks[channel])) return;
        if (env.began) env.began();
        const id = ++counter;
        locks[channel] = { ...how.take(kind, want, env.now()), id };
        const mine = () => Boolean(locks[channel]) && locks[channel].id === id;
        let timer = null;
        const look = (ok) => {
            if (!mine()) { env.clear(timer); return; }
            const before = locks[channel];
            const now = env.now();
            const rows = things();
            const why = how.ended(before, rows, now, ok);
            const next = how.tick(before, rows, now, ok);
            locks[channel] = next;
            if (!next) {
                env.clear(timer);
                if (why && env.ended) env.ended(why);
            }
        };
        timer = env.every(() => look(), 100);
        let asked;
        try { asked = Promise.resolve(send()); } catch (e) { asked = Promise.reject(e); }
        asked.then((ok) => {
            if (!mine()) return;
            if (ok === false) return look(false);
            if (ok === true) {
                locks[channel] = { ...locks[channel], answered: true };
                look(true);
            }
        }, () => { if (mine()) look(false); });
    }

    // WHO HOLDS A HANDLE NOW, by name: `begin` is a hand taking it (the number it gets is its token), `seen` the token standing, `is` whether a token is the one standing. A request that ends lets its hand go only when its token is still
    // the one standing - an older request's end does not take the handle from a newer hand.
    function latest() {
        const seq = {};
        return { begin: (key) => (seq[key] = (seq[key] || 0) + 1), seen: (key) => seq[key], is: (key, token) => seq[key] === token };
    }

    // WHAT CANNOT BE TAKEN BACK IS ASKED ABOUT: the first press of a button the board calls irreversible only asks, the second sends; the asking ends by itself after ASK_MS, so a button left asking is not a button a passer-by presses into the command.
    // CLOCK is {set(fn, ms) -> handle, clear(handle)}; CHANGED(sure) tells the page when the asking begins and when it ends. `tap(ask)` says what the press does: "ask" - it only asked - or "send" - the command goes now.
    const ASK_MS = 6000;
    function asker(clock, changed) {
        let sure = false;
        let handle = null;
        const set = (value) => {
            if (sure !== value) {
                sure = value;
                changed(value);
            }
        };
        const clear = () => {
            if (handle !== null) clock.clear(handle);
            handle = null;
        };
        return {
            sure: () => sure,
            tap(ask) {
                clear();
                if (ask && !sure) {
                    set(true);
                    handle = clock.set(() => {
                        handle = null;
                        set(false);
                    }, ASK_MS);
                    return "ask";
                }
                set(false);
                return "send";
            },
            cancel() {
                clear();
                set(false);
            },
        };
    }

    root.press = { LIMIT_MS, COOL_MS, ASK_MS, take, held, phase, ended, tick, rowLock, once, latest, asker };
})(typeof globalThis !== "undefined" ? globalThis : this);
