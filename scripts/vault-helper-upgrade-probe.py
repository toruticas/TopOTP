"""Qualify the real login Keychain using random public fixture namespaces only.

Proves same-DR direct updates fail, and that two different native Rust clients
share a fixed helper safely. No real vault, password, ACL or trust setting changes.
"""
import json
import os
import re
import shutil
import subprocess
import tempfile
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / ".artifacts/vault-helper-upgrade-probe"


def run(args, env=None):
    result = subprocess.run(list(map(str, args)), env=env, capture_output=True, text=True, timeout=180)
    if result.returncode:
        raise RuntimeError(f"{Path(args[0]).name}: {result.stderr.strip()}")
    return result.stdout


def main():
    os.umask(0o077)
    OUTPUT.mkdir(parents=True, exist_ok=True)
    helper = json.loads(run(["python3", ROOT / "scripts/prepare-vault-helper.py", "aarch64-apple-darwin"]))
    identity = helper["certificateSHA1"]
    report = {"syntheticOnly": True, "realVaultTouched": False, "defaultLoginKeychain": True,
              "keychainUIAllowed": False, "helperCDHash": helper["cdhash"], "touchIDTested": False}
    with tempfile.TemporaryDirectory(prefix="run-", dir=OUTPUT) as folder:
        work = Path(folder)
        service = "com.rafaelmariano.topotp.upgrade-probe." + str(uuid.uuid4())

        def sign(path, identifier, cert=identity):
            run(["codesign", "--force", "--sign", cert, "--identifier", identifier,
                 "--options", "runtime", "--timestamp=none", path])

        direct = []
        for index, flags in [(1, []), (2, ["-DPROBE_V2"])]:
            binary = work / f"direct-{index}"
            run(["xcrun", "swiftc", "-target", "arm64-apple-macos13.0", *flags,
                 ROOT / "scripts/keychain-login-client-probe.swift", "-o", binary])
            sign(binary, service + ".direct")
            direct.append(binary)

        def direct_probe(binary, mode):
            return json.loads(run([binary, mode, service]))

        try:
            assert direct_probe(direct[0], "add")["status"] == 0
            assert direct_probe(direct[0], "read")["matched"]
            denied = direct_probe(direct[1], "read")
            assert denied["status"] in (-25293, -25308) and not denied["matched"]
            requirements = []
            for binary in direct:
                details = subprocess.run(["codesign", "--display", "--requirements", "-", str(binary)], capture_output=True, text=True, check=True)
                requirements.append(re.search(r"designated => (.+)", details.stdout + details.stderr).group(1))
            assert requirements[0] == requirements[1]
            report.update({"sameDRDirectUpgradeDenied": True, "directUpgradeStatus": denied["status"]})
        finally:
            report["directCleanupStatus"] = direct_probe(direct[0], "delete")["status"]

        clients = []
        for variant in ("one", "two"):
            run(["cargo", "build", "--manifest-path", ROOT / "src-tauri/Cargo.toml", "--features", "native-probe", "--bin", "vault-helper-probe"],
                env=dict(os.environ, TOPOTP_PROBE_VARIANT=variant))
            client = work / ("client-" + variant)
            shutil.copy2(ROOT / "src-tauri/target/debug/vault-helper-probe", client)
            sign(client, service)
            clients.append(client)
        key = str(uuid.uuid4())

        def probe(client, mode, pin=helper["cdhash"]):
            return json.loads(run([client, helper["binary"], pin, identity, mode, key]))

        try:
            assert probe(clients[0], "insert")["success"]
            assert probe(clients[0], "read")["matched"]
            assert probe(clients[1], "read")["matched"]
            assert probe(clients[1], "update")["success"]
            assert probe(clients[0], "read")["matched"]
            foreign, unsigned = work / "foreign", work / "unsigned"
            shutil.copy2(clients[1], foreign)
            sign(foreign, service + ".foreign")
            assert not probe(foreign, "read")["matched"]
            shutil.copy2(clients[1], unsigned)
            sign(unsigned, service, "-")
            assert probe(unsigned, "read")["error"]["code"] == "VaultServiceUnavailable"
            assert probe(clients[1], "read", "0" * 40)["error"]["code"] == "VaultServiceUnavailable"
            report.update({"helperUpgradeReadPassed": True, "helperUpdatePassed": True,
                           "helperRollbackReadPassed": True, "foreignNamespaceIsolated": True,
                           "unsignedParentRejected": True, "changedHelperPinRejected": True})
        finally:
            report["helperCleanupPassed"] = probe(clients[0], "delete").get("success", False)
    (OUTPUT / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
