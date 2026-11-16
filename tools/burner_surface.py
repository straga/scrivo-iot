"""What a head can touch on the burner, listed from the code: the console words, the HTTP routes and the things that take a command - each with its class.

    uv run --no-project python tools/burner_surface.py                  # the three tables, as the skill carries them
    uv run --no-project python tools/burner_surface.py settings         # the names of the settings, by the dictionary
    uv run --no-project python tools/burner_surface.py --against <a SKILL.md that carries the tables>

The words come from crates/shell (its own words) and from `module::Verb` of the modules that devices/burner/src/modules/mod.rs lays in `ALL`; the routes from `page::Route`; the
things and the settings from the dictionary the burner's API crate makes (the one the image serves, `page_stand.dictionary_of`). Nothing is read from the board.

THE CLASS IS A DECISION OF THIS FILE, NOT OF THE CODE: reading - asks and changes nothing; setting - changes what the board keeps or how it behaves, and it can be put back by the same word;
acting - moves an organ, restarts or lowers the board, or zeroes a count. A word the code has and `FORMS` has not stops the run (exit 2): a new word gets its class here before anyone reads it as free.
`--against` reads the tables between `<!-- surface:begin -->` and `<!-- surface:end -->` of a skill, takes the first and the second cell of each row, and exits 1 on a name that is only on one side or whose class differs.
"""
import argparse
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

READ, SET, ACT = "чтение", "настройка", "исполнительное"
BY_WORD = "по слову"

# console form -> (class, what it does for a head). The first token of a form is the word the code has; the rest is how it is told.
FORMS = {
    "status": (READ, "версия, раздел, uptime, память, сеть"),
    "help": (READ, "слова платы; встроенное слово cli, не наше"),
    "settings": (READ, "настройки и их значения"),
    "fell": (READ, "что плата записала о последнем падении; адреса расшифровывает tools/where_fell.py"),
    "fell now": (ACT, "роняет плату нарочно"),
    "fell now hard": (ACT, "роняет плату нарочно, жёстко"),
    "log": (READ, "модули и громкость журнала каждого"),
    "log <module> <level>": (SET, "громкость журнала модуля; в журнале платы, не в настройках"),
    "set <name> <value>": (SET, "настройка; значение — всё, что после имени, с пробелами"),
    "restart": (ACT, "перезапускает плату"),
    "down": (ACT, "уводит плату в аварийный образ; огонь остаётся без управления"),
    "toggle <name>": (ACT, "переключает вещь"),
    "on <name>": (ACT, "включает вещь"),
    "off <name>": (ACT, "выключает вещь"),
    "press <name>": (ACT, "нажимает кнопку: start, stop, light, clean"),
    "meter": (READ, "показания датчиков, по строке на каждый"),
    "zc": (READ, "что чип держит на ноге перехода через ноль, счёт фронтов, затвор"),
    "dim <percent>": (ACT, "держит вентилятор на проценте воздуха; dim 0 останавливает"),
    "cleaned <day> <kg>": (SET, "записывает прошлую чистку, один раз, пока чистка не известна"),
    "reset pellets": (ACT, "спрашивает подтверждение, ничего не меняет"),
    "reset pellets yes": (ACT, "обнуляет счёт гранул: сегодня, вчера, месяц, всего; не возвращается"),
}

