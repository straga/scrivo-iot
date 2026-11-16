// A hold, as a value: where a hand went down, when, and whether it is still a hold. The button that is pressed by holding it (page.js, HoldButton) keeps one of these and asks it the share
// of the hold gone and whether the time is up; it is apart from the component so that it is tested on its own (tools/test_page_hold.js).
//
// A HOLD ENDS THE MOMENT THE HAND MOVES: a finger that scrolls the page and a mouse that is dragged off are not pressing the button, and the browser takes the scroll away from the page
// by itself (a `pointercancel`), but only after the finger has gone some way; the slop here is the way a hand may shake and still be holding.
//
//     node --test ./tools/test_page_hold.js
(function (root) {
    "use strict";

    // the time a hold lasts, in ms, and the way a hand may move in it, in px
    const MS = 1000;
    const SLOP = 10;

    // a hand went down at X, Y at NOW (ms)
    function begin(x, y, now) {
        return { x, y, at: now, live: true };
    }

    // the hand is at X, Y: still a hold within SLOP of where it went down, and no hold any longer past it
    function moved(hold, x, y, slop = SLOP) {
        if (!hold || !hold.live) return hold;
        return Math.hypot(x - hold.x, y - hold.y) > slop ? { ...hold, live: false } : hold;
    }

    // the share of the hold gone at NOW, from 0 to 1; 0 for no hold
    function share(hold, now, ms = MS) {
        if (!hold || !hold.live) return 0;
        return Math.max(0, Math.min(1, (now - hold.at) / ms));
    }

    // whether the hold has lasted MS at NOW
    function due(hold, now, ms = MS) {
        return Boolean(hold && hold.live && now - hold.at >= ms);
    }

    // the longest a page may go between two of its frames and still have been in front of a hand: past it the page was put away, frozen or starved, and a hold that went on through that is no hold
    const GAP = 300;

    // whether the page was running at NOW, its last frame having been at LAST (ms; null - no frame yet)
    function watched(last, now, gap = GAP) {
        return last !== null && now - last <= gap;
    }

    root.hold = { MS, SLOP, GAP, begin, moved, share, due, watched };
})(typeof globalThis !== "undefined" ? globalThis : this);
