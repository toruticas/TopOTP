"""Inspect a local production bundle, without launching it or reading a vault."""
import json
import plistlib
import subprocess
import sys
from pathlib import Path

app = Path(sys.argv[1]).resolve()
with (app / "Contents/Info.plist").open("rb") as file:
    info = plistlib.load(file)
assert info["CFBundleIdentifier"] == "com.rafaelmariano.topotp"
assert info["LSMinimumSystemVersion"] == "13.0"
binary = app / "Contents/MacOS" / info["CFBundleExecutable"]
data = binary.read_bytes()
for marker in [b"WDIO_WEBDRIVER_PORT", b"wdio-webdriver", b"synthetic-export-password", b"topotp-synthetic-"]:
    assert marker not in data, f"Test marker in production binary: {marker!r}"
assert not any("chromium" in p.name.lower() or "electron" in p.name.lower() or p.name == "node" for p in app.rglob("*"))
subprocess.run(["rtk", "proxy", "codesign", "--verify", "--deep", "--strict", str(app)], check=True, capture_output=True)
signature = subprocess.run(["rtk", "proxy", "codesign", "-dvv", str(app)], check=True, capture_output=True, text=True)
assert "Signature=adhoc" in signature.stderr
architecture = subprocess.check_output(["rtk", "proxy", "lipo", "-archs", str(binary)], text=True).strip()
capability = json.loads(Path("src-tauri/capabilities/panel.json").read_text())
assert capability["windows"] == ["panel"]
assert not capability.get("remote")
assert all(p.startswith("allow-") or p in ["core:event:allow-listen", "core:event:allow-unlisten"] for p in capability["permissions"])
report = {"bundle": str(app), "architecture": architecture, "minimumMacOS": info["LSMinimumSystemVersion"], "adhocSignatureVerified": True, "testPluginMarkersAbsent": True, "embeddedNodeChromiumAbsent": True, "capabilityReviewed": True, "nativeAuthenticationManual": "pending", "notarized": False}
folder = Path(".artifacts"); folder.mkdir(exist_ok=True)
path = folder / f"release-inspection-{architecture}.json"
path.write_text(json.dumps(report, indent=2) + "\n"); path.chmod(0o600)
print(json.dumps(report, indent=2))
