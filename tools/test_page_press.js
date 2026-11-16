// The lock of a press as the engine keeps it, with what settles it handed in: a press is `accepted` until the board has said it, the lock is let go at the limit or after the moment that follows the word, and the end that is
// no success is told. The burner's own rules for what settles a press are tested with the scheme (tools/test_page_scheme.js).
//
//     node --test tools/test_page_press.js
const test = require("node:test");
const assert = require("node:assert");
const { load } = require("./page_sources");

const scope = {};
load(["press.js"], scope);
const P = scope.press;
const T0 = 5000;

test("a press taken is accepted; once the board has said it, it is answered; with no lock there is no phase", () => {
    const lock = P.take("answer", true, T0);
    assert.strictEqual(P.phase(lock), "accepted");
    assert.strictEqual(P.phase(P.tick(lock, false, T0 + 2000)), "accepted");
    assert.strictEqual(P.phase(P.tick(lock, true, T0 + 90)), "answered");
    assert.strictEqual(P.phase(null), null);
});

test("an answered lock stays for the moment after the word and is gone after it; a press taken meanwhile is the first, unchanged", () => {
    const answered = P.tick(P.take("answer", true, T0), true, T0 + 90);
    assert.strictEqual(P.held(P.tick(answered, true, T0 + 90 + P.COOL_MS - 1)), true);
    assert.strictEqual(P.tick(answered, true, T0 + 90 + P.COOL_MS), null);
    assert.strictEqual(P.take("answer", false, T0 + 100, answered), answered);
});

test("the ends of a press: refused at once, silent at the limit, and no complaint where the board said it", () => {
    const lock = P.take("answer", true, T0);
    assert.strictEqual(P.tick(lock, false, T0 + 10, false), null);
    assert.strictEqual(P.ended(lock, false, T0 + 10, false), "refused");
    assert.strictEqual(P.tick(lock, false, T0 + P.LIMIT_MS), null);
    assert.strictEqual(P.ended(lock, false, T0 + P.LIMIT_MS), "silent");
    assert.strictEqual(P.ended(lock, false, T0 + P.LIMIT_MS - 1), null);
    assert.strictEqual(P.ended(lock, true, T0 + P.LIMIT_MS), null);
});


// ---- the press from the hand to the end of its lock: an answer belongs to the press that asked it ----
// A late answer to an old press (the board was silent for longer than the limit, the person pressed again, then the board spoke) must change nothing of the press that stands on the channel now. The clock, the timers and the
// request are handed in, so a press is held on values and on a clock that is moved by hand.

function world() {
    const timers = [];
    const seen = { began: 0, ended: [] };
    let now = 10000;
    const env = {
        locks: {}, watching: {}, rowLock: P.rowLock,
        now: () => now,
        every: (f) => { const h = { f, on: true }; timers.push(h); return h; },
        clear: (h) => { if (h) h.on = false; },
        began: () => { seen.began++; },
        ended: (why) => { seen.ended.push(why); },
    };
    const advance = (ms) => { for (let spent = 0; spent < ms; spent += 100) { now += 100; for (const h of timers.slice()) if (h.on) h.f(); } };
    return { env, seen, advance, now: () => now, timers };
}

function request() {
    const out = { sent: 0 };
    out.promise = new Promise((resolve, reject) => { out.resolve = resolve; out.reject = reject; });
    out.send = () => { out.sent++; return out.promise; };
    return out;
}

const later = () => new Promise((r) => setImmediate(r));
const phase = (w, channel) => P.phase(w.env.locks[channel]);
const press = (w, channel, req) => P.once(w.env, channel, null, "answer", true, () => [], req.send);

// the press A hangs past the limit and is let go by it; the person presses again: B stands on the channel
async function silenceThenAgain() {
    const w = world();
    const a = request(), b = request();
    press(w, "run", a);
    assert.strictEqual(phase(w, "run"), "accepted");
    w.advance(P.LIMIT_MS + 100);
    assert.strictEqual(w.env.locks.run, null, "the limit let A go");
    assert.deepStrictEqual(w.seen.ended, ["silent"]);
    press(w, "run", b);
    assert.strictEqual(b.sent, 1, "B went to the board");
    assert.strictEqual(phase(w, "run"), "accepted");
    return { w, a, b };
}

