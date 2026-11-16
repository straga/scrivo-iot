"""What holds an object of an old generation: the shortest path from the heap's roots to it, out of a V8 heap snapshot.

    uv run --no-project python tools/heap_path.py SNAPSHOT --shape at,group,state,level [--pick oldest|middle|newest]
                                                            [--count 3] [--weak]

The objects wanted are told by SHAPE, the names of the properties they carry: a row of what the page draws has `at`,
`group`, `state` and `level`, and the snapshot names no more of an object than its constructor and its properties. Of
the objects that carry all of them, the ones PICKed by their id - ids grow with the time of allocation, so the oldest
are the ones a page should have let go of long ago - are followed back to a root. Weak edges - and the entries of a WeakMap, which
hold a value only while its key lives - are not followed unless --weak is given: what is held only weakly is not
what holds it. Prints, for each object, why the collector keeps it. An object held through edges reads as its path, root first:

    (GC roots) -[internal:...]-> Window -[property:state]-> Object ... -[element:3]-> Object

and one that is only the value of a WeakMap entry as that entry, followed by why its table and why its key are kept.

and, first, how many objects of that shape the snapshot holds - the count itself is the leak's size: a page that
keeps one generation of its rows holds as many as it has rows.
"""
import argparse
import json
import re
from collections import deque


