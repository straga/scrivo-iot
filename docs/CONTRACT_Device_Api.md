# Device API contract

Contract version: `10`

The Rust burner talks to a client in data: it describes once what it has - the dictionary - and from then on sends only
addresses and values. The page on the board is one client; the page on a computer, a script and a home system talk to the board the
same way. There is no prose on the board: labels, phrase templates, the words for codes and the levels belong to each client. For the burner's page they are its
table of texts `texts.json` and its menu `menu.json` in the kit: the engine's page (`page/`) lays its own, the burner's page
(`devices/burner/page/`) adds its own, and `tools/page_kit.py` merges them. The board describes the burner and says nothing about where and how to show
it: the layout is the menu's business (section "Page menu").

Every device has a dictionary: the burner (`devices/burner/api`), the Sonoff Dual R3 (`devices/bench_dualr3/api`) and the S3 bench
board (`devices/bench_s3/api`). The entries they share - the board's clock and network, the relays, the wall switch, the translation of a setting into the
dictionary - live in `crates/dictionary` and are taken from there.

```sh
uv run --no-project python tools/state_contract.py <board-ip>   # /state against the dictionary of the same board
```

## How a client reads a device

1. `GET /dictionary` - the dictionary; compare `contract` with your own number. A different number, a smaller one too, means read
   nothing and say so in words: a dictionary of a smaller number does not say what a client of this number reads from it, and
   without the refusal the client would take an empty place for "the board has none of this".
2. `GET /events` - the stream. The first event carries all values, then only the changed ones. How often the board sends is the client's request: `GET /events?every=<s>` (below, "Stream").
3. A value goes to its address; what it means comes from the dictionary.
4. The stream opened again (the board restarted) - fetch `/dictionary` once more and compare `version`. A different version
   means the addresses mean something else: draw nothing and say so in words.

`GET /state` - all values at once, for a client without a stream: `{"contract": 10, "dictionary": "<version>", "values": [...]}`.
The client compares `dictionary` with the version of the dictionary it holds.

## The dictionary

`version` - the image version and eight characters of the dictionary's fingerprint: `stragabases3-26.09.24.13+3b6c823f`. The dictionary is a constant in
the image, served from flash.

| Field | What it is |
|---|---|
| `contract` | the number of this contract |
| `version` | the version of the dictionary |
| `device`, `board` | the device and the board |
| `terminal` | where the console is, a WebSocket |
| `cards` | cards - the board's modules, the names under which their rows and settings stand; they do not set the order of display |
| `things` | things; **the address of a thing is its place in this list** |
| `settings` | settings: `name`, `card` (absent - a setting of the board or of a module whose layout the page menu gives), `default`, `kind` (`choice`, `months`, `seconds`, `tenths`, `whole`); a choice has `choices` - `value`, `takes: true` if the choice takes a word of its own (stored as `<value>:<word>`), `shows` - names of settings of the same card that matter while this option is chosen, `needs` - `{among: [setting names], holds: value}`: the option is offered while one of them holds that value; a number has `from`, `to`, `unit`; a `tenths` number has its `from` and `to` in tenths of its `unit` (`s` for a time of the auger, `°C` for a target of the room; a board that says none means seconds, as the boards before it did), its value is a text with one digit after the point at most, and its refusal `not_tenths` carries the same `unit`; `level: "tuning"` - the setting is folded under the everyday ones; `effect` - what the client computes the "what a turn will give" line from (below) |
| `machine` | the board's automaton, if it has one (below) |
| `stream` | how the board sends: `every` - the default period in seconds (5), `live_s` - how long Live holds (600), `most` - the longest period one may ask for (60). These are the board's numbers, not the client's: the client takes them from here |

A thing:

