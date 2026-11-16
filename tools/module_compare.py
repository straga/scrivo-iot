"""The new burner's modules against the first burner's, second by second, on one scenario.

    uv run --no-project python tools/module_compare.py [SCENARIO] [--modules flame,water,flue] [--frames-new FILE] [--frames-reference FILE]

SCENARIO is a file of tools/stand_scenarios (default flame.json): the readings by the second - the millivolts on the flame's pin, the degrees of the sockets and the flue - and the roles
of the sockets. Each side makes its frames with ITS OWN code: the first burner's by crates/automaton_card (example `scenario`), the new one's by the modules' mapping in
devices/burner/api (example `burner_frames`) - the logic of the readings (crates/flame, crates/burner_map, crates/modbus, crates/max6675) is the one thing both share, and it is what the
burner is meant to keep. Every frame is read through the dictionary of ITS side, so an address that means another thing on the other side is no match, and the entities are compared by
name: `flame/flame`, `flame/pin`..., `temperatures/ntc_1`, `flue/flue_temp`. Exits 1 when a second differs, and says which, and what each said.

WHAT THE FIRST BURNER'S STAND SIMULATES is what the two are held to: the sockets whose role is the boiler's, the return's or the room's - with a line for them in the scenario - and the
flue as a number or as nothing read. A silent bus, an empty socket, an open thermocouple and a chip that says nothing are the new burner's alone (its API says them, the first burner's
stand does not make them up): they are taken out of the scenario before either side plays it, and held by the rows in devices/burner/api/tests instead.
"""
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

# What the flame says, by the name a client finds it by.
FLAME = ["flame/flame", "flame/pin", "flame/pin.spread", "flame/pin.edge", "flame/pin.window"]
# The roles the first burner's stand gives a socket's reading to (crates/automaton_card, example `scenario`).
SIMULATED_ROLES = ["boiler", "return", "room"]
# The pressure the first burner's stand makes up, and the sensor behind a divider of a half that would give it: 0.5 V is no pressure, 4 V of span are 300 Pa.
ZERO_MV, SPAN_MV, FULL_PA = 500.0, 4000.0, 300.0
# What a pin shows when the stand says the fan stands (under the sensor's own zero, alive), and what it shows past the top of the scale.
STILL_PIN_MV, OVER_PIN_MV = 161.0, 2400.0
# The organs the first burner's stand drives by its automaton - the fourth, the power of the modules, it leaves as it is - and what it says of them.
ORGANS = ["auger", "igniter", "pump"]
RELAYS = ["relays/auger", "relays/auger.off_in", "relays/igniter", "relays/igniter.off_in", "relays/pump"]
# What the first burner's stand says of the fan: the mode, the state, the draught asked and the one that came out, the slider and the purge. NOT THE ANGLE: its stand makes it up - 40 % held, 100 % purging - and the new burner
# runs the regulator of crates/regulator on the draught it is given, so the two angles are not the same kind of number.
FAN = ["fan/holding", "fan/holding.state", "fan/holding.asked", "fan/holding.came_out", "fan/draught", "fan/purge"]
# What the first burner's stand says of the automaton, by the name a client finds it by: NOT the silence of the sensors (its stand does not make it up and its card says nothing of it). The organs the automaton commands are
# held with it - the relays it closes and opens - and what it asks of the fan: the mode, the pascals asked, the slider, the purge. NOT THE FAN'S STATE, THE DRAUGHT THAT CAME OUT OR THE PRESSURE ON THE DRAUGHT'S CARD: the first burner's
# stand makes them up - the pressure of the same second the fan is asked, 40 % held -, and the new one runs the regulator on a draught that follows the ask a second later, as a sensor's does. They are the stand's, not the automaton's.
CLOCKS = ["lighting_in", "dose_ends_in", "igniter_off_in", "attempt_ends_in", "fed_after_flame_for", "full_step_in", "auger_switches_in", "relight_in",
          "no_relight_burning_off_in", "burning_off_in", "burning_out_ends_in", "blowing_clear_ends_in"]
AUTOMATON = (["automaton/%s" % n for n in ("stage", "doing")] + ["automaton/doing.%s" % n for n in ("attempt", "why", "tries", "water")]
             + ["automaton/broke_at", "automaton/for"] + ["automaton/%s" % n for n in CLOCKS]
             + ["automaton/%s" % n for n in ("auger", "feed", "semi", "igniter", "pump", "pump_by")]
             + ["automaton/pump_by.water", "automaton/fan", "automaton/alarm", "automaton/alarm.past", "automaton/organs", "automaton/refused"]
             + RELAYS + ["fan/holding", "fan/holding.asked", "fan/draught", "fan/purge"])
