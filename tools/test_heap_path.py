"""tools/heap_path.py on a heap snapshot made up here, whose answer is known: which objects carry a shape, the shortest
way to each from the roots, and what a weak edge and an unreachable object answer.

    uv run --no-project --with pytest pytest tools/test_heap_path.py
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from heap_path import load, path_to, shaped, show, why_alive

STRINGS = ["", "root", "Window", "Object", "state", "at", "group", "text", "held", "Table", "map",
           "7 / part of key (Window @3) -> value (Object @9) pair in WeakMap (table @15)",
           "8 / part of key (Window @3) -> value (Object @17) pair in WeakMap (table @19)"]
NODE_TYPES = ["hidden", "array", "string", "object", "synthetic"]
EDGE_TYPES = ["context", "element", "property", "internal", "hidden", "shortcut", "weak"]
PROPERTY, INTERNAL, WEAK = 2, 3, 6


def snapshot(tmp_path):
    """root -> Window. Window -[state]-> row A; Window -[map]-> table 15; Window -weak-> row D. Two entries have Window for
    their key: one with row B as value in table 15, which Window holds; one with row E in table 19, which nothing holds.
    Row C is held by nothing. Every row has the properties `at` and `group`, pointing at one string."""
    node = lambda kind, name, id, edges: [NODE_TYPES.index(kind), STRINGS.index(name), id, 10, edges, 0, 0]
    nodes = (node("synthetic", "root", 1, 1) + node("object", "Window", 3, 5) + node("object", "Object", 5, 2)
             + node("string", "text", 7, 0) + node("object", "Object", 9, 2) + node("object", "Object", 11, 2)
             + node("object", "Object", 13, 2) + node("array", "Table", 15, 0) + node("object", "Object", 17, 2)
             + node("array", "Table", 19, 0))
    at = lambda i: i * 7
    row = [PROPERTY, STRINGS.index("at"), at(3), PROPERTY, STRINGS.index("group"), at(3)]
    edges = ([PROPERTY, STRINGS.index("held"), at(1)]
             + [PROPERTY, STRINGS.index("state"), at(2), INTERNAL, 11, at(4), PROPERTY, STRINGS.index("map"), at(7),
                INTERNAL, 12, at(8), WEAK, STRINGS.index("held"), at(6)]
             + row + row + row + row + row)
    doc = {"snapshot": {"meta": {"node_fields": ["type", "name", "id", "self_size", "edge_count", "trace_node_id", "detachedness"],
                                 "node_types": [NODE_TYPES],
                                 "edge_fields": ["type", "name_or_index", "to_node"], "edge_types": [EDGE_TYPES]}},
           "nodes": nodes, "edges": edges, "strings": STRINGS}
    path = tmp_path / "made.heapsnapshot"
    path.write_text(json.dumps(doc))
    return load(str(path))


def test_the_objects_that_carry_a_shape_are_found_by_id_and_nothing_else_is(tmp_path):
    g = snapshot(tmp_path)
    assert [g["id"](k) for k in shaped(g, ["at", "group"])] == [5, 9, 11, 13, 17]
    assert shaped(g, ["at", "no_such_property"]) == []


def test_the_path_to_an_object_is_the_shortest_way_from_the_roots(tmp_path):
    g = snapshot(tmp_path)
    path = path_to(g, 2)
    assert [(kind, name) for kind, name, _ in path] == [("property", "held"), ("property", "state")]
    assert show(g, path) == "(GC roots) -[property:held]-> Window -[property:state]-> Object"


def test_an_object_held_only_weakly_has_no_path_unless_weak_edges_are_asked_for(tmp_path):
    g = snapshot(tmp_path)
    assert path_to(g, 6) is None
    assert [(kind, name) for kind, name, _ in path_to(g, 6, weak=True)] == [("property", "held"), ("weak", "held")]


def test_an_object_held_only_by_a_weakmap_entry_has_no_path_of_its_own(tmp_path):
    g = snapshot(tmp_path)
    assert path_to(g, 4) is None and path_to(g, 4, weak=True) is not None


def test_a_value_of_a_weakmap_entry_is_kept_by_its_key_and_its_table_and_says_so(tmp_path):
    g = snapshot(tmp_path)
    assert why_alive(g, 4) == [
        (0, "Object (id 9) is the value of an entry of Table (id 15) whose key is Window (id 3)"),
        (1, "the table is kept because:"), (2, "(GC roots) -[property:held]-> Window -[property:map]-> Table"),
        (1, "the key is kept because:"), (2, "(GC roots) -[property:held]-> Window")]


def test_a_value_whose_table_nothing_keeps_is_garbage_though_its_key_lives(tmp_path):
    g = snapshot(tmp_path)
    assert why_alive(g, 8) == []


def test_an_object_with_a_path_is_reported_by_its_path_alone_and_one_nothing_holds_by_nothing(tmp_path):
    g = snapshot(tmp_path)
    assert why_alive(g, 2) == [(0, "(GC roots) -[property:held]-> Window -[property:state]-> Object")]
    assert why_alive(g, 5) == [] and why_alive(g, 6) == []