test("a late `true` of the old press does not confirm the new one: B stays accepted until B's own answer", async () => {
    const { w, a, b } = await silenceThenAgain();
    const mine = w.env.locks.run;
    a.resolve(true);
    await later();
    assert.strictEqual(phase(w, "run"), "accepted", "A's answer is not B's");
    assert.strictEqual(w.env.locks.run, mine, "the lock of B is the lock B took, as it was");
    w.advance(2000);
    assert.strictEqual(phase(w, "run"), "accepted", "and B is still waiting two seconds on");
    b.resolve(true);
    await later();
    assert.strictEqual(phase(w, "run"), "answered", "B's own answer settles B");
    w.advance(P.COOL_MS + 100);
    assert.strictEqual(w.env.locks.run, null);
});

test("a late `false` of the old press does not take the lock of the new one off: no third press goes while B waits", async () => {
    const { w, a, b } = await silenceThenAgain();
    a.resolve(false);
    await later();
    assert.strictEqual(P.held(w.env.locks.run), true, "B is held");
    assert.strictEqual(phase(w, "run"), "accepted");
    const c = request();
    press(w, "run", c);
    assert.strictEqual(c.sent, 0, "a third press is not sent while B waits");
    w.advance(500);
    assert.strictEqual(P.held(w.env.locks.run), true, "B's watch goes on");
    b.resolve(false);
    await later();
    assert.strictEqual(w.env.locks.run, null, "B's own refusal lets B go");
});

test("a late throw of the old press changes nothing of the new one, and is no unhandled rejection", async () => {
    const seenRejections = [];
    const note = (e) => seenRejections.push(e);
    process.on("unhandledRejection", note);
    try {
        const { w, a, b } = await silenceThenAgain();
        a.reject(new Error("the request failed"));
        await later(); await later();
        assert.strictEqual(phase(w, "run"), "accepted");
        assert.strictEqual(P.held(w.env.locks.run), true);
        b.resolve(true);
        await later();
        assert.strictEqual(phase(w, "run"), "answered");
        assert.deepStrictEqual(seenRejections, [], "a request that throws is a refusal of ITS press, said nowhere as an error nobody caught");
    } finally {
        process.off("unhandledRejection", note);
    }
});

test("the watch of the old press does not stop the watch of the new one: the limit still lets B go", async () => {
    const { w, a, b } = await silenceThenAgain();
    a.resolve(false);
    await later();
    w.advance(P.LIMIT_MS + 200);
    assert.strictEqual(w.env.locks.run, null, "B was let go by the limit, which only B's watch can do");
    assert.deepStrictEqual(w.seen.ended, ["silent", "silent"]);
    void b;
});

test("a press whose own request throws is a refusal of it: the lock is let go at once", async () => {
    const w = world();
    const a = request();
    press(w, "run", a);
    a.reject(new Error("the request failed"));
    await later(); await later();
    assert.strictEqual(w.env.locks.run, null);
    assert.deepStrictEqual(w.seen.ended, ["refused"]);
    const thrown = { sent: 0, send: () => { throw new Error("sent badly"); } };
    press(w, "run", thrown);
    await later();
    assert.strictEqual(w.env.locks.run, null, "a send that throws before it has a promise is the same refusal");
});

test("a single press answered `true` is accepted, then answered, then let go after the moment that follows; answered `false` it is let go at once", async () => {
    const w = world();
    const a = request();
    press(w, "run", a);
    assert.strictEqual(phase(w, "run"), "accepted");
    a.resolve(true);
    await later();
    assert.strictEqual(phase(w, "run"), "answered");
    w.advance(P.COOL_MS - 100);
    assert.strictEqual(P.held(w.env.locks.run), true);
    w.advance(200);
    assert.strictEqual(w.env.locks.run, null);
    assert.deepStrictEqual(w.seen.ended, []);
    const b = request();
    press(w, "run", b);
    b.resolve(false);
    await later();
    assert.strictEqual(w.env.locks.run, null);
    assert.deepStrictEqual(w.seen.ended, ["refused"]);
});

