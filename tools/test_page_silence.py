"""The verdicts of tools/page_silence.py, held on what a browser could answer: the thermostat's card with the row of the return's silence, without it, with the wrong seconds."""
import page_silence as f

LABEL = "датчик обратки молчит"
HEAT = "что говорит вода"


def card(rows, has_card=True):
    return {"card": has_card, "rows": [{"label": l, "value": v} for l, v in rows]}


def test_the_row_of_the_return_with_the_seconds_the_run_counted_passes():
    assert f.judge_silent_return_said(card([("длится", "19 с"), (LABEL, "19 с")]), LABEL, 19)[0]
    assert f.judge_silent_return_said(card([(LABEL, "27 с")]), LABEL, 19)[0], "the page counted on since it drew"
    assert f.judge_silent_return_said(card([(LABEL, "1 мин 5 с")]), LABEL, 62)[0]


def test_a_card_without_the_row_fails_and_names_the_rows_it_has():
    ok, words = f.judge_silent_return_said(card([("длится", "19 с"), ("насос", "вкл")]), LABEL, 19)
    assert not ok and "длится" in words, words
    assert not f.judge_silent_return_said(card([]), LABEL, 19)[0]


def test_seconds_that_are_not_the_runs_fail():
    ok, words = f.judge_silent_return_said(card([(LABEL, "3 с")]), LABEL, 19)
    assert not ok and "19" in words, words
    assert not f.judge_silent_return_said(card([(LABEL, "300 с")]), LABEL, 19)[0], "seconds from nowhere"


def test_a_card_with_no_row_of_the_silence_passes_and_with_it_fails():
    assert f.judge_no_row(card([("длится", "19 с")]), LABEL)[0]
    ok, words = f.judge_no_row(card([(LABEL, "19 с")]), LABEL)
    assert not ok and "19" in words, words


def test_a_page_with_no_card_of_the_thermostat_fails_the_check_of_no_row_too():
    ok, words = f.judge_no_row(card([], has_card=False), LABEL)
    assert not ok and "no card" in words, words


def test_the_burner_naming_the_return_is_told_with_the_row_and_without_it_the_burner_is_not_explained():
    said = card([(HEAT, "обратка молчит"), (LABEL, "35 с")])
    assert f.judge_the_burner_names_it(said, LABEL, "обрат", HEAT)[0]
    ok, words = f.judge_the_burner_names_it(card([(HEAT, "обратка молчит"), ("длится", "35 с")]), LABEL, "обрат", HEAT)
    assert not ok and "no row" in words, words
    ok, words = f.judge_the_burner_names_it(card([(HEAT, "жечь"), (LABEL, "35 с")]), LABEL, "обрат", HEAT)
    assert not ok and "says no" in words, words


def test_the_three_runs_are_the_committed_one_one_that_speaks_and_one_with_no_role():
    runs = dict(f.variants())
    assert set(runs) == {"silent", "speaks", "no-role"}
    assert runs["silent"]["return_c"][-1] == [500, None]
    assert runs["speaks"]["return_c"] == [[0, 30], [500, 30]]
    assert "return_c" not in runs["no-role"] and "role_ntc_3" not in runs["no-role"]["settings"]
