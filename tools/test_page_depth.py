"""The light theme has depth: the surfaces of a device page are told apart, not guessed at.

    uv run --no-project --with pytest pytest tools

WHAT IS BEING JUDGED. In the dark theme every level is lighter than the one under it and the eye reads that.
Turned the other way round - each level a little darker than the page - it gave four shades of beige a few
per cent apart, and on 21.09 a person looking at the light theme could not find the edge of the graph, the
buttons or the fields. So the numbers are asked for here: a surface a level up must differ from the one under
it by more than a rounding error, and the line that draws an edge must be dense enough to be an edge.

The measure is the contrast ratio of WCAG - the one the browsers, the checkers and the eye agree on - taken
between the tokens as tokens.css writes them. A screen says whether it LOOKS right; this says whether the
theme can look right at all, and it says it without a browser.
"""
import math
import os
import re
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

# **DEPTH IS NOT BUILT OUT OF FILL HERE, SO FILL IS NOT WHAT IS MEASURED.** The first version of this file
# demanded a contrast between the card and the page, which is the dark theme's way of stacking surfaces; asked
# of the light theme it forces the card white, and a white card was not the fix - the owner said so on 21.09,
# and the beige card came back. What the light theme stacks WITH is an edge and a shadow, and those are what
# the checks below ask for. Surfaces only have to be distinguishable at all, not spaced apart.
SURFACES_DIFFER = 1.02
# How solid the line that draws an edge is. At .16 it was a hint; an edge has to be an edge.
LINE_AT_LEAST = 0.25
# How much the shadow has to darken the page under a card's edge, as a contrast ratio against the page.
#
# **MEASURED, WITH A FLOOR AND A CEILING EITHER SIDE OF IT.** Composited on the page, the shadow in force
# (.06 over .05, together .107) gives 1.208; halving it to .03/.02 gives 1.088 and a shadow of .01 gives 1.036,
# which is the region where a wash over a large flat area stops being arguable at all. The edge we accepted
# (.28) gives 1.706 and the one we rejected as a hint (.16) gives 1.337 - a shadow is softer work than an edge
# by nature, so it is not held to those. 1.15 sits above the region nobody can see and well under the edge's.
SHADOW_DARKENS = 1.15
# Ink on the surface it is written on. The floor of WCAG AA for text of this size.
INK_ON_PANEL = 4.5


def luminance(colour):
    colour = colour.lstrip("#")
    parts = [int(colour[i:i + 2], 16) / 255 for i in (0, 2, 4)]
    straight = [c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4 for c in parts]
    return 0.2126 * straight[0] + 0.7152 * straight[1] + 0.0722 * straight[2]


def contrast(one, other):
    high, low = sorted((luminance(one), luminance(other)), reverse=True)
    return (high + 0.05) / (low + 0.05)


def themes(text):
    """The tokens of each theme, by the block they are written in: what :root says is the light theme, what the
    [data-t=d] block says is the dark one."""
    blocks = {}
    for head, body in re.findall(r"([^{}]*)\{([^{}]*)\}", text):
        head = head.strip()
        which = "dark" if "data-t=d" in head or "prefers-color-scheme" in head else "light"
        if which == "light" and not head.endswith(":root"):
            continue
        kept = blocks.setdefault(which, {})
        for name, value in re.findall(r"--([\w-]+)\s*:\s*([^;]+);", body):
            kept[name] = value.strip()
    return blocks


@pytest.fixture(scope="module")
def light():
    tokens = themes(open(os.path.join(ROOT, "page", "style", "tokens.css")).read())
    assert "light" in tokens, "tokens.css carries no light theme"
    return tokens["light"]


def alpha_of(value):
    """The alpha of an rgba(), or 1 for a colour written without one."""
    found = re.search(r"rgba?\([^)]*,\s*([\d.]+)\s*\)", value)
    return float(found.group(1)) if found else 1.0


def over(front, alpha, back):
    """What FRONT at ALPHA looks like painted over BACK."""
    f = [int(front.lstrip("#")[i:i + 2], 16) for i in (0, 2, 4)]
    b = [int(back.lstrip("#")[i:i + 2], 16) for i in (0, 2, 4)]
    return "#%02X%02X%02X" % tuple(round(f[i] * alpha + b[i] * (1 - alpha)) for i in range(3))