| Field | What it is |
|---|---|
| `card`, `name` | the card and the code of the name (`flue_temp`, `pump_by`) |
| `part_of` | the code of the row the thing is a part of (the ip of `wifi`, the spread of `pin`); absent - the thing is a row itself |
| `kinds` | which kinds of value it carries |
| `unit` | the unit of the number |
| `words` | the codes of a word or a list of words |
| `reasons` | the codes of the reason there is no number now |
| `can` | commands: `switch`, `toggle`, `press`, `hold` |
| `range` | the limits of `hold`: `min`, `max`, `step`, `at` - the address of the part where the slider stands |
| `curves` | names of curves for `GET /curve/<name>` |
| `fixed` | a number built into the image; it does not come over the wire |
| `once` | comes when the stream opens and not after - the uptime; the client counts on by itself |
| `decimals` | how many places follow the point; the board rounds before it decides "did it change", so the number arrives already rounded - in an event, in `/state` and in a command's answer. No field - the number goes as it is |
| `counts` | `up` or `down`: seconds that run on their own - how long a stage lasts, how long is left to an event. The board sends the value when the stream opens and when it jumps (a new stage, a new target of the countdown, the number gone and back) - when it differs by more than a second from what the client has counted since the last send; in between the client counts: up adds, down subtracts to zero |
| `clock` | the row counts seconds to an event; the time a stage lasts and a sensor's silence count up and are never clocks |
| `irreversible` | the command of this thing cannot be taken back (zeroing the energy count): the client asks the person before it sends it. No field - nothing to ask. The field is only on a thing that takes a command. The contract number does not change: a client that does not know the field sends the command without asking, as it did before |

### The automaton and what a client draws from it

`machine` is a dictionary in which the board names what the client would otherwise have to know by name. The words, the order of the stages in
the cells and what to show large stay with the client.

| Field | What it is |
|---|---|
| `card` | the card of the automaton's rows |
| `fixed` | the card of the numbers built into the image: in the "it works with" table they stand beside the settings |
| `stage`, `says`, `cause` | rows: the stage, what it is doing now, at which stage it broke |
| `press` | the rows that are pressed to start and to stop, in order |
| `aside` | rows of the card that are not for its list (what the automaton commands, for the scheme of the installation) |
| `sections` | the numbers the automaton works with, by stage: `name`, `items` - lists of setting names or of image numbers (several in one item - one line at the client), `by` - the setting whose choice decides which numbers the stage shows |
| `now` | the rows of the summary card in the order of display: `{card, row}` - a row; `{card, part, is}` - a row of the card whose part holds the code (a sensor playing a role); `with` - a row of the same card whose number stands beside the word. The rows of a module that the menu lays out do not stand here |

`effect` of a setting is `{kind, uses}`: the kind of calculation the client knows (`gain`, `ramp`, `settle`, `share`), and for each
place of the calculation - a setting name (a string) or a number of the board `[card, row, part]`. The words of the line and what the client
chooses to show (on which miss it says the line) are its own; a client that does not know the kind does not say the line.

