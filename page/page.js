// The board's page as components. Everything drawn comes from the board - its dictionary and its values, kept in board.js - and reaches the
// component that draws it by address; nothing reads the DOM, and nothing here names a thing, a group or a setting - add one on the board and it
// appears by itself (tools/names_in_page.py counts the names of the board written here, and finds none). What only one device has - the drawing
// of its plant, say - is not here: a device's page folder adds it through `page.parts`, a tab or a kind of section.
//
// Two lasting connections and no more: /events carries the state (one per open page), /console carries the terminal
// and is opened only while its tab is shown. The board has room for two of each, and the short requests - /on, /off, /press,
// /settings, /restart - are what the rest of the sockets are kept free for.
//
// **WHAT A CONTROL LOOKS LIKE SAYS WHAT PRESSING IT DOES.** Four kinds stand on these cards, and a person has to
// tell them apart BEFORE pressing, not after - this is a boiler, and On sends current to an auger:
//
//   acts now      the accent's own tint, with the accent's border and ink (`border-acc bg-acc-soft text-acc`) -
//                 On, Off, Stop, restart, Send. The relay clicks while the finger is still on it, so the
//                 control is unmistakably a control - but it does not shout.
//   cannot be undone  filled with the accent, or with `bad` where the act is worse than a relay
//                 (`bg-bad text-acc-ink`) - "go down" and nothing else on this page.
//   confirms      outlined in the accent, no tint (`border-acc bg-panel text-acc`) - Save, and only Save.
//                 It ends an entry; nothing left the page until it was pressed.
//   takes input   sunken into the card (`bg-bg`) and never filled - a field, a list. Changing it does nothing
//                 at all yet, so it must not look like a thing that can be pressed.
//   changes the view  quiet, on the card's own sunken colour (`bg-sunk`) - minute, fit, the theme, the tab,
//                 "recovery image…" which opens a question rather than answering it. None of them reaches the board.
//
// **A SOLID FILL IS THE LOUDEST THING A PAGE HAS, SO IT IS SPENT ON WHAT CANNOT BE TAKEN BACK.** On 22.09 four
// relays stood filled on the first card, and they were the loudest thing on a page whose work is to be READ:
// the draught sensor's fault, the one line asking for a person, was quieter than four switches that were off
// and wanted nothing. Weight follows consequence and how often a thing is looked at, not
// whether it is a button: the page is read every minute and the relays are pressed once a week.
//
// On 21.09 all of these were one beige rounded rectangle. tools/page_controls.py asks the BROWSER what each one
// came out painted and fails when two kinds share a painting - a class missing from the stylesheet leaves the
// markup right and the page wrong, and that has to be caught too.
const { Component, mount, xml, reactive, useState, useRef, useComponent, onWillStart, onWillRender, onRendered, onWillUnmount, onWillDestroy, onWillUpdateProps } = owl;

// WHAT THE BOARD SAID, kept outside the reactive state (board.js): a component that draws by it says so with `useBoard()` and is drawn again
// when what it read changed, and by nothing else. Nothing of the board is put in a `useState`, which would keep every row a
// component ever read.
const board = new boards.Board();

// WHAT A DEVICE ADDS TO THE PAGE, put here by the device's own script (device.js, laid after this file and before main.js): a tab of its own,
// drawn first and opened first, and a part that a section of the menu's tree asks for (registry.js). The page names none of them, and says in words an adding
// that is wrong: a name added twice, a part asked for that nobody added.
const parts = registry.make();

// THE WORDS OF THE INTERFACE: the menu's `ui`, in the language of the page. A component says nothing of its own: a button, a heading, a note of what happened is `ui().<key>`.
const ui = () => dictionary.uiOf(board.texts);
const fill = dictionary.spell;
// an adding of the device's that is wrong, in the menu's words: `wrong` is {kind, key[, section]} from the registry
const problemSaid = (wrong) => fill(ui()[wrong.kind], wrong);

// The board for the component being set up: what it reads while it draws - a row's value, the list of rows, the settings, the
// head - is what it is told of, and let go of at the next drawing when it no longer reads it.
function useBoard() {
    const component = useComponent();
    const reads = new Set();
    const sub = { fn: () => component.render() };
    let outer = null;
    onWillRender(() => {
        outer = board.reader;
        board.reader = reads;
        reads.clear();
    });
    onRendered(() => {
        board.reader = outer;
        board.follows(sub, reads);
    });
    onWillDestroy(() => board.forgets(sub));
    return board;
}

// THE BOARD THIS PAGE SPEAKS TO, chosen once as the page loads: ?board=<host> names it, and without it this is the board
// that served the page. Every request below and the console go there and nowhere else.
const BOARD = wire.boardAt(location.search);
// the language a person chose and the browser kept, if it keeps anything
function storedLang() {
    try {
        return localStorage.getItem("lang");
    } catch (e) {
        return null;
    }
}
const onBoard = (path) => wire.at(BOARD.base, path);

// A row of a card: a dim label at the left, the value at the right edge. The value carries a level, and the level
// is what paints it - ok, bad, warn, dim - so a card never decides on colour by itself.
//
// **A LEVELLED VALUE IS ALSO HEAVIER, AND THAT IS WHAT CARRIES ITS EMPHASIS IN THE LIGHT THEME.** The dark theme
// can make a colour lighter than the label and be done; the light one cannot go darker without turning green
// into navy. So the colour is spent on being recognisable and the weight does
// the emphasis - the same in both themes, so neither leans on something the other has not got. `dim` stays
// light: it is the one level that means "step back".
//
// **THE VALUE STANDS ON ONE LINE AND ITS DIGITS ARE OF ONE WIDTH.** A value allowed to wrap and set in digits of
// different widths changes the height of its row - and of the card - whenever the number changes (a countdown
// moved the card once a second: tools/test_page_row_steady.py). A value that does not fit beside its label goes
// whole to a line of its own under it, never split and never cut; the row is then two lines high and stays so, because the digits
// are of one width. Only a value longer than a line of the card is cut at its edge, as bad data.
class Row extends Component {
    static props = { name: String, value: { type: String, optional: true }, level: { type: [String, Boolean], optional: true }, mono: { type: Boolean, optional: true } };
    static template = xml`
        <div class="flex flex-wrap items-center gap-3 py-1.5 border-b border-line last:border-b-0">
            <span class="text-dim shrink-0 min-w-[4.5rem] first-letter:uppercase" t-esc="props.name"/>
            <span class="flex-auto text-right truncate tabular-nums"
                  t-att-data-level="props.level or false"
                  t-att-class="{
                      'text-bad': props.level === 'bad',
                      'text-warn': props.level === 'warn', 'text-dim': props.level === 'dim',
                      'font-mono text-sm': props.mono }"
                  t-esc="props.value || '-'"/>
            <t t-slot="default"/>
        </div>`;
}

// The signal ladder: four bars, as many lit as the level deserves.
class Lad extends Component {
    static props = { signal: Number };
    static template = xml`
        <span class="inline-flex items-end gap-0.5 h-4 shrink-0">
            <i t-foreach="[25,50,75,100]" t-as="h" t-key="h"
               class="block w-1 rounded-[1px]"
               t-att-class="h_index &lt; bars ? 'bg-acc' : 'bg-line'"
               t-attf-style="height: {{h}}%"/>
        </span>
        <span class="font-mono text-xs text-dim shrink-0" t-esc="props.signal + ' dBm'"/>`;
    get bars() {
        const s = this.props.signal;
        return s >= -55 ? 4 : s >= -65 ? 3 : s >= -75 ? 2 : 1;
    }
}

// THE UNIT BESIDE A NUMBER: every field a person types a number into is followed by this, the one place the page draws a unit. The words are the menu's `units` in the language of the page, laid on the setting by dictionary.js (`unit`);
// a number that has none - an address, a count - draws nothing. `data-unit` is what tools/page_units.py looks for after each numeric field; the hours and minutes of a schedule window are drawn by it too, with the menu's words as the unit.
class Unit extends Component {
    static props = { of: Object };
    static template = xml`<span data-unit="" class="text-dim text-[13px] shrink-0 whitespace-nowrap" t-esc="props.of.unit"/>`;
}

// A PRESS OF A BUTTON THAT COMMANDS THE BOARD IS TAKEN ONCE, AND THE BUTTON SAYS SO. The lock of a press (press.js) is held by CHANNEL here, and every button that sends a command - the burner's own (parts.js) and the engine's rows (Thing) -
// reads its PHASE from it: `accepted` in the frame the press ends, until the board has said it, and its word after that. A press taken while the lock of its channel is held is not sent. ENGINE says what settles the press
// (scheme.js, pressLock, for the burner's buttons); without one the press is settled by the board's answer to the request, which is what the engine's rows wait for. The lock is watched every 100 ms until it is let go,
// and a lock let go by the limit with no word of the board is said on the page (pressHooks.ended), because a button that is free again says nothing of why.
const locks = reactive({});
const pressHooks = { began: null, ended: null };
const pressEnv = {
    locks,
    now: () => Date.now(), every: (f, ms) => setInterval(f, ms), clear: (h) => clearInterval(h),
    began: () => { if (pressHooks.began) pressHooks.began(); },
    ended: (why) => { if (pressHooks.ended) pressHooks.ended(why); },
};
const pressOnce = (channel, engine, kind, want, things, send) => press.once(pressEnv, channel, engine, kind, want, things, send);

// what a button says while its press waits for the board: the long word where the button is wide, the short one where it is not, and the button's own word the rest of the time
function pressWord(phase, own, short) {
    if (phase !== "accepted") return own;
    return short ? ui().pressAcceptedShort : ui().pressAccepted;
}

// A BUTTON THAT IS PRESSED BY HOLDING IT. What sends a command a person cannot take back by a touch - Stop, Start, Light now - is not a button that a thumb passing over it, or a scroll that began on it,
// can press: the hand has to stay on it for MS (hold.js, a second), while the button fills, and the command goes at the end of that, once. A hand let go earlier sends nothing and the button says for a moment what to
// do (HINT).
//
// A HOLD ENDS WHEN WHO HOLDS IS GONE, with no command and no word, and these are all the ways it can be gone:
//   the hand lets go early (`pointerup`, the key's `keyup`)         - with the word of HINT; the rest are silent
//   the hand moves away (a finger that scrolls, a mouse dragged off) - `pointermove` past the slop (hold.js)
//   the browser takes the touch (`pointercancel`), the pointer's capture is lost (`lostpointercapture`)
//   the button loses its focus (`blur`) and the window loses its focus (`blur` of the window)
//   the page is put away: `visibilitychange` to hidden, `pagehide`, `freeze`; and a page that stopped drawing frames for longer than hold.GAP without any of those (a phone that suspends a page
//   with no word, a page starved of its time) - the next frame sees the gap, and the timer that comes after it finds the page unwatched and sends nothing
//   Escape is pressed
//   the host disables the button (the lock of a press, 11.20), the button leaves the page
// A hold belongs to whoever began it - a pointer by its id, a key by its name: a second finger, a second key, the other key's release are not the hold's and change nothing. These listeners of the document and
// the window stand only while a hold goes on.
//
// It is the one look of a button of this kind: the page marks it with ATTRS (the attributes the measures look for), the look of the colours is the host's (LOOK), and what it does at the end is ONHOLD. The same with
// the keyboard: Enter or Space held.
class HoldButton extends Component {
    static props = {
        onHold: Function,
        attrs: { type: Object, optional: true },
        look: { type: String, optional: true },
        disabled: { type: Boolean, optional: true },
        hint: { type: String, optional: true },
        ms: { type: Number, optional: true },
        slots: Object,
    };
    static template = xml`
        <button type="button" t-att="props.attrs" t-att-class="'relative overflow-hidden select-none touch-manipulation ' + (props.look || '')" t-att-disabled="props.disabled"
                t-on-pointerdown="down" t-on-pointermove="move" t-on-pointerup="up" t-on-pointercancel="cancel" t-on-lostpointercapture="lost"
                t-on-keydown="key" t-on-keyup="keyup" t-on-blur="gone" t-on-contextmenu.prevent="">
            <span data-hold-fill="" class="absolute inset-y-0 left-0 pointer-events-none" t-att-style="'width: ' + (state.share * 100) + '%; background: var(--acc); opacity: .28'"/>
            <span t-if="state.early" data-hold-hint="" class="relative" t-esc="props.hint"/>
            <span t-else="" class="relative"><t t-slot="default"/></span>
        </button>`;