def shadow_over(shadow, page):
    """How dark the page goes under a shadow of several layers, as a contrast ratio against the page itself."""
    layers = re.findall(r"rgba?\(([^)]*)\)", shadow)
    if not layers:
        return 1.0
    together = 1.0
    ink = "#000000"
    for layer in layers:
        parts = [p.strip() for p in layer.split(",")]
        alpha = float(parts[3]) if len(parts) > 3 else 1.0
        together *= 1 - alpha
        ink = "#%02X%02X%02X" % tuple(int(float(p)) for p in parts[:3])
    return contrast(over(ink, 1 - together, page), page)


def test_the_light_theme_carries_a_shadow_and_the_dark_one_does_not(light):
    # The edge of a card in the light theme is drawn, not implied by a change of fill. In the dark theme the
    # same shadow would be a smudge, and there the stack of fills is what works.
    tokens = themes(open(os.path.join(ROOT, "page", "style", "tokens.css")).read())
    assert light.get("shadow-card", "none") != "none", "the light theme has no card shadow to stack with"
    assert tokens["dark"].get("shadow-card") == "none", "the dark theme carries a shadow, which reads as a smudge"


def test_the_shadow_is_strong_enough_to_be_seen_at_all(light):
    # `!= none` is not a floor: a shadow at .01 passes it and is invisible. What the light theme leans on has to
    # be measurable, the same way the edge is.
    darkens = shadow_over(light["shadow-card"], light["bg"])
    assert darkens >= SHADOW_DARKENS, (
        "the shadow darkens the page to %.3f of itself, and %.2f is what it takes to be seen"
        % (darkens, SHADOW_DARKENS))


def test_the_surfaces_are_at_least_different_colours(light):
    # Not a demand for contrast - the edge does that work - but a card painted EXACTLY the page's colour would
    # leave the border drawing a box around nothing.
    for one, other in (("panel", "bg"), ("sunk", "panel")):
        apart = contrast(light[one], light[other])
        assert apart >= SURFACES_DIFFER, ("%s and %s are the same colour (%.3f)" % (one, other, apart))


def test_the_line_that_draws_an_edge_is_solid_enough_to_be_one(light):
    alpha = alpha_of(light["line"])
    assert alpha >= LINE_AT_LEAST, (
        "the line is drawn at %.2f, and an edge needs %.2f" % (alpha, LINE_AT_LEAST))


def test_the_words_are_readable_on_the_surface_they_are_written_on(light):
    apart = contrast(light["ink"], light["panel"])
    assert apart >= INK_ON_PANEL, ("ink on a card is %.2f, and %.1f is the floor" % (apart, INK_ON_PANEL))


# How far a label may fade on its card, as a contrast ratio. Held below the value's floor on purpose - a label's
# job in the light theme is to step back - and above the point where it reads as fog: at 2.34 (#99A2B8) the owner
# saw the labels as if behind a haze, at 2.86 (#8992A8) he took them.
LABEL_ON_PANEL = 2.8
# How much lighter than the value a label has to be in the LIGHT theme, in L*.
#
# **THE TWO THEMES DO NOT TELL THE WORDS APART THE SAME WAY.** In the dark theme 24 L* between label and value
# is enough: light text on a dark card glows, and the eye ranks two light words by brightness at once. Dark ink
# on light paper does not do that - everything dark enough reads as "ink" - and at 23 the owner saw label and
# value as one text. Shown side by side against the dark theme, he took the label at 41 (#8992A8) and passed
# over 34 (#877F70) and 39 (#948B7C).
LABEL_ABOVE_VALUE_LIGHT = 40.0


def test_a_label_is_not_fainter_than_a_person_can_still_read(light):
    apart = contrast(light["dim"], light["panel"])
    assert apart >= LABEL_ON_PANEL, ("the label on a card is %.2f, and %.1f is the floor" % (apart, LABEL_ON_PANEL))


def test_in_the_light_theme_the_label_is_far_lighter_than_the_value(light):
    apart = lightness(light["dim"]) - lightness(light["ink"])
    assert apart >= LABEL_ABOVE_VALUE_LIGHT, (
        "the label is %.1f L* lighter than the value, and %.0f is the floor: dark on light, anything nearer "
        "reads as one text" % (apart, LABEL_ABOVE_VALUE_LIGHT))


@pytest.mark.parametrize("label,why", [
    ("#6B6355", "the light theme before 23.09: 23 L* from the value, the owner's arrows"),
    ("#877F70", "variant C of 23.09: 34, the labels still leant on the values"),
    ("#948B7C", "variant E of 23.09: 39, passed over for the dark theme's own label"),
])
def test_a_label_too_near_the_value_in_the_light_theme_is_refused(label, why):
    assert lightness(label) - lightness("#1F2F48") < LABEL_ABOVE_VALUE_LIGHT, "this was let through: " + why