# route template -> (class, what it is). The way (GET / POST) is the code's.
ROUTES = {
    "/": (READ, "страница"),
    "/icon.svg": (READ, "значок страницы"),
    "/favicon.ico": (READ, "значок страницы"),
    "/state": (READ, "все значения по адресам словаря; не опрашивать в цикле, поток — /events"),
    "/restart": (ACT, "перезапуск"),
    "/down/*": (ACT, "вниз, только с именем платы"),
    "/toggle/*": (ACT, "вещь по адресу из словаря"),
    "/on/*": (ACT, "вещь по адресу из словаря"),
    "/off/*": (ACT, "вещь по адресу из словаря"),
    "/press/*": (ACT, "кнопка по адресу из словаря"),
    "/hold/*": (ACT, "число, которое вещь держит (тяга, затвор), по адресу из словаря"),
    "/settings": (SET, "форма настроек: пары имя=значение"),
    "/energy/*": (READ, "CSV счёта за месяц"),
    "/curve/*": (READ, "кривая по имени из словаря"),
    "/journal": (READ, "журнал на флеше, текстом"),
    "/dictionary": (READ, "что есть каждый адрес"),
    "/console": (BY_WORD, "консоль по WebSocket: слова — по таблице слов"),
    "/events": (READ, "поток значений; у платы два места под поток"),
    "/*": (READ, "файлы страницы"),
}

# what a command does to a thing, by the dictionary's `can`
CAN_WAY = {"switch": "on, off", "toggle": "toggle", "press": "press", "hold": "hold"}


def read(*path):
    with open(os.path.join(ROOT, *path), encoding="utf-8") as f:
        return f.read()


def shell_words():
    """The shell's own words from the OWN table of crates/shell, and whether the forms `fell now` / `hard` are in its source."""
    source = read("crates", "shell", "src", "lib.rs")
    block = re.search(r"const OWN:[^=]*=\s*\[(.*?)\];", source, re.S).group(1)
    words = re.findall(r'\(\s*"(\w+)"\s*,', block)
    if not ('Some("now")' in source and '"hard"' in source):
        raise SystemExit("burner_surface: the forms `fell now` and `fell now hard` are not in crates/shell any more")
    return words


def module_files():
    laid = read("devices", "burner", "src", "modules", "mod.rs")
    laid_in = laid.split("pub static ALL", 1)[1].split("= &[", 1)[1]
    return re.findall(r"&(\w+)::Module\b", laid_in)


def verbs_and_takes():
    verbs, takes = [], {}
    for name in module_files():
        source = read("devices", "burner", "src", "modules", name + ".rs")
        for word in re.findall(r'module::Verb\s*\{\s*name:\s*"(\w+)"', source):
            verbs.append((word, name))
        m = re.search(r"fn takes\(&self\)[^{]*\{\s*&\[([^\]]*)\]", source)
        if m:
            takes[name] = re.findall(r'"(\w+)"', m.group(1))
    return verbs, takes


def routes():
    source = read("crates", "page", "src", "lib.rs")
    uri_block = re.search(r"pub fn uri\(self\)[^{]*\{\s*match self \{(.*?)\n        \}", source, re.S).group(1)
    uris = {name: (re.search(r'"([^"]*)"', rest).group(1) if '"' in rest else None) for name, rest in re.findall(r"Route::(\w+)\s*=>\s*([^,]*),", uri_block)}
    if uris.get("Console") is None:
        uris["Console"] = re.search(r'pub const CONSOLE: &str = "([^"]*)"', source).group(1)
    post = re.search(r"pub fn way\(self\)[^{]*\{\s*match self \{(.*?)=> Way::Post", source, re.S).group(1)
    posted = set(re.findall(r"Route::(\w+)", post))
    return [(uris[name], "POST" if name in posted else "GET") for name in uris]


def dictionary():
    import page_stand
    return page_stand.dictionary_of(device="burner")


def console_rows(verbs, takes):
    """[(form, class, note, where)] for every form the code has, and the names the code has and FORMS has not."""
    own = shell_words()
    code_words = set(own) | {"help"} | {w for w, _ in verbs}
    optional = {"toggle", "on", "off", "press"}
    carried = {w for t in takes.values() for w in t}
    rows, seen = [], set()
    for form, (klass, note) in FORMS.items():
        word = form.split()[0]
        if word not in code_words:
            raise SystemExit("burner_surface: FORMS has `%s`, the code has no such word" % form)
        if word in optional and word not in carried:
            continue
        where = "" if word in own or word == "help" else "модуль " + next(m for w, m in verbs if w == word)
        rows.append((form, klass, note, where))
        seen.add(word)
    lost = sorted(code_words - seen - {w for w in optional if w not in carried})
    if lost:
        sys.stderr.write("burner_surface: the code has words with no class in FORMS: %s\n" % ", ".join(lost))
        raise SystemExit(2)
    return rows


