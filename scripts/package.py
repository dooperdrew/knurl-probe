"""Builds the release zip of knurl-probe and checks it.

    python scripts/package.py --exe target/release/knurl-probe.exe [--out dist]
    python scripts/package.py --notes <version>   # prints the CHANGELOG section

The zip is knurl-probe-<version>-windows-x64.zip with knurl-probe.exe, both
READMEs, both licences and THIRD-PARTY-NOTICES.txt, plus a .sha256 next to it.
After writing it the script opens the zip again and fails unless it holds
exactly those files, the exe inside is byte for byte the one given, the
notices are up to date with Cargo.lock, and the checksum file matches. The
last line printed is the zip's path.
"""

import argparse
import hashlib
import pathlib
import re
import subprocess
import sys
import zipfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
DOCS = ["README.md", "README.it.md", "LICENSE-MIT", "LICENSE-APACHE", "THIRD-PARTY-NOTICES.txt"]


def fail(message):
    print(f"error: {message}", file=sys.stderr)
    sys.exit(1)


def version():
    m = re.search(r'^version = "([^"]+)"', (ROOT / "Cargo.toml").read_text(encoding="utf-8"), re.M)
    if not m:
        fail("version not found in Cargo.toml")
    return m.group(1)


def notes(ver):
    """The body of the `## <ver>` section of CHANGELOG.md."""
    text = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    for section in re.split(r"(?m)^## ", text)[1:]:
        heading, _, body = section.partition("\n")
        if heading.split()[:1] == [ver]:
            return body.strip()
    fail(f"CHANGELOG.md has no section for {ver}")


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build(exe, out):
    ver = version()
    if not exe.is_file():
        fail(f"{exe} not found: build it first (cargo build --release -p knurl-probe)")
    check = subprocess.run([sys.executable, str(ROOT / "scripts" / "third_party_notices.py"), "--check"])
    if check.returncode != 0:
        fail("THIRD-PARTY-NOTICES.txt is out of date: run python scripts/third_party_notices.py")
    out.mkdir(parents=True, exist_ok=True)
    zip_path = out / f"knurl-probe-{ver}-windows-x64.zip"
    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as z:
        z.write(exe, "knurl-probe.exe")
        for name in DOCS:
            z.write(ROOT / name, name)
    digest = sha256(zip_path)
    sha_path = zip_path.with_name(zip_path.name + ".sha256")
    sha_path.write_text(f"{digest}  {zip_path.name}\n", encoding="utf-8", newline="\n")

    # Check what was written, not what was meant to be written.
    with zipfile.ZipFile(zip_path) as z:
        names = sorted(z.namelist())
        want = sorted(["knurl-probe.exe", *DOCS])
        if names != want:
            fail(f"zip holds {names}, expected {want}")
        if z.read("knurl-probe.exe") != exe.read_bytes():
            fail("the exe in the zip differs from the one built")
        if z.testzip() is not None:
            fail("the zip is corrupt")
    recorded = sha_path.read_text(encoding="utf-8").split()[0]
    if recorded != sha256(zip_path):
        fail("the .sha256 file does not match the zip")
    print(f"{zip_path.name}: {len(names)} files, sha256 {digest}")
    print(zip_path)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--exe", type=pathlib.Path, help="the built knurl-probe.exe")
    parser.add_argument("--out", type=pathlib.Path, default=ROOT / "dist")
    parser.add_argument("--notes", metavar="VERSION", help="print the CHANGELOG section of VERSION and exit")
    args = parser.parse_args()
    if args.notes:
        print(notes(args.notes))
        return
    if not args.exe:
        parser.error("--exe is required")
    build(args.exe, args.out)


if __name__ == "__main__":
    main()