@pytest.mark.parametrize("label,why", [
    ("#C9C3B6", "a label almost the colour of the card"),
    ("#99A2B8", "the label the owner saw as behind a haze, 2.34"),
])
def test_a_label_too_faint_to_read_is_refused(label, why):
    assert contrast(label, "#F8F5EC") < LABEL_ON_PANEL, "this was let through: " + why


def test_a_level_a_reading_is_painted_with_is_readable_too(light):
    # ok, warn and bad are not decoration: they are how the page says a flue at 148 °C wants looking at. On the
    # white card the old amber stood at 2.81, which is a colour a person sees as "some shade of yellow".
    for name in ("ok", "warn", "bad"):
        apart = contrast(light[name], light["panel"])
        assert apart >= INK_ON_PANEL, ("%s on a card is %.2f, and %.1f is the floor" % (name, apart, INK_ON_PANEL))


def test_a_line_of_data_stands_out_from_the_floor_it_is_drawn_on(light):
    # The curve is painted with the accent, on the sunken floor of the picture. In the dark theme the accent is
    # a LIGHT colour on a dark floor; turned around without being darkened it was a pale scratch on beige.
    apart = contrast(light["acc"], light["sunk"])
    assert apart >= INK_ON_PANEL, ("a curve on its floor is %.2f, and %.1f is the floor" % (apart, INK_ON_PANEL))


# --- the pair on bad values: a check that passes anything proves nothing ---

def test_a_card_painted_exactly_the_page_is_refused():
    # The pair on fill. A check that passes anything proves nothing, and this is the one case the fill still has
    # to answer for: an edge around a card the same colour as the page draws a box around nothing.
    assert contrast("#F1EDE0", "#F1EDE0") < SURFACES_DIFFER


@pytest.mark.parametrize("shadow,why", [
    ("0 1px 2px rgba(31,47,72,.03), 0 2px 8px rgba(31,47,72,.02)", "half of what is in force"),
    ("0 1px 2px rgba(31,47,72,.01), 0 2px 8px rgba(31,47,72,.01)", "a shadow nobody can see"),
    ("0 1px 2px rgba(31,47,72,0), 0 2px 8px rgba(31,47,72,0)", "a shadow that is not there but is not `none`"),
])
def test_a_shadow_too_faint_to_be_seen_is_refused(shadow, why):
    # The pair on the shadow, and the reason this file needed one at all: the check used to ask only that the
    # token was not `none`, so every one of these would have passed while the light theme leaned on nothing.
    assert shadow_over(shadow, "#F1EDE0") < SHADOW_DARKENS, "this was let through: " + why


@pytest.mark.parametrize("line", ["rgba(31,47,72,.16)", "rgba(31,47,72,0)"])
def test_a_line_too_faint_to_be_an_edge_is_refused(line):
    assert alpha_of(line) < LINE_AT_LEAST, "a line at %s was let through" % line


# --- a level is read as a colour first, and as emphasis second ---

# How far a level's colour has to stand from the row's own label in L*, perceived lightness.
#
# **FIVE, AND IT IS ONLY HALF THE ANSWER.** Picked against the card alone, the colours sit exactly on the label
# - warn at L*42 against a label at L*42 - and the one line on the page asking for a person has no separation
# from the word in front of it. Pushed down in lightness to fix that, they lose their hue: green at chroma 25
# against the ink's own 18 reads as dark navy, and the wifi line
# - green in the dark theme - merges into the text. So lightness is asked for a little, and
# the rest of the emphasis is carried by WEIGHT, which costs a light theme nothing.
LEVEL_FROM_LABEL = 5.0
# How colourful a level has to be to be read as a colour at all, in L*a*b* chroma.
#
# **TWENTY-EIGHT, AND THE FLOOR IS OUR OWN NEUTRAL INK.** The label is chroma 13 and the value's ink 18: a
# "colour" near those numbers is not a colour, it is ink with a tint. The dark theme's levels run 27 to 51, and
# these are the same order - 41 to 61, higher because a light ground needs more of it.
LEVEL_IS_COLOURED = 28.0


def lightness(colour):
    """L* of a hex colour: how light it looks, 0 black, 100 white."""
    y = luminance(colour)
    return 116 * (y ** (1 / 3)) - 16 if y > 0.008856 else 903.3 * y