    setup() {
        this.state = useState({ share: 0, early: false });
        this.held = null;
        this.owner = null;
        this.frame = 0;
        this.seen = null;
        this.timer = 0;
        this.shown = 0;
        // the exits of the page and the window, the same functions to add and to take away
        this.gone = () => this.stop();
        this.hidden = () => { if (document.hidden) this.stop(); };
        this.escape = (e) => { if (e.key === "Escape") this.stop(); };
        onWillUpdateProps((next) => { if (next.disabled) this.stop(); });
        onWillUnmount(() => { this.stop(); clearTimeout(this.shown); });
    }

    get ms() { return this.props.ms || hold.MS; }

    // the listeners of the exits that are not the button's: while a hold goes on and not otherwise
    watch(on) {
        const how = on ? "addEventListener" : "removeEventListener";
        document[how]("visibilitychange", this.hidden);
        document[how]("freeze", this.gone);
        document[how]("keydown", this.escape);
        window[how]("pagehide", this.gone);
        window[how]("blur", this.gone);
    }

    // a hand went down at X, Y - WHO is a pointer's id or a key's name; a hold that goes on keeps its owner (the keyboard: nowhere, and so it cannot move away)
    begin(who, x, y) {
        if (this.props.disabled || this.held) return;
        clearTimeout(this.shown);
        this.state.early = false;
        this.owner = who;
        this.held = hold.begin(x, y, performance.now());
        this.seen = performance.now();
        this.watch(true);
        this.timer = setTimeout(() => this.fire(), this.ms);
        const tick = () => {
            const now = performance.now();
            // no frame for longer than the gap: the page was not in front of anyone
            if (!hold.watched(this.seen, now)) return this.stop();
            this.seen = now;
            this.state.share = hold.share(this.held, now, this.ms);
            if (this.held && this.held.live) this.frame = requestAnimationFrame(tick);
        };
        this.frame = requestAnimationFrame(tick);
    }

    // the time is up with the hand still on, and the page in front of it: the command goes, once, and the button is empty again
    fire() {
        if (!this.held || !this.held.live) return;
        const watched = !document.hidden && hold.watched(this.seen, performance.now());
        this.stop();
        if (watched) this.props.onHold();
    }

    // the hold is over without a command; EARLY says the hand was let go and not taken away
    stop(early = false) {
        const was = this.held;
        clearTimeout(this.timer);
        cancelAnimationFrame(this.frame);
        this.watch(false);
        this.held = null;
        this.owner = null;
        this.seen = null;
        this.state.share = 0;
        if (early && was && was.live && this.props.hint) {
            this.state.early = true;
            this.shown = setTimeout(() => { this.state.early = false; }, 1500);
        }
    }

    down(e) {
        if (e.button !== 0 || this.held) return;
        this.begin(e.pointerId, e.clientX, e.clientY);
        try { e.currentTarget.setPointerCapture(e.pointerId); } catch (_) { /* a pointer that is gone */ }
    }
    move(e) {
        if (!this.held || e.pointerId !== this.owner) return;
        this.held = hold.moved(this.held, e.clientX, e.clientY);
        if (!this.held.live) this.stop();
    }
    up(e) { if (e.pointerId === this.owner) this.stop(true); }
    cancel(e) { if (e.pointerId === this.owner) this.stop(); }
    lost(e) { if (e.pointerId === this.owner) this.stop(); }
    key(e) {
        if (e.key !== "Enter" && e.key !== " ") return;
        e.preventDefault();
        if (!e.repeat) this.begin("key " + e.key, 0, 0);
    }
    keyup(e) { if (this.owner === "key " + e.key) this.stop(true); }
}

// THE CONTROL OF A SETTING: the one place the page draws a number a person types or a choice from a list, for a setting that stands alone and for either field of a pair. SIZE says where it stands:
// `zone` - the column every single row keeps its field in, a choice as wide as the column and a number a part of it, so the fields of a card stand in one vertical; `first` and `second` - the two fields of a pair, narrower, because four words and two
// numbers have to fit one line of a phone.
class Field extends Component {
    get ui() { return ui(); }
    static props = { setting: Object, picked: Object, edit: Function, size: String };
    static template = xml`
        <select t-if="how === 'list'" t-att-data-name="props.setting.name" t-att-aria-label="props.setting.label"
                class="bg-bg border border-line rounded-[10px] px-2 py-1.5 text-sm min-h-[44px] min-w-0" t-att-class="props.size === 'zone' ? 'w-full' : 'w-[120px] shrink-0'"
                t-on-change="(ev) => this.pick(ev.target.value)">
            <option t-foreach="props.setting.options" t-as="o" t-key="o.value"
                    t-att-value="o.value" t-att-selected="o.value === props.picked.value" t-esc="o.words"/>
            <option t-if="unknown" t-att-value="props.picked.value" selected="selected"
                    t-esc="props.picked.value + ' - ' + ui.notAChoice"/>
        </select>

        <!-- A NUMBER IS TYPED, NOT STEPPED: the phone's digit keyboard and no
             spinner arrows, too small to hit. Every key goes into what is picked at once, so /state arriving
             every second redraws the field with what the person is typing and not with the board's old value;
             the board checks the bounds on Save and says why it refuses. -->
        <input t-else="" type="text" t-att-inputmode="props.setting.tenths ? 'decimal' : 'numeric'" autocomplete="off" spellcheck="false" t-att-data-name="props.setting.name" t-att-aria-label="props.setting.label"
               class="bg-bg border border-line rounded-[10px] py-1.5 text-right min-h-[44px] shrink-0" style="font-size: 16px"
               t-att-class="{'w-16 px-2': props.size === 'zone', 'w-9 px-1': props.size === 'first', 'w-8 px-1': props.size === 'second'}"
               t-att-value="props.picked.value"
               t-on-input="(ev) => this.pick(ev.target.value.trim())"/>`;

    get how() { return form.kind(this.props.setting); }
    // A value the board keeps but no longer offers - an open page against a board that was updated under it. Shown
    // as itself rather than dropped, so a person sees what is set instead of an empty field.
    get unknown() {
        const options = this.props.setting.options || [];
        return options.length >= form.LIST_FROM && !options.some(o => o.value === this.props.picked.value);
    }
    pick(value) { this.props.edit(this.props.setting.name, value, this.props.picked.own); }
}

// One setting: what it is, what a person picks, what it means now. HOW it is drawn is form.kind's answer - up to
// three choices are a switch, a person reads those at a glance; four or more become a list, because a row of four wraps to a second line and seven
// sockets of seven choices each were fifty-six lines of radio buttons, which is the thing this stage removes. The field for a word
// of one's own belongs to the choice that takes it and shows only while that choice is picked.
//
// **THE ROW IS ONE GRID: the label on the left, the field in the column of 112 px on the right, the unit in the column's own end**, so the fields of a card begin at one left edge, whatever
// the unit after them is called. A list or a number is a `Field`.
class Setting extends Component {
    get ui() { return ui(); }
    static components = { Unit, Field };
    static props = { setting: Object, picked: Object, edit: Function, send: Function, label: { type: [String, Boolean], optional: true }, narrow: { type: Boolean, optional: true }, zone: { type: Number, optional: true } };
    static template = xml`
        <div class="py-1.5 border-b border-line last:border-b-0" data-row="">
            <div class="flex items-center min-h-[44px] text-[13px]" t-att-class="columned ? 'gap-[3px]' : 'gap-1.5'">
                <span class="text-dim" t-att-class="columned ? 'w-[70px] shrink-0' : 'flex-1 min-w-0'" t-esc="props.label or props.setting.label"/>

                <div t-if="how === 'list' or how === 'number'" class="shrink-0 flex items-center gap-1.5" t-att-style="columned ? '' : 'width: ' + (props.zone || 112) + 'px'">
                    <Field setting="props.setting" picked="props.picked" edit="props.edit" size="columned ? 'first' : 'zone'"/>
                    <Unit t-if="how === 'number' and props.setting.unit" of="props.setting"/>
                </div>

                <input t-elif="how === 'text'" type="text" autocomplete="off" spellcheck="false"
                       class="bg-bg border border-line rounded-[10px] px-2 py-1.5 text-sm max-w-[60%] min-h-[44px]"
                       t-att-value="props.picked.value"
                       t-on-change="(ev) => this.pick(ev.target.value)"/>
            </div>

            <!-- A SWITCH: the choices side by side in a line, the one set in the colour of what acts now, and a press
                 sets it on the board at once - no Save (form.atOnce). Under it one line: what the
                 set choice means now, in the board's words (form.means). -->
            <div t-if="how === 'switch'" class="flex gap-1 mt-1.5">
                <button t-foreach="props.setting.options" t-as="o" t-key="o.value"
                        class="flex-1 min-w-0 text-sm px-2 py-1.5 rounded-[10px] border min-h-[44px] whitespace-nowrap"
                        t-att-class="o.value === props.picked.value ? 'border-acc bg-acc-soft text-acc font-semibold' : 'border-line bg-sunk text-dim'"
                        t-on-click="() => this.choose(o.value)" t-esc="o.words"/>
            </div>
            <div t-if="how === 'switch' and meaning" class="text-[13px] text-ink mt-1" t-esc="meaning"/>

            <input t-if="takes" type="text" autocomplete="off" autocapitalize="off" spellcheck="false"
                   class="w-full bg-bg border border-line rounded-[10px] px-2 py-1.5 text-sm mt-1.5 min-h-[44px]"
                   t-att-placeholder="takes" t-att-value="props.picked.own"
                   t-on-input="(ev) => this.props.edit(props.setting.name, props.picked.value, ev.target.value)"/>

            <div class="text-[13px] text-dim mt-1" t-if="props.setting.note" t-esc="props.setting.note"/>
            <div class="text-[13px] text-ink mt-1" t-if="props.setting.says" t-esc="props.setting.says"/>
            <div class="text-[13px] text-warn mt-1" t-if="aside" t-esc="aside"/>
        </div>`;

    get how() { return form.kind(this.props.setting); }
    // a field in the column of the first fields (the list has pairs); a switch has no field of the grid, and its label keeps the row
    get columned() { return !!this.props.narrow && (this.how === 'list' || this.how === 'number'); }
    get span() { return this.props.setting.seconds || this.props.setting.months || this.props.setting.whole || null; }
    get takes() { return form.takesOf(this.props.setting, this.props.picked.value); }
    // what the board holds when the person's unsaved hand holds otherwise (form.aside)
    get aside() { return form.aside(this.props.setting, this.props.picked); }
    pick(value) { this.props.edit(this.props.setting.name, value, this.props.picked.own); }
    // what the choice that is set means now (form.means)
    get meaning() { return form.means(this.props.setting, this.props.picked.value); }
    // a press on a switch: picked, and sent at once unless it needs a word written first (form.atOnce)
    choose(value) {
        this.pick(value);
        if (form.atOnce(this.props.setting, value)) this.props.send(this.props.setting);
    }
}

// The settings of one place - a card's, or the whole tab's - with the one Save that sends them, and what the board
// answered standing under it until the next save.
class Settings extends Component {
    get ui() { return ui(); }
    static components = { Setting, Unit, Field };
    static props = { list: Array, pairs: { type: Array, optional: true }, labels: { type: Object, optional: true }, edits: Object, edit: Function, save: Function, said: { type: Object, optional: true } };
    static template = xml`
        <div>
            <t t-foreach="rows" t-as="line" t-key="line.pair ? line.first.name : line.setting.name">
                <Setting t-if="!line.pair" setting="line.setting" picked="pickedOf(line.setting)" edit="props.edit" label="line.label" narrow="paired" zone="zone"
                         send="(one) => props.save([one])"/>
                <!-- A PAIR ON ONE LINE: the first's label, its field and unit, the second's label, its field and unit, and at the end the range they make where the menu says they make one.
                     The row is as high and as large in type as a single one, and nothing in it wraps: four words and two fields are one line of a phone or the pair is not a pair. -->
                <div t-else="" class="py-1.5 border-b border-line last:border-b-0" data-row="" t-att-data-pair="line.first.name" t-att-data-range="line.range ? '' : false">
                    <div class="flex items-center gap-[3px] text-[13px] min-h-[44px] whitespace-nowrap">
                        <span class="text-dim w-[70px] shrink-0" t-esc="line.firstLabel or line.first.label"/>
                        <Field setting="line.first" picked="pickedOf(line.first)" edit="props.edit" size="'first'"/>
                        <Unit t-if="line.first.unit and isNumber(line.first)" of="line.first"/>
                        <span class="text-dim" t-esc="line.label"/>
                        <Field setting="line.second" picked="pickedOf(line.second)" edit="props.edit" size="'second'"/>
                        <Unit t-if="line.second.unit and isNumber(line.second)" of="line.second"/>
                        <span t-if="line.range" class="shrink-0 ml-auto text-right text-ink tabular-nums whitespace-nowrap" t-esc="range(line)"/>
                    </div>
                </div>
            </t>
            <button class="mt-2.5 text-sm px-3.5 py-1.5 rounded-[10px] border border-acc bg-panel text-acc font-semibold active:bg-acc-soft min-h-[44px]"
                    t-on-click="() => this.props.save(props.list)" t-esc="ui.save"/>
            <p t-if="props.said" class="text-[13px] mt-1.5"
               t-att-class="props.said.bad ? 'text-bad' : 'text-dim'" t-esc="props.said.words"/>
        </div>`;

