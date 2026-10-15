"""Package the signed app with hdiutil, without Finder/AppleScript automation."""
import plistlib
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

app = Path(sys.argv[1]).resolve()
assert app.name == "TopOTP.app" and app.is_dir()
with (app / "Contents/Info.plist").open("rb") as file:
    version = plistlib.load(file)["CFBundleShortVersionString"]
arch = subprocess.check_output(["lipo", "-archs", str(app / "Contents/MacOS/topotp")], text=True).strip()
arch = {"arm64": "aarch64", "x86_64": "x64"}.get(arch, "universal")
output = app.parent.parent / "dmg" / f"TopOTP_{version}_{arch}.dmg"
output.parent.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix="topotp-dmg-") as folder:
    stage = Path(folder)
    shutil.copytree(app, stage / app.name, symlinks=True)
    (stage / "Applications").symlink_to("/Applications")
    subprocess.run(["hdiutil", "create", "-volname", "TopOTP", "-srcfolder", str(stage), "-ov", "-format", "UDZO", str(output)], check=True)
print(output)