# The settings both sides play the automaton on: the first burner's stand state keeps them, and each side is handed the same.
AUTOMATON_SETTINGS = ["auto_hold", "auto_hold_c", "auto_drop_c", "auto_step", "auto_purge", "auto_igniter", "auto_wait", "auto_tries", "auto_kindle_pa", "auto_pa", "auto_dose", "auto_dose_again",
                      "auto_relight_1", "auto_relight_2", "auto_relight_3", "auto_rise_s", "auto_no_flame_s", "auto_flue_win_s", "auto_burn_off_s", "auto_feed_on_s", "auto_feed_off_s",
                      "auto_pump_on_c", "auto_pump_off_c", "auger_limit", "poll_every"]
# THE FIRST BURNER'S RULE FOR A LEVEL OF POWER - (draught, dose, pause) by the level in kilowatts - which it burns a hold of power by; the new burner reads the same numbers off its table of steps.
FACTORY_STEPS = {"14": ("51", "2", "8"), "10": ("36", "2", "12"), "8": ("29", "2", "15"), "6": ("22", "2", "21"), "5": ("18", "2", "25"), "4": ("15", "2", "32"), "3": ("11", "2", "43")}
PAIR = ("auto_pa", "auto_feed_on_s", "auto_feed_off_s")


# How long the first burner smouldered, in seconds. The new burner's own default is the owner's 900 (the boiler is slow), so the comparison asks it for the first burner's number explicitly.
FIRST_BURNERS_SMOULDER_S = "300"


def for_the_new_burner(settings):
    """SETTINGS as the new burner reads them: the first burner feeds the fire of a hold by a temperature by a pair of its own and burns a hold of power by its rule for the level; the new burner has no pair and burns
    on the chosen row of its table in every hold. So the pair is put into the row of the chosen level - or, in a hold of power, the first burner's rule for that level - and the three names of the pair are left out. Smouldering lasts as long as the first burner's did."""
    out = {k: v for k, v in settings.items() if k not in PAIR}
    out.setdefault("auto_smoulder_s", FIRST_BURNERS_SMOULDER_S)
    level = settings.get("auto_step", "6")
    if level not in FACTORY_STEPS:
        return out
    pa, on_s, off_s = FACTORY_STEPS[level] if settings.get("auto_hold", "boiler") == "power" else tuple(settings.get(n, "") for n in PAIR)
    out["auto_%s_pa" % level], out["auto_%s_on_s" % level], out["auto_%s_off_s" % level] = pa, on_s, off_s
    return out


# What only the new burner's generator reads.
NEW_ONLY = ["empty_sockets", "bus_silent", "flue_open", "flue_silent"]


def entities_of(scenario, modules):
    """The entities the two sides are held to for SCENARIO, in the order of MODULES: the flame's five, the sockets the first burner's stand reads by their roles, the flue's one."""
    out = []
    for module in modules:
        if module == "flame":
            out += FLAME
        elif module == "water":
            roles = scenario.get("settings") or {}
            out += ["temperatures/ntc_%d" % n for n in range(1, 8) if roles.get("role_ntc_%d" % n) in SIMULATED_ROLES]
        elif module == "flue":
            out.append("flue/flue_temp")
        elif module == "draught":
            out.append("draught/blast_pressure")
        elif module == "relays":
            out += RELAYS
        elif module == "fan":
            out += FAN
        elif module == "automaton":
            out += AUTOMATON
        else:
            raise SystemExit("module_compare: no module %r - flame, water, flue, draught, relays, fan or automaton" % module)
    return out


def draught_line(series):
    """The pin's millivolts, as [[second, millivolts or None]], that would make the first burner's stand's pressures: SERIES is what its stand said of the pressure each second - a number of
    pascals, {"at_least": top}, {"unknown": reason}, None - and "<nothing>" where it said nothing, the pin then staying as it was. The one place the physics of the sensor and its divider is
    written for the comparison: the new burner reads millivolts, and what it makes of them is what is compared."""
    out, pin = [], None
    for second, said in enumerate(series):
        if said != "<nothing>":
            if isinstance(said, (int, float)):
                pin = round((ZERO_MV + said * SPAN_MV / FULL_PA) / 2, 2)
            elif isinstance(said, dict) and "at_least" in said:
                pin = OVER_PIN_MV
            elif isinstance(said, dict) and said.get("unknown") == "own_zero":
                pin = STILL_PIN_MV
            else:
                pin = None
        out.append([second, pin])
    return out