    // the lines of the list: a setting each, and the pairs the menu names on one line (form.rows)
    get rows() { return form.rows(this.props.list, this.props.pairs, this.props.labels); }
    // a list that has a pair keeps the first label of every row in one column, so the first fields stand in one vertical
    get paired() { return this.rows.some((line) => line.pair); }
    // the column the fields of the single rows stand in: a number of 64 px, a gap and the longest unit of the list (8 px a letter), never under 112 px - so the fields of a card begin at one left edge whatever their units are called
    get zone() {
        const longest = Math.max(0, ...this.rows.filter((line) => !line.pair && form.kind(line.setting) === 'number' && line.setting.unit).map((line) => String(line.setting.unit).length));
        return Math.max(112, 64 + 6 + longest * 8 + 4);
    }
    // the range a pair makes with what stands in its fields now (form.pairRange)
    range(line) { return form.pairRange({ ...line.first, value: this.pickedOf(line.first).value }, this.pickedOf(line.second).value); }
    // what a person typed and has not saved, or what the board holds
    pickedOf(setting) { return this.props.edits[setting.name] || form.picked(setting); }
    // a number is typed and has a unit after it; a choice is not, and its words are the unit
    isNumber(setting) { return form.kind(setting) === 'number'; }
}


// THE WINDOWS OF THE SCHEDULE: a form a window, and in it the fields - the days, the kind, the start and the end or the hours and the minutes, the profile - and one Save under them all. The board keeps a window as one line and the
// page builds it from the fields and takes it apart again (form.windowEdit, form.windowParse); the line is never drawn. The fields a person is editing are kept here, not in what the board holds: a window half made - no day picked, the same time
// twice - has no line to keep, and an edit that has none is not sent.
class Windows extends Component {
    get ui() { return ui(); }
    static components = { Unit };
    static props = { list: Array, edits: Object, edit: Function, save: Function, said: { type: Object, optional: true } };
    static template = xml`
        <div>
            <div t-foreach="props.list" t-as="s" t-key="s.name" t-att-data-window="s.name" class="py-2.5 border-b border-line last:border-b-0">
                <t t-set="f" t-value="fieldsOf(s)"/>
                <div class="flex items-center gap-2">
                    <span class="text-dim flex-1" t-esc="s.label"/>
                    <select class="bg-bg border border-line rounded-[10px] px-2 py-1.5 text-sm min-h-[44px]" style="font-size: 16px" t-att-data-field="'profile'"
                            t-att-aria-label="w.profile_is" t-on-change="(ev) => this.set(s, { profile: ev.target.value })">
                        <option t-foreach="profiles" t-as="p" t-key="p" t-att-value="p" t-att-selected="p === f.profile" t-esc="w.profile[p]"/>
                    </select>
                </div>
                <div class="flex flex-wrap items-center gap-1.5 mt-1.5">
                    <select class="bg-bg border border-line rounded-[10px] px-2 py-1.5 text-sm min-h-[44px]" style="font-size: 16px" t-att-data-field="'days'"
                            t-on-change="(ev) => this.pickDays(s, ev.target.value)">
                        <option t-foreach="dayChoices" t-as="d" t-key="d" t-att-value="d" t-att-selected="d === f.days" t-esc="w.days[d]"/>
                    </select>
                    <select class="bg-bg border border-line rounded-[10px] px-2 py-1.5 text-sm min-h-[44px]" style="font-size: 16px" t-att-data-field="'kind'"
                            t-on-change="(ev) => this.set(s, { kind: ev.target.value })">
                        <option value="between" t-att-selected="f.kind === 'between'" t-esc="w.kind.between"/>
                        <option value="every" t-att-selected="f.kind === 'every'" t-esc="w.kind.every"/>
                    </select>
                </div>
                <div t-if="f.days === 'custom'" class="flex gap-1 mt-1.5" t-att-data-field="'custom'">
                    <button t-foreach="week" t-as="i" t-key="i"
                            class="flex-1 min-w-0 text-sm px-1 py-1.5 rounded-[10px] border min-h-[44px]"
                            t-att-class="f.own[i] ? 'border-acc bg-acc-soft text-acc font-semibold' : 'border-line bg-sunk text-dim'"
                            t-att-aria-pressed="f.own[i] ? 'true' : 'false'"
                            t-on-click="() => this.toggle(s, i)" t-esc="w.day[i]"/>
                </div>
                <div t-if="f.kind === 'between'" class="flex flex-wrap items-center gap-1.5 mt-1.5">
                    <span class="text-dim text-sm" t-esc="w.from"/>
                    <input type="time" step="60" class="bg-bg border border-line rounded-[10px] px-2 py-1.5 min-h-[44px]" style="font-size: 16px" t-att-data-field="'from'"
                           t-att-value="f.from" t-on-input="(ev) => this.set(s, { from: ev.target.value })"/>
                    <span class="text-dim text-sm" t-esc="w.to"/>
                    <input type="time" step="60" class="bg-bg border border-line rounded-[10px] px-2 py-1.5 min-h-[44px]" style="font-size: 16px" t-att-data-field="'to'"
                           t-att-value="f.to" t-on-input="(ev) => this.set(s, { to: ev.target.value })"/>
                </div>
                <div t-else="" class="flex flex-wrap items-center gap-1.5 mt-1.5">
                    <span class="text-dim text-sm" t-esc="w.start"/>
                    <input type="time" step="60" class="bg-bg border border-line rounded-[10px] px-2 py-1.5 min-h-[44px]" style="font-size: 16px" t-att-data-field="'from'"
                           t-att-value="f.from" t-on-input="(ev) => this.set(s, { from: ev.target.value })"/>
                    <span class="text-dim text-sm" t-esc="w.every"/>
                    <input type="text" inputmode="numeric" autocomplete="off" spellcheck="false" class="bg-bg border border-line rounded-[10px] px-1.5 py-1.5 w-12 text-right min-h-[44px]" style="font-size: 16px"
                           t-att-data-field="'every'" t-att-value="f.every" t-on-input="(ev) => this.set(s, { every: ev.target.value.trim() })"/>
                    <Unit of="{ unit: w.hours }"/>
                    <span class="text-dim text-sm" t-esc="w.minutes"/>
                    <input type="text" inputmode="numeric" autocomplete="off" spellcheck="false" class="bg-bg border border-line rounded-[10px] px-1.5 py-1.5 w-14 text-right min-h-[44px]" style="font-size: 16px"
                           t-att-data-field="'minutes'" t-att-value="f.minutes" t-on-input="(ev) => this.set(s, { minutes: ev.target.value.trim() })"/>
                    <Unit of="{ unit: w.min }"/>
                </div>
                <div t-if="problem(f)" class="text-[13px] text-bad mt-1" t-esc="w.bad[problem(f)]"/>
                <div t-if="startsOf(f)" class="text-[13px] text-dim mt-1" t-att-data-field="'starts'" t-esc="startsOf(f)"/>
            </div>
            <button class="mt-2.5 text-sm px-3.5 py-1.5 rounded-[10px] border border-acc bg-panel text-acc font-semibold active:bg-acc-soft min-h-[44px]"
                    t-on-click="() => this.save()" t-esc="ui.save"/>
            <p t-if="props.said" class="text-[13px] mt-1.5"
               t-att-class="props.said.bad ? 'text-bad' : 'text-dim'" t-esc="props.said.words"/>
        </div>`;

    setup() {
        // the fields of each window as the person has them now, by the name of its setting
        this.local = {};
    }

    get w() { return ui().window || {}; }
    get profiles() { return form.WINDOW_PROFILES; }
    get dayChoices() { return form.WINDOW_DAYS; }
    get week() { return [0, 1, 2, 3, 4, 5, 6]; }
    problem(f) { return form.windowProblem(f); }
    // WHEN AN EVERY WILL START, in the day: the list of times (form.windowStarts), said once under the fields; nothing for a from-to window or for fields that make no window
    startsOf(f) {
        const list = form.windowStarts(f).join(", ");
        return list ? dictionary.spell(this.w.starts, { list }) : "";
    }

    // what the person's hand holds for the window, or what the board keeps
    pickedOf(setting) { return this.props.edits[setting.name] || form.picked(setting); }

    // THE FIELDS OF A WINDOW: the ones the person is editing while what they made is what the hand holds, else the fields of what the hand - or the board - holds now
    fieldsOf(setting) {
        const held = this.pickedOf(setting);
        const mine = this.local[setting.name];
        const edit = mine && form.windowEdit(mine);
        if (edit && edit.value === held.value && edit.own === (held.own || "")) return mine;
        const fields = form.windowOf(held.value === "set" ? "set:" + held.own : "off");
        this.local[setting.name] = fields;
        return fields;
    }

    set(setting, change) {
        const fields = { ...this.fieldsOf(setting), ...change };
        this.local[setting.name] = fields;
        const edit = form.windowEdit(fields);
        this.props.edit(setting.name, edit.value, edit.own);
    }

    // The days of one's own start from the days the choice before it made: a person turns a day off and on, and does not start from nothing.
    pickDays(setting, days) {
        const f = this.fieldsOf(setting);
        const was = { daily: [0, 1, 2, 3, 4, 5, 6], week: [0, 1, 2, 3, 4], end: [5, 6], custom: null }[f.days];
        this.set(setting, days === "custom" && was ? { days, own: this.week.map((i) => was.includes(i)) } : { days });
    }

    toggle(setting, day) {
        const own = [...this.fieldsOf(setting).own];
        own[day] = !own[day];
        this.set(setting, { own });
    }

    // A WINDOW HALF MADE HAS NO LINE: nothing is sent while one of the windows the person touched is wrong - the line under its fields says which field.
    save() {
        const wrong = this.props.list.some((s) => this.props.edits[s.name] && form.windowProblem(this.fieldsOf(s)) !== null);
        if (!wrong) this.props.save(this.props.list);
    }
}

// THE POWER STEPS: a row a level of power, and in it the three numbers the burner burns on - the dose, the pause, the draught -, typed like any number of the page and sent together by one Save. The row the burner burns on is
// in the colour of what acts now (REF_Device_Page_Look, rule 1); it is the row the BOARD holds, and it changes when the level is changed in the section Heat, never while a number here is being typed.
class Steps extends Component {
    get ui() { return ui(); }
    static components = { Unit };
    static props = { table: Object, edits: Object, edit: Function, save: Function, said: { type: Object, optional: true } };
    static template = xml`
        <div>
            <div class="grid grid-cols-[3.5rem_1fr_1fr_1fr] gap-x-2 items-end pb-1 text-[13px] text-dim">
                <span/>
                <span t-foreach="props.table.columns" t-as="column" t-key="column.key" class="text-right" t-esc="column.title"/>
            </div>
            <div t-foreach="props.table.rows" t-as="row" t-key="row.step" t-att-data-step="row.step" t-att-aria-current="row.chosen ? 'true' : null"
                 class="grid grid-cols-[3.5rem_1fr_1fr_1fr] gap-x-2 items-center py-1.5 border-b border-line last:border-b-0"
                 t-att-class="row.chosen ? 'bg-acc-soft' : ''">
                <span t-att-class="row.chosen ? 'text-acc font-semibold' : 'text-ink'" t-esc="row.title"/>
                <t t-foreach="row.cells" t-as="cell" t-key="cell_index">
                    <div t-if="cell" class="flex items-center gap-1 min-w-0">
                        <input type="text" inputmode="numeric" autocomplete="off" spellcheck="false" t-att-data-name="cell.name" t-att-aria-label="cell.label"
                               class="flex-1 min-w-0 w-full bg-bg border border-line rounded-[10px] px-2 py-1.5 text-right min-h-[44px]" style="font-size: 16px"
                               t-att-value="pickedOf(cell).value"
                               t-on-input="(ev) => this.props.edit(cell.name, ev.target.value.trim(), '')"/>
                        <Unit t-if="cell.unit" of="cell"/>
                    </div>
                    <span t-else="" class="text-right text-dim">-</span>
                </t>
            </div>
            <button class="mt-2.5 text-sm px-3.5 py-1.5 rounded-[10px] border border-acc bg-panel text-acc font-semibold active:bg-acc-soft min-h-[44px]"
                    t-on-click="() => this.props.save(cells)" t-esc="ui.save"/>
            <p t-if="props.said" class="text-[13px] mt-1.5"
               t-att-class="props.said.bad ? 'text-bad' : 'text-dim'" t-esc="props.said.words"/>
        </div>`;

