"""The name of a card is painted with the ink of the values and is heavier than the label of a row (REF_Device_Page_Look, rule 3): the verdict on what the browser computed.

The browser is asked by tools/page_controls.py; what it said is judged here, with no browser, so that the verdict itself is held by rows (tools/test_page_depth.py).
"""

# The weight of a card's name: as heavy as a levelled value (rule 2), and heavier than the label of a row, which is written at the weight of text.
TITLE_WEIGHT = 600


def problems(titles, label, value):
    """What is wrong with TITLES - [{text, color, weight}], the names of the cards - against LABEL and VALUE - {color, weight} of a row's label and of a value with no level -,
    as computed by the browser; empty when all is as the rule says."""
    out = []
    if not titles:
        return ["no card name to judge"]
    for t in titles:
        if t["color"] == label["color"]:
            out.append("'%s' is painted the colour of a row's label (%s) and merges with the rows" % (t["text"], t["color"]))
        elif t["color"] != value["color"]:
            out.append("'%s' is painted %s, not the ink of the values (%s)" % (t["text"], t["color"], value["color"]))
        if t["weight"] < TITLE_WEIGHT or t["weight"] <= label["weight"]:
            out.append("'%s' is weight %s, not heavier than the label (%s) from %s up" % (t["text"], t["weight"], label["weight"], TITLE_WEIGHT))
    return out
