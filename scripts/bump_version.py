#!/usr/bin/env python3
"""Set the elk-rs package version in every manifest that carries it.

Usage:
    python3 scripts/bump_version.py <version> [--no-cargo-lock]
    python3 scripts/bump_version.py --check

<version> is the bare package version, for example 0.11.1: no leading `v` and
no `+rs.N` build metadata. Git tags are `v<version>+rs.<N>`; npm strips build
metadata on publish and crates.io rejects a second version that differs only
in build metadata, so the registries only ever see <version>.

Files updated (all must agree; `--check` verifies that and prints the version):
  Cargo.toml                                    workspace.package.version and the
                                                `=<version>` pins in workspace.dependencies
  Cargo.lock                                    via `cargo update --workspace`
                                                (skip with --no-cargo-lock)
  plugins/org.eclipse.elk.js/package.json       version + optionalDependencies (@elk-rs/*)
  plugins/org.eclipse.elk.js/package-lock.json  root package version entries
  plugins/org.eclipse.elk.js/npm/*/package.json version
  plugins/org.eclipse.elk.napi/package.json     version (private helper manifest)
  packages/elk-live/package.json                version (private)
  packages/elk-live/package-lock.json           root package version entries

Run from the repository root. `cargo` is taken from $CARGO or PATH.
"""

import glob
import json
import os
import re
import subprocess
import sys

ROOT_CARGO = "Cargo.toml"
MAIN_NPM = "plugins/org.eclipse.elk.js/package.json"
MAIN_NPM_LOCK = "plugins/org.eclipse.elk.js/package-lock.json"
PLATFORM_NPM_GLOB = "plugins/org.eclipse.elk.js/npm/*/package.json"
NAPI_NPM = "plugins/org.eclipse.elk.napi/package.json"
LIVE_NPM = "packages/elk-live/package.json"
LIVE_NPM_LOCK = "packages/elk-live/package-lock.json"

VERSION_RE = re.compile(r"^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$")


def fail(message):
    print(f"bump_version: {message}", file=sys.stderr)
    sys.exit(1)


def read(path):
    with open(path, encoding="utf-8") as handle:
        return handle.read()


def write(path, text):
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(text)


# --- Cargo.toml -------------------------------------------------------------

def cargo_versions(text):
    """Return (package_version, [pinned dependency versions])."""
    section = None
    package_version = None
    pins = []
    for line in text.splitlines():
        header = re.match(r"^\[(.+)\]$", line)
        if header:
            section = header.group(1)
            continue
        if section == "workspace.package":
            match = re.match(r'^version = "([^"]+)"$', line)
            if match:
                package_version = match.group(1)
        elif section == "workspace.dependencies" and line.startswith("org-eclipse-elk-"):
            match = re.search(r'version = "=([^"]+)"', line)
            if match:
                pins.append(match.group(1))
    if package_version is None:
        fail(f"{ROOT_CARGO}: workspace.package.version not found")
    if not pins:
        fail(f"{ROOT_CARGO}: no `version = \"=X\"` pins in [workspace.dependencies]")
    return package_version, pins


def set_cargo_version(text, version):
    section = None
    out = []
    for line in text.splitlines():
        header = re.match(r"^\[(.+)\]$", line)
        if header:
            section = header.group(1)
        elif section == "workspace.package" and re.match(r'^version = "[^"]+"$', line):
            line = f'version = "{version}"'
        elif section == "workspace.dependencies" and line.startswith("org-eclipse-elk-"):
            line = re.sub(r'version = "=[^"]+"', f'version = "={version}"', line)
        out.append(line)
    return "\n".join(out) + "\n"


# --- JSON manifests ---------------------------------------------------------

def load_json(path):
    return json.loads(read(path))


def dump_json(path, data):
    # npm writes 2-space JSON; json.dumps(indent=2) reproduces it byte for byte
    # for these files (verified with a round trip before the first use).
    write(path, json.dumps(data, indent=2, ensure_ascii=False) + "\n")


def json_versions(path, data):
    versions = {f"{path}:version": data.get("version")}
    for key, value in (data.get("optionalDependencies") or {}).items():
        if key.startswith("@elk-rs/"):
            versions[f"{path}:optionalDependencies.{key}"] = value
    return versions


def set_json_version(data, version):
    data["version"] = version
    for key in list((data.get("optionalDependencies") or {}).keys()):
        if key.startswith("@elk-rs/"):
            data["optionalDependencies"][key] = version