    // every number of the table the board has, row by row: what one Save sends
    get cells() { return this.props.table.rows.flatMap((row) => row.cells).filter(Boolean); }
    // what a person typed and has not saved, or what the board holds
    pickedOf(setting) { return this.props.edits[setting.name] || form.picked(setting); }
}

// The curve of one value: the points a module keeps in its ring, asked for ONE at a time and only while the page
// is open. They are far too many for /state, which every open page receives on every change - a minute of the
// draught is six hundred numbers, and /state is sent a couple of times a second.
//
// THE PICTURE IS SVG AND NOT A LIBRARY. A chart library is tens of kilobytes over a link that serves a board with
// 120 KB of heap; a polyline in a viewBox of 100 by 30 is four lines of arithmetic and scales itself.
class Curve extends Component {
    get ui() { return ui(); }
    static props = { curves: Array };
    static template = xml`
        <div class="basis-full">
            <div class="flex gap-1.5 mb-1">
                <button t-if="props.curves.length > 1" t-foreach="props.curves" t-as="one" t-key="one.name"
                        class="text-xs px-2 py-0.5 rounded-lg border min-h-[28px]"
                        t-att-class="one.name === state.which ? 'border-acc bg-acc-soft text-acc' : 'border-line bg-sunk text-dim'"
                        t-on-click="() => this.pick(one.name)" t-esc="one.words"/>
                <!-- WHAT THE PICTURE IS DRAWN AGAINST, and it is one button rather than a pair: a phone has room
                     for the span buttons and this one, not for two more. THE BUTTON SAYS WHAT PRESSING IT
                     GIVES - "fit" to let the curve fill the picture, "0-300 Pa" to put it back against what the
                     value can be - because a button that names the state it is already in reads as a label. -->
                <button t-if="full" class="text-xs px-2 py-0.5 rounded-lg border border-line bg-sunk text-dim min-h-[28px] ml-auto"
                        t-on-click="() => this.flip()" t-esc="state.fitted ? whole : ui.fit"/>
            </div>
            <div class="relative">
                <!-- THE FLOOR OF THE PICTURE IS FENCED, not merely tinted. A fill a few per cent off the card
                     is what the dark theme can afford and the light one cannot: there the edge of the graph
                     could not be found at all. An edge costs one class and works in both themes. -->
                <svg viewBox="0 0 100 30" preserveAspectRatio="none"
                     class="block w-full h-[72px] bg-sunk rounded-[10px] border border-line">
                    <line t-foreach="marks" t-as="mark" t-key="mark.at" x1="0" x2="100"
                          t-att-y1="mark.at" t-att-y2="mark.at" stroke="currentColor" stroke-width="0.3"
                          class="text-line" vector-effect="non-scaling-stroke"/>
                    <!-- AS MANY LINES AS THE BOARD SENT, and the first one is the one the picture is about: it
                         is drawn boldest and the numbers down the side are its. A regulator sends three - what
                         was asked, what came out, and the angle - and without the third "the regulator is
                         twitching" and "the air is breathing" look the same. -->
                    <t t-foreach="drawnLines" t-as="one" t-key="one_index">
                        <polyline t-foreach="one.runs" t-as="run" t-key="one_index + ':' + run_index"
                                  t-att-points="run" fill="none" stroke="currentColor"
                                  t-att-stroke-width="one.width" t-att-class="one.paint"
                                  t-att-stroke-dasharray="one.dashed" t-att-stroke-linecap="one.cap" vector-effect="non-scaling-stroke"/>
                    </t>
                </svg>
                <!-- THE NUMBERS ARE HTML AND NOT SVG TEXT. The viewBox is stretched to the width of the card
                     (preserveAspectRatio=none), and text inside it stretches with it - the letters would come
                     out wide and flat. -->
                <div t-foreach="marks" t-as="mark" t-key="'n' + mark.at"
                     class="absolute left-1 text-[10px] text-dim leading-none pointer-events-none"
                     t-attf-style="top: calc({{mark.at * 100 / 30}}% - 5px)" t-esc="mark.words"/>
            </div>
            <!-- WHICH LINE IS WHICH, and it is not decoration on a picture of three: a person reading it has to
                 know which one is the setpoint. Each key is painted as its line is, and carries a sample of its
                 look - drawn by the same dash - because a colour alone is what the light theme fades. -->
            <div t-if="drawnLines.length > 1" class="flex flex-wrap gap-x-3 gap-y-0.5 text-[11px] mt-0.5">
                <span t-foreach="drawnLines" t-as="one" t-key="'k' + one_index" t-att-class="one.paint">
                    <svg viewBox="0 0 22 6" width="22" height="6" class="inline-block align-middle mr-1" aria-hidden="true">
                        <line x1="1" x2="21" y1="3" y2="3" stroke="currentColor" t-att-stroke-width="one.width"
                              t-att-stroke-dasharray="one.dashed" t-att-stroke-linecap="one.cap"/>
                    </svg><t t-esc="one.words"/><t t-esc="one.shape"/>
                </span>
            </div>
            <div class="flex justify-between text-xs text-dim mt-0.5">
                <span t-esc="span === null ? ui.nothingYet : span"/>
                <span t-esc="pace"/>
            </div>
        </div>`;

    setup() {
        // fitted: drawn to its own data, where the SHAPE of the noise is readable. The other one is drawn to
        // what the value can be at all, where eight pascals are a ripple and a fan spinning up is an arc - and
        // two curves taken hours apart can be compared by eye.
        this.state = useState({ which: this.props.curves[0].name, drawn: null, fitted: false });
        this.ask();
        // A curve of a hundred points a second would be a request a second for a picture nobody watches that
        // closely. Two seconds is what a person reading a page notices, and it is one request per open curve.
        this.timer = setInterval(() => this.ask(), 2000);
        onWillUnmount(() => clearInterval(this.timer));
    }

    pick(which) {
        this.state.which = which;
        this.state.drawn = null;
        this.ask();
    }

    flip() {
        this.state.fitted = !this.state.fitted;
    }

    // The lines the board sent, in its order.
    get all() {
        const drawn = this.state.drawn;
        return (drawn && drawn.lines) || [];
    }

    // The line the picture is ABOUT: the first one the board sent that has any number in it. The board puts it
    // first on purpose - for the draught that is the pascals and not the millivolts - and the fallback is for a
    // reading that meant nothing for the whole span: a sensor gone, a divider nobody filled in.
    get main() {
        return this.all.find((one) => (one.points || []).some((v) => Number.isFinite(v))) || this.all[0] || null;
    }

    // The fixed span of that series, when it has one. Without it there is nothing to flip to, and the button
    // does not appear at all.
    get full() {
        return this.main && this.main.span ? this.main.span : null;
    }

    get whole() {
        const full = this.full;
        return full ? full[0] + '-' + full[1] + ' ' + dictionary.unit(this.main.unit) : '';
    }

    // From and to, for both series: the fixed span when it is in force, else what the points came to.
    //
    // LINES OF ONE UNIT THAT IS NOT THE PICTURE'S SHARE ONE SCALE: the regulator's P and I are
    // both percent of angle a second, and drawn each against its own a small half looked as big as a large one.
    bounds(series) {
        if (!series) return null;
        const main = this.main;
        const kin = main && series.unit !== main.unit ? this.all.filter((one) => one.unit === series.unit) : [series];
        const good = kin.flatMap((one) => one.points || []).filter((v) => Number.isFinite(v));
        if (!good.length) return null;
        if (!this.state.fitted && series.span) return series.span;
        const low = Math.min(...good), high = Math.max(...good);
        return low === high ? [low - 0.5, high + 0.5] : [low, high];
    }

    // The lines across the picture, with the number each one stands for: bottom, middle, top of the series the
    // picture is about. Three, because a phone's 72 pixels hold three numbers without them touching - and
    // because bottom, middle and top are what a person actually reads off a curve.
    get marks() {
        const at = this.bounds(this.main);
        if (!at) return [];
        const digits = at[1] - at[0] < 10 ? 1 : 0;
        return [0, 0.5, 1].map((part) => ({
            at: (29 - part * 28).toFixed(2),
            words: (at[0] + (at[1] - at[0]) * part).toFixed(digits) + (part === 1 ? ' ' + dictionary.unit(this.main.unit) : ''),
        }));
    }

    async ask() {
        if (document.hidden) return;
        const which = this.state.which;
        try {
            const drawn = await (await fetch(onBoard('/curve/' + which))).json();
            // the answer of a curve nobody asked for any more is thrown away: a person can press while it flies
            if (which === this.state.which && drawn && drawn.lines) {
                // the words and span of a line the board does not say are the menu's (dictionary.curveLines)
                const said = this.props.curves.find((one) => one.name === which);
                this.state.drawn = { ...drawn, lines: dictionary.curveLines(drawn.lines, said) };
            }
        } catch (e) { /* a board that did not answer keeps the picture it had */ }
    }

    // Every line as polylines, with how it is painted. A reading that meant nothing - a sensor gone, a setting
    // that is not one - BREAKS THE LINE instead of being drawn as a zero: a curve dropping to the floor says
    // "it read zero", which is the one thing that did not happen.
    //
    // A LINE IN ANOTHER UNIT IS DRAWN AGAINST ITS OWN SCALE, and the key says so in words. One axis cannot hold
    // pascals and percent, and a percent squeezed into a scale of pascals is a flat line at the bottom saying
    // nothing. Which is why the contract needs no flag for it: the unit already says.
    get drawnLines() {
        const main = this.main;
        const unit = main ? main.unit : '';
        const paints = ['text-acc', 'text-ink', 'text-warn', 'text-ok'];
        // THE LOOK OF A LINE IS ITS PLACE IN THE ORDER, NOT ITS SCALE: solid, dashed, dotted, dash and dot. Two lines
        // on one scale - asked and came out, P and I - differ by nothing else, and a colour is what the light theme
        // fades and what not everyone tells apart. A dot is a dash of no length with a round end.
        const looks = [{ dashed: null, cap: 'butt' }, { dashed: '6 3', cap: 'butt' }, { dashed: '0.1 4', cap: 'round' }, { dashed: '6 3 0.1 3', cap: 'round' }];
        return this.all.map((one, at) => {
            const own = one.unit !== unit;
            return {
                words: one.words,
                unit: one.unit,
                own,
                shape: own ? ' (' + dictionary.unit(one.unit) + ', ' + ui().ownScale + ')' : '',
                // ONE WEIGHT FOR EVERY LINE, NOT BY BEING FADED. Fading is what the dark theme can afford - a pale
                // line on a dark floor is soft already - and on the light theme a line thinner than the first was a
                // scratch nobody could find (260921-0349).
                paint: paints[at % paints.length],
                width: 1.5,
                ...looks[at % looks.length],
                runs: this.runs(one),
            };
        });
    }

    runs(series) {
        const points = series && series.points;
        if (!points || points.length < 2) return [];
        const at = this.bounds(series);
        if (!at) return [];
        const [low, high] = at;
        // A VALUE THAT DOES NOT MOVE IS DRAWN DOWN THE MIDDLE OF A FITTED PICTURE. Scaled by its own span it
        // would sit on the floor, where a line is hard to tell from no line at all - and a pin pinned to the
        // ceiling of the converter is exactly the case where a person needs to see the board still reading.
        // Against a fixed span there is no such trouble: the value stands where it belongs.
        const flat = high === low;
        const span = flat ? 1 : high - low;
        const runs = [];
        let run = [];
        points.forEach((v, i) => {
            if (v === null || !Number.isFinite(v)) { if (run.length > 1) runs.push(run.join(' ')); run = []; return; }
            const x = i * 100 / (points.length - 1);
            const y = flat ? 15 : Math.max(0.5, Math.min(29.5, 29 - (v - low) * 28 / span));
            run.push(x.toFixed(2) + ',' + y.toFixed(2));
        });
        if (run.length > 1) runs.push(run.join(' '));
        return runs;
    }

