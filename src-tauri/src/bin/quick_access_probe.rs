#[cfg(not(debug_assertions))]
compile_error!("The native UI probe is forbidden in release builds");
fn main() {
    topotp_lib::platform::macos::quick_access::probe();
}