def lock_versions(path, data):
    root = data.get("packages", {}).get("", {})
    versions = {f"{path}:version": data.get("version"), f"{path}:packages..version": root.get("version")}
    for key, value in (root.get("optionalDependencies") or {}).items():
        if key.startswith("@elk-rs/"):
            versions[f"{path}:packages..optionalDependencies.{key}"] = value
    return versions


def set_lock_version(data, version):
    data["version"] = version
    root = data.setdefault("packages", {}).setdefault("", {})
    root["version"] = version
    for key in list((root.get("optionalDependencies") or {}).keys()):
        if key.startswith("@elk-rs/"):
            root["optionalDependencies"][key] = version


# Platform package.json files use compact arrays (`"os": ["darwin"]`), which a
# JSON round trip would expand, so they are edited line by line.
PLATFORM_VERSION_RE = re.compile(r'^(\s*"version":\s*)"([^"]+)"(,?)$', re.M)


def platform_version(text, path):
    matches = PLATFORM_VERSION_RE.findall(text)
    if len(matches) != 1:
        fail(f"{path}: expected exactly one \"version\" line, found {len(matches)}")
    return matches[0][1]


def set_platform_version(text, version):
    return PLATFORM_VERSION_RE.sub(lambda m: f'{m.group(1)}"{version}"{m.group(3)}', text, count=1)


# --- driver -----------------------------------------------------------------

def collect():
    found = {}
    package_version, pins = cargo_versions(read(ROOT_CARGO))
    found[f"{ROOT_CARGO}:workspace.package.version"] = package_version
    for index, pin in enumerate(pins):
        found[f"{ROOT_CARGO}:workspace.dependencies[{index}]"] = pin
    for path in (MAIN_NPM, NAPI_NPM, LIVE_NPM):
        found.update(json_versions(path, load_json(path)))
    for path in (MAIN_NPM_LOCK, LIVE_NPM_LOCK):
        found.update(lock_versions(path, load_json(path)))
    for path in sorted(glob.glob(PLATFORM_NPM_GLOB)):
        found[f"{path}:version"] = platform_version(read(path), path)
    return found


def check():
    found = collect()
    versions = sorted(set(found.values()), key=str)
    if len(versions) != 1:
        print("bump_version: version mismatch", file=sys.stderr)
        for key, value in sorted(found.items()):
            print(f"  {value!s:>12}  {key}", file=sys.stderr)
        sys.exit(1)
    print(versions[0])


def bump(version, update_lock):
    if not VERSION_RE.match(version):
        fail(f"invalid version {version!r}: expected MAJOR.MINOR.PATCH[-pre] without `v` or `+build`")
    write(ROOT_CARGO, set_cargo_version(read(ROOT_CARGO), version))
    for path in (MAIN_NPM, NAPI_NPM, LIVE_NPM):
        data = load_json(path)
        set_json_version(data, version)
        dump_json(path, data)
    for path in (MAIN_NPM_LOCK, LIVE_NPM_LOCK):
        data = load_json(path)
        set_lock_version(data, version)
        dump_json(path, data)
    platform_paths = sorted(glob.glob(PLATFORM_NPM_GLOB))
    for path in platform_paths:
        write(path, set_platform_version(read(path), version))
    print(f"set version {version} in {ROOT_CARGO}, {MAIN_NPM}, {MAIN_NPM_LOCK}, {NAPI_NPM}, "
          f"{LIVE_NPM}, {LIVE_NPM_LOCK} and {len(platform_paths)} platform package.json files")
    if update_lock:
        cargo = os.environ.get("CARGO", "cargo")
        try:
            subprocess.run([cargo, "update", "--workspace", "--offline"], check=True)
        except FileNotFoundError:
            fail(f"{cargo!r} not found; run `cargo update --workspace` yourself or pass --no-cargo-lock")
        except subprocess.CalledProcessError as error:
            fail(f"`cargo update --workspace` failed with exit code {error.returncode}")
        print("updated Cargo.lock")
    else:
        print("skipped Cargo.lock; run `cargo update --workspace` before committing")
    check()


def main(argv):
    args = [arg for arg in argv if not arg.startswith("--")]
    flags = {arg for arg in argv if arg.startswith("--")}
    unknown = flags - {"--check", "--no-cargo-lock", "--help"}
    if unknown or "--help" in flags or (("--check" in flags) == bool(args)):
        print(__doc__.strip(), file=sys.stderr)
        sys.exit(0 if "--help" in flags else 2)
    if not os.path.isfile(ROOT_CARGO):
        fail("run from the repository root")
    if "--check" in flags:
        check()
    else:
        bump(args[0], update_lock="--no-cargo-lock" not in flags)


if __name__ == "__main__":
    main(sys.argv[1:])