    // What the picture spans, so a line without numbers is not a picture of nothing.
    get span() {
        if (!this.state.drawn) return null;
        const shape = (one) => {
            const good = (one.points || []).filter((v) => Number.isFinite(v));
            if (!good.length) return '';
            const low = Math.min(...good), high = Math.max(...good);
            const digits = high - low < 10 ? 1 : 0;
            return low.toFixed(digits) + '-' + high.toFixed(digits) + ' ' + dictionary.unit(one.unit);
        };
        return this.all.map(shape).filter((s) => s).join(' \u00b7 ');
    }

    get pace() {
        const drawn = this.state.drawn;
        if (!drawn || !this.main) return '';
        return dictionary.paceWords((this.main.points || []).length, drawn.step_ms, ui());
    }
}

// A BUTTON THAT SENDS A COMMAND, and asks first where the command cannot be taken back (`ask`: the board says it in its dictionary, `irreversible`): the first press only asks, the second sends (press.js, asker). One press at a time (pressOnce): the
// button is deaf from its press until the board has answered and for a moment after it, and says `accepted` meanwhile. It is the one button of the page that asks, in a row (Thing) and in a part of a device (page.AskedPress), so that the asking, its words and its term stand once.
// It draws the button and, while it asks, a line of its own under it that takes the full width of what it stands in (basis-full): the place it stands in is a row that wraps (flex-wrap).
class AskedPress extends Component {
    static props = {
        channel: String, label: String, send: Function,
        ask: { type: Boolean, optional: true }, thing: { type: String, optional: true }, tag: { type: String, optional: true },
    };

    get ui() { return ui(); }

    setup() {
        this.b = useBoard();
        this.state = useState({ sure: false });
        this.locks = useState(locks);
        this.asker = press.asker({ set: (fn, ms) => setTimeout(fn, ms), clear: (h) => clearTimeout(h) }, (sure) => { this.state.sure = sure; });
        onWillUnmount(() => this.asker.cancel());
    }

    get phase() { return press.phase(this.locks[this.props.channel]); }

    tap() {
        if (this.asker.tap(Boolean(this.props.ask)) === "ask") return;
        pressOnce(this.props.channel, null, "answer", true, () => this.b.things, () => this.props.send());
    }

    static template = xml`
            <!-- THE BUTTON KEEPS ITS WIDTH WHATEVER IT SAYS: On and Off are not the same width, and a button that grows two pixels when the organ closes moves under a finger that has not lifted
                 (page_steady, buttons-steady) -->
            <button class="text-sm px-3.5 py-1.5 rounded-[10px] border border-acc bg-acc-soft text-acc font-semibold active:bg-acc active:text-acc-ink shrink-0 min-h-[44px] min-w-[6.5rem] text-center"
                    t-att-data-sure="state.sure ? 'yes' : null" t-att-data-thing="props.thing || null" t-att-data-tag="props.tag || null" t-att-data-phase="phase || false" t-att-aria-busy="phase === 'accepted' ? 'true' : null"
                    t-on-click="() => this.tap()">
                <t t-if="state.sure" t-esc="ui.sure"/>
                <t t-elif="phase === 'accepted'"><span data-accepted="" class="inline-block w-1.5 h-1.5 mr-1.5 rounded-full bg-acc align-middle motion-safe:animate-pulse"/><t t-esc="ui.pressAcceptedShort"/></t>
                <t t-else="" t-esc="props.label"/>
            </button>
            <!-- WHAT THE PRESS WOULD DO CANNOT BE TAKEN BACK, and the person is told so beside the choice they are given: do it, or leave it. -->
            <div t-if="state.sure" class="basis-full flex items-center gap-3" data-asking="yes">
                <span class="text-sm text-dim flex-1 min-w-0" t-esc="ui.irreversible"/>
                <button class="text-sm px-3.5 py-1.5 rounded-[10px] border border-line text-dim shrink-0 min-h-[44px] min-w-[4.5rem] text-center"
                        t-on-click="() => this.asker.cancel()" t-esc="ui.cancel"/>
            </div>`;
}

// A row of a card, drawn of the board's row at its ADDRESS and by nothing else: the row is read when it is drawn, so what it depends on is
// the row's own value, and an event of another address does not draw it again (board.js).
class Thing extends Component {
    get ui() { return ui(); }
    static components = { Row, Lad, Curve, Unit, AskedPress };
    static props = { at: Number, aimed: Object, act: Function, hold: Function, aim: Function, read: { type: Boolean, optional: true } };

    setup() {
        this.b = useBoard();
        this.locks = useState(locks);
    }

    get row() { return this.b.row(this.props.at); }

    get channel() { return "@" + this.props.at; }
    // whether the press of this row's button is asked about first: the board calls what it does irreversible
    asks(thing) { const one = this.control(thing); return Boolean(one && one.ask); }
    // the row's slider has Stop, which sets its zero; it is a press like the others
    zero(thing) {
        pressOnce(this.channel + "/0", null, "answer", true, () => this.b.things, () => this.props.hold(thing, 0));
    }
    get zeroPhase() { return press.phase(this.locks[this.channel + "/0"]); }

    // WHOSE VALUE THE HANDLE SHOWS. While a finger holds it, the hand; otherwise the board. Bound straight to the
    // board it was thrown back to the old percent every time the state arrived, which is every couple of seconds -
    // the settings beside it have always done it the other way round (picked[name] or the board's), and this row
    // now does the same. Rule and its tests: form.handle, tools/test_page_form.js.
    handle(thing) {
        return form.handle(this.props.aimed[thing.name], thing.range.value);
    }

    // The word of the unit that follows the number of a slider, and NOTHING when the board named none. A board that sends a range without one
    // is a board we have not updated yet, and "30 undefined" beside a slider is worse than a bare 30. Drawn by Unit, like every unit beside a number.
    unit(thing) {
        const unit = thing.range && thing.range.unit;
        return unit ? dictionary.unit(unit) : "";
    }
    static template = xml`
        <t t-set="thing" t-value="row"/>
        <Row t-if="thing" name="thing.name" value="thing.state" level="thing.level">
            <Lad t-if="thing.signal" signal="thing.signal"/>
            <AskedPress t-if="control(thing) and !props.read" channel="channel" ask="asks(thing)" label="ui[control(thing).words] || control(thing).words"
                        thing="thing.group + '/' + thing.key" send="() => this.props.act(control(thing).path)"/>
            <!-- A CONTROL TAKES A LINE OF ITS OWN. basis-full wraps it under the name and the value, and
                 that is what makes room for a slider a finger can aim: squeezed into the right-hand end of
                 the row, the words went three wrapped lines wide and the handle stayed 120px narrow.
                 Stop is the slider's zero under one press - dragging a handle to exactly 0 while a motor
                 runs is not what a person wants to be doing. It goes where the slider goes, with 0. -->
            <Curve t-if="thing.curves and thing.curves.length" curves="thing.curves"/>
            <div t-if="thing.range and !props.read" class="basis-full flex items-center gap-3">
                <button class="text-sm px-3.5 py-1.5 rounded-[10px] border border-acc bg-acc-soft text-acc font-semibold active:bg-acc active:text-acc-ink shrink-0 min-h-[44px] min-w-[6.5rem] text-center"
                        t-att-data-phase="zeroPhase || false" t-att-aria-busy="zeroPhase === 'accepted' ? 'true' : null" data-zero=""
                        t-on-click="() => this.zero(thing)">
                    <t t-if="zeroPhase === 'accepted'"><span data-accepted="" class="inline-block w-1.5 h-1.5 mr-1.5 rounded-full bg-acc align-middle motion-safe:animate-pulse"/><t t-esc="ui.pressAcceptedShort"/></t>
                    <t t-else="" t-esc="ui.stop"/>
                </button>
                <!-- The board hears the slider when the finger LIFTS, not while it drags: every step of a
                     drag would be a request, and each one moves a triac. That is what change gives and input
                     does not. The NUMBER follows the finger, though - aiming at a percent you cannot see is
                     not aiming. While the handle is held it shows where the handle is; let go, and what the
                     board answers takes over.
                     NO BACKTICKS IN HERE: this whole template is one backtick string, a stray pair ends it. -->
                <input type="range" class="flex-1 min-w-0 min-h-[44px] accent-acc"
                       t-att-min="thing.range.min" t-att-max="thing.range.max" t-att-step="thing.range.step"
                       t-att-value="handle(thing)"
                       t-on-input="(e) => this.props.aim(thing.name, e.target.value)"
                       t-on-change="(e) => this.props.hold(thing, e.target.value)"/>
                <!-- THE UNIT COMES FROM THE BOARD. Written as a percent here, the draught's slider read
                     "40%" while it was asking for forty PASCALS - and a percent of a dimmer's knob carries
                     no physical meaning at all, which is the whole reason the draught is asked for in
                     pascals (scrivo-iot). -->
                <span class="shrink-0 flex items-baseline gap-1 tabular-nums">
                    <span class="min-w-[2rem] text-right" t-esc="handle(thing)"/>
                    <Unit t-if="unit(thing)" of="{ unit: unit(thing) }"/>
                </span>
            </div>
        </Row>`;

    // what a thing's button sends and says: on, off or a press (wire.js, control)
    control(thing) { return wire.control(thing); }
}

// A card: one group of things, its rows and its curves - the readings of a module, and nothing to set: the settings stand in the menu's tree
// (Section), the controls in its manual section. A group added to /state adds a card here and costs no markup; a row changing draws the row (Thing).
class Card extends Component {
    static components = { Thing };
    static props = { title: String, aimed: Object, act: Function, hold: Function, aim: Function, read: { type: Boolean, optional: true } };
    static template = xml`
        <section class="bg-panel border border-line rounded-[14px] p-3.5 shadow-card">
            <h2 class="text-[15px] text-ink font-semibold mb-2 first-letter:uppercase" t-esc="heading"/>
            <Thing t-foreach="things" t-as="thing" t-key="thing.at"
                   at="thing.at" aimed="props.aimed" act="props.act" hold="props.hold" aim="props.aim" read="props.read"/>
        </section>`;

    setup() {
        this.b = useBoard();
    }

    // what the card is headed by: its title in the menu, or its own name
    get heading() { return dictionary.cardTitle(this.b.texts, this.props.title); }
    get things() { return this.b.things.filter((t) => t.group === this.props.title && !t.apart); }
}

// A SECTION OF A MODULE'S PAGE (the menu's tree): what the menu puts in it - the rows of the cards it names, which are the module's readings, its curves and its controls, then the settings it
// holds, each with the one Save that sends them, and a table of them where it is one. A setting stands on ONE page, its module's; the main screen shows the
// daily few again, and that is the same record. What it says of a save stands under it.
class Section extends Component {
    static components = { Settings, Steps, Windows, Thing };
    static props = { section: Object, edits: Object, aimed: Object, act: Function, hold: Function, aim: Function, edit: Function, save: Function, said: { type: Object, optional: true } };
    static template = xml`
        <section class="bg-panel border border-line rounded-[14px] p-3.5 shadow-card">
            <h2 t-if="props.section.title" class="text-[15px] text-ink font-semibold mb-2" t-esc="props.section.title"/>
            <Thing t-foreach="things" t-as="thing" t-key="thing.at"
                   at="thing.at" aimed="props.aimed" act="props.act" hold="props.hold" aim="props.aim"/>
            <Settings t-if="listed.length" list="listed" pairs="props.section.pairs" labels="props.section.labels" edits="props.edits" edit="props.edit" save.bind="saveHere" said="props.said"/>
            <!-- THE COMMANDS THE MENU PLACES HERE (11.14) stand after the settings, drawn as any row of a card is: a command that cannot be taken back asks first -->
            <Thing t-foreach="actions" t-as="thing" t-key="thing.at"
                   at="thing.at" aimed="props.aimed" act="props.act" hold="props.hold" aim="props.aim"/>
            <Steps t-if="table" table="table" edits="props.edits" edit="props.edit" save.bind="saveHere" said="props.said"/>
            <Windows t-if="windows.length" list="windows" edits="props.edits" edit="props.edit" save.bind="saveHere" said="props.said"/>
        </section>`;

    setup() {
        this.b = useBoard();
    }