def state_settings(dictionary, state, names):
    """The text of the settings NAMES as the first burner's stand state keeps them, by name: what its automaton and its relays run on. Both sides are given the same numbers, so that what is compared
    is what each makes of them and not which it was handed."""
    at = {t["name"]: i for i, t in enumerate(dictionary["things"]) if t["card"] == "settings" and not t.get("part_of")}
    kept = {a: v for a, v in state["values"]}
    return {n: kept[at[n]]["text"] for n in names if n in at and isinstance(kept.get(at[n]), dict) and "text" in kept[at[n]]}


def relay_commands(series):
    """The commands that make an organ do what the first burner's stand says of it: SERIES is whether it was closed each second - True, False, or "<nothing>" where the stand said nothing, the
    organ then staying as it was. An On at the second it closes, an Off at the second it opens. The new burner's organs take commands; the first burner's automaton closes them, and what is
    compared is what each side says of the organs once they are closed and opened at the same seconds."""
    out, held = [], False
    for second, said in enumerate(series):
        if said == "<nothing>":
            continue
        if said and not held:
            out.append([second, "on"])
        elif not said and held:
            out.append([second, "off"])
        held = bool(said)
    return out


def fan_commands(modes, asked):
    """The commands that make the new burner's fan do what the first burner's stand says of it: MODES is the mode word each second - off, hold, full, by_hand, or "<nothing>" -, ASKED the draught asked (the slider's
    number while held). A command at each second the mode or the pascals change: hold with the pascals, purge for full, stop for off. The first burner's automaton asks; what is compared is what each side says of the
    fan once it is asked the same things at the same seconds."""
    out, held = [], None
    for second, (mode, pa) in enumerate(zip(modes, asked)):
        if mode == "<nothing>":
            continue
        now = ("hold", pa) if mode == "hold" and isinstance(pa, (int, float)) else ("purge",) if mode == "full" else ("stop",) if mode == "off" else None
        if now is not None and now != held:
            out.append([second, *now])
            held = now
    return out


def whole(said):
    """A pressure as the card shows it - whole pascals - and anything else as it is."""
    return round(said) if isinstance(said, (int, float)) and not isinstance(said, bool) else said


def without_what_only_the_new_one_reads(scenario):
    """SCENARIO as both sides can play it: without the faults only the new burner's generator makes up."""
    return {k: v for k, v in scenario.items() if k not in NEW_ONLY}


def named(dictionary, frames, entities):
    """The ENTITIES second by second, as [{name: value}], each second carrying what the earlier ones last said of the entities it does not mention."""
    wanted = set(entities)
    keys = {}
    for at, t in enumerate(dictionary["things"]):
        key = "%s/%s.%s" % (t["card"], t["part_of"], t["name"]) if t.get("part_of") else "%s/%s" % (t["card"], t["name"])
        if key in wanted:
            keys[at] = key
    state, out = {}, []
    for frame in frames:
        for at, value in frame["values"]:
            if at in keys:
                state[keys[at]] = value
        out.append(dict(state))
    return out


def differences(reference, new, entities):
    """The seconds at which the two sides do not say the same of ENTITIES: [(second, name, reference said, new said)]. An entity one side never says is a difference."""
    out = []
    for second, (r, n) in enumerate(zip(reference, new)):
        for name in entities:
            if r.get(name, "<nothing>") != n.get(name, "<nothing>"):
                out.append((second, name, r.get(name, "<nothing>"), n.get(name, "<nothing>")))
    if len(reference) != len(new):
        out.append((min(len(reference), len(new)), "seconds", len(reference), len(new)))
    return out


def frames_of(command):
    out = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    if out.returncode != 0:
        raise SystemExit("module_compare: %s failed:\n%s" % (" ".join(command[:6]), out.stderr[-1500:]))
    return json.loads(out.stdout)["frames"]


def compare_frames(reference_frames, new_frames, entities):
    """The differences between the first burner's frames and the new one's on ENTITIES, each side read through its own dictionary: [(second, name, reference said, new said)]. What `main` says and what
    a test can hand frames to - made wrong on purpose, to see that the comparison finds them."""
    sys.path.insert(0, HERE)
    from page_stand import dictionary_of

    reference = named(dictionary_of(ROOT), reference_frames, entities)
    new = named(dictionary_of(ROOT, "burner"), new_frames, entities)
    for side in (reference, new):
        for second in side:
            for name in ("draught/blast_pressure", "fan/holding.came_out"):
                if name in second:
                    second[name] = whole(second[name])
    return differences(reference, new, entities)


