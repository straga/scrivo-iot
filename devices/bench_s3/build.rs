fn main() {
    // the dictionary this image serves, OUT_DIR/dictionary.json: made of `bench_s3_api`, and refused - the build stops - when its table does not hold together
    dictionary::image::emit(&bench_s3_api::ALL, bench_s3_api::json);
    embuild::espidf::sysenv::output();
    version::emit_app_version();
    fell_where::emit_link_args();
}