    // the rows of the cards the menu puts here, in the order it names the cards and the board's within each
    get things() { return dictionary.sectionRows(this.tree, this.props.section, this.b.things).shown; }
    // the commands the menu places in this section, in the order it names them
    get actions() { return dictionary.sectionRows(this.tree, this.props.section, this.b.things).actions; }
    get tree() { return dictionary.treeOf(this.b.texts); }
    // the section's settings as the board holds them and the person has picked, in the order the menu lists them
    get listed() { return dictionary.sectionSettings(this.props.section, this.b.everySetting, this.props.edits); }
    // the windows of the section's schedule, drawn as forms
    get windows() { return dictionary.windowsOf(this.props.section, this.b.everySetting, this.props.edits); }
    // the section's table of steps with the board's numbers in it and the row the burner burns on marked, or none
    get table() { return dictionary.tableOf(this.props.section, this.b.everySetting); }
    // what the board answered stands under the section it was sent from
    async saveHere(list) {
        return this.props.save(list, this.props.section.id);
    }
}

// WHAT A SECTION OF THE MENU IS DRAWN BY, decided once for every place that draws sections - the main screen, a module's page, the system tab: the part it asks for with its `part` field (registry.js), the line that says
// the part is wrong (one added twice, one nobody added), and otherwise what the caller puts between the tags, its plain section. The caller says which of its own props the part is handed: `act` and/or `put`.
class Placed extends Component {
    static props = { section: Object, act: { type: Function, optional: true }, put: { type: Function, optional: true }, slots: { optional: true } };
    static template = xml`
        <t t-set="drawn" t-value="parts.partOf(props.section)"/>
        <t t-if="drawn and drawn.component" t-component="drawn.component" t-props="handed(drawn)"/>
        <section t-elif="drawn" data-card="refused" class="bg-panel border border-bad rounded-[14px] p-3.5 shadow-card text-bad text-sm" t-esc="said(drawn.problem)"/>
        <t t-else=""><t t-slot="default"/></t>`;

    get parts() { return parts; }
    said(wrong) { return problemSaid(wrong); }
    // what the part is given: what the section asks it for, and the callers' own
    handed(drawn) {
        const { act, put } = this.props;
        return { given: drawn.given, ...(act ? { act } : {}), ...(put ? { put } : {}) };
    }
}

// The terminal on the board's own console - the same shell as over USB and TCP. The log arrives by itself, a typed
// line goes back. The socket is opened when this tab is shown and closed when it is left: the board has room for two
// consoles, and a page holding one it is not showing takes that room from a phone.
class Terminal extends Component {
    get ui() { return ui(); }
    static props = { at: String };
    static template = xml`
        <section class="bg-panel border border-line rounded-[14px] p-3.5 shadow-card">
            <h2 class="text-[15px] text-ink font-semibold mb-2" t-esc="ui.terminal"/>
            <pre t-ref="log" class="font-mono text-xs bg-sunk rounded-[10px] p-2 h-56 overflow-auto whitespace-pre-wrap" t-esc="state.text"/>
            <div class="flex gap-2 mt-2.5">
                <input t-ref="line" class="flex-1 bg-bg border border-line rounded-[10px] px-2 py-1.5 text-sm min-h-[44px]"
                       autocomplete="off" autocapitalize="off" spellcheck="false"
                       t-on-keydown="(ev) => ev.key === 'Enter' ? this.send() : null"/>
                <button class="text-sm px-3.5 py-1.5 rounded-[10px] border border-acc bg-acc-soft text-acc font-semibold active:bg-acc active:text-acc-ink min-h-[44px]"
                        t-on-click="() => this.send()" t-esc="ui.send"/>
            </div>
            <p class="text-[13px] text-dim mt-1.5" t-esc="state.note"/>
        </section>`;

    setup() {
        this.state = useState({ text: "", note: ui().consoleOpening });
        this.log = useRef("log");
        this.line = useRef("line");
        this.open();
        onWillUnmount(() => this.close());
    }

    open() {
        this.ws = new WebSocket(wire.socketAt(BOARD.base, this.props.at, location.href));
        this.ws.onopen = () => { this.state.note = ui().consoleConnected; };
        this.ws.onmessage = (ev) => this.add(ev.data);
        // The board says in words when both consoles are taken and then closes the socket itself, so its words are
        // already in the log above: a close is not an error of its own.
        this.ws.onclose = () => { this.state.note = ui().consoleClosed; };
        this.ws.onerror = () => { this.state.note = ui().consoleUnreachable; };
    }

    close() {
        if (this.ws) this.ws.close();
        this.ws = null;
    }

    // What a board says over a day does not belong in a phone's memory: the last screenful is what gets read.
    add(text) {
        const lines = (this.state.text + text).replace(/\r/g, "").split("\n");
        this.state.text = lines.slice(-200).join("\n");
        Promise.resolve().then(() => { if (this.log.el) this.log.el.scrollTop = this.log.el.scrollHeight; });
    }

    send() {
        const said = this.line.el.value;
        if (!said || !this.ws || this.ws.readyState !== WebSocket.OPEN) return;
        this.ws.send(said);
        this.line.el.value = "";
    }
}

// What the burner is doing NOW, in one card: what a person switched, what the fan is held at, what the sensors
// read. The form is the one a pellet boiler's own panel uses (iot/burner/docs/02_REF_Ecoforest_Controller.md).
//
// **WITH ONE DIFFERENCE, AND THE CARD DOES NOT DISCUSS IT.** Theirs shows the stage of the fire - lighting,
// burning, damping - because their controller runs the fire. Ours does not: there is no automaton yet, and a
// person switches the auger and the igniter by hand. So those stages are simply not drawn. They were not drawn
// greyed out either - a grey step reads as "not now" rather than "we cannot do this yet" - and the sentence that
// used to explain their absence has gone the same way: a person opening the page for the first time has never
// seen stages here, and reads an explanation of what is missing as a description of what is there.
class Now extends Component {
    get ui() { return ui(); }
    static props = {};
    static components = { Row };
    static template = xml`
        <section t-if="rows.length" class="bg-panel border border-line rounded-[14px] p-3.5 shadow-card">
            <h2 class="text-[15px] text-ink font-semibold mb-2" t-esc="ui.now"/>
            <Row t-foreach="rows" t-as="row" t-key="row.name" name="row.name" value="row.value" level="row.level"/>
        </section>`;

    setup() {
        this.b = useBoard();
    }

    // the rows the board puts in the card (`machine.now`), in its order, by the page's words (texts.json, `now`), and the rows the menu puts in it
    get rows() {
        return wire.now(dictionary.nowRows(this.b.things, this.b.texts, this.b.machine));
    }
}

class Tabs extends Component {
    static props = { tabs: Array, labels: Object, current: String, pick: Function };
    static template = xml`
        <nav class="flex gap-0.5 bg-sunk rounded-xl p-1">
            <button t-foreach="props.tabs" t-as="tab" t-key="tab"
                    class="flex-1 min-w-0 min-h-[44px] px-0 text-[12px] rounded-[9px] border-0 overflow-hidden text-ellipsis"
                    t-att-class="tab === props.current ? 'bg-panel text-ink font-semibold' : 'bg-transparent text-dim'"
                    t-on-click="() => this.props.pick(tab)" t-esc="props.labels[tab] || tab"/>
        </nav>`;
}

class Vitals extends Component {
    static components = { Row };
    static props = { board: String, kit: String };
    static template = xml`
        <Row name="ui.rowBoard" value="props.board" mono="true"/>
        <Row name="ui.rowVersion" value="props.kit" mono="true"/>
        <Row name="ui.rowMemory" value="memory" level="head and head.low_memory ? 'warn' : false"/>
        <Row name="ui.rowUptime" value="uptime"/>
        <Row t-foreach="cores" t-as="core" t-key="core.name" name="core.name" value="core.value"/>
        <Row t-if="head and head.time" name="ui.rowTime" value="head.time"/>
        <p t-if="head and head.low_memory" class="text-[13px] text-warn mt-1.5" t-esc="ui.lowMemory"/>`;

    setup() {
        this.b = useBoard();
        const timer = setInterval(() => this.render(), 10000);
        onWillDestroy(() => clearInterval(timer));
    }

    get ui() { return ui(); }
    get head() { return this.b.vitals; }
    // how busy each core is, a row a core with its load in per cent; a core not measured yet has no value, and a board that says nothing of its cores has no rows
    get cores() {
        const cores = this.head && this.head.cores;
        if (!cores) return [];
        return cores.map((share, n) => ({ name: fill(ui().rowCore, { n }), value: Number.isFinite(share) ? share + ' ' + dictionary.unit('%') : '' }));
    }
    get memory() { return fill(ui().memory, this.head ? { free: this.head.free, used: this.head.used } : { free: 0, used: 0 }); }
    get uptime() {
        const up = Math.floor((((this.head && this.head.uptime) || 0) + (this.b.since ? Date.now() - this.b.since : 0)) / 1000);
        return `${Math.floor(up / 3600)} ${dictionary.unit("h")} ${Math.floor((up % 3600) / 60)} ${dictionary.unit("min")}`;
    }
}

// THE REFERENCE: what each item of the menu is and what it is for, in the menu's words - the place for every explanation the lines of the page do not carry (REF_Device_Page_Look, rule 7).
class Reference extends Component {
    static template = xml`
        <div class="flex flex-col gap-3">
            <section t-foreach="groups" t-as="group" t-key="group_index" class="bg-panel border border-line rounded-[14px] p-3.5 shadow-card">
                <h2 class="text-[15px] text-ink font-semibold mb-2" t-esc="group.title"/>
                <div t-foreach="group.items" t-as="one" t-key="one.key" class="py-2 border-b border-line last:border-b-0">
                    <div class="text-[13px] text-ink font-semibold" t-esc="one.name"/>
                    <p class="text-[13px] text-dim mt-0.5" t-esc="one.text"/>
                </div>
            </section>
        </div>`;

    setup() {
        this.b = useBoard();
    }

    get groups() { return dictionary.helpOf(this.b.texts); }
}

