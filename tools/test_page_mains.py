"""The judge of the mains line on the scheme: what the browser said of the drawing, held to the line's place, size and clearance. Every verdict has its pair on a bad drawing: a line not drawn, one over another text,
across a line of the drawing, outside it, or smaller than a person reads. The numbers are those of the real page of the bench board (03.11, 375 px)."""
import page_mains as m

SVG = [16.0, 240.0, 359.0, 560.0]


def box(left, top, width, height):
    return [left, top, left + width, top + height]


def text(words, b, px=12.0, mine=False, crosses=None):
    return {"text": words, "mine": mine, "px": px, "box": b, "crosses": crosses}


MINE = text("227 V · 4 W", box(26, 405, 76, 14), mine=True)
ROOM = text("room", box(50, 480, 30, 10), px=9)
OUT = text("62.0 °C", box(130, 410, 40, 15), px=13)


def said(*texts, svg=SVG):
    return {"svg": svg, "texts": list(texts)}


def test_a_line_clear_of_every_text_and_every_line_is_good():
    ok, words = m.judge(said(MINE, ROOM, OUT))
    assert ok and "clear of 2 other texts" in words, words


def test_no_line_at_all_is_named():
    ok, words = m.judge(said(ROOM, OUT))
    assert not ok and "not drawn" in words


def test_a_line_over_another_text_is_named_with_that_text():
    over = text("power", box(40, 400, 30, 12), px=10)
    ok, words = m.judge(said(MINE, over))
    assert not ok and "lies over 'power'" in words


def test_boxes_that_touch_by_a_pixel_are_not_over_each_other():
    touching = text("room", box(102, 405, 30, 14))
    assert m.judge(said(MINE, touching))[0]


def test_a_line_across_a_line_of_the_drawing_is_named_with_the_shape():
    crossed = {**MINE, "crosses": "path M50 170 L50 240"}
    ok, words = m.judge(said(crossed))
    assert not ok and "crosses path M50 170" in words


def test_a_line_outside_the_drawing_is_named():
    out = text("227 V · 4 W", box(2, 405, 76, 14), mine=True)
    ok, words = m.judge(said(out))
    assert not ok and "outside the drawing" in words


def test_a_line_smaller_than_twelve_pixels_is_named_with_its_size():
    small = {**MINE, "px": 9.0}
    ok, words = m.judge(said(small))
    assert not ok and "drawn at 9.0 px" in words


def test_a_page_with_no_scheme_says_so():
    ok, words = m.judge({"error": "no scheme on the page"})
    assert not ok and words == "no scheme on the page"
