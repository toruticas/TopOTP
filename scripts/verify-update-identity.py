"""Compare signed app identities without opening a Keychain or launching apps.

This is an update preflight, not proof that existing Keychain ACLs trust an app.
Two valid ad hoc signatures can still represent incompatible identities.
"""
import argparse
import json
import plistlib
import re
import subprocess
from pathlib import Path


def inspect(app: Path) -> tuple[str, str]:
    with (app / "Contents/Info.plist").open("rb") as file:
        identifier = plistlib.load(file)["CFBundleIdentifier"]
    subprocess.run(
        ["codesign", "--verify", "--deep", "--strict", str(app)],
        check=True, capture_output=True, text=True,
    )
    display = subprocess.run(
        ["codesign", "--display", "--requirements", "-", str(app)],
        check=True, capture_output=True, text=True,
    )
    match = re.search(
        r"^\s*(?:#\s*)?designated\s*=>\s*(.+)$",
        display.stdout + "\n" + display.stderr, re.MULTILINE,
    )
    if not match:
        raise ValueError(f"Identidade designada ausente: {app}")
    return identifier, match.group(1)


def accepts(app: Path, requirement: str) -> bool:
    result = subprocess.run(
        ["codesign", "--verify", "--strict", "-R=" + requirement, str(app)],
        capture_output=True, text=True,
    )
    if result.returncode not in (0, 3):
        raise RuntimeError("Não foi possível avaliar a identidade: " + result.stderr.strip())
    return result.returncode == 0


def code_hash(path: Path) -> str:
    result = subprocess.run(["codesign", "-dvvv", str(path)], capture_output=True, text=True, check=True)
    return re.search(r"^CDHash=([a-f0-9]{40})$", result.stderr, re.MULTILINE).group(1)


def keychain_owner(app: Path) -> dict:
    resources = app / "Contents/Resources"
    helper, metadata = resources / "vault-helper", resources / "vault-helper.json"
    if not helper.exists() and not metadata.exists():
        return {"kind": "direct", "cdhash": code_hash(app)}
    value = json.loads(metadata.read_text())
    if value.get("version") != 1 or value.get("identifier") != "com.rafaelmariano.topotp.vault-helper.v1":
        raise ValueError("Metadados de helper não suportados")
    actual = code_hash(helper)
    if actual != value["cdhash"]:
        raise ValueError("Helper diverge dos metadados fixados")
    subprocess.run(["codesign", "--verify", "--strict", "-R=identifier \""
                    + value["identifier"] + "\" and certificate leaf = H\""
                    + value["certificateSHA1"] + "\"", str(helper)], check=True, capture_output=True)
    subprocess.run(["codesign", "--verify", "--strict", "-R=certificate leaf = H\""
                    + value["certificateSHA1"] + "\"", str(app)], check=True, capture_output=True)
    info = plistlib.loads((app / "Contents/Info.plist").read_bytes())
    binary = (app / "Contents/MacOS" / info["CFBundleExecutable"]).read_bytes()
    if actual.encode() not in binary or value["certificateSHA1"].encode() not in binary:
        raise ValueError("O executável não contém os pins do helper empacotado")
    return {"kind": "helper-v1", "cdhash": actual, "target": value["target"]}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("previous", type=Path)
    parser.add_argument("candidate", type=Path)
    args = parser.parse_args()
    previous, candidate = args.previous.resolve(), args.candidate.resolve()
    previous_id, previous_requirement = inspect(previous)
    candidate_id, candidate_requirement = inspect(candidate)
    forward = accepts(candidate, previous_requirement)
    backward = accepts(previous, candidate_requirement)
    signed_compatible = previous_id == candidate_id and forward and backward
    old_owner, new_owner = keychain_owner(previous), keychain_owner(candidate)
    owner_compatible = old_owner == new_owner
    compatible = signed_compatible and owner_compatible
    print(json.dumps({
        "previous": str(previous), "candidate": str(candidate),
        "previousRequirement": previous_requirement,
        "candidateRequirement": candidate_requirement,
        "candidateSatisfiesPreviousIdentity": forward,
        "previousSatisfiesCandidateIdentity": backward,
        "signedIdentityCompatible": signed_compatible,
        "previousKeychainOwner": old_owner,
        "candidateKeychainOwner": new_owner,
        "keychainOwnerCompatible": owner_compatible,
        "normalUpdateCompatible": compatible,
        "migrationRequired": not owner_compatible,
        "keychainAccessTested": False,
        "result": "Preflight aprovado; validar upgrade com cofre sintético."
        if compatible else "Atualização normal bloqueada: identidade ou dono da partição do Keychain incompatível. Uma migração explícita é um fluxo separado.",
    }, ensure_ascii=False, indent=2))
    return 0 if compatible else 1


if __name__ == "__main__":
    raise SystemExit(main())
