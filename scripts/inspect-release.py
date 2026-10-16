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
config = json.loads(Path("src-tauri/tauri.conf.json").read_text())
version = config["version"]
assert json.loads(Path("package.json").read_text())["version"] == version
assert info["CFBundleShortVersionString"] == version
assert info["CFBundleVersion"] == config["bundle"]["macOS"].get("bundleVersion", version)
binary = app / "Contents/MacOS" / info["CFBundleExecutable"]
data = binary.read_bytes()
for marker in [b"WDIO_WEBDRIVER_PORT", b"wdio-webdriver", b"synthetic-export-password", b"synthetic-import-password", b"imported-numeric-kind", b"topotp-synthetic-", b"import-source.json", b"tests/compatibility/reference-roundtrip.mjs", b"native-probe.json", b"accentInsensitiveResults", b"syntheticOnly", b"TopOTP%20Teste", b"com.rafaelmariano.topotp.native-smoke"]:
    assert marker not in data, f"Test marker in production binary: {marker!r}"
assert not any("chromium" in p.name.lower() or "electron" in p.name.lower() or p.name == "node" for p in app.rglob("*"))
subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True, capture_output=True)
signature = subprocess.run(["codesign", "-dvv", str(app)], check=True, capture_output=True, text=True)
adhoc = "Signature=adhoc" in signature.stderr
requirements = subprocess.run(["codesign", "--display", "--requirements", "-", str(app)], check=True, capture_output=True, text=True)
requirement = (requirements.stdout + requirements.stderr).strip()
if not adhoc:
    assert "Authority=" in signature.stderr
    assert "certificate" in requirement and "cdhash H" not in requirement
architecture = subprocess.check_output(["lipo", "-archs", str(binary)], text=True).strip()
capability = json.loads(Path("src-tauri/capabilities/management.json").read_text())
assert capability["identifier"] == "management"
assert capability["windows"] == ["management"]
assert config["app"]["security"]["capabilities"] == ["management"]
assert len(config["app"]["windows"]) == 1
window = config["app"]["windows"][0]
assert window["label"] == "management" and window["decorations"] and window["resizable"]
assert not window["devtools"] and not window["create"] and not window["visible"]
assert not capability.get("remote")
assert all(p.startswith("allow-") or p in ["core:event:allow-listen", "core:event:allow-unlisten"] for p in capability["permissions"])
report = {"bundle": str(app), "version": version, "bundleVersion": info["CFBundleVersion"], "architecture": architecture, "minimumMacOS": info["LSMinimumSystemVersion"], "signatureVerified": True, "signatureKind": "adhoc" if adhoc else "certificate", "designatedRequirement": requirement, "identityContinuityRequiresUpgradeTest": True, "testPluginMarkersAbsent": True, "embeddedNodeChromiumAbsent": True, "capabilityReviewed": True, "nativeAuthenticationManual": "pending", "notarized": False}
helper = app / "Contents/Resources/vault-helper"
metadata = app / "Contents/Resources/vault-helper.json"
if not adhoc:
    import re
    assert helper.is_file() and metadata.is_file(), "Local production requires the fixed Keychain owner"
    owner = json.loads(metadata.read_text())
    assert owner["version"] == 1 and owner["identifier"] == "com.rafaelmariano.topotp.vault-helper.v1"
    details = subprocess.run(["codesign", "-dvvv", str(helper)], check=True, capture_output=True, text=True)
    actual = re.search(r"^CDHash=([a-f0-9]{40})$", details.stderr, re.MULTILINE).group(1)
    assert actual == owner["cdhash"] and actual.encode() in data
    assert owner["certificateSHA1"].encode() in data
    assert owner["target"] == {"arm64": "aarch64-apple-darwin", "x86_64": "x86_64-apple-darwin"}[architecture]
    subprocess.run(["codesign", "--verify", "--strict", "-R=identifier \"" + owner["identifier"] + "\" and certificate leaf = H\"" + owner["certificateSHA1"] + "\"", str(helper)], check=True, capture_output=True)
    subprocess.run(["codesign", "--verify", "--strict", "-R=certificate leaf = H\"" + owner["certificateSHA1"] + "\"", str(app)], check=True, capture_output=True)
    report["fixedKeychainOwner"] = {"cdhash": actual, "target": owner["target"], "version": owner["version"]}
else:
    report["vaultUseSupported"] = False
    report["ciCompilationArtifactOnly"] = True
folder = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(".artifacts")
folder.mkdir(parents=True, exist_ok=True)
path = folder / f"release-inspection-{architecture}.json"
path.write_text(json.dumps(report, indent=2) + "\n"); path.chmod(0o600)
print(json.dumps(report, indent=2))
