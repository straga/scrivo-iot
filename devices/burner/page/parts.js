// What the burner adds to the engine's page (page/page.js): the drawing of the plant, the card of the automaton under it, the card of what asks
// for a person, and the tab that holds them - the main screen, drawn first and opened first. Every one of them knows the burner and none is
// the engine's: a board whose dictionary names no automaton has no such tab (`absent`), and the engine's own page is what it carries.
// What they read of the board is `page.board`; what they say is `page.ui` and the menu's `ui` the burner's page folder adds to.
//
// THE WHOLE FILE IS ONE BLOCK: the engine's page.js and this file are scripts of one page, and a `const` or a `class` at the top of a script is
// shared by all of them - `board` and `ui` here would be declared twice and the page would not start.
{
const { Component, xml, reactive, useState } = owl;
const { board, ui, fill, useBoard, HoldButton, locks, pressWord } = page;

// ONE PRESS AT A TIME, for the buttons of this file that send an absolute command by the state they see (page.pressOnce, the engine's; scheme.js, pressLock, says what settles each press): the burner's one button and its "light now"
// share a channel - both go to the thermostat -, the hands and the feed have one each. A press taken while the lock of its channel is held is not sent, and the button says `accepted` until the board has said the press.
const pressOnce = (channel, kind, want, things, send) => page.pressOnce(channel, scheme.pressLock, kind, want, things, send);

// HOW LONG THE HAND IS OFF THE PANEL before its numbers go to the board: every write is a write to the board's flash, and five presses of a person who goes from 20 to 25 are one
const TARGET_WAIT_MS = 2000;

// THE NUMBERS A HAND SET ON THE PANEL AND THE BOARD HAS NOT HEARD YET live HERE, and not in the card: the card is drawn away with its tab, and a number set a moment before the person went to another tab goes in its time and its refusal is said
// where the person finds it. What the panel says of them - the refusal, a counter that draws it again - stands here too. `put` is the page's (page.js), handed in by the card that is drawn.
const targetSaid = reactive({ tick: 0 });
const targetRefusals = scheme.refusals();
let targetPut = null;
const targetSender = scheme.targetSender({
    wait: TARGET_WAIT_MS,
    timer: { set: (fn, ms) => setTimeout(fn, ms), clear: (id) => clearTimeout(id) },
    send: async (name, value, keep) => {
        const answer = targetPut ? await targetPut(name, value, keep) : { ok: false, words: "" };
        targetRefusals.wrote(name, answer.ok, answer.words);
        targetSaid.tick++;
        return answer.ok;
    },
});
// A PAGE PUT AWAY SENDS WHAT THE PERSON HAS SET: they finished choosing, and the number has no other way to the board. The request is the kind that outlives the page (`keepalive`); a hold of the burner's buttons does the
// opposite on the same events, and rightly - a press that needs a hand held has no hand any more.
document.addEventListener("visibilitychange", () => { if (document.visibilityState === "hidden") targetSender.flush(); });
window.addEventListener("pagehide", () => targetSender.flush());
// THE PANEL OF THE TARGET OF THE ROOM, one component in two places: under the line of the reason on the scheme (given `card`) and as the section of the thermostat's page that the menu's tree asks for with `"part": "target"` (given `page`).
// The numbers a hand sets are held by the module (targetSender), not by this component, so a press made in one place and the page left for the other is heard once, in its time.
class TargetPanel extends Component {
    static props = { given: { type: String, optional: true }, put: { type: Function, optional: true } };
    static template = xml`
            <div t-if="panel" data-target-panel="" t-att-data-given="props.given" t-att-class="props.given === 'page' ? 'bg-panel border border-line rounded-[14px] p-3.5 shadow-card grid gap-2.5' : 'border-t border-line pt-3 mb-2 grid gap-2.5'">
                <h2 t-if="props.given === 'page'" class="text-[15px] text-ink font-semibold" t-esc="title"/>
                <div class="flex items-center justify-between gap-2.5">
                    <span class="text-[12px] text-dim" data-target-profile="" t-esc="words.target[panel.profile]"/>
                    <div class="flex items-center gap-1.5">
                        <button type="button" data-target-down="" t-att-aria-label="words.target.lower" t-att-disabled="!panel.canDown" class="min-w-[44px] min-h-[44px] rounded-[10px] border border-line bg-sunk text-ink text-[22px] leading-none active:bg-acc-soft disabled:opacity-40" t-on-click="() => this.tap('target', -1)" t-on-pointerdown="() => this.holdStart('target', -1)" t-on-pointerup="() => this.holdStop(true)" t-on-pointerleave="() => this.holdStop()" t-on-pointercancel="() => this.holdStop()">&#8722;</button>
                        <span class="min-w-[104px] text-center text-ink text-[34px] font-semibold leading-none tabular-nums" data-target-value=""><t t-esc="valueText"/><small class="text-[15px] font-medium text-dim"> <t t-esc="panel.unit"/></small></span>
                        <button type="button" data-target-up="" t-att-aria-label="words.target.raise" t-att-disabled="!panel.canUp" class="min-w-[44px] min-h-[44px] rounded-[10px] border border-line bg-sunk text-ink text-[22px] leading-none active:bg-acc-soft disabled:opacity-40" t-on-click="() => this.tap('target', 1)" t-on-pointerdown="() => this.holdStart('target', 1)" t-on-pointerup="() => this.holdStop(true)" t-on-pointerleave="() => this.holdStop()" t-on-pointercancel="() => this.holdStop()">+</button>
                    </div>
                </div>
                <div class="relative h-[30px]" aria-hidden="true" data-target-strip="">
                    <div class="absolute inset-x-0 top-[9px] h-[6px] rounded-[3px] bg-sunk"/>
                    <div class="absolute top-[9px] h-[6px] rounded-[3px]" data-strip-band="" t-att-style="'left:' + strip.from + '%;width:' + (strip.to - strip.from) + '%;background:var(--flow-hot);opacity:.75'"/>
                    <div class="absolute top-[2px] w-[4px] h-[20px] rounded-[2px]" data-strip-target="" t-att-style="'left:' + strip.target + '%;transform:translateX(-50%);background:var(--acc)'"/>
                    <div t-if="strip.room !== null" class="absolute top-[4px] w-[2px] h-[16px] bg-ink" data-strip-room="" t-att-style="'left:' + strip.room + '%'"/>
                    <span t-foreach="strip.ticks" t-as="tick" t-key="tick_index" class="absolute top-[18px] text-[11px] text-dim tabular-nums" t-att-style="'left:' + tick.at + '%;transform:translateX(-50%)'" t-esc="tick.label"/>
                </div>
                <div class="pt-2">
                    <button type="button" data-gap-toggle="" t-att-aria-expanded="ui_.gapOpen ? 'true' : 'false'" t-att-aria-label="gapAria" class="flex items-center gap-1.5 min-h-[44px] -my-[7px] text-[12px] text-dim tabular-nums" t-on-click="() => this.ui_.gapOpen = !this.ui_.gapOpen">
                        <span data-gap-value="">&#8722;<t t-esc="panel.gap.value"/></span>
                        <svg data-icon="flame" width="11" height="13" viewBox="0 0 11 13" class="shrink-0" aria-hidden="true"><path d="M5.5 0.5C6 3 9.5 4.8 9.5 8.2a4 4 0 0 1-8 0c0-1.6.8-2.7 1.8-3.6.1 1.2.6 1.9 1.2 2.2C4.6 4.6 4.6 2.3 5.5.5z" fill="currentColor"/></svg>
                        <span data-gap-fire="" t-esc="fireText + ' ' + panel.unit"/>
                    </button>
                    <div t-if="ui_.gapOpen" data-gap-panel="" class="flex items-center justify-end gap-1.5">
                        <button type="button" data-gap-down="" t-att-aria-label="words.target.gap_lower" t-att-disabled="!panel.gap.canDown" class="min-w-[44px] min-h-[44px] rounded-[10px] border border-line bg-sunk text-ink text-[18px] leading-none active:bg-acc-soft disabled:opacity-40" t-on-click="() => this.push('gap', -1)">&#8722;</button>
                        <span class="min-w-[64px] text-center text-ink text-[17px] font-semibold tabular-nums">&#8722;<t t-esc="panel.gap.value"/> <t t-esc="panel.unit"/></span>
                        <button type="button" data-gap-up="" t-att-aria-label="words.target.gap_raise" t-att-disabled="!panel.gap.canUp" class="min-w-[44px] min-h-[44px] rounded-[10px] border border-line bg-sunk text-ink text-[18px] leading-none active:bg-acc-soft disabled:opacity-40" t-on-click="() => this.push('gap', 1)">+</button>
                    </div>
                </div>
                <div class="min-h-[16px] text-[12px]" data-target-said="" t-att-class="saying ? 'text-bad' : 'text-dim'" t-esc="saying || (held ? words.target.pending : '')"/>
            </div>`;

    setup() {
        this.b = useBoard();
        // whether the hysteresis is open
        this.ui_ = useState({ gapOpen: false });
        // the refusal and the counter of the numbers on their way are the module's: drawn again when they change
        this.said_ = useState(targetSaid);
        if (this.props.put) targetPut = this.props.put;
    }

    get words() { return { target: {}, ...dictionary.schemeWords(this.b.texts) }; }
    // the title the menu's tree gives the section that asks for this part
    get title() {
        const section = dictionary.sectionsOf(dictionary.treeOf(this.b.texts)).find((x) => x.part === "target");
        return section ? section.title : "";
    }
    // THE TARGET OF THE ROOM (scheme.js, panelOf): the panel as the hand holds it, null where the board has no target of the room to change
    get panel() {
        this.said_.tick;
        const one = scheme.panelOf(this.b.things, this.b.everySetting);
        return one && this.props.put ? scheme.handled(one, (name) => targetSender.held(name)) : null;
    }
    get strip() { return scheme.stripOf(this.panel); }
    // what the board refused of the target and of the hysteresis the panel shows, each said until its own setting is written or pressed again
    get saying() { this.said_.tick; return targetRefusals.of([this.panel.setting, this.panel.gap.setting]); }
    // whether any number waits, of whichever profile's setting: a profile that changed in the wait leaves the wait said
    get held() { this.said_.tick; return targetSender.pending(); }
    get gapAria() { return fill(this.words.target.gap_aria, { gap: this.panel.gap.value, fire: this.panel.fire, unit: this.panel.unit }); }
    // the number of the target as it is said, and the fire's line beside the gap
    get valueText() { return scheme.valueText(this.panel); }
    get fireText() { return scheme.valueText({ value: this.panel.fire, tenths: this.panel.tenths }); }
    // a button held: the first step comes with the press's end (a click), and a hold goes on by itself from HOLD_FIRST_MS, every HOLD_EVERY_MS; the click that ends a hold that has stepped is not a step of its own
    holdStart(which, way) {
        this.holdStop();
        this.hold_ = { stepped: 0 };
        this.hold_.first = setTimeout(() => {
            this.push(which, way);
            this.hold_.stepped++;
            this.hold_.every = setInterval(() => { this.push(which, way); this.hold_.stepped++; }, scheme.HOLD_EVERY_MS);
        }, scheme.HOLD_FIRST_MS);
    }
    holdStop(released) {
        const hold = this.hold_;
        if (!hold) return;
        clearTimeout(hold.first);
        clearInterval(hold.every);
        this.hold_ = null;
        // the click that follows the release of a hold which went on by itself is swallowed once; a finger that slid off the button gives no click, and the next press is a step
        this.swallow_ = Boolean(released) && hold.stepped > 0;
    }
    tap(which, way) {
        if (this.swallow_) { this.swallow_ = false; return; }
        this.push(which, way);
    }
    // a press of a button of the panel: the next number of the target or of the gap, nothing at the limit
    push(which, way) {
        const one = which === "gap" ? this.panel.gap : this.panel;
        const next = scheme.stepped(one, way);
        if (next === null) return;
        targetRefusals.pressed(one.setting);
        targetSender.press(one.setting, next);
        targetSaid.tick++;
    }
}

// The burner drawn: hopper, auger, pot, flue and the water loop, with the board's own numbers standing where
// they are measured. The shape is taken from the sandbox (page_lab, class Scheme); what fills it is /state.
//
// A PICTURE OF THE PLANT AND NOT A DASHBOARD. The cards above already say every number in words; this answers
// the other question - WHERE in the boiler that number is - which a list of rows cannot answer at all.
class Scheme extends Component {
    static components = { HoldButton, TargetPanel };
    static props = { act: { type: Function, optional: true }, put: { type: Function, optional: true }, slots: { optional: true } };
    static template = xml`
        <section class="bg-panel border border-line rounded-[14px] p-3.5 shadow-card">
            <div class="flex items-center justify-between gap-2 mb-2">
                <h2 class="text-[15px] text-ink font-semibold" t-esc="words.title"/>
                <!-- THE THERMOSTAT'S BADGE: what it holds and to how much - "boiler 44 °C −2", "return 40 °C −2", "power 6 kW" -, between the word of the card and the buttons of the mode, in the
                     row where the card says what it is. One badge in each of the three modes; none where the board has not said what it holds. -->
                <span t-if="drawn.badge" t-att-data-badge="drawn.badge.mode" class="flex items-center gap-1 min-w-0 text-[12px] text-ink">
                    <svg data-icon="thermo" width="10" height="12" viewBox="0 0 10 12" class="shrink-0" fill="none" stroke="currentColor" stroke-width="1" stroke-opacity=".8" aria-hidden="true">
                        <rect x="2.8" y=".6" width="2.4" height="6.4" rx="1.2"/>
                        <circle cx="4" cy="9" r="2.6"/>
                        <path d="M7 2 h2 M7 4 h2 M7 6 h2"/>
                    </svg>
                    <span class="truncate tabular-nums" data-drawn="badge" t-esc="drawn.badge.text"/>
                </span>
            </div>
            <!-- WHY THE BURNER STANDS OR BURNS: one line under the title's row and above the drawing, as a thermostat's screen - "auto 19.6 → 22 °C", "auto 23.8 / 22 °C · ⏸ 1:58", "✋ 21.5 → 22 °C", "stop".
                 It is read off the values of the switch, the lock and the demand's rows (scheme.js, askOf); the colour of the text says whether the fire is asked for. THE PLACE OF ONE LINE, as tall as the button beside it, STANDS WHETHER IT SPEAKS OR NOT,
                 so a line that comes and goes does not move the drawing and the buttons under it. -->
            <!-- LIGHT NOW stands in the same row, right of the line that says why the burner stands: a press of Light. Its box is 44 tall as every button's, and its margins take the 14 off, so the row is as tall with it as without it. -->
            <div t-att-data-ask="drawn.ask ? drawn.ask.word : false" style="font-variant-emoji: text" class="mb-2 text-[12px] leading-tight min-h-[2lh] flex items-center gap-2" t-att-class="drawn.ask and drawn.ask.word === 'cold' ? 'text-ink' : 'text-dim'">
                <!-- THE LINE OPENS THE TARGET: pressed, the panel stands under it (scheme.js, panelOf); the line is the same line, underlined as what can be pressed -->
                <button t-if="panel" type="button" data-target-toggle="" t-att-aria-expanded="ui_.open ? 'true' : 'false'" class="flex-1 min-w-0 text-left border-b border-dashed border-line min-h-[44px] -my-[7px]" t-on-click="() => this.toggle()">
                    <span t-if="drawn.ask" data-drawn="ask" t-esc="drawn.ask.text"/>
                    <span t-else="" data-drawn="target" class="tabular-nums" t-esc="targetLine"/>
                </button>
                <span t-else="" class="flex-1 min-w-0"><span t-if="drawn.ask" data-drawn="ask" t-esc="drawn.ask.text"/></span>
                <HoldButton t-if="props.act and lightNow and light" attrs="{'data-light-now': '', 'data-phase': runPhase || false}"
                            look="'shrink-0 text-[12px] px-2 rounded-[10px] border min-h-[44px] -my-[7px] border-acc bg-acc-soft text-acc font-semibold min-w-[7.5rem]'"
                            disabled="locked" hint="words.run.hold" onHold="() => this.lightNowPress()"><span t-if="runPhase === 'accepted'" data-accepted="" class="inline-block w-1.5 h-1.5 mr-1.5 rounded-full bg-acc align-middle motion-safe:animate-pulse"/><t t-esc="word(runPhase, words.run.light_now, true)"/></HoldButton>
            </div>
            <!-- THE TARGET OF THE ROOM, under the line that says it: the panel (TargetPanel), the same one the thermostat's page holds. Closed it is not drawn and the card is as tall as without it. -->
            <TargetPanel t-if="panel and ui_.open" put="props.put" given="'card'"/>
            <div style="position: relative">
            <svg viewBox="0 0 320 250" class="w-full h-auto text-ink" font-family="var(--sans)">
                <path d="M14 18 h74 l-12 44 h-50 z" fill="none" stroke="currentColor" stroke-opacity=".45"/>
                <rect x="34" y="62" width="34" height="58" rx="4" fill="none" stroke="currentColor" stroke-opacity=".45"/>
                <text x="51" y="14" text-anchor="middle" class="fill-dim" font-size="10" t-esc="words.parts.hopper"/>
                <!-- THE FEED'S BUTTON IS DRAWN ON THE HOPPER, so a person sees there is one to press:
                     Pause in the colour of Start while the feed is allowed, Banned in amber while it is banned - one word each.
                     The press is the button laid over the hopper below. -->
                <g t-if="props.act and feed.allowed !== null">
                    <rect x="25" y="25" width="52" height="20" rx="6" stroke-width="1.2"
                          t-att-style="feed.allowed === false ? 'fill: none; stroke: var(--warn)' : 'fill: var(--acc-soft); stroke: var(--acc)'"/>
                    <text x="51" y="39" text-anchor="middle" font-size="11" font-weight="600"
                          t-att-style="feed.allowed === false ? 'fill: var(--warn)' : 'fill: var(--acc)'"
                          t-esc="word(feedPhase, feed.words, true)"/>
                </g>

                <!-- THE POT RUNS RIGHT UP TO THE FLUE AND THE WATER'S LOOP: the plug, the flame's word, its millivolts and
                     their names stand in it side by side and one under another, none over another -->
                <rect x="96" y="96" width="120" height="58" rx="8" fill="none" stroke="currentColor" stroke-opacity=".45"/>
                <text x="211" y="110" text-anchor="end" class="fill-dim" font-size="8" t-esc="words.parts.pot"/>

                <path d="M216 112 h30 v-92" fill="none" stroke="currentColor" stroke-opacity=".45"/>
                <!-- THE BOILER'S THERMOMETER, put in from above as on the boiler itself: a well up from the pot, its
                     degrees at the head -->
                <path d="M200 96 v-76" fill="none" stroke="currentColor" stroke-opacity=".45"/>
                <circle cx="200" cy="20" r="2" fill="currentColor" fill-opacity=".45"/>

                <!-- THE FAN, over the pot it blows into, the duct between them; it turns in the colour of a closed relay
                     when its mode is anything but off (scheme.js, fanOnScheme). Its setpoint is the words above it,
                     what the sensor reads the number beside it. -->
                <path d="M112 79 v17" fill="none" stroke="currentColor" stroke-opacity=".45"/>
                <circle cx="112" cy="70" r="9" fill="none" stroke="currentColor" stroke-opacity=".5"/>
                <path d="M112 70 l0 -7 M112 70 l6 4 M112 70 l-6 4" fill="none" stroke-width="2"
                      t-att-class="fan.turning ? 'stroke-ok' : 'stroke-dim'" stroke="currentColor"/>
                <text x="99" y="73" text-anchor="end" class="fill-dim" font-size="9" t-esc="words.parts.fan"/>
                <text x="252" y="14" text-anchor="end" class="fill-dim" font-size="10" t-esc="words.parts.flue"/>

                <!-- THE WATER LEAVES BY THE POT'S FLOOR: the room left of it is the auger's words, under it the igniter's -->
                <path d="M104 154 v62 h188 v-84 h-76" fill="none" stroke="currentColor" stroke-opacity=".45"/>
                <!-- THE FLOW OF THE WATER: the supply leaves the boiler by the left side and the floor of the loop, WARM; the return comes back up the right side, COLD, through the pump and into the boiler. The pump stands on the
                     return - the water goes from the boiler to the manifold by the supply, comes back by the return and is pushed into the boiler by the pump. The colours are the water's, never a level's, and do not change with the state. -->
                <g stroke="none">
                    <path d="M-3 -3 L4 0 L-3 3 Z" transform="translate(104 166) rotate(90)" class="fill-flow-hot"/>
                    <path d="M-3 -3 L4 0 L-3 3 Z" transform="translate(166 216)" class="fill-flow-hot"/>
                    <path d="M-3 -3 L4 0 L-3 3 Z" transform="translate(230 216)" class="fill-flow-cold"/>
                    <path d="M-3 -3 L4 0 L-3 3 Z" transform="translate(292 196) rotate(270)" class="fill-flow-cold"/>
                    <path d="M-3 -3 L4 0 L-3 3 Z" transform="translate(250 132) rotate(180)" class="fill-flow-cold"/>
                </g>
                <!-- THE MANIFOLD, a radiator on the floor of the loop with no word: the water gives its heat up where the supply goes, and the return that leaves it is cold. Ink, and not by the burner's state; the reference says what it is. -->
                <g data-drawn="manifold" fill="none" stroke="currentColor" stroke-opacity=".7" aria-hidden="true">
                    <rect x="188" y="209" width="20" height="14" rx="2" style="fill: var(--panel)"/>
                    <path d="M193 209 v14 M198 209 v14 M203 209 v14"/>
                </g>
                <text x="290" y="230" text-anchor="end" class="fill-dim" font-size="10" t-esc="words.parts.water"/>

                <g t-foreach="parts" t-as="part" t-key="part.key">
                    <circle t-att-cx="part.x" t-att-cy="part.y" r="7"
                            t-att-class="part.on ? 'fill-ok' : 'fill-none'"
                            stroke="currentColor" stroke-opacity=".5"/>
                    <!-- the label stands over its dot, or - where the dot is on a vertical pipe, as the pump is - beside it, to the left, so the pipe does not run through it -->
                    <text t-att-x="part.side === 'end' ? part.x - 11 : part.x" t-att-y="part.side === 'end' ? part.y + 3 : part.y - 11" t-att-text-anchor="part.side or 'middle'" font-size="10"
                          t-att-class="part.on ? 'fill-ok' : 'fill-dim'" t-esc="part.words"/>
                </g>

                <!-- THE MAINS UNDER THE POWER'S DOT: the voltage and the power the socket the burner is plugged into measures, "227 V · 4 W" (scheme.js, mainsOnScheme); not drawn while either has no number -->
                <text t-if="mains" x="44" y="194" text-anchor="middle" font-size="12" class="fill-ink" font-variant-numeric="tabular-nums" data-drawn="mains" t-esc="mains"/>

                <!-- WHAT EACH ORGAN DOES NOW, beside it (scheme.js, onScheme and PLACES), in the colour of a closed
                     relay (REF_Device_Page_Look, rule 5) -->
                <t t-foreach="actions" t-as="act" t-key="act.organ">
                    <text t-att-x="act.x" t-att-y="act.y" t-att-text-anchor="act.side or 'middle'" font-size="12"
                          font-variant-numeric="tabular-nums" t-att-class="act.look" t-esc="act.text"/>
                </t>

                <!-- THE FLAME: its word, the millivolts on its pin under it - a dash while the pin reads nothing, drawn in
                     every state - and its name, one under another. -->
                <text t-if="flame" x="166" y="115" text-anchor="middle" font-size="13"
                      t-att-class="flame.level === 'ok' ? 'fill-ok' : (flame.level === 'warn' ? 'fill-warn' : (flame.level === 'bad' ? 'fill-bad' : 'fill-dim'))"
                      t-esc="flame.words"/>
                <text x="166" y="132" text-anchor="middle" font-size="12" class="fill-dim" font-variant-numeric="tabular-nums"
                      t-esc="flameVolts"/>
                <text x="166" y="147" text-anchor="middle" font-size="9" class="fill-dim" t-esc="words.parts.flame"/>

                <g t-foreach="numbers" t-as="one" t-key="one.place">
                    <text t-att-x="one.x" t-att-y="one.y" t-att-text-anchor="one.side or 'middle'" font-size="13"
                          class="fill-ink" font-variant-numeric="tabular-nums" t-esc="one.value"/>
                    <text t-att-x="one.x" t-att-y="one.y + 12" t-att-text-anchor="one.side or 'middle'"
                          font-size="9" class="fill-dim" t-esc="one.words"/>
                </g>

                <!-- WHAT THE THERMOSTAT HOLDS DOES NOT STAND IN THE DRAWING: the badge is in the title row of the card, above. In the drawing it stood close over the boiler's number and read as its caption, and
                     in the air mode it would have said the boiler's water was held to the air's number. What stands here is what is about a thing of the drawing: the step of the fire and the water it burns until at the pot, while the fire works; and,
                     small and only while they act, the window of the schedule and the wait before a light, left of the boiler's name and its number: the first on the row of the name, the second on the row of the number -->
                <text t-if="drawn.step" x="211" y="166" text-anchor="end" font-size="10" class="fill-ok" font-variant-numeric="tabular-nums" data-drawn="step" t-esc="drawn.step"/>
                <text t-foreach="drawn.notes" t-as="note" t-key="note_index" t-att-x="note_index === 0 ? 158 : 134" t-att-y="note_index === 0 ? 38 : 26" text-anchor="end" font-size="8" class="fill-dim"
                      font-variant-numeric="tabular-nums" data-drawn="note" t-esc="note"/>

                <!-- THE PELLETS AT THE HOPPER: burned today, yesterday, this month, in all - four numbers, each beside its icon and no word among them; what each is, is the title it carries. Left of the auger's tube,
                     under the hopper, where nothing else stands; a number longer than four figures is drawn narrower and never into the tube -->
                <t t-if="pellets">
                    <g t-foreach="pellets" t-as="one" t-key="one.key" t-att-data-pellet="one.key" role="img" t-att-aria-label="words.pellets[one.key]" stroke="currentColor" fill="none" stroke-width="1" stroke-opacity=".8">
                        <title t-esc="words.pellets[one.key]"/>
                        <g t-att-data-icon="one.key" t-att-transform="'translate(1 ' + (one.y - 9) + ')'">
                            <rect width="10" height="10" stroke="none"/>
                            <t t-if="one.key === 'today'"><circle cx="5" cy="5" r="2.2"/><path d="M5 .5v1.6 M5 7.9v1.6 M.5 5h1.6 M7.9 5h1.6 M1.8 1.8l1.1 1.1 M7.1 7.1l1.1 1.1 M8.2 1.8l-1.1 1.1 M2.9 7.1l-1.1 1.1"/></t>
                            <t t-elif="one.key === 'yesterday'"><circle cx="6.6" cy="5" r="2.2"/><path d="M.6 5h2.6 M.6 5l1.5-1.5 M.6 5l1.5 1.5"/></t>
                            <t t-elif="one.key === 'month'"><rect x="1" y="1.8" width="8" height="7.4" rx="1"/><path d="M1 4.4 h8 M3.2 .6 v2 M6.8 .6 v2"/></t>
                            <t t-else=""><path d="M8.6 1.2 H1.4 L5.2 5 L1.4 8.8 H8.6"/></t>
                        </g>
                        <text x="12.5" t-att-y="one.y" font-size="10" class="fill-ink" stroke="none" font-variant-numeric="tabular-nums" data-drawn="pellet" t-att-data-key="one.key"
                              t-att-textLength="one.text.length > 3 ? 21 : false" lengthAdjust="spacingAndGlyphs" t-esc="one.text"/>
                    </g>
                </t>

                <!-- THE ZONE OF THE THREE MAIN NUMBERS: a short bar under the number, its tolerance as a band on it, the value as a mark - ink inside, the warning colour where the fire works and the mark is out -->
                <g t-foreach="zones" t-as="z" t-key="z.name" t-att-data-zone="z.name">
                    <rect data-zone-bar="" t-att-x="z.x" t-att-y="z.y" t-att-width="z.w" height="2.5" rx="1.2" class="fill-dim" fill-opacity=".35"/>
                    <rect data-zone-band="" t-att-x="z.x + z.w * z.from" t-att-y="z.y" t-att-width="z.w * (z.to - z.from)" height="2.5" class="fill-ink" fill-opacity=".75"/>
                    <rect data-zone-mark="" t-att-x="z.x + z.w * z.at - 0.9" t-att-y="z.y - 1.4" width="1.8" height="5.3" t-att-style="z.warn ? 'fill: var(--warn)' : 'fill: currentColor'"/>
                </g>
            </svg>
            <!-- THE FEED IS BANNED AND ALLOWED BY TOUCHING THE HOPPER: a button laid
                 over it, as wide and tall as the hopper drawn, the button drawn on it says what the press does. Its words are for whoever reads
                 the page without eyes and for the measure; on the screen the drawing is the button. Before the board
                 has said the feed, it stands idle. -->
            <button t-if="props.act" type="button" data-feed="" t-att-data-phase="feedPhase || false"
                    style="position: absolute; left: 4.4%; top: 7.2%; width: 23.1%; height: 17.6%; background: transparent; border: 0; outline: none; font-size: 0; color: transparent; cursor: pointer"
                    t-att-disabled="feed.allowed === null or feedLocked"
                    t-on-click="() => this.pressFeed()"
                    t-esc="feed.press"/>
            </div>
            <p t-foreach="gaps" t-as="gap" t-key="gap" class="text-[13px] text-bad mt-1" t-esc="gap"/>
            <t t-slot="default"/>
        </section>`;

    setup() {
        this.b = useBoard();
        this.since = {};
        this.locks = useState(locks);
        // whether the panel is open
        this.ui_ = useState({ open: false });
        // the refusal and the counter of the numbers on their way are the module's: drawn again when they change
        this.said_ = useState(targetSaid);
        targetPut = this.props.put || null;
    }

    // THE TARGET OF THE ROOM (scheme.js, panelOf): the panel as the hand holds it, null where the board has no target of the room to change. The line over the drawing opens it whatever the line says.
    get panel() {
        this.said_.tick;
        const one = scheme.panelOf(this.b.things, this.b.everySetting);
        return one && this.props.put ? scheme.handled(one, (name) => targetSender.held(name)) : null;
    }
    // where the board says nothing of why the burner stands, the line is the target itself: the number with its unit
    get targetLine() { return `${scheme.valueText(this.panel)} ${this.panel.unit}`; }
    toggle() { this.ui_.open = !this.ui_.open; }

    // whether the burner stands switched on with no fire and a Light would light it (scheme.js, lightNow), and the row of the thermostat's card the press goes to
    get lightNow() { return scheme.lightNow(this.b.things); }
    get light() {
        const m = this.b.machine;
        return m ? this.b.things.find((t) => t.group === (m.pressCard || m.card) && t.key === "light" && m.press.includes(t.key)) : null;
    }
    control(thing) { return wire.control(thing); }
    // "light now" sends Light, and the stop it ends is what the lock waits to see gone; the channel is the burner button's, so the two are never sent one over the other
    lightNowPress() {
        pressOnce("run", "light", true, () => this.b.things, () => this.props.act(this.control(this.light).path));
    }
    get locked() { return scheme.pressLock.held(this.locks.run); }
    // what the buttons of the press say: `accepted` from the press until the board has said it (scheme.js, pressLock.phase)
    get runPhase() { return scheme.pressLock.phase(this.locks.run); }
    get feedPhase() { return scheme.pressLock.phase(this.locks.feed); }
    word(phase, own, short) { return pressWord(phase, own, short); }

    // the thermostat's numbers for the drawing: the setpoint at the boiler, the step at the pot, what is small and acts only now (scheme.js, drawn)
    get drawn() {
        return scheme.drawn(this.b.things, this.b.everySetting, this.words, scheme.stairsOf(this.b.everySetting));
    }

    // A COMMAND TO A ROW OF THE AUTOMATON'S CARD BY THE NAME THE DRAWING KNOWS IT BY, sent to the board by its address: the drawing is the
    // one part of this page that knows the plant, and looks the row up here, in the dictionary.
    press(way, name) {
        const at = this.b.dict ? dictionary.address(this.b.dict, "automaton", name) : null;
        return at !== null ? this.props.act(`/${way}/${at}`) : false;
    }
    // the feed's button: Ban where it is allowed, Allow where it is banned - one press at a time, and the lock waits for the feed to be at the state pressed
    pressFeed() {
        const want = !this.feed.allowed;
        pressOnce("feed", "feed", want, () => this.b.things, () => this.press(want ? "on" : "off", "feed"));
    }
    get feedLocked() { return scheme.pressLock.held(this.locks.feed); }

    // the words the drawing says, the menu's (dictionary.js, schemeWords)
    get words() {
        const said = dictionary.schemeWords(this.b.texts);
        return { parts: {}, places: {}, feed: {}, run: {}, actions: {}, why: {}, heat: {}, target: {}, ...said };
    }

    // what each organ does now, where it stands on the drawing (scheme.js)
    get actions() {
        return Object.entries(scheme.PLACES)
            .map(([organ, place]) => {
                const label = scheme.onScheme(organ, this.b.things, this.words, this.b.everySetting);
                return label && { organ, ...place, text: scheme.schemeText(label), look: "fill-ok" };
            })
            .filter(Boolean);
    }

    // an organ the automaton commands that the drawing has no place for, in words (scheme.js, schemeGaps)
    get gaps() {
        return scheme.schemeGaps(this.b.things, this.words);
    }

    // the feed's switch by its value: allowed, banned or not said yet (scheme.js, feedLine)
    get feed() { return scheme.feedLine(this.b.things, this.words); }

    // the fan's picture (scheme.js, fanOnScheme)
    get fan() {
        return scheme.fanOnScheme(this.b.things);
    }

    // the row the menu puts in the pot (scheme.js, standing)
    get pot() { return scheme.standing(this.b.things, "pot"); }

    // the volts on the pot's pin, or a dash (scheme.js, flameVolts): the pin the menu names beside the pot's row
    get flameVolts() { return scheme.flameVolts(this.b.things, this.pot && this.pot.placeNumber !== undefined ? { group: this.pot.group, key: this.pot.placeNumber } : undefined); }

    // the pot's word (scheme.js, flameOnScheme)
    get flame() {
        return scheme.flameOnScheme(this.pot);
    }

    get parts() {
        const where = {
            auger: { x: 51, y: 96 },
            igniter: { x: 118, y: 126 },
            // ON THE RETURN BRANCH, the right side of the loop, on its way to the boiler; its label stands beside it, to the left, for a pipe runs up through it
            pump: { x: 292, y: 168, side: "end" },
            // BESIDE WHAT IT POWERS, and that is the modules on the bus - the ones that read the water. It used
            // to sit on the line of the flue, where there was room, and a person reads a dot on a line as a
            // thing standing in that line.
            aux_power: { x: 44, y: 174 },
        };
        return scheme.dots(this.b.things)
            .filter((dot) => where[dot.key])
            .map((dot) => ({ ...dot, ...where[dot.key] }));
    }

    // the voltage and the power of the mains at the power's dot, or none (scheme.js, mainsOnScheme)
    get mains() {
        return scheme.mainsOnScheme(this.b.things);
    }

    // the numbers where they are measured, from their values (scheme.js, schemeNumbers)
    get numbers() {
        return scheme.schemeNumbers(this.b.things, this.words);
    }

    // the pellets at the hopper, one row each - today, yesterday, the month, in all -, from the card `pellets` (scheme.js, pelletsOnScheme); none for a board without the card
    get pellets() {
        const rows = scheme.pelletsOnScheme(this.b.things);
        return rows && rows.map((one, at) => ({ ...one, y: 78 + at * 15 }));
    }

    // the zones of the draught, the boiler's water and the flame, each under its number (scheme.js, zonesOf and ZONE_PLACES); a mark is in the warning colour after it has stood out for a while (scheme.js, settle), which this card remembers between draws
    get zones() {
        const zones = scheme.settle(this.since, scheme.zonesOf(this.b.things, this.b.everySetting), Date.now());
        return Object.entries(scheme.ZONE_PLACES).filter(([name]) => zones[name]).map(([name, place]) => ({ name, ...place, ...zones[name] }));
    }
}

// WHAT THE AUTOMATON'S CARD SAYS, read the same way by the card of its rows and its numbers: the card, and the rows that say the stage,
// what the automaton does and where a fault happened, are the board's word - `machine` in its dictionary -, never a name written here.
class AutomatonPart extends Component {
    get machine() { return this.b.machine; }
    // the rows of the automaton's card, drawn ones
    mine() {
        const m = this.b.machine;
        return m ? this.b.things.filter((t) => t.group === m.card) : [];
    }
    // the row the board calls KEY: its words for a person to read, and the word its value is - what the page decides by
    said(key) {
        const thing = this.mine().find((t) => t.key === key);
        return thing ? thing.state : null;
    }
    word(key) {
        const thing = this.mine().find((t) => t.key === key);
        return thing && thing.value && thing.value.word ? thing.value.word : null;
    }
    // what the page's table says of the row the board calls KEY of the automaton's card
    table(key) {
        const m = this.b.machine;
        return m ? dictionary.textsOf(this.b.texts, { card: m.card, name: key }) : {};
    }
    get stage() { return this.machine ? this.word(this.machine.stage) : null; }
}

// THE BURNER RUN FROM UNDER ITS SCHEME: ONE BUTTON, with the action on it - Stop while the switch is on, Start while it is off - and, where a press would be taken and not carried out, the reason under it
// (scheme.js, runButton). Start is the acting colour (REF_Device_Page_Look, rule 1) and Stop sits quiet on the card's sunken colour.
// The press goes to the row of the thermostat's card the menu names as pressed (`press` of the cycle). The feed is banned on the drawing's hopper.
class Controls extends Component {
    static components = { HoldButton };
    static props = { act: Function };
    static template = xml`
        <div class="mt-3 pt-3 border-t border-line">
            <HoldButton t-if="button" attrs="{'data-run': '', 'data-press': run.press, 'data-phase': phase || false}"
                        look="'w-full text-sm px-3.5 py-1.5 rounded-[10px] border min-h-[44px] ' + look"
                        disabled="run.cannot !== null or locked" hint="said.hold" onHold="() => this.press()"><span t-if="phase === 'accepted'" data-accepted="" class="inline-block w-1.5 h-1.5 mr-1.5 rounded-full bg-acc align-middle motion-safe:animate-pulse"/><t t-esc="word(phase, said[run.press])"/></HoldButton>
            <p t-if="run.cannot" data-run-cannot="" class="text-[13px] text-dim mt-1.5" t-esc="said[run.cannot]"/>
        </div>`;

    setup() {
        this.b = useBoard();
        this.locks = useState(locks);
    }

    // the press of the moment, one at a time: it waits for the switch to say the word pressed - on for Start, off for Stop (scheme.js, pressLock)
    press() {
        const button = this.button;
        if (!button) return;
        const press = this.run.press;
        pressOnce("run", "run", press === "start" ? "on" : "off", () => this.b.things, () => this.props.act(this.control(button).path));
    }
    get locked() { return scheme.pressLock.held(this.locks.run); }
    get phase() { return scheme.pressLock.phase(this.locks.run); }
    word(phase, own) { return pressWord(phase, own, false); }

    // the press of the moment by the values of the switch and the stage (scheme.js, runButton)
    get run() { return scheme.runButton(this.b.things); }
    // the words of the button and of the reason, the menu's (`scheme.run`)
    get said() { return dictionary.schemeWords(this.b.texts).run || {}; }
    // the row the press goes to: the one of the card's pressed rows the button names
    get button() {
        const m = this.b.machine;
        return m && this.run.press ? this.b.things.find((t) => t.group === (m.pressCard || m.card) && t.key === this.run.press && m.press.includes(t.key)) : null;
    }
    get look() {
        return this.run.press === "start" ? "border-acc bg-acc-soft text-acc font-semibold" : "border-line bg-sunk";
    }
    control(thing) { return wire.control(thing); }
}

// THE AUTOMATON'S CARD AND ITS NUMBERS, under the controls: the stage as a strip - where the fire is and where it goes -,
// what the automaton does, what the stage means, then one row for every thing of its
// card - the clocks said as one, what comes next - and its numbers stage by stage, each saying
// whether it is set on this page or fixed in the image. What reads how is burning.js. Its settings are not here: they stand in the menu's tree.
// WHAT ASKS FOR A PERSON: the failures that are active, in one card under the scheme's, each with the one thing to do - the menu's words for the cause's code (scheme.js, attention). It is not on the page while nothing asks, and it stands under
// the scheme's card and not over it, so the buttons of Start and Stop - the Stop that acknowledges a fault - do not move when it comes. Outlined, not filled (REF_Device_Page_Look, rule 1): in the colour of the worst of what it lists.
class Attention extends Component {
    static props = {};
    static template = xml`
        <section t-if="items.length" data-card="attention" class="bg-panel border rounded-[14px] p-3.5 shadow-card"
                 t-att-style="'border-width: 2px; border-color: var(--' + worst + ')'">
            <h2 class="text-[15px] text-ink font-semibold mb-1" t-esc="words.attention.title"/>
            <ul>
                <li t-foreach="items" t-as="one" t-key="one.key" t-att-data-attention="one.key" class="py-1.5 border-b border-line last:border-b-0">
                    <div data-what="" class="text-[13px] first-letter:uppercase" t-att-class="one.level === 'bad' ? 'text-bad' : 'text-warn'" t-esc="one.words"/>
                    <div data-action="" class="text-[14px] text-ink" t-esc="one.action"/>
                </li>
            </ul>
        </section>`;

    setup() {
        this.b = useBoard();
    }

    get words() {
        return { attention: {}, ...dictionary.schemeWords(this.b.texts) };
    }

    get items() {
        return scheme.attention(this.b.things, this.words, this.b.everySetting);
    }

    // the colour of the worst: a failure is the colour of bad, and a card of warnings alone is the colour of warn
    get worst() {
        return this.items.some((one) => one.level === "bad") ? "bad" : "warn";
    }
}

class Burning extends AutomatonPart {
    get ui() { return ui(); }
    // GIVEN says which of its two panels: the readings (the stage as a strip, what it does, its rows) on the main screen and on the automaton's page, the table of what it works by on the automaton's page
    static props = { given: String, put: { type: Function, optional: true } };
    setup() {
        this.b = useBoard();
    }
    static template = xml`
        <t t-if="machine">
        <section t-if="props.given === 'readings'" class="bg-panel border border-line rounded-[14px] p-3.5 shadow-card">
            <h2 class="text-[15px] text-ink font-semibold mb-2 first-letter:uppercase" t-esc="heading"/>
            <!-- THE WHOLE WIDTH GOES TO THE WORDS: at 375 six cells have about fifty pixels each, and "burn-off" at
                 12 px did not fit and was cut to "burn-..." -->
            <!-- A CELL IS AS WIDE AS ITS WORD AND THE REST OF THE LINE IS SHARED: equal cells cut the longest Russian word ("разгоран…") -->
            <div class="flex gap-0.5 mb-2" data-strip="stage">
                <span t-foreach="strip" t-as="cell" t-key="cell.key"
                      class="flex-auto text-center text-[10px] leading-tight rounded-[6px] px-0.5 py-1 whitespace-nowrap"
                      t-att-class="cellLook(cell)"
                      t-esc="cell.words"/>
            </div>
            <!-- ONE LINE FOR WHAT IT DOES: the stage's word and, after a failure, the cause as a code - never a phrase (REF_Device_Page_Look, rule 7); what a stage means is the reference's. In smouldering the numbers
                 of it follow the word, and the room of two lines is always there so the card keeps its height -->
            <p class="text-[14px] min-h-[2lh] mb-2 tabular-nums" t-att-class="stopped ? 'text-bad' : 'text-ink'" t-att-data-level="stopped ? 'bad' : false" data-line="doing"
               t-esc="doingLine"/>
            <div t-foreach="rows" t-as="row" t-key="row.name"
                 class="flex items-center gap-3 py-1.5 border-b border-line last:border-b-0">
                <span class="text-dim shrink-0 min-w-[4.5rem]" t-esc="row.name"/>
                <span class="flex-1 min-w-0 text-right break-words"
                      t-att-data-level="row.level or false"
                      t-att-class="{ 'text-bad': row.level === 'bad',
                                     'text-warn': row.level === 'warn', 'text-dim': row.level === 'dim' }"
                      t-esc="row.value"/>
            </div>
        </section>
        <section t-if="props.given === 'works'" class="bg-panel border border-line rounded-[14px] p-3.5 shadow-card">
            <h2 class="text-[15px] text-ink font-semibold mb-1" t-esc="ui.howItRuns"/>
            <div t-foreach="parameters" t-as="row" t-key="row.stage" class="py-1.5 border-b border-line last:border-b-0">
                <div class="text-[13px] font-semibold mb-0.5" t-esc="row.stage"/>
                <div t-foreach="row.parts" t-as="part" t-key="part.words" class="flex gap-3 text-[13px]">
                    <span class="flex-1 min-w-0 text-dim" t-esc="part.words"/>
                    <span class="text-right" t-att-class="part.set ? 'text-acc' : 'text-dim'" t-esc="part.value === null ? ui.notSent : part.value"/>
                </div>
            </div>
        </section>
        </t>`;

    // what the card is headed by: its title in the menu, or its own name
    get heading() { return dictionary.cardTitle(this.b.texts, this.machine.card); }

    // what it does: the stage's word and its attempt or cause, and in smouldering the numbers of it after the word (scheme.js, smoulderLine)
    get doingLine() {
        const said = this.said(this.machine.says) || this.ui.starting;
        const tuning = scheme.smoulderOf(this.b.everySetting);
        const more = tuning ? scheme.smoulderLine(this.b.things, tuning, dictionary.schemeWords(this.b.texts)) : "";
        return more ? `${said} \u00b7 ${more}` : said;
    }

    // the page's table of the stage's cells and what each means, and the stage the board is in and where a fault happened
    get spec() { return this.machine ? this.table(this.machine.stage) : {}; }
    get strip() { return burning.strip(this.spec.strip, this.stage, this.word(this.machine.cause)); }
    // a stage the page's table marks bad - a fault - is said in red
    get stopped() { return ((this.spec.strip || {}).marks || {})[this.stage] === "bad"; }
    // A FAULT IS NOT FILLED: a solid fill is spent on what cannot be taken back, and a fault is cleared by Stop. It
    // is ringed in red, a burning off's cause in amber, inside the cell so the strip keeps one height.
    cellLook(cell) {
        if (cell.level === "bad") return "bg-sunk text-bad font-semibold ring-1 ring-inset ring-bad";
        if (cell.now) return "bg-acc text-acc-ink font-semibold";
        if (cell.level === "warn") return "bg-sunk text-warn ring-1 ring-inset ring-warn";
        return "bg-sunk text-dim";
    }
    // every row of the card but the ones the board names for something else - the stage, what it does, where a fault happened, what is
    // pressed, what is for the drawing -; the feed stands here too, in the board's words of what a ban does. The clocks give way to one row of
    // what comes next, standing where the first of them stood
    get rows() {
        const m = this.machine;
        const apart = new Set([m.stage, m.says, m.cause, ...m.aside, ...m.press]);
        const rows = [];
        let nextSaid = false;
        for (const t of this.mine()) {
            if (apart.has(t.key) || t.fixed) continue;
            if (burning.isClock(t)) {
                if (!nextSaid) rows.push({ name: this.ui.next, ...burning.next(this.mine()) });
                nextSaid = true;
                continue;
            }
            rows.push({ name: t.name, value: t.state, level: t.level });
        }
        return rows;
    }
    get parameters() {
        const m = this.machine;
        const every = this.b.everySetting;
        return burning.parameters(m, m.words || this.b.texts.sections, [...every, ...dictionary.chosenStep(dictionary.treeOf(this.b.texts), every)], this.b.things.filter((t) => t.group === m.fixed));
    }
}

// THE MANUAL CONTROL, a section of the Service: ONE BUTTON, Take by hand or Give back to the automaton, by the value of the automaton's switch `semi`, and under it what that press does - the automaton keeps running and lights the fire
// itself, and a part the hand holds stays as set until the automaton decides it again; giving back, it takes the parts the hand held (scheme.js, handsLine). The rows of the relays and the fan's hands stand in the sections under it (the menu's tree).
class Hands extends Component {
    static props = { act: Function, given: { optional: true } };
    static template = xml`
        <section class="bg-panel border border-line rounded-[14px] p-3.5 shadow-card">
            <h2 t-if="title" class="text-[15px] text-ink font-semibold mb-2" t-esc="title"/>
            <button type="button" data-hands="" t-att-data-phase="phase || false" t-att-aria-pressed="line.semi === null ? false : line.semi"
                    class="w-full text-sm px-3.5 py-1.5 rounded-[10px] border min-h-[44px]"
                    t-att-class="line.semi === true ? 'border-acc bg-acc-soft text-acc font-semibold active:bg-acc active:text-acc-ink' : 'border-line bg-sunk active:bg-acc-soft'"
                    t-att-disabled="line.semi === null or locked" t-on-click="() => this.press()"><span t-if="phase === 'accepted'" data-accepted="" class="inline-block w-1.5 h-1.5 mr-1.5 rounded-full bg-acc align-middle motion-safe:animate-pulse"/><t t-esc="word(phase, line.button)"/></button>
            <p data-hands-text="" class="text-[13px] text-dim mt-1.5 min-h-[2lh]" t-esc="line.text"/>
        </section>`;

    setup() {
        this.b = useBoard();
        this.locks = useState(locks);
    }
    get locked() { return scheme.pressLock.held(this.locks.hands); }
    get phase() { return scheme.pressLock.phase(this.locks.hands); }
    word(phase, own) { return pressWord(phase, own, false); }

    get line() { return scheme.handsLine(this.b.things, { hands: {}, ...dictionary.schemeWords(this.b.texts) }); }
    // the title the menu's tree gives the section that asks for this part
    get title() {
        const tree = dictionary.treeOf(this.b.texts);
        const section = dictionary.sectionsOf(tree).find((s) => s.part === "hands");
        return section ? section.title : "";
    }
    // Take or Give back, one press at a time: the lock waits for the switch to be at the state pressed
    press() {
        const at = this.b.dict ? dictionary.address(this.b.dict, "automaton", "semi") : null;
        if (at === null || this.line.semi === null) return;
        const want = !this.line.semi;
        pressOnce("hands", "hands", want, () => this.b.things, () => this.props.act(`/${want ? "on" : "off"}/${at}`));
    }
}

// THE MAIN SCREEN: the burner as a drawing, and under it what the automaton does with it. WHAT HAPPENS STANDS ABOVE WHAT IS SET:
// a person looks at the automaton every minute and at the settings once a week - the engine puts the menu's main sections under this tab's parts.
class SchemeTab extends Component {
    static components = { Scheme, Controls, Attention, Burning };
    static props = { act: Function, put: Function };
    static template = xml`
        <Scheme act="props.act" put="props.put">
            <Controls act="props.act"/>
        </Scheme>
        <Attention/>
        <Burning given="'readings'"/>`;
}

// HOW CLEAN THE BOILER IS, a part of the main screen under the scheme and its cards (the menu's `main` section `soot`): the percent the page makes of the number the board sends and the two edges a person set, the kilograms
// burned since the cleaning, the day of it, and the button of the cleaning. The degrees the percent is made of are not a row: a person cannot tell what that difference says, and the help of the row says it in words (the menu's `row:soot/soot_c`). The words are the menu's (`scheme.soot`); what asks for the cleaning is not here but in the card of what asks for a person (scheme.js, attention). The board sends numbers only.
// The cleaning cannot be taken back (the board says so: `irreversible`), so the button asks first and the second press sends: it is the page's one asking button (page.AskedPress), the same as the rows of the cards use.
class SootCard extends Component {
    static components = { AskedPress: page.AskedPress };
    static props = { act: Function, given: { optional: true } };
    static template = xml`
        <section t-if="soot" data-card="soot" class="bg-panel border border-line rounded-[14px] p-3.5 shadow-card grid gap-2">
            <h2 class="text-[15px] text-ink font-semibold" t-esc="words.soot.title"/>
            <div class="flex items-baseline justify-between gap-2.5" data-soot-row="clean">
                <span class="text-[12px] text-dim" t-esc="words.soot.clean"/>
                <span class="text-ink text-[34px] font-semibold leading-none tabular-nums" data-soot-value="percent" t-esc="percentText"/>
            </div>
            <div class="flex items-baseline justify-between gap-2.5" data-soot-row="since">
                <span class="text-[13px] text-dim" t-esc="words.soot.since"/>
                <span class="text-[15px] text-ink tabular-nums" data-soot-value="since" t-esc="sinceText"/>
            </div>
            <div class="flex items-baseline justify-between gap-2.5" data-soot-row="cleaned">
                <span class="text-[13px] text-dim" t-esc="words.soot.cleaned"/>
                <span class="text-[15px] text-ink tabular-nums" data-soot-value="cleaned" t-esc="cleanedText"/>
            </div>
            <div t-if="button" class="flex flex-wrap items-center gap-3 pt-1">
                <AskedPress channel="'soot/clean'" ask="asks" label="words.soot.button" thing="'soot/clean'" send="() => this.props.act(control.path)"/>
            </div>
        </section>`;

    setup() {
        this.b = useBoard();
    }

    get words() { return { soot: {}, ...dictionary.schemeWords(this.b.texts) }; }
    // None for a board with no soot card: the section draws nothing
    get soot() { return scheme.sootOf(this.b.things, this.b.everySetting); }
    get button() { return this.b.things.find((t) => t.group === "soot" && t.key === "clean") || null; }
    // the number as a person reads it: the percent where both edges are known, a dash where the board has no number yet
    get percentText() { return this.soot.percent === null ? this.words.soot.none : `${this.soot.percent} %`; }
    get sinceText() { return this.soot.kg === null ? this.words.soot.none : `${this.soot.kg.toFixed(1)} ${dictionary.unit("kg")}`; }
    // the day of the cleaning as the date alone; what it is, is the label's
    get cleanedText() { return this.soot.day === null ? this.words.soot.none : scheme.dayText(this.soot.day); }
    // the button's way to send: the board's own path of the row, by its address
    get control() { return wire.control(this.button); }
    get asks() { return Boolean(this.control && this.control.ask); }

}

page.parts.tab({ id: "scheme", label: "scheme", component: SchemeTab, absent: (board) => !board.machine });
page.parts.section("burning", Burning);
page.parts.section("hands", Hands);
page.parts.section("target", TargetPanel);
page.parts.section("soot", SootCard);
}
