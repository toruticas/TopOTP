"""Reproduce Keychain upgrades with disposable identities and synthetic data.

No login-Keychain items, trust settings, Apple account or authentication UI.
The temporary signing private key and both test keychains are removed afterwards.
"""
import argparse
import json
import os
import secrets
import shutil
import subprocess
import tempfile
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / ".artifacts/keychain-upgrade-probe"


def run(args, env=None, expected=0):
    result = subprocess.run(list(map(str, args)), capture_output=True, text=True, env=env, timeout=45)
    if result.returncode != expected:
        # Never print invocations or data; all payloads are synthetic.
        raise RuntimeError(f"{Path(args[0]).name}: exit {result.returncode}: {result.stderr.strip()}")
    return result.stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--local-identity", action="store_true", help="Use the persistent local build identity; OTP data remains disposable")
    args = parser.parse_args()
    persistent = run(["python3", ROOT / "scripts/local-signing-identity.py"]).strip() if args.local_identity else None
    os.umask(0o077)
    OUTPUT.mkdir(parents=True, exist_ok=True)
    report = {"syntheticOnly": True, "realVaultTouched": False, "keychainUIAllowed": False,
              "appleAccountUsed": False, "globalTrustSettingsChanged": False, "touchIDTested": False,
              "keychainDomain": "disposable-custom", "qualifiesDefaultLogin": False}
    with tempfile.TemporaryDirectory(prefix="run-", dir=OUTPUT) as directory:
        folder = Path(directory)
        folder.chmod(0o700)
        env = dict(os.environ, TOPOTP_SYNTHETIC_PASSWORD=secrets.token_hex(24))
        service = "com.rafaelmariano.topotp.upgrade-probe." + uuid.uuid4().hex
        first, second, foreign = [folder / name for name in ("version-one", "version-two", "foreign")]
        source = ROOT / "scripts/keychain-update-probe.swift"
        for binary, flags in [(first, []), (second, ["-DPROBE_V2"])]:
            run(["xcrun", "swiftc", "-parse-as-library", "-target", "arm64-apple-macos13.0", *flags, source, "-o", binary])
        importer = folder / "import-identity"
        run(["xcrun", "swiftc", "-parse-as-library", ROOT / "scripts/import-local-signing-identity.swift", "-o", importer])
        identifier = service + ".app"
        chains = []

        def probe(binary, mode, chain):
            return json.loads(run([binary, mode, chain, service], env=env))

        def create_chain(name):
            chain = folder / ("topotp-test-" + name + ".keychain-db")
            assert probe(first, "create-chain", chain)["status"] == 0
            chains.append(chain)
            return chain

        def sign(binary, identity="-", keychain=None, app_id=identifier):
            args = ["codesign", "--force", "--sign", identity, "--identifier", app_id,
                    "--options", "runtime", "--timestamp=none"]
            if keychain:
                args += ["--keychain", keychain]
            run([*args, binary])

        try:
            sign(first)
            sign(second)
            data_chain = create_chain("data")
            assert probe(first, "add", data_chain)["status"] == 0
            assert probe(first, "read", data_chain)["syntheticPayloadMatched"]
            incompatible = probe(second, "read", data_chain)
            assert incompatible["status"] != 0
            report["adhocUpgradeReadDenied"] = True
            report["adhocUpgradeReadStatus"] = incompatible["status"]
            assert probe(first, "delete", data_chain)["status"] == 0

            if persistent:
                signing_chain, fingerprint = None, persistent
            else:
                signing_chain = create_chain("signing")
                private, cert, p12 = [folder / name for name in ("test-private.pem", "test-cert.pem", "test-identity.p12")]
                config = folder / "certificate.cnf"
                config.write_text("[req]\ndistinguished_name=dn\nprompt=no\nx509_extensions=code_signing\n[dn]\nCN=TopOTP Disposable Upgrade Probe\n[code_signing]\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=codeSigning\nsubjectKeyIdentifier=hash\n")
                run(["/usr/bin/openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1",
                     "-config", config, "-keyout", private, "-out", cert])
                private.chmod(0o600)
                run(["/usr/bin/openssl", "pkcs12", "-export", "-inkey", private, "-in", cert, "-out", p12,
                     "-macalg", "sha1", "-keypbe", "PBE-SHA1-3DES", "-certpbe", "PBE-SHA1-3DES",
                     "-passout", "env:TOPOTP_SYNTHETIC_PASSWORD"], env=env)
                p12.chmod(0o600)
                imported = subprocess.run([str(importer), str(signing_chain)], input=json.dumps({"path": str(p12), "password": env["TOPOTP_SYNTHETIC_PASSWORD"]}), capture_output=True, text=True, timeout=30)
                assert imported.returncode == 0, imported.stdout + imported.stderr
                private.unlink()
                p12.unlink()
                fingerprint = run(["/usr/bin/openssl", "x509", "-in", cert, "-noout", "-fingerprint", "-sha1"]).strip().split("=")[-1].replace(":", "")
                identities = run(["security", "find-identity", "-p", "codesigning", signing_chain])
                if fingerprint not in identities:
                    raise RuntimeError("Identidade sintética indisponível: " + identities)
            sign(first, fingerprint, signing_chain)
            sign(second, fingerprint, signing_chain)
            assert probe(first, "add", data_chain)["status"] == 0
            reads = [probe(second, "read", data_chain) for _ in range(10)]
            assert all(value["status"] == 0 and value["syntheticPayloadMatched"] for value in reads)
            assert probe(second, "update", data_chain)["status"] == 0
            assert probe(first, "read", data_chain)["syntheticPayloadMatched"]
            report["persistentBuildIdentityUsed"] = bool(persistent)
            report["signingCertificateSHA1"] = fingerprint
            report["localIdentityUpgradeTenReadsSucceeded"] = True
            report["localIdentityRollbackReadSucceeded"] = True
            report["localIdentityUpdateSucceeded"] = True
            shutil.copyfile(second, foreign)
            foreign.chmod(0o700)
            sign(foreign, fingerprint, signing_chain, identifier + ".foreign")
            denied = probe(foreign, "read", data_chain)
            assert denied["status"] != 0
            report["sameCertificateForeignIdentifierDenied"] = True
            report["foreignReadStatus"] = denied["status"]
            assert probe(first, "delete", data_chain)["status"] == 0
        finally:
            for chain in reversed(chains):
                if chain.exists():
                    probe(first, "delete-chain", chain)
    report["temporaryIdentitiesAndKeychainsRemoved"] = True
    (OUTPUT / ("persistent-identity-report.json" if persistent else "report.json")).write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