def chroma(colour):
    """How colourful it is, in L*a*b*: 0 is a grey, and our neutral ink sits near 13-18."""
    colour = colour.lstrip("#")
    r, g, b = (int(colour[i:i + 2], 16) / 255 for i in (0, 2, 4))
    r, g, b = (c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in (r, g, b))
    x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047
    y = 0.2126 * r + 0.7152 * g + 0.0722 * b
    z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883
    f = lambda t: t ** (1 / 3) if t > 0.008856 else 7.787 * t + 16 / 116
    fx, fy, fz = f(x), f(y), f(z)
    return ((500 * (fx - fy)) ** 2 + (200 * (fy - fz)) ** 2) ** 0.5


def both_themes():
    tokens = themes(open(os.path.join(ROOT, "page", "style", "tokens.css")).read())
    return (("light", tokens["light"]), ("dark", tokens["dark"]))


def test_a_level_is_colourful_enough_to_be_read_as_a_colour():
    for theme, t in both_themes():
        for name in ("warn", "ok", "bad"):
            got = chroma(t[name])
            assert got >= LEVEL_IS_COLOURED, (
                "%s theme: %s has chroma %.0f, and the neutral ink of this page is already 13-18 - "
                "at that distance it is not a colour, it is a tint" % (theme, name, got))


def test_a_level_does_not_sit_on_the_lightness_of_its_label():
    for theme, t in both_themes():
        for name in ("warn", "ok", "bad"):
            apart = abs(lightness(t[name]) - lightness(t["dim"]))
            assert apart >= LEVEL_FROM_LABEL, (
                "%s theme: %s stands %.0f of L* from the label it is read beside" % (theme, name, apart))


def test_a_levelled_value_is_the_heavier_word():
    # Weight is what carries the emphasis in the light theme, and it travels with the colour rather than with
    # the markup: a level is one thing, and a page that paints it must not be able to forget to weigh it.
    style = open(os.path.join(ROOT, "page", "style", "tailwind.css")).read()
    assert ".text-ok, .text-bad, .text-warn { font-weight: 600 }" in style, (
        "a levelled value is no heavier than its label: the light theme has nothing left to say it with")


@pytest.mark.parametrize("colour,label,why", [
    ("#8A5A12", "#5A6478", "the warning that sat exactly on its label, 22.09 morning"),
    ("#3F7A52", "#5A6478", "ok, four of L* from its label"),
    ("#E88A80", "#99A2B8", "bad in the dark theme, one of L* above its label"),
])
def test_a_level_that_sits_on_its_label_is_refused(colour, label, why):
    assert abs(lightness(colour) - lightness(label)) < LEVEL_FROM_LABEL, "this was let through: " + why


@pytest.mark.parametrize("colour,why", [
    ("#2B5438", "the green of 22.09 midday: separated by lightness, and no longer green"),
    ("#4A4F5C", "a grey with a hint of blue - lightness alone would have passed it"),
    ("#5A6478", "the label's own colour, which is not a level at all"),
])
def test_a_level_too_grey_to_be_a_colour_is_refused(colour, why):
    # The pair on hue, and the reason this file needed one: a check that asks only about lightness passes every
    # one of these, and the page comes out in three shades of the same dark ink.
    assert chroma(colour) < LEVEL_IS_COLOURED, "this was let through: " + why


# --- in the dark theme the label belongs to the paper, the value does not ---

# How near the label's hue has to be to the card's, in degrees of the L*a*b* circle, and how far the value's has
# to be from the label's. Asked of the DARK theme only: that is how it tells the words apart, and there it works.
#
# Measured on the board 22.09: the label sits at 277 degrees and the card at 277 - the same family, so the label
# reads as part of the paper - and the value at 96, half the circle away. The light theme does it by lightness
# instead (LABEL_ABOVE_VALUE_LIGHT above): on light paper a hue between two dark words is not seen, and the
# label the owner chose there is the dark theme's own, cool on a warm card.
LABEL_NEAR_THE_CARD = 30.0
VALUE_AWAY_FROM_LABEL = 120.0


def hue(colour):
    """Where the colour sits on the L*a*b* circle, in degrees. A grey has no meaningful one, which is why the
    checks below are only asked of colours that carry some chroma."""
    colour = colour.lstrip("#")
    r, g, b = (int(colour[i:i + 2], 16) / 255 for i in (0, 2, 4))
    r, g, b = (c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in (r, g, b))
    x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047
    y = 0.2126 * r + 0.7152 * g + 0.0722 * b
    z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883
    f = lambda t: t ** (1 / 3) if t > 0.008856 else 7.787 * t + 16 / 116
    fx, fy, fz = f(x), f(y), f(z)
    return math.degrees(math.atan2(200 * (fy - fz), 500 * (fx - fy))) % 360


