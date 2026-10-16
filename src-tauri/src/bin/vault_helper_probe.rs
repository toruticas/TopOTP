//! Native probe restricted to a signed synthetic namespace. Never production.
use std::{path::Path, process::Command};
use topotp_lib::{application::vault::Store, platform::macos::vault_helper::HelperStore};
use uuid::Uuid;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 6);
    let signature = Command::new("/usr/bin/codesign")
        .args(["-dvv", std::env::current_exe().unwrap().to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&signature.stderr)
            .lines()
            .any(|line| line.starts_with("Identifier=com.rafaelmariano.topotp.upgrade-probe."))
    );
    let id = Uuid::parse_str(&args[5]).unwrap();
    let mut store = match HelperStore::connect(Path::new(&args[1]), &args[2], &args[3]) {
        Ok(store) => store,
        Err(error) => {
            println!("{}", serde_json::json!({"error":error}));
            return;
        }
    };
    let payload = b"TopOTP public broker synthetic fixture";
    let result = match args[4].as_str() {
        "insert" => store.insert(id, payload).map(|_| false),
        "update" => store.update(id, payload).map(|_| false),
        "delete" => store.delete(id).map(|_| false),
        "read" => store
            .read(id)
            .map(|value| value.is_some_and(|bytes| bytes.as_slice() == payload)),
        _ => panic!("Unknown synthetic operation"),
    };
    let report = match result {
        Ok(matched) => {
            serde_json::json!({"success":true,"matched":matched,"variant":option_env!("TOPOTP_PROBE_VARIANT").unwrap_or("one")})
        }
        Err(error) => serde_json::json!({"error":error}),
    };
    println!("{report}");
}