class Page extends Component {
    get ui() { return ui(); }
    static components = { Tabs, Card, Section, Placed, Terminal, Row, Now, Vitals, Reference };
    static template = xml`
        <main class="max-w-[420px] mx-auto flex flex-col gap-3">
            <!-- ONE LINE, AND THE THREE SQUARES AT ITS END: the period the board sends at, grey - a press asks for every change and the square goes red, a second press or the board's own end of Live
                 (its dictionary says when) puts the period back; no count of seconds on it -, the language and the theme -->
            <header class="flex items-center gap-2">
                <h1 class="text-xl shrink-0 max-w-[40%] truncate" t-esc="state.device || ui.board"/>
                <span class="text-[13px] px-2.5 py-0.5 rounded-full bg-acc-soft text-acc flex-1 min-w-0 truncate" t-esc="state.kit"/>
                <button t-if="liveTotal and !state.foreign" class="w-11 h-11 p-0 shrink-0 rounded-[10px] border text-[10px] tabular-nums"
                        t-att-class="live ? 'border-bad bg-bad text-acc-ink font-semibold' : 'border-line bg-sunk text-dim'"
                        t-att-aria-pressed="live ? 'true' : 'false'" t-att-title="ui.live"
                        t-on-click="() => this.goLive()" t-esc="liveWords"/>
                <button class="w-11 h-11 p-0 shrink-0 text-sm rounded-[10px] border border-line bg-sunk"
                        t-att-title="ui.language" t-on-click="() => this.flipLang()" t-esc="lang === 'ru' ? 'EN' : 'RU'"/>
                <button class="w-11 h-11 p-0 shrink-0 text-sm rounded-[10px] border border-line bg-sunk"
                        t-att-title="ui.lightDark" t-on-click="() => this.flip()">◐</button>
            </header>
            <div class="text-[13px]" t-if="state.note" data-note=""
                 t-att-class="state.note_level === 'bad' ? 'text-bad' : (state.note_level === 'warn' ? 'text-warn' : 'text-dim')"
                 t-esc="state.note"/>

            <!-- AN ANSWER OF ANOTHER CONTRACT IS NOT DRAWN: the note above says why and what to do -->
            <t t-if="!state.foreign">
            <Tabs tabs="tabs" labels="labels" current="state.tab" pick.bind="pick"/>

            <t t-if="state.tab === 'modules'">
                <t t-if="!page">
                    <!-- THE LIST OF MODULES, and above it what the burner is doing in one place: a module is a page of its own, and on it everything about it (the menu's tree) -->
                    <Now/>
                    <section t-foreach="moduleGroups" t-as="group" t-key="group.id" class="bg-panel border border-line rounded-[14px] p-3.5 shadow-card">
                        <h2 class="text-[15px] text-ink font-semibold mb-2" t-esc="group.title"/>
                        <div class="flex flex-col gap-2">
                            <button t-foreach="group.pages" t-as="one" t-key="one.id"
                                    class="text-left text-sm px-3.5 py-1.5 rounded-[10px] border border-line bg-sunk active:bg-acc-soft min-h-[44px]"
                                    t-on-click="() => this.open(one.id)" t-esc="one.title"/>
                        </div>
                    </section>
                    <!-- a menu with no tree draws the cards, read-only; a device that has no module yet says so, in its own card, instead of showing a page that looks broken -->
                    <section t-if="groups.length === 0" class="bg-panel border border-line rounded-[14px] p-3.5 shadow-card">
                        <h2 class="text-[15px] text-ink font-semibold mb-2" t-esc="ui.modules"/>
                        <p class="text-sm text-dim" t-esc="ui.noModules"/>
                    </section>
                    <Card t-if="!tree" t-foreach="groups" t-as="group" t-key="group"
                          title="group" aimed="state.aimed" act.bind="act" hold.bind="hold" aim.bind="aim" read="true"/>
                </t>
                <t t-else="">
                    <div class="flex items-center gap-2">
                        <button class="text-sm px-3.5 py-1.5 rounded-[10px] border border-line bg-sunk active:bg-acc-soft min-h-[44px] shrink-0"
                                t-on-click="() => this.open('')"><t t-esc="'\u2039 ' + ui.modules"/></button>
                        <h2 class="text-[17px] text-ink font-semibold flex-1 min-w-0 truncate" t-esc="page.title"/>
                    </div>
                    <Placed t-foreach="page.sections" t-as="section" t-key="section.id" section="section" put.bind="put">
                        <Section section="section" edits="state.edits"
                                 aimed="state.aimed" act.bind="act" hold.bind="hold" aim.bind="aim"
                                 edit.bind="edit" save.bind="save" said="state.said_at[section.id]"/>
                    </Placed>
                </t>
            </t>

            <t t-if="state.tab === 'system'">
                <!-- THE BOARD'S OWN, one tab and its panels side by side (no panel inside a panel): the time and zone it keeps, what it is and what it has, restart and the way down, the console -->
                <t t-foreach="systemSections" t-as="section" t-key="section.id">
                    <!-- THE SERVICE is the sections the menu groups under service: one heading over the first of them -->
                    <h2 t-if="section.group === 'service' and section.id === serviceFirst" data-service="" class="text-[17px] text-ink font-semibold mt-1" t-esc="ui.service"/>
                    <Placed section="section" act.bind="act">
                    <t t-if="section.board">
                        <Card title="'system'" aimed="state.aimed" act.bind="act" hold.bind="hold" aim.bind="aim"/>
                        <section class="bg-panel border border-line rounded-[14px] p-3.5 shadow-card">
                            <h2 class="text-[15px] text-ink font-semibold mb-2" t-esc="ui.about"/>
                            <Vitals board="state.board" kit="state.kit"/>
                            <div class="flex flex-wrap gap-2 mt-2.5">
                                <button class="text-sm px-3.5 py-1.5 rounded-[10px] border border-acc bg-acc-soft text-acc font-semibold active:bg-acc active:text-acc-ink min-h-[44px]"
                                        t-on-click="() => this.restart()" t-esc="ui.restart"/>
                                <button class="text-sm px-3.5 py-1.5 rounded-[10px] border border-line bg-sunk active:bg-acc-soft min-h-[44px]"
                                        t-on-click="() => this.ask_down()" t-esc="ui.goDown"/>
                            </div>
                            <div t-if="state.asking" class="flex flex-wrap gap-2 mt-2.5">
                                <input class="flex-1 bg-bg border border-line rounded-[10px] px-2 py-1.5 text-sm min-h-[44px]"
                                       autocomplete="off" autocapitalize="off" spellcheck="false"
                                       t-att-placeholder="state.board"
                                       t-on-input="(ev) => this.typed(ev.target.value)"/>
                                <button class="text-sm px-3.5 py-1.5 rounded-[10px] border min-h-[44px]"
                                        t-att-class="state.said === state.board ? 'border-bad bg-bad text-acc-ink' : 'border-line text-dim bg-sunk opacity-60'"
                                        t-att-disabled="state.said !== state.board"
                                        t-on-click="() => this.down()" t-esc="ui.goDown"/>
                                <p class="text-[13px] text-dim basis-full" t-esc="downNote"/>
                            </div>
                        </section>
                        <Terminal t-if="state.terminal" at="state.terminal"/>
                        <section t-else="" class="bg-panel border border-line rounded-[14px] p-3.5 text-[13px] text-dim">
                            <t t-esc="ui.noConsole"/>
                        </section>
                    </t>
                    <Section t-else="" section="section" edits="state.edits"
                             aimed="state.aimed" act.bind="act" hold.bind="hold" aim.bind="aim"
                             edit.bind="edit" save.bind="save" said="state.said_at[section.id]"/>
                    </Placed>
                </t>
            </t>

            <t t-if="state.tab === 'main'">
                <!-- THE MAIN SCREEN OF A DEVICE WITH NO TAB OF ITS OWN: the menu's main sections, one under another -->
                <Placed t-foreach="mainSections" t-as="section" t-key="section.id" section="section" act.bind="act">
                    <Section section="section" edits="state.edits"
                             aimed="state.aimed" act.bind="act" hold.bind="hold" aim.bind="aim"
                             edit.bind="edit" save.bind="save" said="state.said_at[section.id]"/>
                </Placed>
            </t>

            <t t-if="partTab">
                <!-- THE MAIN SCREEN is the device's own tab (page.parts.tab), and under it what a person turns every day stands open: the menu's main sections -->
                <t t-component="partTab.component" act.bind="act" put.bind="put"/>
                <Placed t-foreach="mainSections" t-as="section" t-key="section.id" section="section" act.bind="act">
                    <Section section="section" edits="state.edits"
                             aimed="state.aimed" act.bind="act" hold.bind="hold" aim.bind="aim"
                             edit.bind="edit" save.bind="save" said="state.said_at[section.id]"/>
                </Placed>
            </t>

            <t t-if="state.tab === 'help'">
                <Reference/>
            </t>

            </t>
        </main>`;

    setup() {
        // THE SCHEME COMES FIRST AND OPENS FIRST: it is the main screen, where the automaton runs the burner and the few things turned every day stand. The modules are the list of everything else, a page each, and
        // on it all about the module: its readings, its curves, its controls and its settings (the menu's tree). A page whose menu has no tree lists the cards.
        this.b = useBoard();
        // a press let go by the limit with no word of the board is said here: a button free again says nothing of why. The note is a warning, which the board's next values do not take away; the next press does.
        this.aims = press.latest();
        pressHooks.ended = (why) => { if (why === "silent") this.say(ui().pressSilent, "warn", true); };
        pressHooks.began = () => { if (this.state.note_press) this.say("", "dim"); };
        this.tabs = [...parts.tabs.map((one) => one.id), "modules", "system", "help"];
        this.state = useState({
            tab: this.tabs[0], module: "", live: 0, now: Date.now(), aimed: {}, device: "", board: "", kit: "", terminal: "",
            // an address that is no board's draws nothing, as an answer of another contract does
            note: "", note_press: false, note_level: BOARD.error ? "bad" : "dim", asking: false, said: "", foreign: Boolean(BOARD.error),
            // what a person has typed and not saved, by setting: the board's own value stands for the rest
            edits: {}, said_at: {},
        });
        // The names in `edits` a person has changed and not yet saved. While a name is in here the board's own value for it is not
        // drawn over - otherwise the next event, five seconds away, takes the choice out from under their finger.
        this.touched = new Set();
        // The names saved since the last event: the board answers with the value it took, and that is what they show from then on.
        this.settled = new Set();
        onWillStart(() => this.pull());
        this.listen();
        // once a second: the counts the board said once go on, and Live that has run out is taken back (the board has already gone back to the pace of everyone)
        const clock = setInterval(() => {
            this.b.tick();
            this.state.now = Date.now();
            if (this.state.live && this.state.live <= this.state.now) this.state.live = 0;
        }, 1000);
        onWillUnmount(() => {
            clearInterval(clock);
            this.stop();
        });
    }

    // THE BOARD SAYS WHAT EACH ADDRESS IS ONCE, AND THIS PAGE SAYS HOW A PERSON READS IT: the dictionary
    // comes from the board, the table of texts from this page's own kit, and after them only values arrive - over
    // /events, the first event every one of them. Nothing is drawn by a snapshot taken at the start: until a value
    // comes, its row reads a dash.
    async pull() {
        let texts;
        try {
            const [said, menu] = await Promise.all([
                wire.kitFile("texts.json", document, (name) => fetch(name, { cache: "no-store" }).then((r) => r.json())),
                wire.kitFile("menu.json", document, (name) => fetch(name, { cache: "no-store" }).then((r) => r.json())),
            ]);
            // THE LANGUAGE is the address's, else the one a person chose and the browser kept, else Russian; the table of texts is laid over by its Russian, and the menu rides beside it: where and how the modules it names are shown (dictionary.js)
            const lang = dictionary.langOf({ asked: new URLSearchParams(location.search).get("lang") || "", stored: storedLang() });
            document.documentElement.lang = lang;
            texts = { ...dictionary.speak(said, lang), lang, menu };
            dictionary.useUnits(dictionary.menuUnits(texts));
        } catch (e) {
            return;
        }
        this.texts = texts;
        if (BOARD.error) return this.say(this.words(BOARD.error), "bad");
        try {
            const dict = await fetch(onBoard("/dictionary"), { cache: "no-store" }).then((r) => r.json());
            this.dict = dict;
            board.load(dict, texts);
            this.loaded();
        } catch (e) {
            this.say(ui().notAnswering, "bad");
        }
    }

    // a code the page was handed - {kind, ...places} - in the words of the menu
    words(code) { return fill(ui()[code.kind], code); }

    // Whether the board still speaks the dictionary this page draws by; one it does not is said, and nothing is drawn.
    async check() {
        if (!this.dict) return;
        try {
            this.refuse(dictionary.stale(this.dict, await (await fetch(onBoard("/dictionary"), { cache: "no-store" })).json()));
        } catch (e) {
            // not reaching the board is the stream's to say
        }
    }

    // STALE - the words for a dictionary the board no longer speaks - said, and nothing drawn from then on.
    refuse(stale) {
        if (!stale) return;
        this.stale = stale;
        this.state.foreign = true;
        this.say(this.words(stale), "bad");
    }

    // Without a stream: the values of /state, every five seconds - what a page does while two others hold the board.
    // /state names the dictionary it is of, so a board that came back with another image is caught here too.
    async poll() {
        if (BOARD.error) return;
        try {
            const said = await (await fetch(onBoard("/state"), { cache: "no-store" })).json();
            if (this.dict) this.refuse(dictionary.stale(this.dict, { version: said.dictionary }));
            if (this.stale) return;
            this.take(said.values || []);
        } catch (e) {
            this.say(ui().notAnswering, "bad");
        }
    }

    listen() {
        if (BOARD.error) return;
        this.events = new EventSource(onBoard(wire.streamPath(this.state.live, Date.now())));
        this.events.onmessage = (ev) => this.take(JSON.parse(ev.data));
        // A STREAM OPENS AGAIN AFTER THE BOARD WAS AWAY, and it may have come back with another image: its addresses
        // then mean other things, so the dictionary is asked again and a page holding another one draws nothing.
        this.events.onopen = () => this.check();
        // Either the board is gone, or both streams are taken - it answers 503 with words, and a browser reports
        // that as an error of the stream. Asking every five seconds keeps the page alive either way and holds no
        // lasting socket, which is exactly what the board is short of when this happens.
        this.events.onerror = () => {
            this.stop();
            this.say(ui().noStream, "warn");
            if (!this.timer) this.timer = setInterval(() => this.poll(), 5000);
        };
    }

    // how long Live lasts, by the board's own dictionary: 0 - it has none
    get liveTotal() { return wire.liveSeconds(this.dict); }
    // whether Live is on now
    get live() { return this.state.live > this.state.now; }
    // what the button says: the period the board sends at, and while Live lasts the word Live (wire.liveButton)
    get liveWords() { return wire.liveButton(this.dict, this.live, this.ui.live, dictionary.unit("s")); }
    // Live pressed, or pressed again: the stream is opened again asking for every change - or for none of it
    goLive() {
        this.state.live = this.live ? 0 : Date.now() + this.liveTotal * 1000;
        this.state.now = Date.now();
        this.stop();
        this.listen();
    }