def load(path):
    """The snapshot at PATH as parallel lists: per node its name, id and where its edges begin, and the edges."""
    with open(path) as f:
        s = json.load(f)
    meta = s["snapshot"]["meta"]
    nf, ef = meta["node_fields"], meta["edge_fields"]
    node_types, edge_types = meta["node_types"][0], meta["edge_types"][0]
    strings = s["strings"]
    nodes, edges = s["nodes"], s["edges"]
    ns, es = len(nf), len(ef)
    t, n, i, c = nf.index("type"), nf.index("name"), nf.index("id"), nf.index("edge_count")
    et, en, to = ef.index("type"), ef.index("name_or_index"), ef.index("to_node")
    count = len(nodes) // ns
    first = [0] * (count + 1)
    for k in range(count):
        first[k + 1] = first[k] + nodes[k * ns + c]
    return {
        "count": count,
        "type": lambda k: node_types[nodes[k * ns + t]],
        "name": lambda k: strings[nodes[k * ns + n]],
        "id": lambda k: nodes[k * ns + i],
        "edges": lambda k: [(edge_types[edges[e * es + et]], edges[e * es + en], edges[e * es + to] // ns)
                            for e in range(first[k], first[k + 1])],
        "edge_name": lambda kind, x: strings[x] if kind not in ("element", "hidden") else str(x),
    }


def shaped(g, shape):
    """The nodes that carry every property of SHAPE, by id."""
    want = set(shape)
    found = []
    for k in range(g["count"]):
        if g["type"](k) != "object":
            continue
        have = {g["edge_name"](kind, x) for kind, x, _ in g["edges"](k) if kind == "property"}
        if want <= have:
            found.append(k)
    return sorted(found, key=g["id"])


def is_weak(g, kind, x):
    """Whether the edge holds its target only while something else does: a weak edge, or the entry of a WeakMap - a
    value is held by its table only as long as its KEY is alive, and the snapshot draws that as an edge of its own."""
    return kind == "weak" or (kind == "internal" and "part of key" in g["edge_name"](kind, x))


def path_to(g, target, weak=False):
    """The shortest path from node 0 to TARGET as [(edge kind, edge name, node)], or None."""
    before = {0: None}
    queue = deque([0])
    while queue:
        k = queue.popleft()
        if k == target:
            break
        for kind, x, to in g["edges"](k):
            if to in before or (is_weak(g, kind, x) and not weak):
                continue
            before[to] = (k, kind, x)
            queue.append(to)
    if target not in before:
        return None
    out, k = [], target
    while before[k] is not None:
        parent, kind, x = before[k]
        out.append((kind, g["edge_name"](kind, x), k))
        k = parent
    return list(reversed(out))


ENTRY = re.compile(r"part of key \(.*?@(\d+)\).*?\(table @(\d+)\)")


def entries(g, by_id):
    """The entries of every WeakMap in the snapshot, by KEY node: {key: [(table, value)]}. The snapshot draws an entry as an
    edge from the key to the value, named `part of key (<key> @<id>) -> value (<value> @<id>) pair in WeakMap (table
    @<id>)`; the value is held while the key lives AND the table it stands in lives."""
    out = {}
    for k in range(g["count"]):
        for kind, x, to in g["edges"](k):
            if kind == "internal":
                found = ENTRY.search(g["edge_name"](kind, x))
                if found and int(found.group(2)) in by_id:
                    out.setdefault(k, []).append((by_id[int(found.group(2))], to))
    return out


def liveness(g):
    """Why every live node lives: {node: how}, how being ("root",), ("edge", parent, kind, name) or ("entry", table,
    key). What the collector keeps: everything reachable by strong edges from the roots, and then - again and again,
    while it finds any - the value of every WeakMap entry whose key and table are both kept."""
    by_id = {g["id"](k): k for k in range(g["count"])}
    kept = entries(g, by_id)
    live = {}

    def grow(start, how):
        queue = deque()
        for k in start:
            if k not in live:
                live[k] = how
                queue.append(k)
        while queue:
            k = queue.popleft()
            for kind, x, to in g["edges"](k):
                if not is_weak(g, kind, x) and to not in live:
                    live[to] = ("edge", k, kind, g["edge_name"](kind, x))
                    queue.append(to)

    grow([0], ("root",))
    more = True
    while more:
        more = False
        for key, pairs in list(kept.items()):
            for table, value in pairs:
                if key in live and table in live and value not in live:
                    grow([value], ("entry", table, key))
                    more = True
    return live


def why_alive(g, target, live=None, depth=6):
    """The lines that say why TARGET is not garbage, [(indent, text)]; none when it is. A node held through edges reads
    as the path to it from the roots; one that is the value of a WeakMap entry reads as the entry, followed by why its
    table and why its key are kept - which is where the object is really held."""
    live = liveness(g) if live is None else live
    if target not in live:
        return []
    name = lambda k: "%s (id %d)" % (g["name"](k), g["id"](k))
    out, path, k = [], [], target
    while live[k][0] == "edge":
        _, parent, kind, edge = live[k]
        path.append((kind, edge, k))
        k = parent
    if live[k][0] == "root":
        return [(0, show(g, list(reversed(path))))]
    _, table, key = live[k]
    if path:
        down = " ".join("-[%s:%s]-> %s" % (kind, edge, g["name"](node)) for kind, edge, node in reversed(path))
        out.append((0, "%s is held by %s: %s %s" % (name(target), name(k), g["name"](k), down)))
    out.append((0, "%s is the value of an entry of %s whose key is %s" % (name(k), name(table), name(key))))
    if depth:
        for what, node in (("the table", table), ("the key", key)):
            out.append((1, "%s is kept because:" % what))
            out += [(indent + 2, text) for indent, text in why_alive(g, node, live, depth - 1)]
    return out


def show(g, path):
    return "(GC roots) " + " ".join("-[%s:%s]-> %s" % (kind, name, g["name"](k)) for kind, name, k in path)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("snapshot")
    ap.add_argument("--shape", required=True, help="the properties an object carries, comma separated")
    ap.add_argument("--pick", choices=["oldest", "middle", "newest"], default="middle")
    ap.add_argument("--count", type=int, default=3, help="how many objects to follow")
    ap.add_argument("--weak", action="store_true", help="follow weak edges too")
    args = ap.parse_args()
    g = load(args.snapshot)
    live = liveness(g)
    found = shaped(g, args.shape.split(","))
    print("%d objects carry %s, of %d nodes" % (len(found), args.shape, g["count"]))
    if not found:
        return
    at = {"oldest": 0, "middle": len(found) // 2, "newest": len(found) - args.count}[args.pick]
    for k in found[max(0, at):max(0, at) + args.count]:
        print("\nid %d:" % g["id"](k))
        why = why_alive(g, k, live)
        for indent, text in why:
            print("  " * indent + text)
        if not why:
            print("held by nothing the collector keeps: garbage it has not taken yet")


if __name__ == "__main__":
    main()