Settings are the rows of the card `settings`: the value of each is its stored text. What a setting says now (where the time
came from, where the draught sensor's zero stands on the pin, whether the flame thresholds are in order) are parts of its row, in codes and numbers.

## Page menu

A module that the menu lays out is described by the board only as being there: the name, the kind, the unit, the limits, the key, the names of the curves.
Where its card stands, what it is called, in which order and on which level its settings stand, where the row is in the summary card and
on the scheme, what the curves are labelled with and against which scale they are drawn - these are words of the menu, not of the dictionary. `menu.json` lies in the kit of
the page beside `texts.json`; the keys in it are dictionary names (`<card>`, `<row>`, `<row>.<part>`, a setting name,
a curve name), every text is `{"en": "...", "ru": "..."}`, an empty `ru` reads as `en`.

| What the menu says | Where |
|---|---|
| a card: `title`, `after` (which card it stands after) | `modules.<card>.card` |
| a row: `label`, `says`, `level`, `hide`, `look`; `now` - `after`, `as`, `beside`, `spec`; `scene` - `place`, `number` | `modules.<card>.rows.<row>` |
| a curve: `title`, `lines` - by the place of the line in the answer of `/curve`: `legend`, `span` | `rows.<row>.curves.<name>` |
| a setting: `label`, `unit`, `level` (`work` or `tuning`), `note` - the name of the part whose words it says | `modules.<card>.settings.<name>` |
| a cycle that a module runs (the automaton): `stage`, `says`, `cause` - the rows of the stage, of what it is doing and of where it broke; `press` - the buttons and `press_card` - the card they stand on (the Rust burner's is `thermostat`; without `press_card` the buttons are on the cycle's own card); `aside` - rows not for the card's list; `sections` - numbers by stage: `name`, `title`, `by`, `items` (`of` - names of settings or of image numbers, `label`, `spec`), `notes`. This is what the dictionary called `machine` for the first burner | `modules.<card>.cycle` |
| the words of a part of a setting, by code | `modules.<card>.notes.<name>.<part>` |

The answer of `/curve/<name>` of a module that the menu lays out carries `step_ms` and, for every line, `unit` and `points`; it has no `words` and `span` of a line.
The other modules say them in the answer while the menu has not taken them. A module that the menu does not name is drawn by the page from the
dictionary and the table of texts, as before: both forms live side by side while modules move over.

## A value on the wire

The kind comes from the dictionary, the value is short:

| Kind | Value |
|---|---|
| `on` | `true`, `false` |
| `number` | `61.5` - the unit is in the dictionary |
| `seconds` | `52` - whole seconds of the count |
| `word` | `"kindling"` - a code from `words` |
| `words` | `["auger", "pump"]` |
| `at_least` | `{"at_least": 300}` - the edge of the sensor's scale |
| `unknown` | `{"unknown": "own_zero"}` - there is no number now, the reason is a code from `reasons` |
| `text` | `{"text": "..."}` - where the words cannot be listed beforehand: the name of a time server |
| nothing to say | `null` |

A button (`kinds: ["button"]`) and a `fixed` number have no value and do not come over the wire.

## Stream

`GET /events?every=<s>`, `text/event-stream`. An event is an array of records: `data: [[address, value], ...]`. A reader that opened the stream
gets all values; after that only those that changed for it, **no oftener than once in `every` seconds**: the changes of a period
pile up and go out in one event, and the last value is the one it is at the end of the period. `every` is a whole number from 0
to `stream.most`; not asked, or asked wrongly - the default period (`stream.every`, 5 s). **`every=0` is Live:** every
change at once; `stream.live_s` seconds after the opening the board itself moves such a stream to the default period, so that a forgotten
tab does not keep the board at full tilt. The first event goes out at once, whatever the period. A quiet board sends a heartbeat `:` every
5 s - an empty event by which the board learns that a reader has gone. There are two streams; the third gets a `503` in words.

### Versions of the contract

| Version | What changed for a client, and why the number was raised |
|---|---|
| 4 | A number comes with the places the dictionary says (`decimals`), counters of seconds do not come every second (`counts`), the reader asks the period (`every`). A client of 3 would take a counter that does not arrive for a counter that stands still. |
| 5 | The answer to saving settings (`POST /settings`) is data, not phrases: accepted `{"changed": [{"name", "value"}]}`, refused `{"error": {"code", ...fields}}` (section "The answer to saving settings" below); the row `refused` of the automaton card is a word-code with parts `field`, `value`, `than`, `limit`, not a text; a new row `wanted` of the same card. A page of 4 would show raw data instead of words, and a page of 5 would not understand the words of a board of 4. |
| 6 | The decision "is heat needed and at which stage" moved from the burn automaton to the thermostat (`crates/thermostat`), which has its own card `thermostat` (section "Boiler: the card `thermostat`" below). The buttons `start` and `stop` and the row `wanted` left the card `automaton`; the four settings `auto_hold`, `auto_hold_c`, `auto_drop_c`, `auto_step` stand on the card `thermostat` and are stored under the key `s.thermostat` (the names of the settings are as before). A client of 5 would look for the buttons and "heat is needed" on the automaton's card and not find them. |
| 7 | The silence of the return and of the air (`silent_return`, `silent_room`) moved from the card `automaton` to the card `thermostat`, because the thermostat decides by these sensors; the rule is the same - seconds, empty while the sensor talks and while the role is not given. The silence of the draught, the flame, the flue and the water - of the fire - stayed on the card `automaton`. A client of 6 would look for these two rows on the automaton's card and not find them. |
| 8 | The page's "thermostat" is a floor above the water and has its own card `demand` (section "Demand: the card `demand`" below). The rows `silent_room`, `profile` and `source` with the parts `window` and `until`, the word `silent_room` in the row `heat`, the settings `auto_eco_c` and `auto_off_c` (water setpoints by profile) and the water mode `room` in `auto_hold` left the card `thermostat`; the profile, the windows and the silence of the room moved to the card `demand`, where the new rows `asked`, `why`, `target` and the settings `auto_asked_by`, `auto_room_c`, `auto_room_eco_c`, `auto_room_gap_c` are. A client of 7 would look for the profile and the silence of the room on the card `thermostat` and not find them. |
| 9 | The card `thermostat` has a third button, `light`, and the meaning of `start` changed: `start` only switches the heat on (`switch` becomes `on`) and lights nothing by itself - the fire is lit from a stop by the water, when the word of the water is `burn`; `light` lights the fire at once, whatever the water says, ends the term before a light and does not touch the switch. In a burning off and in a fault `light` does nothing and answers no error: the page shows the reason by the state of the board. A client of 8 would send `start` to light the fire and get a switch. |
| 10 | A time of the auger (the dose and the pause of a step, the lighting's dose, the doses of lighting a lost flame again) is a setting of the kind `tenths`: its `from` and `to` are in tenths of a second, and its value is a text with one digit after the point at most (`2`, `0.8`, `1.5`), kept as typed. A refusal of it has the code `not_tenths`, and its `from` and `to` are said in seconds, as texts (`"0.1"`, `"120"`). A client of 9 would read `0.8` as a number of seconds it cannot take. |

A client of another number refuses to read and says so; a page and a board of different numbers refuse each other and name both.

## Boiler: the card `thermostat`

The card `thermostat` is the layer of the water, on the page "Boiler": eleven rows, and the body of the decision does not go over the wire: the order to the fire (the word of the water, the stage, the switch, the counters of presses) lives between the thermostat and the automaton on the board. The page's "thermostat" is the floor above, the card `demand` (section "Demand" below).

| Row | Kind | What it is |
|---|---|---|
| `switch` | word `unasked`, `on`, `off` | whether heat was asked: `unasked` - the person has not pressed yet, `on` - the last press was "Start", `off` - "Stop"; the board remembers the last press after a restart |
| `switch_from` | word `person`, `memory` or `unknown` (reason `none`) | whose word it is: `person` - the person pressed after the board came up, `memory` - "Start" or "Stop" was pressed before a restart and the board returned it from flash; `none` - the switch is `unasked` |
| `heat` | a word or `unknown` (reason `starting` before the first order) | what the held temperature says about burning: `burn` - not above the setpoint minus the hysteresis, `keep` - inside the hysteresis band, `enough` - it reached the setpoint, `level` - the power mode (there is no water), `silent_return` - the held sensor has been silent too long; the word of the water already with the house's word laid over it (section "The house's word" below) |
| `silent_return` | seconds, counts up, or empty | how many seconds the return sensor has been silent while the thermostat holds by it, beside the reason `silent_return` in the row `heat`: since the sensor stopped giving a number (three periods of its polling); empty while the sensor talks and while no socket plays its role (`settings::plays`) |
| `rung` | word `big`, `mid`, `small` or `unknown` (reason `none`) | the stage at which the thermostat orders to burn: `big` - far from the setpoint (the power stage `auto_step`), `mid` and `small` - nearer to it; `none` - the mode does not hold a temperature or there is no reading yet. A middle one equal to the big or the small row of the table is called the one whose row it burns on |
| `left` | a number in °C or `unknown` (reason `none`) | by how many degrees the held temperature is below the setpoint, below zero - above it; the stage is chosen by this number |
| `setpoint` | a number in °C or `unknown` (reason `none`) | the setpoint of the water by which this second's order was made: `auto_hold_c`; `none` - the power mode, there is no water |
| `lock` | seconds, the clock runs backwards, or empty | how long is left until the thermostat lights the fire by itself again: the term after the end of burning; empty where there is no term - the fire burns, the term is over, it is not set, or the board did not see the end of burning |
| `start`, `stop`, `light` | a button (`press`) | "Start" switches the heat on and lights nothing by itself: the fire is lit by the water when it says `burn`; "Stop" switches it off, puts the fire out through burn-out and confirms an alarm; "Light" lights the fire from a stop at once, whatever the water says, past the term before a light and the first round of the sensors, and leaves the switch as it was (in a burning off and in a fault it does nothing) |

**The switch survives a restart of the board:** the last press of "Start" or "Stop" is written to flash (the key `m.heat_switch`, the word `on` or `off`; it is the module's memory, not a setting: the key lies in `nvs`, and laying a kit writes the files of `vfs`) and after the board comes up the switch is as it was, without a press from itself (a restart is not a person, and a press of "Light" is no word of the switch). One write per press that changes the word in flash: a repeat of the same press, the seconds and a restart do not write. The word counts as written when flash has taken it: a refusal leaves it waiting, and the board tries again on the following seconds, once a second, until flash takes it; a power cut while the word waits leaves what flash held before it. after a restart the burner stands in stop and is lit by the same doors as at any other time, the restart itself opens none: "Start" lights it when the water is below the setpoint by the hysteresis and the house is cold (the room under its band, the target less the room's hysteresis) or nobody plays the demand; a room inside its band does not light it; until the sensors have been asked all round once since the board came up the thermostat orders no light from a stop, whoever plays the demand (the room, the schedule, nobody) and whatever the water reads, the floor `auto_lock_c` included: for a stop a word of the water that would burn is said as `keep`; a fire that is lit, rising, working, smouldering or burning off takes the word of the water as it is (a smouldering fire goes back to work on water that burns); and a person's "Start" is not held back (the room's word in that time is `why`: `starting`, said with the word `warm`); after that round the order is the order of every other second: a room that answered decides, and a room that did not answer (`why`: `not_read`, the word `warm`) is a dead sensor that says neither warm nor cold, so the boiler is lit only by the floor of its water, the setting `auto_lock_c`: the water under the floor lights it, the water over the floor (the setpoint or under it) leaves it standing; "Stop" stays stop; a board on which nobody has ever pressed is `unasked` and neither lights the fire nor puts it out by itself. An alarm after a restart stays an alarm: the board first clears the pot and shows it again, and only "Stop" removes it, whatever the switch is. The protections - overheating and alarm - do not depend on the switch.

The stages by the approach to the setpoint: in the mode by the sensor the big stage is `auto_step` while the water is below the setpoint by more than `auto_mid_c`; the middle one is the row `auto_step_mid` when the distance to the setpoint is from `auto_mid_c` to `auto_small_c`; the small one is the row `auto_step_small` when it is less than `auto_small_c`. Down, the stage goes by the threshold, and up it returns when the water is farther than the threshold by the hysteresis `auto_drop_c`. The value `chosen` of `auto_step_mid` and `auto_step_small` is "as the power stage": there is no stage of its own, and by default it is so for both. The four settings - `auto_step_mid`, `auto_step_small`, `auto_mid_c`, `auto_small_c` - stand on the card `thermostat` beside `auto_hold`, `auto_hold_c`, `auto_drop_c`, `auto_step` and are seen only in the mode by the sensor.

**The house's word.** The thermostat reads the role `demand` and does not know who plays it. "Warm" (`warm`) is the word of the water `enough` at any water: the fire goes to smoulder and burns out to stop in its own time, and a standing fire is not lit; "cold" (`cold`) is the band of the water as it was, and in the power mode, where the water never says `burn`, it is `burn`; while nobody plays the role the water decides by itself, as before. A held sensor that is silent is stronger than both words: it is a protection, not a request. The term before lighting (below) stands after the house's word and holds the lightings by it, as it holds the lightings by the water.

The term before lighting: the setting `auto_lock_min` (minutes, 0 - no term) does not let the thermostat light the fire by itself sooner than that long after the end of burning. The end of burning is the second when the automaton stood in stop after a second when it burned (or burned out); a board that came up with the fire in stop did not see the end and counts nothing from power-up. While the term runs, `Burn` is ordered as `Keep` (the other words of the water do not change), the row `lock` counts the rest; the term runs in the thermostat, the house's word neither resets nor bypasses it: "cold" gives `Keep` until the end of the term too - also in the power mode, where "cold" itself orders `Burn`. Bypasses: "Light" (lights at once, the term ends, the journal says how much was left; "Start" is the switch and the term goes on)  and the water of the boiler below `auto_lock_c` (30 °C by default, strictly below; the water of the boiler is looked at in any mode, because the boundary says "the boiler has cooled", and the held air or return is always below it). A smouldering fire does not hold the term (it works again), the power mode knows no asking water and has no term, the protections are not touched by the term. Both settings are seen in the mode by the sensor. **The term does not survive a restart of the board, and the switch does:** the count is in RAM, a board that comes up with the fire out starts no term, and the switch comes back as it was (above) - a person who wants the fire at once presses Light, which the page offers as "light now" while the term runs.

## Demand: the card `demand`

The card `demand` is the floor above the water, on the pages "Thermostat" and "Schedule": who asks for heat now and why. Six rows and two parts of the row `source`; there is no body of the decision over the wire, the board gives codes and numbers, the words are the page's table.

| Row | Kind | What it is |
|---|---|---|
| `asked` | word `cold`, `warm` or `unknown` (reason `none`) | what the house says to the boiler; `none` - nobody plays the role (`auto_asked_by` is `none`), the boiler works from "Start" to "Stop" by its water |
| `why` | a word or `unknown` (reason `none`) | why: `below_target` - the room is below the target (or fell to it minus the room's hysteresis and has not risen to the target yet), `target_reached` - the room reached the target, `profile_off` - the profile in force has no target (profile `off`), `scheduled_on` - the schedule stands on a profile that heats, `scheduled_off` - on the profile `off`, `source_silent` - the source spoke and has been silent longer than the term of its role: the boiler holds its own water, because a house without heat is worse than extra heat, `not_read` - the first round of the sensors is over and the room did not answer: the house says neither warm nor cold (the word `warm` is shown) and the water is left to the floor `auto_lock_c` alone, until it is read, `starting` - the sensors have not been asked all round once since the board came up: the room's word is not heard (the word `warm` is shown); until the round is over the thermostat orders no light from a stop for any source of the demand, the floor included, and the room's word asks nothing of the water |
| `silent_room` | seconds, counts up, or empty | how many seconds the room sensor has been silent (three periods of its polling); empty while it talks and while no socket plays the room role |
| `profile` | word `comfort`, `eco`, `off` or `unknown` (reason `none`) | the profile that stands now; `none` - nobody asks |
| `source` | word `main`, `window`, `no_clock` or `unknown` (reason `none`) | what chose the profile: `main` - the main one, no window is open; `window` - a window, its number and end in the parts `window` and `until`; `no_clock` - windows are written and the board does not know the time, and the main profile stands |
| `source.window` | a number or `unknown` (reason `none`), unit `window` | the number of the open window from 1; `none` outside a window |
| `source.until` | a number of minutes from midnight or `unknown` (reason `none`), unit `min` | the minute of the local day when the window ends; `none` outside a window |
| `target` | a number in °C or `unknown` (reason `none`) | the target of the room under the profile that stands; `none` - the source is not a room or the profile has no target |

**Who asks** is the setting `auto_asked_by`: `none` (the default: the boiler works from "Start" to "Stop" by its water), `room` (the room sensor against the target of the profile) or `schedule` (the schedule alone). The choice `room` is taken only while some socket plays the room role. The room: the target is `auto_room_c` under the profile `comfort` and `auto_room_eco_c` under `eco` (under `off` there is no target, and the house is always "warm"); "warm" when the room has reached the target; "cold" when it is `auto_room_gap_c` below the target or lower; between these lines the previous word stays, and before the first word it is "cold". The numbers are whole, in degrees: the room's hysteresis is from 1 to 5 °C - a number of its own, not the water's hysteresis. The schedule as a source: the profile `off` is "warm", any other is "cold"; "burn several times a day" is written as the main profile `off` and windows with `comfort`. The source went silent (the room does not answer longer than the term of its role) - "cold" with the reason `source_silent`. "Start" allows work by demand and lights at once, "Stop" always puts the fire out.

**Profiles and windows by the clock:** a profile names the target of the room (`comfort`, `eco`) or "warm" without a target (`off`); the main profile `auto_profile` stands outside the windows. There are six windows, the settings `window_1`...`window_6`, each is `off` (no window) or `set:<line>`: `<days> <from>-<to> <profile>` or `<days> <from>+<N>h/<M>m <profile>`. Days are `daily`, a name (`mon`...`sun`), a range (`mon-fri`, `sat-mon` goes through the end of the week) and lists with commas; the time is `HH:MM`; `<from>` is not equal to `<to>`, and a window whose `<to>` is not later than `<from>` goes through midnight; N is from 1 to 24 hours, M is from 1 minute to N hours; the count "every N hours" starts anew every day from its `<from>` time and goes through the night to the `<from>` of the next day: five hours from 20:00 give 20:00, 01:00, 06:00, 11:00, 16:00, and 21:00 is not there, because 20:00 starts the count again, and the same times every day; the starts after midnight belong to the day the count began; the profile is `comfort`, `eco`, `off` or `none` (the window is written and switched off). A window belongs to the day it begins. Where two windows are open at once, the one with the smaller number acts. The board takes the time from the clock with the zone `zone`; while no time server has answered, the windows are silent, the main profile acts, `source` says `no_clock`. The schedule does not move the water setpoint: there is one water, `auto_hold_c`. A window line is not longer than 64 characters (the card is read from flash through one buffer).

The settings of the card: `auto_asked_by`, `auto_room_c`, `auto_room_eco_c`, `auto_room_gap_c`, `auto_profile`, `window_1`...`window_6`. The key in `nvs` is the card's, so `auto_profile` and the windows that stood on the card `thermostat` are not read under the new key: `tools/board_settings.py` carries them when it lays a kit. `auto_eco_c` and `auto_off_c`, the water setpoints by profile, do not exist.

## Commands

A request goes by an address from the dictionary. The answer is the values of the row and of its parts: `{"values": [[address, value], ...]}`, or
`{"error": "<why, in words>"}`. The same request twice gives the same state, except `toggle`.

| Command | Request | What it does |
|---|---|---|
| `switch` | `POST /on/<address>`, `POST /off/<address>` | switches on or off |
| `toggle` | `POST /toggle/<address>` | the other side of `switch` |
| `press` | `POST /press/<address>` | presses a button |
| `hold` | `POST /hold/<address>`, the body is a whole number | holds a number within `range` |

Refusals in words: the address is not a number, there is no such address, the address of a part, a command the thing does not take, a number outside the limits. A manual
command to an organ of the burner while the automaton runs the burner is turned back in words; `stop` always passes.

Other requests: `GET /curve/<name>` - a curve, `GET /journal` - the last 64 KB of the journal as text from the board's memory, the old on top (the first line is a note if
older lines remain on flash beyond the limit), `GET /journal?all` - the whole journal from flash, `POST /settings`
- save settings as pairs `name=value`, `POST /restart`, `POST /down/<board>` - go down to the recovery image,
WebSocket `/console` - the console.

### The answer to saving settings

The board answers in data, not phrases: the label of a setting, the words of a choice and the phrase of a refusal are the page's table (`menu.json`, `ui`),
in the language the page speaks.

- accepted: `{"changed": [{"name": "<setting>", "value": "<what is stored>"}, ...]}`; an empty list - nothing changed;
- refused: `{"error": {"code": "<code>", ...fields}}`. The fields are what the refusal stands on: `name`, `text` (what was sent), `from` and
  `to` (the limits of a number), `choices` (the values of a choice), `own` (the value of a choice that has a word of its own), `role`, `max`, `got`,
  `within_s`. The codes: `no_setting`, `not_months`, `not_seconds`, `not_tenths`, `not_whole`, `none_of`, `no_socket`, `not_kept`,
  `not_written`, `owner_busy`, `owner_silent` (the board, `settings::Refusal`; `not_written` has no fields - the text of the storage driver goes to the journal, not to the answer) and `value_too_long`, `nothing_to_save`,
  `form_too_long` (the page's contract, `page::refusal`). A code the page does not know it says as unknown, with the code itself.

The answer to the other actions of the page (`/on`, `/hold`, `/restart`...) is still `{"error": "<string>"}`.

### Refusal by the automaton's door

The row `refused` of the card `automaton` is a word-code from `automaton::REFUSALS` (`above_nought`, `not_below`, `below`...) and the parts
`field` (the field as `Tuning` writes it), `value` (the number that stood there), `than` and `limit` (the field and its number the order of two numbers
was checked against). No refusal - all `none`. The phrases by code are at the page, in `modules.automaton.rows.refused.says`.

## A client from another address

The answers a client reads carry `Access-Control-Allow-Origin: *`: a page from any address reads them. Anyone in the network could always send a
command; who may command the board is a question of the password sign-in, not of this header.
The burner's page is the same on the board and on a computer; the board's address is given to it by `?board=<host>[:port]`.

```sh
uv run --no-project python tools/page_stand.py --board <board-ip>    # then open http://127.0.0.1:8790/?board=<board-ip>
```

## A script without a page

`tools/board_client.py`:

```python
from board_client import Board, number
board = Board("<board-ip>")
number(board.value("draught", "blast_pressure"))     # 31.5, or None when there is no number now
board.command("on", "relays", "pump")                  # {"values": [[68, true]]}
board.stream(10, lambda at, size, records: print(records))
```

The client turns back an answer of another contract and of another dictionary before it reads a single value.