def hue_apart(one, other):
    gap = abs(hue(one) - hue(other)) % 360
    return min(gap, 360 - gap)


def dark_theme():
    return dict(both_themes())["dark"]


def test_in_the_dark_theme_the_label_is_of_the_paper_it_is_written_on():
    t = dark_theme()
    apart = hue_apart(t["dim"], t["panel"])
    assert apart <= LABEL_NEAR_THE_CARD, (
        "dark theme: the label stands %.0f degrees from its card - it reads as ink, not as part of the page" % apart)


def test_in_the_dark_theme_the_value_is_of_another_family_than_the_label():
    t = dark_theme()
    apart = hue_apart(t["ink"], t["dim"])
    assert apart >= VALUE_AWAY_FROM_LABEL, (
        "dark theme: the value stands %.0f degrees from the label - one colour at two brightnesses, and a person "
        "reads them as the same text" % apart)


def test_a_dark_label_of_the_value_s_own_family_is_refused():
    # A cream label under cream values: close to the card in nothing, one colour at two brightnesses.
    label, card, value = "#B8B2A3", "#1B2537", "#E9E4D5"
    assert (hue_apart(label, card) > LABEL_NEAR_THE_CARD
            or hue_apart(value, label) < VALUE_AWAY_FROM_LABEL), "a cream label under cream values was let through"


@pytest.mark.parametrize("label,why", [
    ("#5A6478", "the light theme of 22.09: a cool label one degree from the value, and 23 L* from it"),
    ("#4A5568", "a cooler grey - darker still, and nearer the value"),
])
def test_a_light_label_that_is_neither_lighter_nor_of_another_family_is_refused(label, why):
    # The pair this file most needed: both clear contrast and readability, and both left the page one text.
    assert lightness(label) - lightness("#1F2F48") < LABEL_ABOVE_VALUE_LIGHT, "this was let through: " + why


# ---- the name of a card (REF_Device_Page_Look, rule 3) ----
#
# The name of a card is painted with the ink of the values, and so stands off the label of a row exactly as a value does: in the light theme by lightness (LABEL_ABOVE_VALUE_LIGHT, measured
# 41.3 L*), in the dark by hue (VALUE_AWAY_FROM_LABEL, measured 180°). A name painted the label's colour is read as one of the rows under it.
# What the browser painted is asked by tools/page_controls.py (card-title-is-ink); the verdict on its answer is tools/card_title.py, held here.
from card_title import TITLE_WEIGHT, problems


def test_in_the_light_theme_the_name_of_a_card_stands_off_the_label_by_the_floor_or_more(light):
    apart = lightness(light["dim"]) - lightness(light["ink"])
    assert apart >= LABEL_ABOVE_VALUE_LIGHT, "the name of a card (ink) is only %.1f L* from the label; %.1f is the floor" % (apart, LABEL_ABOVE_VALUE_LIGHT)


def test_in_the_dark_theme_the_name_of_a_card_is_of_another_family_than_the_label():
    t = dark_theme()
    apart = hue_apart(t["ink"], t["dim"])
    assert apart >= VALUE_AWAY_FROM_LABEL, "the name of a card (ink) is only %.0f degrees of hue from the label; %.0f is the floor" % (apart, VALUE_AWAY_FROM_LABEL)


LABEL = {"color": "rgb(137, 146, 168)", "weight": 400}
VALUE = {"color": "rgb(31, 47, 72)", "weight": 400}


def title(colour, weight=600, text="Now"):
    return {"text": text, "color": colour, "weight": weight}


def test_a_name_painted_with_the_ink_of_the_values_and_heavier_than_the_label_is_as_the_rule_says():
    assert problems([title(VALUE["color"]), title(VALUE["color"], 700, "Flame")], LABEL, VALUE) == []


def test_a_name_painted_the_colour_of_the_label_is_refused():
    said = problems([title(LABEL["color"])], LABEL, VALUE)
    assert said and "merges with the rows" in said[0], "this was let through: the name of 01.10"


def test_a_name_painted_some_other_colour_is_refused_too():
    said = problems([title("rgb(45, 68, 144)")], LABEL, VALUE)
    assert said and "not the ink of the values" in said[0]


def test_a_name_no_heavier_than_the_label_is_refused():
    assert problems([title(VALUE["color"], 400)], LABEL, VALUE), "a name at the weight of a label was let through"
    assert problems([title(VALUE["color"], TITLE_WEIGHT - 100)], LABEL, VALUE)


def test_nothing_to_judge_is_not_a_pass():
    assert problems([], LABEL, VALUE) == ["no card name to judge"]
