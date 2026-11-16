from flash_file_paths import paths_of

PATH = "/data/x/state"


def tree(tmp_path, files):
    for name, text in files.items():
        (tmp_path / name).parent.mkdir(parents=True, exist_ok=True)
        (tmp_path / name).write_text(text)
    return str(tmp_path)


def kinds(rows):
    return sorted((row[0].rsplit("/", 1)[-1], row[2]) for row in rows)


def test_a_module_that_reads_and_writes_the_file_through_its_const_is_listed_by_what_each_line_does(tmp_path):
    root = tree(tmp_path, {
        "a.rs": 'const STATE: &str = "/data/x/state";\nfn f() {\n    let r = vfs::read(STATE, sink);\n    flash_owner::write(STATE, b"");\n    flash_owner::remove(STATE);\n}\n',
    })
    assert kinds(paths_of(PATH, root)) == [("a.rs", "named"), ("a.rs", "read"), ("a.rs", "remove"), ("a.rs", "write")]


def test_a_file_that_names_the_path_in_its_text_is_listed_and_another_path_or_another_const_is_not(tmp_path):
    root = tree(tmp_path, {
        "doc.md": "the count lies in /data/x/state\n",
        "b.rs": 'const OTHER: &str = "/data/x/other";\nfn g() { vfs::read(OTHER, sink); }\n',
        "c.rs": 'const STATE: &str = "/data/y/state";\nfn g() { vfs::write(STATE, b""); }\n',
    })
    assert kinds(paths_of(PATH, root)) == [("doc.md", "named")]


def test_a_tree_without_the_path_gives_nothing(tmp_path):
    assert paths_of(PATH, tree(tmp_path, {"a.rs": "fn f() {}\n"})) == []
