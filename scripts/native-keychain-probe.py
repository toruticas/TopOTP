"""Build and run an isolated native .app probe using public synthetic data only."""

import argparse
import json
import os
import pathlib
import plistlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUTPUT = ROOT / ".artifacts" / "native-keychain-probe"
BUNDLE = OUTPUT / "TopOTP Keychain Probe.app"
IDENTIFIER = "com.rafaelmariano.topotp.probe"


def run(args):
    subprocess.run(["rtk", "proxy", *map(str, args)], check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--interactive", action="store_true", help="Request native authentication")
    parser.add_argument("--rust", action="store_true", help="Probe Rust bindings (noninteractive creation only)")
    parser.add_argument("--data-protection", action="store_true", help="Reproduce the original provisioning-dependent model")
    parser.add_argument("--identity", help="Installed signing identity for the probe")
    parser.add_argument("--profile", type=pathlib.Path, help="Provisioning profile authorizing the probe identifier")
    parser.add_argument("--entitlements", type=pathlib.Path, help="Matching entitlements plist (no invented Team ID)")
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("This probe requires macOS")
    if args.rust and args.interactive:
        parser.error("The Rust probe verifies item creation only; use Swift for interactive API checks")
    contents = BUNDLE / "Contents"
    executable = contents / "MacOS" / "probe"
    executable.parent.mkdir(parents=True, exist_ok=True)
    with (contents / "Info.plist").open("wb") as target:
        plistlib.dump({
            "CFBundleIdentifier": IDENTIFIER,
            "CFBundleName": "TopOTP Keychain Probe",
            "CFBundleExecutable": "probe",
            "CFBundlePackageType": "APPL",
            "CFBundleVersion": "1",
            "LSUIElement": True,
            "LSMinimumSystemVersion": "13.0",
        }, target)
    if args.rust:
        run(["cargo", "build", "--manifest-path", ROOT / "src-tauri" / "Cargo.toml",
             "--features", "native-probe", "--bin", "keychain-probe"])
        shutil.copyfile(ROOT / "src-tauri" / "target" / "debug" / "keychain-probe", executable)
    else:
        run(["xcrun", "swiftc", "-parse-as-library", "-target", "arm64-apple-macos13.0",
             ROOT / "scripts" / "native-keychain-probe.swift", "-o", executable])
    if args.profile:
        shutil.copyfile(args.profile, contents / "embedded.provisionprofile")
    os.chmod(executable, 0o755)
    sign = ["codesign", "--force", "--sign", args.identity or "-", "--options", "runtime"]
    if args.entitlements:
        sign += ["--entitlements", args.entitlements.resolve()]
    run([*sign, BUNDLE])
    invocation = [str(executable)] + (["--interactive"] if args.interactive else [])
    if args.data_protection:
        invocation += ["--data-protection"]
    if not args.rust and not args.data_protection:
        reader = OUTPUT / "foreign-reader"
        shutil.copyfile(executable, reader)
        os.chmod(reader, 0o755)
        run(["codesign", "--force", "--sign", "-", "--identifier", IDENTIFIER + ".foreign-reader", reader])
        invocation += ["--reader", str(reader)]
    result = subprocess.run(invocation, check=False, capture_output=True, text=True)
    if result.returncode not in (0, 2):
        result.check_returncode()
    report = json.loads(result.stdout)
    report["signing"] = "configured-identity" if args.identity else "ad-hoc"
    report["embeddedProfile"] = bool(args.profile)
    report["implementation"] = "rust-bindings" if args.rust else "swift-sdk"
    OUTPUT.mkdir(parents=True, exist_ok=True)
    destination = OUTPUT / ("rust-report.json" if args.rust else "report.json")
    destination.write_text(json.dumps(report, indent=2) + "\n")
    os.chmod(destination, 0o600)
    print(json.dumps(report, indent=2))
    if report.get("addStatus") != 0:
        print("Native protected storage is unavailable; no fallback was used.", file=sys.stderr)
        return 2
    if args.interactive and not report.get("authenticatedReadVerified"):
        return 3
    if not args.rust and args.data_protection and not args.interactive and not report.get("unauthenticatedReadDenied"):
        return 4
    if not args.rust and not args.data_protection and not report.get("foreignReadDenied"):
        return 5
    return 0


if __name__ == "__main__":
    sys.exit(main())
