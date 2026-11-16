"""The verdicts of tools/page_press_state.py held on answers of the browser that are good and answers that are not: a probe that cannot fail proves nothing about the page.

    uv run --no-project --with pytest --with pyyaml pytest -q tools/test_page_press_state.py
"""
import page_press_state as probe

PHRASE = "Плата не ответила на нажатие за 8 с"
PROBE_ARGS = type("A", (), {"board": None})()


def lag(**over):
    got = {"acceptedAt": 30, "text": "Принято", "w0": 96, "w1": 96, "wide": 375, "posts": 1, "afterAnswer": {"phase": "", "free": True}, "drawn": "Принято"}
    got.update(over)
    return got


def setup_module(_):
    probe.ARGS = PROBE_ARGS


def test_a_press_that_says_accepted_within_100_ms_and_sends_one_command_is_good():
    assert probe.judge("relay", "lag", lag(), PHRASE)[0] is True


def test_a_button_that_says_it_late_or_never_is_bad():
    assert probe.judge("relay", "lag", lag(acceptedAt=450), PHRASE)[0] is False
    assert probe.judge("relay", "lag", lag(acceptedAt=None), PHRASE)[0] is False


def test_a_second_press_that_goes_to_the_board_is_bad():
    assert probe.judge("relay", "lag", lag(posts=2), PHRASE)[0] is False


def test_a_button_that_jumps_in_width_or_a_page_wider_than_the_phone_is_bad():
    assert probe.judge("relay", "lag", lag(w1=104), PHRASE)[0] is False
    assert probe.judge("relay", "lag", lag(wide=420), PHRASE)[0] is False


def test_a_button_still_held_after_the_board_answered_is_bad():
    assert probe.judge("relay", "lag", lag(afterAnswer={"phase": "accepted", "free": False}), PHRASE)[0] is False


def test_the_feeds_word_is_the_drawings_not_the_buttons():
    assert probe.judge("feed", "lag", lag(drawn="Запретить подачу"), PHRASE)[0] is False
    assert probe.judge("feed", "lag", lag(), PHRASE)[0] is True


def test_a_refusal_frees_the_button_and_the_page_says_the_boards_reason():
    good = {"posts": 1, "acceptedAt": None, "at500": {"phase": "", "free": True}, "note": "refused_by_probe"}
    assert probe.judge("stop", "refuse", good, PHRASE)[0] is True
    assert probe.judge("stop", "refuse", {**good, "note": ""}, PHRASE)[0] is False
    assert probe.judge("stop", "refuse", {**good, "at500": {"phase": "accepted", "free": False}}, PHRASE)[0] is False


def test_silence_frees_the_button_and_the_page_says_the_phrase():
    good = {"posts": 1, "at8500": {"phase": "", "free": True, "note": PHRASE + ". Перезагрузи страницу"}}
    assert probe.judge("stop", "silent", good, PHRASE)[0] is True
    assert probe.judge("stop", "silent", {"posts": 1, "at8500": {"phase": "", "free": True, "note": ""}}, PHRASE)[0] is False
    assert probe.judge("stop", "silent", {"posts": 1, "at8500": {"phase": "accepted", "free": False, "note": PHRASE}}, PHRASE)[0] is False
    assert probe.judge("stop", "silent", good, None)[0] is False


def test_no_command_sent_is_never_good():
    assert probe.judge("stop", "lag", lag(posts=0), PHRASE)[0] is False


# ---- the late word of an old press ----

def again(**over):
    got = {"atLimit": {"at": 8100, "phase": "", "free": True, "note": "the board did not answer"}, "pressedB": 8200,
           "lateA": {"phase": "accepted", "free": False, "posts": 2, "note": ""}, "afterThird": {"phase": "accepted", "free": False, "posts": 2},
           "final": {"phase": "", "free": True, "posts": 2}, "posts": 2}
    got.update(over)
    return got


def test_a_press_that_stays_held_through_the_late_word_of_the_old_one_and_ends_by_its_own_answer_is_good():
    assert probe.judge("relay", "again-false", again(), None)[0] is True
    assert probe.judge("relay", "again-true", again(), None)[0] is True


def test_a_press_that_the_late_word_of_the_old_one_let_go_or_confirmed_is_bad():
    free = {"phase": "", "free": True, "posts": 2}
    assert probe.judge("relay", "again-false", again(lateA=free), None)[0] is False, "A's refusal freed B"
    assert probe.judge("relay", "again-true", again(lateA={"phase": "answered", "free": False, "posts": 2, "note": ""}), None)[0] is False, "A's answer confirmed B"


def test_a_third_press_that_went_to_the_board_while_the_second_waited_is_bad():
    assert probe.judge("relay", "again-false", again(afterThird={"phase": "accepted", "free": False, "posts": 3}), None)[0] is False


def test_a_press_that_never_went_or_a_limit_that_never_let_go_is_bad():
    assert probe.judge("relay", "again-false", again(pressedB=None), None)[0] is False
    assert probe.judge("relay", "again-false", again(atLimit=None), None)[0] is False
    assert probe.judge("relay", "again-false", again(final={"phase": "accepted", "free": False, "posts": 2}), None)[0] is False
    assert probe.judge("relay", "again-false", again(final={"phase": "answered", "free": False, "posts": 2}), None)[0] is True, "B answered, its moment after the word is not yet over"


def test_the_buttons_the_boards_rows_settle_are_not_asked_the_forwarded_case():
    ok, words = probe.judge("stop", "again-true", {}, None)
    assert ok is True and "not asked" in words


def test_a_button_of_the_engines_rows_that_is_not_disabled_while_it_waits_is_held_by_its_phase():
    waiting = {"phase": "accepted", "free": True, "posts": 2}
    assert probe.judge("relay", "again-false", again(lateA=waiting, afterThird={"phase": "accepted", "free": True, "posts": 2}), None)[0] is True
    assert probe.judge("stop", "again-false", again(lateA=waiting, afterThird={"phase": "accepted", "free": True, "posts": 2}), None)[0] is False, "a burner's button that waits is disabled"
