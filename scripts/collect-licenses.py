"""Collect attribution from the active macOS Cargo graph and npm runtime graph."""
import json
import subprocess
from pathlib import Path

metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--manifest-path", "src-tauri/Cargo.toml", "--format-version", "1", "--locked", "--filter-platform", "aarch64-apple-darwin"], text=True))
active = {node["id"] for node in metadata["resolve"]["nodes"]}
sections = []
for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
    if package["id"] not in active or package["name"] == "topotp":
        continue
    folder = Path(package["manifest_path"]).parent
    files = sorted({p for pattern in ["LICENSE*", "LICENCE*", "COPYING*", "license*"] for p in folder.glob(pattern) if p.is_file()})
    if package.get("license_file"):
        specific = folder / package["license_file"]
        if specific.is_file() and specific not in files:
            files.append(specific)
    text = "\n\n".join(p.read_text(errors="replace") for p in files)
    sections.append(f"Rust: {package['name']} {package['version']} — {package.get('license', '')}\n{package.get('repository', '')}\n{text}")
lock = json.loads(Path("package-lock.json").read_text())
for location, package in sorted(lock["packages"].items()):
    if not location or package.get("dev"):
        continue
    folder = Path(location)
    files = sorted(p for pattern in ["LICENSE*", "LICENCE*", "COPYING*"] for p in folder.glob(pattern) if p.is_file())
    text = "\n\n".join(p.read_text(errors="replace") for p in files)
    sections.append(f"JavaScript: {location} {package.get('version', '')} — {package.get('license', '')}\n{text}")
output = Path("src-tauri/resources/THIRD_PARTY_LICENSES.txt")
output.parent.mkdir(exist_ok=True)
output.write_text("TopOTP — third-party licenses and attribution\n\n" + "\n\n".join(sections) + "\n")
print(f"Collected attribution for {len(sections)} dependencies.")
