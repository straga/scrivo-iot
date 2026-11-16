"""Low pressure is not a lost sensor, on the stand: the burner's own automaton, thermostat and regulator play lighting.json with the draught sensor answering under its own zero (stand_scenarios/low_pressure_dip.json,
low_pressure_stays.json) and with the sensor gone (draught_gone.json). The pairs: a dip that comes back inside the term does not put the fire out; a dip that stays does, by the cause "low_pressure" and not by the sensor's; a sensor
that is gone does, at once, by the sensor's cause as it always did. Second by second the table of stage, cause, air asked and draught is the proof of each.

    uv run --no-project --with pytest --with pyyaml --with websocket-client pytest tools/test_low_pressure_on_the_stand.py
"""
import json
import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

from module_compare import frames_of, named  # noqa: E402
from page_stand import dictionary_of  # noqa: E402

COLUMNS = ["automaton/stage", "automaton/doing.why", "fan/holding.asked", "draught/blast_pressure"]
RUNS = {}


def run(name):
    if name not in RUNS:
        sc = json.load(open(os.path.join(HERE, "stand_scenarios", name + ".json")))
        with tempfile.TemporaryDirectory() as work:
            path = os.path.join(work, "scenario.json")
            with open(path, "w") as f:
                json.dump(sc, f)
            frames = frames_of(["cargo", "+stable", "run", "-q", "-p", "burner_api", "--example", "burner_frames", path])
        RUNS[name] = (sc, named(dictionary_of(ROOT, "burner"), frames, COLUMNS))
    return RUNS[name]


def first_burning_off(frames):
    """The second the automaton went to burning off, and the cause it named; none if it did not."""
    for t, frame in enumerate(frames):
        if frame["automaton/stage"] == "burning_off":
            return t, frame.get("automaton/doing.why")
    return None, None


def table(frames, lo, hi):
    return "\n".join("%4d %-12s %-14s %-6s %s" % (t, frames[t]["automaton/stage"], frames[t].get("automaton/doing.why"), frames[t].get("fan/holding.asked"), frames[t].get("draught/blast_pressure")) for t in range(lo, hi))


def test_a_dip_under_the_zero_that_comes_back_inside_its_term_leaves_the_fire_alight():
    sc, frames = run("low_pressure_dip")
    (start, end) = sc["draught_under"][0]
    went, why = first_burning_off(frames)
    assert went is None, "the fire was burnt off at %s for %r by a dip of %d s:\n%s" % (went, why, end - start + 1, table(frames, start - 2, end + 12))
    # the dip happened while the fan was asked for air - otherwise the row proves nothing
    assert all(frames[t].get("fan/holding.asked") for t in range(start, end + 1)), "the fan was not asked for air in the dip:\n" + table(frames, start - 2, end + 3)


def test_a_dip_that_stays_burns_the_fire_off_as_low_pressure_and_not_as_a_sensor_that_gives_no_number():
    sc, frames = run("low_pressure_stays")
    start = sc["draught_under"][0][0]
    went, why = first_burning_off(frames)
    assert went is not None, "the fire was not burnt off by a draught that stayed under the zero:\n" + table(frames, start, start + 40)
    assert why == "low_pressure", "burnt off at %s for %r:\n%s" % (went, why, table(frames, start, went + 3))
    # after the term (20 s) and not before it
    assert went - start >= 20, "burnt off %d s into the dip, the term is 20 s:\n%s" % (went - start, table(frames, start, went + 3))


def test_a_sensor_that_is_gone_burns_the_fire_off_at_once_as_a_sensor_that_gives_no_number():
    sc, frames = run("draught_gone")
    start = sc["draught_gone"][0][0]
    went, why = first_burning_off(frames)
    assert went is not None, "the fire was not burnt off by a sensor that is gone:\n" + table(frames, start, start + 40)
    assert why == "no_draught", "burnt off at %s for %r:\n%s" % (went, why, table(frames, start, went + 3))
    assert went - start <= 12, "burnt off %d s after the sensor went, later than the spin-up of 10 s allows:\n%s" % (went - start, table(frames, start, went + 3))
