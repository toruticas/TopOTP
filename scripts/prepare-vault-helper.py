"""Create once, then reuse the exact native Keychain owner for local builds.

Only public build metadata/binaries live here. No OTP items are accessed.
Changing this helper requires a new protocol/version and explicit migration.
"""
import argparse
import fcntl
import hashlib
import json
import os
import re
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STATE = Path.home() / "Library/Application Support/TopOTP Build/vault-helper-v1"
SOURCE = ROOT / "scripts/vault-helper-v1.swift"
IDENTIFIER = "com.rafaelmariano.topotp.vault-helper.v1"


def run(args):
    result = subprocess.run(list(map(str, args)), capture_output=True, text=True, timeout=120)
    if result.returncode:
        raise RuntimeError(f"{Path(args[0]).name}: {result.stderr.strip()}")
    return result.stdout


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cdhash(path):
    result = subprocess.run(["codesign", "-dvvv", str(path)], capture_output=True, text=True, check=True)
    return re.search(r"^CDHash=([a-f0-9]{40})$", result.stderr, re.MULTILINE).group(1)


def prepare(target):
    architectures = {"aarch64-apple-darwin": "arm64", "x86_64-apple-darwin": "x86_64"}
    if target not in architectures:
        raise ValueError("Arquitetura macOS não suportada")
    identity = run(["python3", ROOT / "scripts/local-signing-identity.py"]).strip()
    source_hash = digest(SOURCE)
    folder = STATE / target
    folder.mkdir(parents=True, exist_ok=True, mode=0o700)
    binary, metadata = folder / "vault-helper", folder / "metadata.json"
    with (folder / "build.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if metadata.exists():
            value = json.loads(metadata.read_text())
            if (value.get("version") != 1 or value.get("target") != target or value.get("certificateSHA1") != identity
                    or value.get("sourceSHA256") != source_hash
                    or not binary.is_file() or value.get("binarySHA256") != digest(binary)
                    or value.get("cdhash") != cdhash(binary)):
                raise RuntimeError("Helper fixo divergente/ausente. Não regenerar: recupere o artefato original ou planeje migração explícita de versão.")
        else:
            if binary.exists():
                raise RuntimeError("Helper sem metadados; não substituir automaticamente")
            if (Path.home() / "Library/Application Support/com.rafaelmariano.topotp/vault-broker-v1/manifest.json").exists():
                raise RuntimeError("Existe cofre do helper, mas o artefato desta arquitetura não está preservado. Recupere o helper original; não criar outro dono automaticamente.")
            with tempfile.TemporaryDirectory(prefix="create-", dir=folder) as temporary:
                work = Path(temporary)
                generated = work / "helper.swift"
                generated.write_text(SOURCE.read_text().replace("@@SIGNING_CERTIFICATE_SHA1@@", identity))
                built = work / "vault-helper"
                run(["xcrun", "swiftc", "-suppress-warnings", "-O", "-target",
                     architectures[target] + "-apple-macos13.0", generated, "-o", built])
                run(["codesign", "--force", "--sign", identity, "--identifier", IDENTIFIER,
                     "--options", "runtime", "--timestamp=none", built])
                value = {"version": 1, "target": target, "identifier": IDENTIFIER,
                         "certificateSHA1": identity, "sourceSHA256": source_hash,
                         "binarySHA256": digest(built), "cdhash": cdhash(built)}
                # Record ownership before moving the artifact. An interrupted
                # creation must fail closed rather than silently mint a new owner.
                with metadata.open("x") as file:
                    file.write(json.dumps(value, indent=2) + "\n")
                    file.flush()
                    os.fsync(file.fileno())
                os.replace(built, binary)
                directory = os.open(folder, os.O_RDONLY)
                try:
                    os.fsync(directory)
                finally:
                    os.close(directory)
        run(["codesign", "--verify", "--strict", "-R=identifier \"" + IDENTIFIER
             + "\" and certificate leaf = H\"" + identity + "\"", binary])
        return {**value, "binary": str(binary), "metadata": str(metadata)}


if __name__ == "__main__":
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target")
    args = parser.parse_args()
    try:
        print(json.dumps(prepare(args.target)))
    except (RuntimeError, ValueError, OSError) as error:
        parser.exit(1, f"TopOTP: {error}\n")
