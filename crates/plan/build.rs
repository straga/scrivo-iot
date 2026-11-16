// The profile of the image is the board's word (tools/board_env.py): STRAGA_PSRAM is 1 for a board with external RAM and 0 for
// one whose internal RAM is all it has. Unset - a test on the computer, a build by hand - is the large profile, the one the
// burner has run with. Any other word is refused: a profile chosen by a misspelling is chosen by nobody.
fn main() {
    println!("cargo::rustc-check-cfg=cfg(small_heap)");
    println!("cargo:rerun-if-env-changed=STRAGA_PSRAM");
    match std::env::var("STRAGA_PSRAM").as_deref() {
        Ok("0") => println!("cargo:rustc-cfg=small_heap"),
        Ok("1") | Err(std::env::VarError::NotPresent) => {}
        Ok(other) => panic!("STRAGA_PSRAM is {other:?}: tools/build.sh hands 1 or 0 from the board's psram"),
        Err(e) => panic!("STRAGA_PSRAM is not text: {e}"),
    }
}
