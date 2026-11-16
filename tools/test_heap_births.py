"""tools/heap_births.py on small trees with a known answer, and on the shape of the call that made an image fall at start.

    uv run --no-project --with pytest pytest tools/test_heap_births.py
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from heap_births import places  # noqa: E402

REAL_ROOT = os.path.dirname(HERE)

SOURCE = """
pub async fn serve(n: usize) {}
async fn page_handler(n: usize) {}
fn plain(n: usize) -> u8 { 0 }

fn run() {
    let served = block_on(Box::pin(async move {
        serve(1).await
    }));
    let serving = serve(2);
    block_on(serving);
    tasks.push(Box::pin(page_handler(3)));
    let mut fut = core::pin::pin!(serve(4));
    let sent = async {
        1
    };
    let small = Box::new(plain(5));
    // block_on(serve(6));
    let named = block_on_all(7);
}

#[cfg(test)]
mod tests {
    fn t() {
        block_on(async {});
    }
}
"""


def tree(tmp_path, text):
    src = tmp_path / "crates" / "one" / "src"
    src.mkdir(parents=True)
    (src / "lib.rs").write_text(text)
    return {(line, kind): future for _, line, kind, future, _ in places(str(tmp_path))}


def test_a_future_handed_on_is_named_a_future(tmp_path):
    got = tree(tmp_path, SOURCE)
    assert got.get((7, "Box::pin")) == "future", got  # an async block, boxed
    assert got.get((7, "block_on")) == "future", got
    assert got.get((11, "block_on")) == "future", got  # whatever it is given lives on the caller's stack
    assert got.get((12, "Box::pin")) == "future", got  # a call of an async fn of the tree
    assert got.get((13, "pin!")) == "future", got
    assert got.get((14, "async block")) == "future", got


def test_what_is_no_future_or_never_runs_is_not_named_one(tmp_path):
    got = tree(tmp_path, SOURCE)
    assert got.get((17, "Box::new")) == "", got  # a plain value: listed, not as a future
    assert not [k for k in got if k[0] == 18], got  # a comment
    assert not [k for k in got if k[0] == 19], got  # a name that only starts with block_on
    assert not [k for k in got if k[0] > 20], got  # a test module


NET_LOOP = """
pub async fn net_loop(n: usize) {}

pub fn net(n: usize) {
    block_on(Box::pin(net_loop(n)));
}
"""


def test_the_loop_that_fell_at_start_is_named_on_its_code(tmp_path):
    """An image fell at start because the net loop's whole future was built where the call stood, on the stack of the
    task that called it: `block_on(Box::pin(net_loop(n)))` in the crate that defines the async fn `net_loop`."""
    folder = tmp_path / "crates" / "web" / "src"
    folder.mkdir(parents=True)
    (folder / "net_loop.rs").write_text(NET_LOOP)
    got = {(path, line, kind): future for path, line, kind, future, _ in places(str(tmp_path))}
    assert got.get(("crates/web/src/net_loop.rs", 5, "block_on")) == "future", sorted(got)
    assert got.get(("crates/web/src/net_loop.rs", 5, "Box::pin")) == "future", sorted(got)


def test_a_namesake_of_an_async_fn_in_another_crate_is_no_future(tmp_path):
    (tmp_path / "crates" / "web" / "src").mkdir(parents=True)
    (tmp_path / "crates" / "web" / "src" / "lib.rs").write_text("async fn words() {}\nfn f() { let b = Box::pin(words()); }\n")
    (tmp_path / "crates" / "fell" / "src").mkdir(parents=True)
    (tmp_path / "crates" / "fell" / "src" / "lib.rs").write_text("fn words() -> u8 { 0 }\nfn f() { let b = Box::new(words()); }\n")
    got = {(path, kind): future for path, _, kind, future, _ in places(str(tmp_path))}
    assert got[("crates/web/src/lib.rs", "Box::pin")] == "future", got
    assert got[("crates/fell/src/lib.rs", "Box::new")] == "", got
