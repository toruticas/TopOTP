fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/ipc/generated.ts");
    std::fs::write(path, topotp_lib::bindings::source()).expect("write generated contracts");
}
