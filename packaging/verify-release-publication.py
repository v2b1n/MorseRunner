#!/usr/bin/env python3
"""Validate an immutable pre-release before and after stable promotion."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
import subprocess
import tomllib


VERSION_PATTERN = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)")
SHA_PATTERN = re.compile(r"[0-9a-f]{40}")


def asset_snapshot(release: dict) -> list[dict]:
    return sorted(
        (
            {
                "id": asset.get("id"),
                "name": asset.get("name"),
                "digest": asset.get("digest"),
                "size": asset.get("size"),
                "state": asset.get("state"),
            }
            for asset in release.get("assets", [])
        ),
        key=lambda asset: asset["name"] or "",
    )


def validate_before(args: argparse.Namespace) -> None:
    version = args.version
    if not VERSION_PATTERN.fullmatch(version):
        raise SystemExit("Promotion version must be numeric MAJOR.MINOR.PATCH")
    source_manifest = tomllib.loads(Path(args.source_cargo).read_text(encoding="utf-8"))
    source_version = source_manifest.get("package", {}).get("version", "")
    if not VERSION_PATTERN.fullmatch(source_version) or version.split(".")[:2] != source_version.split(".")[:2]:
        raise SystemExit("Pre-release version does not match MAJOR.MINOR in its source Cargo.toml")

    release = json.loads(Path(args.release_json).read_text(encoding="utf-8"))
    tag = f"v{version}"
    if (
        release.get("tag_name") != tag
        or release.get("draft")
        or not release.get("prerelease")
    ):
        raise SystemExit("The requested version is not an available GitHub pre-release")

    directory = Path(args.assets_dir)
    info = json.loads((directory / "BUILD-INFO.json").read_text(encoding="utf-8"))
    run_number = info.get("run_number")
    source_sha = str(info.get("source_sha", "")).lower()
    base_sha = str(info.get("base_sha", "")).lower()
    if (
        info.get("version") != version
        or info.get("kind") != "ci"
        or info.get("source_ref") != "main"
        or type(run_number) is not int
        or run_number < 1
        or version.rsplit(".", 1)[1] != str(run_number)
        or source_sha != args.tag_sha.lower()
        or not SHA_PATTERN.fullmatch(source_sha)
        or not SHA_PATTERN.fullmatch(base_sha)
    ):
        raise SystemExit("BUILD-INFO.json does not match the tag, source commit or version")
    try:
        subprocess.run(["git", "merge-base", "--is-ancestor", base_sha, source_sha], check=True)
    except subprocess.CalledProcessError:
        raise SystemExit("BUILD-INFO.json baseline is not an ancestor of the release source") from None

    if info.get("github_artifact_attestations") is not True:
        raise SystemExit("Pre-release metadata does not confirm GitHub Artifact Attestations")

    changelog = (directory / "CHANGELOG.md").read_bytes()
    if hashlib.sha256(changelog).hexdigest() != info.get("changelog_sha256"):
        raise SystemExit("CHANGELOG.md does not match its hash in BUILD-INFO.json")
    if (directory / "CHANGELOG.md").read_text(encoding="utf-8") != release.get("body", ""):
        raise SystemExit("Pre-release notes differ from the packaged changelog snapshot")

    expected_manifest_names = {
        f"morse-runner_{version}_amd64.deb",
        f"morse-runner_{version}_linux_amd64.tar.gz",
        f"morse-runner_{version}_windows_x86_64.zip",
        f"morse-runner_{version}_windows_x86_64_setup.exe",
        "BUILD-INFO.json",
        "CHANGELOG.md",
    }
    manifest = (directory / "SHA256SUMS").read_text(encoding="utf-8")
    manifest_entries = []
    for line in manifest.splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  (.+)", line)
        if not match:
            raise SystemExit("SHA256SUMS has an invalid line")
        manifest_entries.append((match.group(1), match.group(2)))
    manifest_names = [name for _, name in manifest_entries]
    if set(manifest_names) != expected_manifest_names or len(manifest_names) != len(expected_manifest_names):
        raise SystemExit("SHA256SUMS does not cover exactly the expected immutable assets")
    for expected_hash, name in manifest_entries:
        file_path = directory / name
        if not file_path.is_file() or hashlib.sha256(file_path.read_bytes()).hexdigest() != expected_hash:
            raise SystemExit(f"SHA256SUMS verification failed for {name}")

    expected_assets = expected_manifest_names | {"SHA256SUMS", "SHA256SUMS.asc"}
    assets = asset_snapshot(release)
    if {asset["name"] for asset in assets} != expected_assets or len(assets) != len(expected_assets):
        raise SystemExit("Pre-release has an unexpected asset set")
    for asset in assets:
        name = asset["name"]
        local_hash = hashlib.sha256((directory / name).read_bytes()).hexdigest()
        if asset.get("state") != "uploaded" or asset.get("digest") != f"sha256:{local_hash}":
            raise SystemExit(f"GitHub asset digest mismatch for {name}")

    snapshot = {
        "release_id": release.get("id"),
        "tag_name": release.get("tag_name"),
        "name": release.get("name"),
        "body": release.get("body"),
        "assets": assets,
    }
    Path(args.snapshot).write_text(json.dumps(snapshot, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def validate_after(args: argparse.Namespace) -> None:
    before = json.loads(Path(args.snapshot).read_text(encoding="utf-8"))
    after = json.loads(Path(args.release_json).read_text(encoding="utf-8"))
    if (
        after.get("id") != before.get("release_id")
        or after.get("tag_name") != before.get("tag_name")
        or after.get("name") != before.get("name")
        or after.get("body") != before.get("body")
        or after.get("draft") is not False
        or after.get("prerelease") is not False
        or asset_snapshot(after) != before.get("assets")
    ):
        raise SystemExit("Promotion changed the release or one of its published assets")


def main() -> None:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    before = subparsers.add_parser("before")
    before.add_argument("--version", required=True)
    before.add_argument("--release-json", required=True)
    before.add_argument("--assets-dir", required=True)
    before.add_argument("--tag-sha", required=True)
    before.add_argument("--source-cargo", required=True)
    before.add_argument("--snapshot", required=True)
    after = subparsers.add_parser("after")
    after.add_argument("--release-json", required=True)
    after.add_argument("--snapshot", required=True)
    args = parser.parse_args()
    if args.command == "before":
        validate_before(args)
    else:
        validate_after(args)


if __name__ == "__main__":
    main()