test("two channels do not touch each other: the answer of one is nothing to the press standing on the other", async () => {
    const w = world();
    const run = request(), feed = request();
    press(w, "run", run);
    press(w, "feed", feed);
    feed.resolve(true);
    await later();
    assert.strictEqual(phase(w, "feed"), "answered");
    assert.strictEqual(phase(w, "run"), "accepted", "the press of the other channel waits for its own answer");
    run.resolve(false);
    await later();
    assert.strictEqual(w.env.locks.run, null);
    assert.strictEqual(phase(w, "feed"), "answered", "and its refusal is nothing to the first");
});

// ---- the same belonging for what a hand holds: a slider's number is the hand's until ITS request ends, and a newer hand has its own ----

test("a hand that took the handle again while the old write was on its way is not let go by the old write's end", () => {
    const aims = P.latest();
    aims.begin("fan");                      // the first drag
    const first = aims.seen("fan");         // its release sends the write
    aims.begin("fan");                      // the hand takes the handle again before the answer
    assert.strictEqual(aims.is("fan", first), false, "the old write may not let the new hand go");
    const second = aims.seen("fan");
    assert.strictEqual(aims.is("fan", second), true, "the new hand's own write lets it go");
});

test("a hand that took the handle once is let go by its own write, and another handle is nothing to it", () => {
    const aims = P.latest();
    aims.begin("fan");
    const token = aims.seen("fan");
    aims.begin("gate");
    assert.strictEqual(aims.is("fan", token), true);
    assert.strictEqual(aims.is("nothing", undefined), true, "a handle nobody took has no hand to lose");
});


// ---- what cannot be taken back is asked about: the first press only asks, the second sends, and the asking ends by itself ----

const timers = () => {
    const pending = new Map();
    let id = 0;
    return {
        set: (fn, ms) => { pending.set(++id, { fn, ms }); return id; },
        clear: (h) => pending.delete(h),
        fire: (ms) => { for (const [h, t] of [...pending]) if (t.ms <= ms) { pending.delete(h); t.fn(); } },
        count: () => pending.size,
    };
};

test("a press that need not be asked about sends at once and never asks", () => {
    const asker = P.asker(timers(), () => {});
    assert.strictEqual(asker.tap(false), "send");
    assert.strictEqual(asker.sure(), false);
});

test("a press that must be asked about asks first and sends on the second press, and the asking is over after it", () => {
    const seen = [];
    const asker = P.asker(timers(), (sure) => seen.push(sure));
    assert.strictEqual(asker.tap(true), "ask");
    assert.strictEqual(asker.sure(), true);
    assert.strictEqual(asker.tap(true), "send");
    assert.strictEqual(asker.sure(), false);
    assert.deepStrictEqual(seen, [true, false], "the page is told when the asking begins and when it ends");
});

test("the asking ends by itself after the term, so a button left asking is not pressed into the command by a passer-by", () => {
    const clock = timers();
    const asker = P.asker(clock, () => {});
    asker.tap(true);
    clock.fire(P.ASK_MS - 1);
    assert.strictEqual(asker.sure(), true, "not yet");
    clock.fire(P.ASK_MS);
    assert.strictEqual(asker.sure(), false);
    assert.strictEqual(asker.tap(true), "ask", "after it the next press asks again and sends nothing");
});

test("leaving the asking sends nothing and clears its term; the second press after it asks again", () => {
    const clock = timers();
    const asker = P.asker(clock, () => {});
    asker.tap(true);
    asker.cancel();
    assert.strictEqual(asker.sure(), false);
    assert.strictEqual(clock.count(), 0, "no term left to fire later");
    assert.strictEqual(asker.tap(true), "ask");
});

test("the second press clears the term of the first: nothing fires after the command is sent", () => {
    const clock = timers();
    const seen = [];
    const asker = P.asker(clock, (sure) => seen.push(sure));
    asker.tap(true);
    asker.tap(true);
    assert.strictEqual(clock.count(), 0);
    clock.fire(P.ASK_MS * 2);
    assert.deepStrictEqual(seen, [true, false], "the page is told the end of the asking once");
});