def main(argv):
    import argparse

    ap = argparse.ArgumentParser()
    ap.add_argument("scenario", nargs="?", default=os.path.join(HERE, "stand_scenarios", "flame.json"))
    ap.add_argument("--modules", default="flame", help="flame, water, flue, draught, relays, fan and automaton, comma between")
    ap.add_argument("--frames-new", default=None, help="where to keep the new burner's frames, for a stand to play")
    ap.add_argument("--frames-reference", default=None, help="where to keep the first burner's frames, for a stand to play")
    args = ap.parse_args(argv)
    with open(args.scenario) as f:
        scenario = json.load(f)
    modules = args.modules.split(",")
    sys.path.insert(0, HERE)
    from page_stand import dictionary_of

    entities = entities_of(scenario, modules)
    if not entities:
        raise SystemExit("module_compare: nothing to compare - the scenario gives no role to a socket of the water, and no other module was asked for")
    work = tempfile.mkdtemp(prefix="module_compare_")
    both = os.path.join(work, "scenario.json")
    mine = os.path.join(work, "new.json")
    shared = without_what_only_the_new_one_reads(scenario)
    if "automaton" in modules:
        # the automaton runs on the numbers the first burner's stand state keeps, the same on both sides, over what the scenario names
        with open(os.path.join(HERE, "page_stand_values.json")) as f:
            kept = state_settings(dictionary_of(ROOT), json.load(f), AUTOMATON_SETTINGS)
        shared = dict(shared, settings=dict(kept, **shared.get("settings", {})))
    if "relays" in modules:
        # the limits of the organs are settings: the auger's own, and the igniter's from the automaton's time of the igniter - the numbers the first burner's stand keeps
        with open(os.path.join(HERE, "page_stand_values.json")) as f:
            kept = state_settings(dictionary_of(ROOT), json.load(f), ["auger_limit", "auto_igniter"])
        shared = dict(shared, settings=dict(kept, **shared.get("settings", {})))
    with open(both, "w") as f:
        json.dump(shared, f)
    try:
        reference_frames = frames_of(["cargo", "+stable", "run", "-q", "-p", "automaton_card", "--example", "scenario",
                                      os.path.join(HERE, "page_stand_values.json"), both])
        if ("draught" in modules or "fan" in modules) and "automaton" not in modules:
            # the first burner's stand says pascals; the new one reads the pin: the pin that makes those pascals is the new one's input
            said = named(dictionary_of(ROOT), reference_frames, ["draught/blast_pressure"])
            shared = dict(shared, draught_mv=draught_line([s.get("draught/blast_pressure", "<nothing>") for s in said]))
        if "relays" in modules and "automaton" not in modules:
            said = named(dictionary_of(ROOT), reference_frames, ["relays/%s" % o for o in ORGANS])
            shared = dict(shared, organs={o: relay_commands([s.get("relays/%s" % o, "<nothing>") for s in said]) for o in ORGANS})
        if "fan" in modules and "automaton" not in modules:
            said = named(dictionary_of(ROOT), reference_frames, ["fan/holding", "fan/draught"])
            shared = dict(shared, fan=fan_commands([s.get("fan/holding", "<nothing>") for s in said], [s.get("fan/draught", "<nothing>") for s in said]))
        with open(mine, "w") as f:
            json.dump(dict(shared, settings=for_the_new_burner(shared["settings"])) if "automaton" in modules else shared, f)
        new_frames = frames_of(["cargo", "+stable", "run", "-q", "-p", "burner_api", "--example", "burner_frames", mine])
    finally:
        for path in (both, mine):
            if os.path.exists(path):
                os.remove(path)
        os.rmdir(work)
    for path, frames in ((args.frames_reference, reference_frames), (args.frames_new, new_frames)):
        if path:
            with open(path, "w") as f:
                json.dump({"frames": frames}, f)
    found = compare_frames(reference_frames, new_frames, entities)
    for second, name, was, now in found[:40]:
        print("second %d, %s: the first burner says %r, the new one %r" % (second, name, was, now))
    print("%d seconds, %d entities each (%s): %d differences" % (len(reference_frames), len(entities), ", ".join(entities), len(found)))
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
