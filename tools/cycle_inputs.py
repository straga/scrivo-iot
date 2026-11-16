"""Every input a manual cycle depends on, collected from the source and not from memory.

    uv run --no-project python tools/cycle_inputs.py > inputs.md

The question is "what can change under a cycle, and does a row hold it". The script answers the first half by reading `crates/thermostat/src/lib.rs`:

1. the fields of the structs the cycle reads (`Tuning`, `Lockout`, `Temperatures`, `Fire`, `Thermostat`, `Manual`),
2. the bodies of `nominal`, `held`, `cycled` and `light` (and what they call among themselves), with every field they touch and the line,
3. every field of those structs the cycle does NOT read, with the functions that do (so a field is never "left out" by forgetting it),
4. where each input comes from on the board: the settings that make the thermostat's and the demand's tuning (`devices/burner/settings/src/fire.rs`) and the
   device's own `temperatures()` (`devices/burner/src/modules/thermostat.rs`), and what the demand says of the room's target (`crates/demand/src/lib.rs`).

WHAT IT DOES NOT SEE: a field read through a helper this script does not parse (it follows calls among the four functions only), a macro, a value passed on by a
`Sound` or a closure; the board's own restart and the page's saving of a setting are code of other crates, read by hand. The names of the
functions it follows are the first thing to check when it is run on another revision: a function renamed is a function missing in its output.
"""
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
THERMOSTAT = os.path.join(ROOT, "crates/thermostat/src/lib.rs")
FIRE = os.path.join(ROOT, "devices/burner/settings/src/fire.rs")
DEVICE = os.path.join(ROOT, "devices/burner/src/modules/thermostat.rs")
DEMAND = os.path.join(ROOT, "crates/demand/src/lib.rs")

CYCLE_FNS = ["nominal", "held", "cycled", "light"]
STRUCTS = ["Tuning", "Lockout", "Temperatures", "Fire", "Thermostat", "Manual"]


def read(path):
    text = open(path, encoding="utf-8").read()
    return [re.sub(r"//.*$", "", line) for line in text.split("\n")]


def block_from(lines, start):
    """The text from the first `{` at or after line START to its matching `}`: (first line, last line, body lines with their numbers)."""
    depth, begun, body = 0, False, []
    for at in range(start, len(lines)):
        for ch in lines[at]:
            if ch == "{":
                depth += 1
                begun = True
            elif ch == "}":
                depth -= 1
        if begun:
            body.append((at + 1, lines[at]))
        if begun and depth == 0:
            return at + 1, body
    return len(lines), body


def struct_fields(lines, name):
    for at, line in enumerate(lines):
        if re.match(r"\s*(pub )?struct %s\b" % name, line):
            _, body = block_from(lines, at)
            return [m.group(1) for _, l in body[1:] if (m := re.match(r"\s*(?:pub )?(\w+)\s*:", l))]
    return []


def function_body(lines, name):
    for at, line in enumerate(lines):
        if re.search(r"\bfn %s\b" % name, line):
            _, body = block_from(lines, at)
            return body
    return []


def reads_of(body):
    """(struct, field, line) for every field the body touches, by the names the functions of this file give their parameters."""
    found = []
    for number, line in body:
        for m in re.finditer(r"\b(t|readings|cycle)\.(?:0\.)?(\w+)(?:\.(\w+))?", line):
            base, first, second = m.groups()
            if base == "cycle":
                found.append(("Manual", first, number))
            elif first == "lockout" and second:
                found.append(("Lockout", second, number))
            elif first == "fire" and second and base == "readings":
                found.append(("Fire", second, number))
            elif base == "t":
                found.append(("Tuning", first, number))
            else:
                found.append(("Temperatures", first, number))
        for m in re.finditer(r"\bself\.(\w+)", line):
            found.append(("Thermostat", m.group(1), number))
    # `let fire = readings.fire;` is an alias: the fields read through it are the fire's
    if any(re.search(r"let fire = readings\.fire", line) for _, line in body):
        for number, line in body:
            for m in re.finditer(r"(?<![\w.])fire\.(\w+)", line):
                found.append(("Fire", m.group(1), number))
    return found


def calls(body, among):
    text = "\n".join(line for _, line in body)
    return [name for name in among if re.search(r"(?<![\w.])%s\(" % name, text)]


