#[cfg(target_os = "macos")]
fn main() {
    match topotp_lib::platform::macos::probe::probe() {
        Ok(report) => {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
            if report.add_status != 0 {
                std::process::exit(2);
            }
        }
        Err(_) => {
            eprintln!("Native protected storage is unavailable; no fallback was used.");
            std::process::exit(2);
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("The native Keychain probe requires macOS.");
    std::process::exit(2);
}
