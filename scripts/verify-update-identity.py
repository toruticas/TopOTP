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
    compatible = previous_id == candidate_id and forward and backward
    print(json.dumps({
        "previous": str(previous), "candidate": str(candidate),
        "previousRequirement": previous_requirement,
        "candidateRequirement": candidate_requirement,
        "candidateSatisfiesPreviousIdentity": forward,
        "previousSatisfiesCandidateIdentity": backward,
        "signedIdentityCompatible": compatible,
        "keychainAccessTested": False,
        "result": "Preflight aprovado; validar upgrade com cofre sintético."
        if compatible else "Atualização bloqueada: identidades de assinatura incompatíveis.",
    }, ensure_ascii=False, indent=2))
    return 0 if compatible else 1


if __name__ == "__main__":
    raise SystemExit(main())
