"""The switch of the thermostat survives a restart of the board: a Start the person pressed lights the burner by itself when the board comes up and the water asks, a Stop stays a Stop, a board nobody
pressed anything on stays as it was, a fault stays a fault, and the flash is written once for a press and never for a restart or a second. Played on the new burner's own automaton and thermostat (burner_api, example
`burner_frames`, scenario key `restart`) on runs of the readings by the second. Expected from what a person at the boiler must find after a power cut - and not from how the code keeps the word.

    uv run --no-project --with pytest --with pyyaml --with websocket-client pytest tools/test_start_survives_restart_on_the_stand.py
"""
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

from module_compare import named  # noqa: E402
from page_stand import dictionary_of  # noqa: E402

ENTITIES = ["thermostat/switch", "thermostat/switch_from", "automaton/doing"]


def scenario(presses, restarts, seconds, boiler_c=40, flash_refuses=None):
    """A cold boiler (under the setpoint less the hysteresis), no flame at any second - every light fails - and the presses and restarts a case needs."""
    return {
        "seconds": seconds,
        "press": [[at, press] for at, press in presses],
        "restart": restarts,
        "flame_mv": [[0, 3150], [seconds, 3150]],
        "boiler_c": [[0, boiler_c], [seconds, boiler_c]],
        "flue_c": [[0, 20], [seconds, 20]],
        **({"flash_refuses": flash_refuses} if flash_refuses else {}),
    }


_DICTIONARY = []


def played(case):
    """The run: what the card said each second (the switch, whose word it is, what the fire does) and how many times the flash was written for the switch."""
    with tempfile.TemporaryDirectory() as work:
        path = os.path.join(work, "scenario.json")
        with open(path, "w") as f:
            json.dump(case, f)
        out = subprocess.run(["cargo", "+stable", "run", "-q", "-p", "burner_api", "--example", "burner_frames", path], cwd=ROOT, capture_output=True, text=True)
    assert out.returncode == 0, out.stderr[-1500:]
    played_ = json.loads(out.stdout)
    if not _DICTIONARY:
        _DICTIONARY.append(dictionary_of(ROOT, "burner"))
    frames = [{"at": f["at"], "values": f["values"]} for f in played_["frames"]]
    return named(_DICTIONARY[0], frames, ENTITIES), played_["switch_writes"]


def doing(run, at):
    return run[at]["automaton/doing"]


def test_a_start_pressed_before_the_restart_lights_the_burner_by_itself_over_cold_water():
    # the restart is in the purge: a cold pot, nothing to burn off, the burner stands at a stop for a second and the water asks for heat
    run, _ = played(scenario([(2, "start")], [20], 100))
    assert doing(run, 19) == "purge" and run[19]["thermostat/switch_from"] == "person", "the case did not stand where it was meant to: %s" % run[19]
    assert run[20]["thermostat/switch"] == "on" and run[20]["thermostat/switch_from"] == "memory", "after the restart the card must say the switch is back, and whose word it is: %s" % run[20]
    after = [doing(run, at) for at in range(20, 30)]
    assert "purge" in after, "heat was asked for before the power went and the water is cold: the burner lights by itself, and it did %s" % after
    assert "burning_off" not in after, "a cold pot is not burnt off: %s" % after


def test_a_start_pressed_before_the_restart_lights_again_when_the_pot_was_cleared_first():
    # the restart is in the kindling: the pot may hold fuel, so it is burnt off first - and the switch that came back lights it again when it is a stop
    run, _ = played(scenario([(2, "start")], [60], 400))
    assert doing(run, 59) == "kindling", "the case did not stand where it was meant to: %s" % doing(run, 59)
    assert doing(run, 61) == "burning_off", "the pot is cleared first, as it always was"
    later = [doing(run, at) for at in range(61, 400)]
    assert "purge" in later[later.index("stop"):] if "stop" in later else False, "the burning off ended in a stop and the switch that came back never lit it again: %s" % sorted(set(later))


def test_a_start_pressed_before_the_restart_does_not_light_a_burner_over_water_that_asks_for_nothing():
    # a Start and a Light in the purge, water inside the hysteresis: the Light lights at once, a restart is no press, and the water asks for nothing
    run, _ = played(scenario([(2, "start"), (3, "light")], [20], 140, boiler_c=58))
    assert doing(run, 19) == "purge", "the case did not stand where it was meant to: %s" % doing(run, 19)
    after = [doing(run, at) for at in range(20, 140)]
    assert set(after) == {"stop"}, "water inside the hysteresis asks for nothing, restored Start or not: %s" % sorted(set(after))


