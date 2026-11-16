// page/hold.js: a hold as a value - the share of it gone, whether the time is up, and the hand that moves away. Expected from the time and the places given, not from the code.
//
//     node --test ./tools/test_page_hold.js
const test = require("node:test");
const assert = require("node:assert");
const { load } = require("./page_sources");

const scope = {};
load(["hold.js"], scope);
const { hold } = scope;

test("a hold is a third gone after a third of its time, and the whole after all of it, and never more", () => {
    const h = hold.begin(100, 200, 5000);
    assert.strictEqual(hold.share(h, 5000), 0);
    assert.ok(Math.abs(hold.share(h, 5500, 1000) - 0.5) < 1e-9);
    assert.strictEqual(hold.share(h, 6000, 1000), 1);
    assert.strictEqual(hold.share(h, 9000, 1000), 1);
});

test("the time is up at the end of the hold and not a moment before", () => {
    const h = hold.begin(0, 0, 0);
    assert.strictEqual(hold.due(h, 999, 1000), false);
    assert.strictEqual(hold.due(h, 1000, 1000), true);
});

test("a hand that shakes within the slop is still holding, one that goes past it is not, and a hold that ended does not begin again", () => {
    const h = hold.begin(100, 100, 0);
    assert.strictEqual(hold.moved(h, 104, 103).live, true);
    const gone = hold.moved(h, 100, 100 + hold.SLOP + 1);
    assert.strictEqual(gone.live, false);
    assert.strictEqual(hold.moved(gone, 100, 100).live, false, "back at the place, the hold stays over");
    assert.strictEqual(hold.share(gone, 500, 1000), 0);
    assert.strictEqual(hold.due(gone, 5000, 1000), false);
});

test("no hold has no share and is never due", () => {
    assert.strictEqual(hold.share(null, 100), 0);
    assert.strictEqual(hold.due(null, 100), false);
    assert.strictEqual(hold.moved(null, 1, 1), null);
});

test("a page is watched while its frames come close and not once they stop for longer than the gap", () => {
    assert.strictEqual(hold.watched(1000, 1016), true);
    assert.strictEqual(hold.watched(1000, 1000 + hold.GAP), true);
    assert.strictEqual(hold.watched(1000, 1000 + hold.GAP + 1), false);
    assert.strictEqual(hold.watched(null, 1000), false, "no frame yet is no one watching");
});