def main():
    lines = read(THERMOSTAT)
    fields = {name: struct_fields(lines, name) for name in STRUCTS}
    bodies = {name: function_body(lines, name) for name in CYCLE_FNS}
    missing = [name for name, body in bodies.items() if not body]
    if missing:
        sys.exit("not found in %s: %s" % (THERMOSTAT, ", ".join(missing)))
    direct = {name: reads_of(body) for name, body in bodies.items()}
    follows = {name: calls(body, [other for other in CYCLE_FNS if other != name]) for name, body in bodies.items()}

    def closure(name, seen=None):
        seen = seen or set()
        if name in seen:
            return set()
        seen.add(name)
        out = {name}
        for other in follows[name]:
            out |= closure(other, seen)
        return out

    print("# Входы ручного цикла (собрано `tools/cycle_inputs.py` из `%s`)\n" % os.path.relpath(THERMOSTAT, ROOT))
    print("## Функции цикла и кого они зовут\n")
    for name in CYCLE_FNS:
        print("- `%s` (строка %d): зовёт %s; сводит входы %s" % (name, bodies[name][0][0], ", ".join("`%s`" % f for f in follows[name]) or "никого", ", ".join("`%s`" % f for f in sorted(closure(name)))))

    table = {}
    for name in CYCLE_FNS:
        for struct, field, number in direct[name]:
            if struct in fields and field in fields[struct]:
                table.setdefault((struct, field), {}).setdefault(name, []).append(number)
    print("\n## Каждый вход, который цикл читает (поле структуры — кто читает — строки)\n")
    print("| вход | читает | строки |")
    print("|---|---|---|")
    for (struct, field), by in sorted(table.items()):
        print("| `%s.%s` | %s | %s |" % (struct, field, ", ".join("`%s`" % f for f in sorted(by)), "; ".join("%s: %s" % (f, ",".join(map(str, sorted(set(n))))) for f, n in sorted(by.items()))))

    # what comes into `cycled` ready-made: the water that `order` hands it was made by these, so a change of any of their inputs changes what the cycle lays itself over
    below = {}
    for name in ["water", "demanded", "locked"]:
        for struct, field, number in reads_of(function_body(lines, name)):
            if struct in fields and field in fields[struct]:
                below.setdefault((struct, field), set()).add(name)
    for number, line in function_body(lines, "order"):
        for struct, field, _ in reads_of([(number, line)]):
            if (struct, field) in (("Thermostat", "waiting"), ("Fire", "out")) or (struct == "Temperatures" and field == "fire"):
                below.setdefault((struct, field), set()).add("order")
    print("\n## Что приходит в `cycled` готовым (параметр `water`: слой ниже) и чем он сделан\n")
    print("`cycled` кладётся последним слоем `order`; слово воды, которое он получает или оставляет (`Silent`), делают эти функции из этих входов.\n")
    print("| вход | делает слово воды |")
    print("|---|---|")
    for (struct, field), by in sorted(below.items()):
        print("| `%s.%s` | %s |" % (struct, field, ", ".join("`%s`" % f for f in sorted(by))))

    # reads by the rest of the file, to say who reads what the cycle does not
    rest = {}
    for name in ["order", "water", "demanded", "locked", "next", "told", "press", "faulted", "restored", "retry", "attempt", "write_now", "pending"]:
        for struct, field, number in reads_of(function_body(lines, name)):
            rest.setdefault((struct, field), set()).add(name)
    print("\n## Каждое поле тех же структур, которого цикл НЕ читает, и кто его читает\n")
    print("| поле | читает цикл | читают прочие (`order`, `water`, `locked`…) |")
    print("|---|---|---|")
    for struct in STRUCTS:
        for field in fields[struct]:
            used = (struct, field) in table
            others = sorted(rest.get((struct, field), []))
            if not used and field in ("lockout",):
                print("| `%s.%s` | контейнер: его поля — `Lockout.*` ниже | — |" % (struct, field))
            elif not used:
                print("| `%s.%s` | нет | %s |" % (struct, field, ", ".join("`%s`" % o for o in others) or "никто в разобранных функциях (хранится / пишет только `light`/`end`/`order`)"))

    print("\n## Откуда приходят входы на плате\n")
    fire = read(FIRE)
    for fn, label in (("thermostat_tuning", "`thermostat::Tuning` ← настройки"), ("demand_tuning", "`demand::Tuning` ← настройки"), ("schedule_of", "`schedule::Schedule` ← настройки")):
        print("### %s (`fire.rs` `%s`)\n" % (label, fn))
        body = function_body(fire, fn)
        for number, line in body:
            settings = re.findall(r"\bAUTO_[A-Z_]+\b|\bWINDOW_SETTINGS\b", line)
            m = re.match(r"\s*(\w+)\s*:", line)
            if settings:
                print("- строка %d: %s ← %s" % (number, m.group(1) if m else line.strip()[:50], ", ".join("`%s`" % s for s in dict.fromkeys(settings))))
        print()
    print("### `thermostat::Temperatures` ← устройство (`modules/thermostat.rs`, `temperatures`)\n")
    device = read(DEVICE)
    for number, line in function_body(device, "temperatures"):
        m = re.match(r"\s*(\w+)\s*:\s*(.*?),?\s*$", line)
        if m and m.group(1) in fields["Temperatures"]:
            print("- строка %d: %s ← `%s`" % (number, m.group(1), m.group(2)))
    print("\n### Что дом говорит о цели комнаты (`crates/demand/src/lib.rs`, `ask`)\n")
    demand = read(DEMAND)
    for number, line in function_body(demand, "ask"):
        if "target_c" in line or "(By::" in line:
            print("- строка %d: `%s`" % (number, line.strip()))
    print("\n- поля `demand::Tuning`: %s" % ", ".join("`%s`" % f for f in struct_fields(demand, "Tuning")))
    print("- поля `demand::Verdict`: %s" % ", ".join("`%s`" % f for f in struct_fields(demand, "Verdict")))


if __name__ == "__main__":
    main()