def test_a_stop_pressed_before_the_restart_stays_a_stop():
    run, _ = played(scenario([(2, "start"), (10, "stop")], [250], 400))
    assert doing(run, 249) == "stop", "the case did not stand where it was meant to: the fire should be out by 249: %s" % doing(run, 249)
    assert run[250]["thermostat/switch"] == "off" and run[250]["thermostat/switch_from"] == "memory"
    assert {doing(run, at) for at in range(250, 400)} == {"stop"}, "a burner that was switched off lit itself"


def test_a_board_nobody_pressed_anything_on_comes_up_as_one_nobody_asked_and_does_not_light():
    run, writes = played(scenario([], [30], 150))
    assert {run[at]["thermostat/switch"] for at in range(150)} == {"unasked"}
    assert {str(run[at]["thermostat/switch_from"]) for at in range(150)} == {"{'unknown': 'none'}"}, "nobody has pressed anything: nobody's word is said"
    assert {doing(run, at) for at in range(150)} == {"stop"}
    assert writes == 0, "nothing was pressed, nothing was written"


def test_a_fault_is_still_a_fault_after_the_restart_over_a_restored_start_and_cold_water_and_a_stop_clears_it():
    seconds = 1700
    before, _ = played(scenario([(2, "start")], [], seconds))
    fault_at = next((at for at in range(seconds) if doing(before, at) == "fault"), None)
    assert fault_at is not None, "the case did not stand where it was meant to: the lights never failed into a fault in %d s" % seconds
    restart = fault_at + 30
    run, _ = played(scenario([(2, "start")], [restart], seconds))
    after = [doing(run, at) for at in range(restart + 1, seconds)]
    assert "purge" not in after and "lighting" not in after, "a restart lit a burner in a fault: %s" % sorted(set(after))
    assert after[-1] == "fault", "after clearing the pot the burner shows the fault again and waits for a person: %s" % after[-1]
    cleared, _ = played(scenario([(2, "start"), (restart + 400, "stop")], [restart], seconds))
    assert doing(cleared, restart + 401) == "stop", "a person's Stop is what clears it"
    assert {doing(cleared, at) for at in range(restart + 420, seconds)} == {"stop"}


def test_the_flash_is_written_once_for_a_press_that_changes_the_word_and_never_for_a_restart_or_a_second():
    # Start, the same Start again, Stop, Start: three words written - on, off, on; two restarts and four hundred seconds write nothing
    _, writes = played(scenario([(2, "start"), (5, "start"), (8, "stop"), (12, "start")], [50, 90], 400))
    assert writes == 3, "a write for each press that changed what the flash holds, and none else: %d" % writes


def test_a_press_after_a_restart_is_written_and_a_restart_by_itself_is_not():
    _, writes = played(scenario([(2, "start"), (100, "stop")], [60], 200))
    assert writes == 2, "Start before the restart, Stop after it: two words, and the restart wrote nothing: %d" % writes


def test_a_stop_whose_write_the_flash_refused_is_written_when_it_answers_and_a_restart_after_that_finds_the_stop():
    # Start at 2 is saved; the Stop at 100 meets a flash that refuses until 130, is tried again each second, and goes through at 130
    run, writes = played(scenario([(2, "start"), (100, "stop")], [140], 300, flash_refuses=[[100, 130]]))
    assert writes == 2, "the Start and, once the flash took it, the Stop - and nothing for the refused tries: %d" % writes
    assert run[140]["thermostat/switch"] == "off" and run[140]["thermostat/switch_from"] == "memory", "the restart after the Stop was written finds the Stop: %s" % run[140]
    assert {doing(run, at) for at in range(250, 300)} == {"stop"}, "a burner a person stopped lit itself after the restart"


def test_a_restart_inside_the_refusal_finds_the_word_the_flash_still_held_which_is_the_one_loss_the_board_cannot_prevent():
    # the Stop at 100 is waiting for a flash that refuses until 130 - and the power goes at 120: what is in RAM is lost, and the flash holds the Start
    run, writes = played(scenario([(2, "start"), (100, "stop")], [120], 300, flash_refuses=[[100, 130]]))
    assert run[120]["thermostat/switch"] == "on", "the flash held the Start when the board came up: %s" % run[120]
    assert writes == 1, "only the Start was ever taken: %d" % writes
