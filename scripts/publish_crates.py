#!/usr/bin/env python3
"""Publish the elk-rs workspace crates to crates.io, skipping versions already there.

Usage:
    python3 scripts/publish_crates.py [--dry-run] [--no-verify] [--allow-dirty]
                                      [--expect-version <version>] [--only <crate> ...] [--list]

Behaviour:
  * Reads the workspace with `cargo metadata`; crates with `publish = false`
    (the napi/wasm bindings) are ignored.
  * Asks crates.io which crate@version pairs already exist and skips them, so
    rerunning after a partial publish only uploads what is missing. Nothing on
    crates.io can be overwritten.
  * Hands the remaining crates to one `cargo publish -p a -p b ...` call, which
    orders them by dependency and waits for each upload to reach the index.
  * When crates.io answers with its publish rate limit (new crates: a burst of
    five, then one every ten minutes), sleeps $PUBLISH_CRATES_RATE_LIMIT_WAIT
    seconds (default 660) and retries with the crates still missing, up to
    $PUBLISH_CRATES_MAX_ATTEMPTS times (default 24).
  * --expect-version fails fast unless every publishable crate has that version
    (CI passes the tag's base version, e.g. 0.12.0 for v0.12.0 or v0.12.0+rs.1).
  * --dry-run runs `cargo publish --dry-run` for the missing crates and never
    touches the registry; --no-verify skips the verification build; --allow-dirty
    is passed through for dry runs on an uncommitted working tree (rejected
    without --dry-run).

Authentication: release.yml passes only the short-lived CARGO_REGISTRY_TOKEN that
rust-lang/crates-io-auth-action obtains through Trusted Publishing; CI has no
stored-token fallback. A local publish uses `cargo login` or CARGO_REGISTRY_TOKEN.
`cargo` is taken from $CARGO or PATH. Run from the repository root.
"""

import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request

CRATES_IO_API = "https://crates.io/api/v1/crates"
USER_AGENT = "elk-rs publish_crates.py (https://github.com/openedges/elk-rs)"
RATE_LIMIT_MARKERS = ("429 too many requests", "too many new crates", "too many updates", "too many crate versions", "rate limit")


def log(message):
    print(f"publish_crates: {message}", flush=True)


def fail(message):
    print(f"publish_crates: {message}", file=sys.stderr, flush=True)
    sys.exit(1)


def parse_args(argv):
    options = {"dry_run": False, "no_verify": False, "allow_dirty": False, "expect_version": None,
               "only": [], "list": False}
    it = iter(argv)
    for arg in it:
        if arg == "--dry-run":
            options["dry_run"] = True
        elif arg == "--no-verify":
            options["no_verify"] = True
        elif arg == "--allow-dirty":
            options["allow_dirty"] = True
        elif arg == "--list":
            options["list"] = True
        elif arg == "--expect-version":
            options["expect_version"] = next(it, None) or fail("--expect-version needs a value")
        elif arg == "--only":
            options["only"].append(next(it, None) or fail("--only needs a crate name"))
        elif arg in ("-h", "--help"):
            print(__doc__.strip())
            sys.exit(0)
        else:
            fail(f"unknown argument {arg!r} (see --help)")
    if options["allow_dirty"] and not options["dry_run"]:
        fail("--allow-dirty is only accepted together with --dry-run; commit before publishing")
    return options


def workspace_packages(cargo):
    result = subprocess.run(
        [cargo, "metadata", "--no-deps", "--format-version", "1"],
        check=True, capture_output=True, text=True,
    )
    metadata = json.loads(result.stdout)
    members = set(metadata["workspace_members"])
    packages = [p for p in metadata["packages"] if p["id"] in members]
    publishable = [p for p in packages if p.get("publish") != []]
    return sorted(publishable, key=lambda p: p["name"])


def crates_io_has(name, version):
    """True if crate@version exists on crates.io, False if not, raises otherwise."""
    request = urllib.request.Request(f"{CRATES_IO_API}/{name}/{version}", headers={"User-Agent": USER_AGENT})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return response.status == 200
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return False
        raise


def missing_on_registry(packages):
    missing = []
    for index, package in enumerate(packages):
        if index:
            time.sleep(1)  # crates.io asks for at most one API request per second
        if crates_io_has(package["name"], package["version"]):
            log(f"skip {package['name']}@{package['version']}: already on crates.io")
        else:
            missing.append(package)
    return missing


def run_publish(cargo, packages, options):
    command = [cargo, "publish"]
    for package in packages:
        command += ["-p", package["name"]]
    if options["dry_run"]:
        command.append("--dry-run")
    if options["no_verify"]:
        command.append("--no-verify")
    if options["allow_dirty"]:
        command.append("--allow-dirty")
    log("running: " + " ".join(command))
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    captured = []
    for line in process.stdout:
        sys.stdout.write(line)
        sys.stdout.flush()
        captured.append(line)
    process.wait()
    return process.returncode, "".join(captured)


def main(argv):
    options = parse_args(argv)
    if not os.path.isfile("Cargo.toml"):
        fail("run from the repository root")
    cargo = os.environ.get("CARGO", "cargo")
    packages = workspace_packages(cargo)
    if options["only"]:
        unknown = set(options["only"]) - {p["name"] for p in packages}
        if unknown:
            fail(f"--only names unknown or unpublishable crates: {sorted(unknown)}")
        packages = [p for p in packages if p["name"] in options["only"]]
    if options["expect_version"]:
        mismatched = [f"{p['name']}@{p['version']}" for p in packages if p["version"] != options["expect_version"]]
        if mismatched:
            fail(f"expected version {options['expect_version']} but found: {', '.join(mismatched)}")
    if options["list"]:
        for package in packages:
            print(f"{package['name']} {package['version']}")
        return

    wait_seconds = int(os.environ.get("PUBLISH_CRATES_RATE_LIMIT_WAIT", "660"))
    max_attempts = int(os.environ.get("PUBLISH_CRATES_MAX_ATTEMPTS", "24"))
    for attempt in range(1, max_attempts + 1):
        try:
            missing = missing_on_registry(packages)
        except (urllib.error.URLError, OSError) as error:
            fail(f"crates.io lookup failed: {error}")
        if not missing:
            log(f"nothing to publish: all {len(packages)} crates are on crates.io")
            return
        log(f"attempt {attempt}: {len(missing)} crate(s) to publish: " + ", ".join(p["name"] for p in missing))
        returncode, output = run_publish(cargo, missing, options)
        if returncode == 0:
            if options["dry_run"]:
                log(f"dry run finished for {len(missing)} crate(s); nothing was uploaded")
            else:
                try:
                    still_missing = missing_on_registry(missing)
                except (urllib.error.URLError, OSError) as error:
                    log(f"warning: published {len(missing)} crate(s) but the crates.io confirmation lookup failed: {error}")
                    return
                if still_missing:
                    fail("cargo publish exited 0 but these are not on crates.io yet: "
                         + ", ".join(p["name"] for p in still_missing))
                log(f"published {len(missing)} crate(s)")
            return
        lowered = output.lower()
        if not options["dry_run"] and any(marker in lowered for marker in RATE_LIMIT_MARKERS):
            if attempt == max_attempts:
                break
            log(f"crates.io rate limit hit; sleeping {wait_seconds}s before retrying the remaining crates")
            time.sleep(wait_seconds)
            continue
        fail(f"cargo publish failed with exit code {returncode}")
    fail(f"gave up after {max_attempts} attempts; rerun later to publish the remaining crates")


if __name__ == "__main__":
    main(sys.argv[1:])
