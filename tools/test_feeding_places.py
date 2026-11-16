"""The words of the automaton to the auger live on a term: every file that keeps a plan has its end beside it, every file that says a word to the auger is one the list below knows. A new place to say or keep one
fails here until it is looked at - the class is "a word without a term", and a place not looked at is where it comes back.

    uv run --no-project --with pytest pytest -q tools/test_feeding_places.py
"""
import feeding_places as fp

# the shipped files that make or hand on a word to the auger, and why each is what it is
SAYS = {
    "crates/automaton/src/lib.rs",            # decides the rhythm every second (Doing.feeding)
    "crates/automaton/src/hands.rs",          # hands it on: a command when it changes, a confirmation when it stands
    "devices/burner/src/modules/automaton.rs",  # turns it into the owner's words in ms
    "devices/burner/src/modules/relays.rs",   # carries the role AUGER to the owner
    "devices/burner/src/roles.rs",            # the role
    "crates/relay/src/lib.rs",                # the task's types and the plan it holds
    "crates/relay/src/board.rs",              # the owner's task
}


def by_kind(kind):
    return {rel for rel, _, k, _ in fp.places() if k == kind}


def test_no_file_says_a_word_to_the_auger_that_the_list_does_not_know():
    unknown = by_kind("saying") - SAYS
    assert not unknown, "a place that makes or hands on a word to the auger and has not been looked at for its term: %s" % sorted(unknown)


def test_the_file_that_keeps_a_plan_keeps_its_end_in_the_same_place():
    keeping = by_kind("keeping")
    assert keeping == {"crates/relay/src/lib.rs"}, "a plan is kept outside the relay task: %s" % sorted(keeping)
    assert keeping <= by_kind("term"), "a file keeps a plan and has no end for it"


def test_every_hands_that_commands_the_auger_confirms_it_too():
    # the trait asks for both, so a new implementation without the confirmation does not compile; this holds the list of implementations the shipped code has
    commands = {rel for rel, _, k, t in fp.places() if k == "saying" and t.startswith("fn auger(")}
    confirms = {rel for rel, _, k, t in fp.places() if k == "saying" and t.startswith("fn auger_still(")}
    assert commands == confirms, (sorted(commands), sorted(confirms))
