#!/usr/bin/env python3
"""Check a newly assembled draft before and after publishing it as a pre-release."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re


VERSION_PATTERN = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)")


def snapshot(release: dict) -> dict:
    assets = sorted(
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
    return {
        "release_id": release.get("id"),
        "tag_name": release.get("tag_name"),
        "name": release.get("name"),
        "body": release.get("body"),
        "assets": assets,
    }


def validate_draft(args: argparse.Namespace) -> None:
    version = args.version
    if not VERSION_PATTERN.fullmatch(version):
        raise SystemExit("Build version must be numeric MAJOR.MINOR.PATCH")
    directory = Path(args.assets_dir)
    release = json.loads(Path(args.release_json).read_text(encoding="utf-8"))
    if release.get("tag_name") != f"v{version}" or not release.get("draft") or release.get("prerelease"):
        raise SystemExit("A new draft with the expected tag is required before publication")

    info = json.loads((directory / "BUILD-INFO.json").read_text(encoding="utf-8"))
    if (
        info.get("version") != version
        or info.get("source_sha") != args.source_sha
        or info.get("base_sha") != args.base_sha
        or info.get("kind") != "ci"
        or info.get("source_ref") != "main"
        or type(info.get("run_number")) is not int
        or info.get("run_number") != int(version.rsplit(".", 1)[1])
        or info.get("github_artifact_attestations") is not True
    ):
        raise SystemExit("Build metadata does not match this main workflow run")

    changelog = (directory / "CHANGELOG.md").read_bytes()
    if hashlib.sha256(changelog).hexdigest() != info.get("changelog_sha256"):
        raise SystemExit("Changelog snapshot hash does not match BUILD-INFO.json")
    if changelog.decode("utf-8") != release.get("body", ""):
        raise SystemExit("Release notes differ from the packaged changelog snapshot")

    manifest = (directory / "SHA256SUMS").read_text(encoding="utf-8")
    entries = []
    for line in manifest.splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  (.+)", line)
        if not match:
            raise SystemExit("SHA256SUMS has an invalid line")
        entries.append((match.group(1), match.group(2)))
    expected_names = {
        f"morse-runner_{version}_amd64.deb",
        f"morse-runner_{version}_linux_amd64.tar.gz",
        f"morse-runner_{version}_windows_x86_64.zip",
        f"morse-runner_{version}_windows_x86_64_setup.exe",
        "BUILD-INFO.json",
        "CHANGELOG.md",
    }
    if {name for _, name in entries} != expected_names or len(entries) != len(expected_names):
        raise SystemExit("SHA256SUMS does not cover exactly the packages and build metadata")
    for expected_hash, name in entries:
        if hashlib.sha256((directory / name).read_bytes()).hexdigest() != expected_hash:
            raise SystemExit(f"SHA256SUMS verification failed for {name}")

    expected_assets = expected_names | {"SHA256SUMS"}
    current = snapshot(release)
    assets = current["assets"]
    if {asset["name"] for asset in assets} != expected_assets or len(assets) != len(expected_assets):
        raise SystemExit("Draft release has an unexpected asset set")
    for asset in assets:
        local_hash = hashlib.sha256((directory / asset["name"]).read_bytes()).hexdigest()
        if asset.get("state") != "uploaded" or asset.get("digest") != f"sha256:{local_hash}":
            raise SystemExit(f"GitHub asset digest mismatch for {asset['name']}")

    Path(args.snapshot).write_text(json.dumps(current, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def validate_published(args: argparse.Namespace) -> None:
    before = json.loads(Path(args.snapshot).read_text(encoding="utf-8"))
    after_release = json.loads(Path(args.release_json).read_text(encoding="utf-8"))
    after = snapshot(after_release)
    if (
        after_release.get("draft") is not False
        or after_release.get("prerelease") is not True
        or after != before
    ):
        raise SystemExit("Published pre-release differs from the verified draft")


def main() -> None:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    draft = subparsers.add_parser("draft")
    draft.add_argument("--version", required=True)
    draft.add_argument("--source-sha", required=True)
    draft.add_argument("--base-sha", required=True)
    draft.add_argument("--release-json", required=True)
    draft.add_argument("--assets-dir", required=True)
    draft.add_argument("--snapshot", required=True)
    published = subparsers.add_parser("published")
    published.add_argument("--release-json", required=True)
    published.add_argument("--snapshot", required=True)
    args = parser.parse_args()
    if args.command == "draft":
        validate_draft(args)
    else:
        validate_published(args)


if __name__ == "__main__":
    main()