def route_rows():
    rows, found = [], set()
    for uri, way in routes():
        if uri not in ROUTES:
            sys.stderr.write("burner_surface: the code has the route %s with no class in ROUTES\n" % uri)
            raise SystemExit(2)
        klass, note = ROUTES[uri]
        rows.append(("%s %s" % (way, uri), klass, note, ""))
        found.add(uri)
    gone = set(ROUTES) - found
    if gone:
        raise SystemExit("burner_surface: ROUTES has %s, the code has no such route" % ", ".join(sorted(gone)))
    return rows


def thing_rows(d):
    rows = []
    for t in d["things"]:
        if not t.get("can") or t.get("part_of"):
            continue
        ways = ", ".join(CAN_WAY[c] for c in t["can"])
        note = ways + ("; необратимо" if t.get("irreversible") else "")
        if t.get("range"):
            note += "; %s..%s" % (t["range"]["min"], t["range"]["max"])
        rows.append(("%s.%s" % (t["card"], t["name"]), ACT, note, ""))
    return rows


def table(title, rows, head):
    out = ["**%s**" % title, "", head, "|---|---|---|"]
    for name, klass, note, where in rows:
        out.append("| `%s` | %s | %s%s |" % (name, klass, note, (" (" + where + ")") if where else ""))
    return "\n".join(out)


def tables():
    verbs, takes = verbs_and_takes()
    d = dictionary()
    return {
        "console": console_rows(verbs, takes),
        "http": route_rows(),
        "things": thing_rows(d),
    }, d


def against(path, got):
    text = open(os.path.expanduser(path), encoding="utf-8").read()
    m = re.search(r"<!-- surface:begin -->(.*?)<!-- surface:end -->", text, re.S)
    if not m:
        raise SystemExit("burner_surface: no `<!-- surface:begin -->` ... `<!-- surface:end -->` in %s" % path)
    theirs = {}
    for line in m.group(1).splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) >= 2 and re.fullmatch(r"`[^`]+`", cells[0]):
            theirs[cells[0].strip("`")] = cells[1]
    mine = {name: klass for rows in got.values() for name, klass, _, _ in rows}
    bad = []
    for name in sorted(set(mine) - set(theirs)):
        bad.append("в коде есть, в скилле нет: %s" % name)
    for name in sorted(set(theirs) - set(mine)):
        bad.append("в скилле есть, в коде нет: %s" % name)
    for name in sorted(set(mine) & set(theirs)):
        if mine[name] != theirs[name]:
            bad.append("класс расходится у %s: код %s, скилл %s" % (name, mine[name], theirs[name]))
    print("в коде: %d, в скилле: %d, расхождений: %d" % (len(mine), len(theirs), len(bad)))
    for line in bad:
        print("  " + line)
    return 1 if bad else 0


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("what", nargs="?", choices=["tables", "settings"], default="tables")
    ap.add_argument("--against", help="a skill file whose tables are checked against the code")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args(argv)
    got, d = tables()
    if args.against:
        return against(args.against, got)
    if args.what == "settings":
        names = [s["name"] for s in d["settings"]]
        print("\n".join(names) if not args.json else json.dumps(names))
        return 0
    if args.json:
        print(json.dumps(got, ensure_ascii=False, indent=1))
        return 0
    print("<!-- surface:begin -->")
    print(table("Слова консоли (порт 23)", got["console"], "| слово | класс | что делает |"))
    print()
    print(table("Адреса HTTP", got["http"], "| адрес | класс | что делает |"))
    print()
    print(table("Вещи, которым можно дать команду (адрес вещи берётся из /dictionary)", got["things"], "| вещь | класс | команды |"))
    print("<!-- surface:end -->")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