    stop() {
        if (this.events) this.events.close();
        this.events = null;
        if (this.timer) clearInterval(this.timer);
        this.timer = null;
    }

    // RECORDS - [[address, value], ...] - from an event, an answer to a command or /state: each value in its place, and the rows that read
    // it, and the components that read those, told (board.js).
    take(records) {
        board.take(records);
        if (!this.dict || !this.texts || this.state.foreign) return;
        for (const name of this.settled) if (!this.touched.has(name)) delete this.state.edits[name];
        this.settled.clear();
        if (this.state.note_level === "bad" && !this.state.note_press) this.say("", "dim");
    }

    // The page as the dictionary and the texts say it is: what the board is called, and whether it speaks the contract this page reads.
    loaded() {
        // read by this page's contract, or not at all: nothing of a board of another is drawn by rules it does not keep
        const foreign = wire.foreign(this.dict) || this.stale;
        this.state.foreign = Boolean(foreign);
        if (foreign) return this.say(this.words(foreign), "bad");
        Object.assign(this.state, {
            device: this.dict.device, board: this.dict.board, kit: String(this.dict.version || "").split("+")[0], terminal: this.dict.terminal,
        });
        // an adding of the device's that is wrong - a part or a tab added twice - is said, and nothing is drawn of a page whose parts are in doubt
        if (parts.problems.length) {
            this.state.foreign = true;
            return this.say(this.problem(parts.problems[0]), "bad");
        }
        if (this.state.note_level === "bad" && !this.state.note_press) this.say("", "dim");
        // A DEVICE WITH NO TAB OF ITS OWN THAT PUTS SECTIONS ON ITS MAIN SCREEN (the menu's `tree.main`) gets a tab for them and opens on it: a Sonoff's relays and its meter are what
        // a person comes for, not a list of modules to open one by one. A device with a tab of its own has its main screen already, and a menu with no `main` has none to open.
        if (!parts.tabs.length && this.mainSections.length && !this.tabs.includes("main")) {
            this.tabs = ["main", ...this.tabs];
            if (this.state.tab === "modules") this.state.tab = "main";
        }
        // a tab of the device's that its board's dictionary says has nothing to draw (`absent`) is not on the page, and the page opens on the modules
        for (const one of parts.tabs.filter((tab) => tab.absent && tab.absent(this.b))) {
            this.tabs = this.tabs.filter((tab) => tab !== one.id);
            if (this.state.tab === one.id) this.state.tab = "modules";
        }
    }

    // PRESS says the note is a press's: the board's refusal of it, or its silence. The board's next values do not take it away - they come every second and the person has not read it yet -; the next press does.
    say(words, level, press) {
        this.state.note = words;
        this.state.note_level = level || "dim";
        this.state.note_press = Boolean(press);
    }

    // the cards of the modules: the automaton's two are the main screen's and the board's own card stands on the board's page
    get groups() {
        const m = this.b.machine;
        return dictionary.cardOrder([...new Set(this.b.things.map(t => t.group))].filter(g => g !== "system" && (!m || (g !== m.card && g !== m.fixed))), this.b.texts);
    }
    // the menu's tree in the page's language, or none
    get tree() { return dictionary.treeOf(this.b.texts); }
    // the names of the tabs: the page's own words
    get labels() {
        return { ...Object.fromEntries(parts.tabs.map((one) => [one.id, this.ui[one.label]])), main: this.ui.mainTab, modules: this.ui.modules, system: this.ui.system, help: this.ui.reference };
    }
    // what the device added, for the template
    get parts() { return parts; }
    // the device's own tab that is open, or none
    get partTab() { return parts.tabs.find((one) => one.id === this.state.tab); }
    problem(wrong) { return problemSaid(wrong); }
    get lang() { return this.b.texts && this.b.texts.lang; }
    // what the page says of the way down, with the board's own name in it
    get downNote() { return fill(this.ui.typeToDown, { board: this.state.board }); }
    // the sections of the main screen
    get mainSections() { return (this.tree && this.tree.main && this.tree.main.sections) || []; }
    // the board's own sections: its time and zone, and what it is
    // the system tab's sections - the board's own, restart and console last, whatever the order the engine and the device added them in
    get systemSections() {
        const all = (this.tree && this.tree.system && this.tree.system.sections) || [];
        return [...all.filter((s) => !s.board), ...all.filter((s) => s.board)];
    }
    // the first section of the Service, which the heading stands over
    get serviceFirst() {
        const first = this.systemSections.find((s) => s.group === "service");
        return first ? first.id : null;
    }
    // the pages of the modules, and the modules by the group the menu puts them in: [{id, title, pages}]
    get moduleGroups() {
        const pages = dictionary.pagesOf(this.tree);
        return ((this.tree && this.tree.modules && this.tree.modules.groups) || []).map((g) => ({ id: g.id, title: g.title, pages: pages.filter((p) => p.group === g.id) }));
    }
    // the page of the module open, or none: the list of modules is shown
    get page() { return dictionary.pagesOf(this.tree).find((p) => p.id === this.state.module) || null; }

    pick(tab) {
        this.state.tab = tab;
        this.state.module = "";
    }
    // a module's page opened, or the list again (an empty id)
    open(id) { this.state.module = id; }
    // Light or dark, and the choice is REMEMBERED. Without the keeping, pressing the button changed the tab and
    // nothing more: refresh, and the page was back to what it started as. The first press has to know what is on
    // the screen NOW - which, before anyone has chosen, is whatever the system says - or it would take two
    // presses to get anywhere.
    // THE LANGUAGE is kept by the browser and the page is drawn again: every word of it comes from the menu, so the page is made again rather than patched
    flipLang() {
        try {
            localStorage.setItem("lang", this.lang === "ru" ? "en" : "ru");
        } catch (e) {
            // a browser that keeps nothing still shows the other language: by the address
        }
        const url = new URL(location.href);
        url.searchParams.set("lang", this.lang === "ru" ? "en" : "ru");
        location.href = url.href;
    }

    flip() {
        const root = document.documentElement;
        const dark = root.dataset.t ? root.dataset.t === "d" : matchMedia("(prefers-color-scheme: dark)").matches;
        const t = dark ? "l" : "d";
        root.dataset.t = t;
        // A browser that refuses to keep it still switches the theme; it just forgets by the next load.
        try { localStorage.setItem("t", t); } catch (e) {}
    }

    edit(name, value, own) {
        this.touched.add(name);
        this.state.edits[name] = { value, own: own || "" };
    }

    // PATH says what it wants - /on/<address>, /off/<address>, /press/<address> - and the answer is the values it left.
    // It says whether the board took the command: true where it did, false where it refused or did not answer - a button that holds itself until the board has answered asks it.
    async act(path) {
        const answer = await this.ask(path);
        if (!answer) return false;
        if (answer.error) {
            // a refusal of a press is a code and its numbers, and the sentence is this page's (form.pressSaid)
            this.say(form.pressSaid(answer.error, ui()), "bad", true);
            return false;
        }
        this.take(answer.values || []);
        return true;
    }

    // Where a slider's handle is being held right now, by thing. Not what the board holds - that is in things -
    // and dropped the moment the board answers, so the number never disagrees with the fan for longer than the
    // request takes.
    aim(name, value) {
        this.aims.begin(name);
        this.state.aimed[name] = Number(value);
    }

    async hold(thing, value) {
        // the hand that sends this write: an older write's end may not let a newer hand go (press.latest)
        const token = this.aims.seen(thing.name);
        const answer = await this.ask("/hold/" + thing.at, String(value));
        // THE HAND IS LET GO WHATEVER HAPPENED, and that is why this is a finally and not a line at the end. The
        // finger is off the handle by the time this runs; leaving the hand's number in place after a refusal or a
        // board that did not answer would keep the handle showing a percent the fan is not at, for good.
        try {
            if (!answer) return false;
            if (answer.error) {
                this.say(answer.error, "bad", true);
                return false;
            }
            this.take(answer.values || []);
            return true;
        } finally {
            if (this.aims.is(thing.name, token)) delete this.state.aimed[thing.name];
        }
    }

    // LIST is saved together; WHERE is the place of the menu's tree what the board answers stands under (a section's id) - the card of the first setting where a place gives none
    async save(list, where) {
        where = where || (list.length && list[0].card ? list[0].card : "board");
        const pairs = list.map(s => {
            const picked = this.state.edits[s.name] || form.picked(s);
            return [s.name, form.sent(s, picked.value, picked.own)];
        });
        const answer = await this.ask("/settings", form.body(pairs));
        if (!answer) return;
        // the board answers with names, values and codes; the words are the page's (form.saveSaid)
        const said = form.saveSaid(answer, ui(), this.b.everySetting);
        this.state.said_at[where] = said;
        if (said.bad) return;
        for (const s of list) {
            this.touched.delete(s.name);
            this.settled.add(s.name);
        }
        return true;
    }

    // ONE SETTING SENT NOW, for a part of a device's tab or of a module's page that changes a setting by a press and keeps no Save of its own: the page's one way of sending (`/settings`) and of saying what came of it (form.saveSaid).
    // Answers {ok, words}: ok where the board took it, WORDS what a person reads of a refusal; ok false and the page's words for a board that did not answer where it did not.
    // KEEP is for a page being put away: the request goes with `keepalive`, which the browser finishes after the page is gone.
    async put(name, value, keep) {
        const one = this.b.setting(name);
        if (!one) return { ok: false, words: "" };
        const answer = await this.ask("/settings", form.body([[name, form.sent(one, String(value))]]), keep);
        if (!answer) return { ok: false, words: ui().didNotAnswer };
        const said = form.saveSaid(answer, ui(), this.b.everySetting);
        // what the board kept stands on the page at once, as the stream will say it a moment later: the number a press set does not jump back to the old one between the answer and the stream
        if (!said.bad && Array.isArray(answer.changed)) {
            this.take(answer.changed.map((c) => [dictionary.address(this.dict, "settings", c.name), { text: String(c.value) }]).filter(([at]) => at !== null));
        }
        return { ok: !said.bad, words: said.bad ? said.words : "" };
    }

    async restart() {
        const answer = await this.ask("/restart");
        if (answer) this.say(fill(ui().boardRestarts, { s: answer.back_in_s }), "warn");
    }

    ask_down() {
        this.state.asking = !this.state.asking;
        this.state.said = "";
    }

    typed(said) { this.state.said = said; }

    async down() {
        const answer = await this.ask("/down/" + encodeURIComponent(this.state.said));
        if (!answer) return;
        if (answer.error) return this.say(answer.error, "bad");
        this.state.asking = false;
        this.say(ui().boardGoesDown, "warn");
    }

    // One short request to the board. A refusal in words is the board's answer and belongs on the page; not
    // reaching the board at all is the page's own failure and reads differently.
    async ask(where, body, keep) {
        if (BOARD.error) return null;
        try {
            return await (await fetch(onBoard(where), keep ? { method: "POST", body: body, keepalive: true } : { method: "POST", body: body })).json();
        } catch (e) {
            this.say(ui().didNotAnswer, "bad");
            return null;
        }
    }
}

// WHAT THE DEVICE'S SCRIPT TAKES FROM THIS PAGE, and what it hands back (page.parts):
//   page.parts.tab({ id, label, component, absent })   a tab drawn before the modules and opened first; LABEL is the key of the menu's `ui` that says its
//                                                      name; `absent(board)` - true when the board's dictionary says there is nothing to draw -, a tab that is
//                                                      absent is not on the page; the component is given `act`, the page's way to send a request to the board, and `put`, its way to send one setting
//   page.parts.section(name, component)                the part a section of a module's page in the menu's tree asks for with `"part": NAME`; the component
//                                                      is given what the section says in its `given` field, as its prop `given`, and, on a module's page, `put`, the page's way to send one setting
// A tab or a part added twice under one name, and a section that asks for a part nobody added, are said on the page in the menu's words, and in the console.
const adding = (call) => {
    const before = parts.problems.length;
    call();
    parts.problems.slice(before).forEach((one) => console.error(`page: ${one.kind} ${one.key}`));
};
globalThis.page = {
    Component, xml, useState, useRef, board, ui, fill, useBoard, HoldButton, AskedPress, locks, pressOnce, pressWord,
    parts: {
        tab(spec) { adding(() => parts.tab(spec)); },
        section(name, component) { adding(() => parts.section(name, component)); },
    },
    start(target) { mount(Page, target); },
};
