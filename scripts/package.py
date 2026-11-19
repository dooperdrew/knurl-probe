"""Builds the release zip of knurl-probe and checks it.

    python scripts/package.py --app target/release/knurl-probe.exe \
        --cli target/release/knurl-probe-cli.exe [--out dist]
    python scripts/package.py --notes <version>   # prints the CHANGELOG section

The zip is knurl-probe-<version>-windows-x64.zip with knurl-probe.exe (the
app, Knurl Probe), knurl-probe-cli.exe (the command line), both READMEs, both
licences and THIRD-PARTY-NOTICES.txt, plus a .sha256 next to it. Before
writing it the script checks that the notices are up to date with Cargo.lock
and that the app exe carries the version information of Cargo.toml (product
name and version, embedded by crates/knurl-probe-app/build.rs). After writing
it the script opens the zip again and fails unless it holds exactly those
files, each exe inside is byte for byte the one given, and the checksum file
matches. The last line printed is the zip's path.
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
            # The section says "Not released yet." until the tag; the release
            # notes are written at the tag, so that line is left out.
            body = re.sub(r"(?m)^Not released yet\.\s*$\n?", "", body)
            return body.strip()
    fail(f"CHANGELOG.md has no section for {ver}")


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


EXES = {"knurl-probe.exe": "-p knurl-probe-app", "knurl-probe-cli.exe": "-p knurl-probe"}


def utf16(text):
    return text.encode("utf-16-le")


def check_version_info(app, ver):
    """The app exe names Knurl Probe and this version in its version resource."""
    data = app.read_bytes()
    if data[:2] != b"MZ":
        fail(f"{app} is not a Windows executable")
    for field, value in (("ProductName", "Knurl Probe"), ("ProductVersion", ver)):
        if utf16(field) not in data or utf16(value) not in data:
            fail(f"{app} has no {field} {value!r} in its version information")


def build(exes, out):
    ver = version()
    for name, path in exes.items():
        if not path.is_file():
            fail(f"{path} not found: build it first (cargo build --release --locked {EXES[name]})")
        if path.read_bytes()[:2] != b"MZ":
            fail(f"{path} is not a Windows executable")
    check_version_info(exes["knurl-probe.exe"], ver)
    check = subprocess.run([sys.executable, str(ROOT / "scripts" / "third_party_notices.py"), "--check"])
    if check.returncode != 0:
        fail("THIRD-PARTY-NOTICES.txt is out of date: run python scripts/third_party_notices.py")
    out.mkdir(parents=True, exist_ok=True)
    zip_path = out / f"knurl-probe-{ver}-windows-x64.zip"
    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as z:
        for name, path in exes.items():
            z.write(path, name)
        for name in DOCS:
            z.write(ROOT / name, name)
    digest = sha256(zip_path)
    sha_path = zip_path.with_name(zip_path.name + ".sha256")
    sha_path.write_text(f"{digest}  {zip_path.name}\n", encoding="utf-8", newline="\n")

    # Check what was written, not what was meant to be written.
    with zipfile.ZipFile(zip_path) as z:
        names = sorted(z.namelist())
        want = sorted([*EXES, *DOCS])
        if names != want:
            fail(f"zip holds {names}, expected {want}")
        for name, path in exes.items():
            if z.read(name) != path.read_bytes():
                fail(f"{name} in the zip differs from the one built")
        if z.testzip() is not None:
            fail("the zip is corrupt")
    recorded = sha_path.read_text(encoding="utf-8").split()[0]
    if recorded != sha256(zip_path):
        fail("the .sha256 file does not match the zip")
    print(f"{zip_path.name}: {len(names)} files, sha256 {digest}")
    print(zip_path)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--app", type=pathlib.Path, help="the built knurl-probe.exe (the app)")
    parser.add_argument("--cli", type=pathlib.Path, help="the built knurl-probe-cli.exe (the command line)")
    parser.add_argument("--out", type=pathlib.Path, default=ROOT / "dist")
    parser.add_argument("--notes", metavar="VERSION", help="print the CHANGELOG section of VERSION and exit")
    args = parser.parse_args()
    if args.notes:
        print(notes(args.notes))
        return
    if not args.app or not args.cli:
        parser.error("--app and --cli are both required")
    build({"knurl-probe.exe": args.app, "knurl-probe-cli.exe": args.cli}, args.out)


if __name__ == "__main__":
    main()
