"""Create/reuse a persistent, local code-signing identity without an Apple account.

Only a new signing identity is imported into the default Keychain. Existing OTP
items and trust settings are never accessed. stdout contains the public SHA-1
identity selector; private material and passphrases are never printed.
"""
import fcntl
import json
import os
import re
import secrets
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STATE = Path.home() / "Library/Application Support/TopOTP Build"


def run(args, *, env=None, stdin=None):
    result = subprocess.run(list(map(str, args)), env=env, input=stdin,
                            capture_output=True, text=True, timeout=90)
    if result.returncode:
        # Neither the invocation nor subprocess diagnostics may contain a passphrase.
        raise RuntimeError(f"{Path(args[0]).name} falhou (status {result.returncode}).")
    return result.stdout


def identities():
    return run(["/usr/bin/security", "find-identity", "-p", "codesigning"])


def save(metadata):
    with tempfile.NamedTemporaryFile(mode="w", dir=STATE, delete=False) as file:
        path = Path(file.name)
        json.dump(metadata, file, indent=2)
        file.write("\n")
        file.flush()
        os.fsync(file.fileno())
    os.replace(path, STATE / "signing.json")


def identity():
    if sys.platform != "darwin":
        raise RuntimeError("Assinatura local requer macOS.")
    os.umask(0o077)
    STATE.mkdir(parents=True, exist_ok=True, mode=0o700)
    STATE.chmod(0o700)
    with (STATE / "signing.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        metadata_path = STATE / "signing.json"
        if metadata_path.exists():
            metadata = json.loads(metadata_path.read_text())
            fingerprint = metadata.get("sha1", "")
            if metadata.get("version") != 1 or not re.fullmatch(r"[A-F0-9]{40}", fingerprint):
                raise RuntimeError("Metadados de assinatura inválidos; identidade não será recriada.")
            if fingerprint not in identities():
                raise RuntimeError("Identidade local ausente/expirada ou Keychain indisponível; "
                                   "não recriar a chave, pois isso impediria atualizações do cofre.")
            certificate = STATE / "certificate.pem"
            if not certificate.is_file():
                raise RuntimeError("Certificado público local ausente; recupere-o sem substituir a identidade.")
            actual = run(["/usr/bin/openssl", "x509", "-in", certificate, "-noout", "-fingerprint", "-sha1"]).strip().split("=")[-1].replace(":", "")
            if actual != fingerprint:
                raise RuntimeError("Certificado público não corresponde à identidade registrada.")
            run(["/usr/bin/openssl", "x509", "-in", certificate, "-noout", "-checkend", "0"])
            if metadata.get("state") != "ready":
                metadata["state"] = "ready"
                save(metadata)
            return fingerprint

        with tempfile.TemporaryDirectory(prefix="identity-", dir=STATE) as directory:
            folder = Path(directory)
            private, cert, p12 = [folder / name for name in ("private.pem", "certificate.pem", "identity.p12")]
            config = folder / "certificate.cnf"
            config.write_text("[req]\ndistinguished_name=dn\nprompt=no\nx509_extensions=code_signing\n"
                              "[dn]\nCN=TopOTP Local Signing\n[code_signing]\n"
                              "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\n"
                              "extendedKeyUsage=codeSigning\nsubjectKeyIdentifier=hash\n")
            env = dict(os.environ, TOPOTP_SIGNING_TEMP_PASSWORD=secrets.token_hex(32))
            run(["/usr/bin/openssl", "req", "-x509", "-sha256", "-newkey", "rsa:3072", "-days", "3650",
                 "-config", config, "-keyout", private, "-out", cert,
                 "-passout", "env:TOPOTP_SIGNING_TEMP_PASSWORD"], env=env)
            run(["/usr/bin/openssl", "pkcs12", "-export", "-inkey", private, "-in", cert, "-out", p12,
                 "-macalg", "sha1", "-keypbe", "PBE-SHA1-3DES", "-certpbe", "PBE-SHA1-3DES",
                 "-passin", "env:TOPOTP_SIGNING_TEMP_PASSWORD",
                 "-passout", "env:TOPOTP_SIGNING_TEMP_PASSWORD"], env=env)
            fingerprint = run(["/usr/bin/openssl", "x509", "-in", cert, "-noout", "-fingerprint", "-sha1"]).strip().split("=")[-1].replace(":", "")
            importer = folder / "import-identity"
            run(["/usr/bin/xcrun", "swiftc", "-parse-as-library", ROOT / "scripts/import-local-signing-identity.swift", "-o", importer])
            # Record the identity before importing: an interruption must never
            # silently generate a second certificate on the next build.
            metadata = {"version": 1, "sha1": fingerprint, "state": "pending", "name": "TopOTP Local Signing"}
            (STATE / "certificate.pem").write_bytes(cert.read_bytes())
            save(metadata)
            run([importer], stdin=json.dumps({"path": str(p12), "password": env["TOPOTP_SIGNING_TEMP_PASSWORD"]}))
            if fingerprint not in identities():
                raise RuntimeError("A identidade importada não está disponível para assinatura.")
            metadata["state"] = "ready"
            save(metadata)
            # No private key file survives the temporary-directory scope.
            return fingerprint


if __name__ == "__main__":
    try:
        print(identity())
    except (RuntimeError, OSError, ValueError) as error:
        print(f"TopOTP: {error}", file=sys.stderr)
        sys.exit(1)
