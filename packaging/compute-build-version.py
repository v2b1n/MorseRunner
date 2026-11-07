#!/usr/bin/env python3
"""Compute the shared CI or official-release version for all platform jobs."""

from __future__ import annotations

import os
from pathlib import Path
import re
import sys
import tomllib


VERSION_PATTERN = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)")


def main() -> int:
    manifest = tomllib.loads(Path("Cargo.toml").read_text(encoding="utf-8"))
    base_version = manifest["package"]["version"]
    if not VERSION_PATTERN.fullmatch(base_version):
        raise SystemExit(f"Cargo.toml package.version is not numeric MAJOR.MINOR.PATCH: {base_version!r}")

    release_tag = os.environ.get("RELEASE_TAG", "").strip()
    if release_tag:
        if not release_tag.startswith("v") or not VERSION_PATTERN.fullmatch(release_tag[1:]):
            raise SystemExit(f"Invalid official release tag: {release_tag!r}")
        version = release_tag[1:]
        if version != base_version:
            raise SystemExit(
                f"Official release version {version} does not match Cargo.toml {base_version}"
            )
        kind = "release"
    else:
        run_number = os.environ.get("GITHUB_RUN_NUMBER", "")
        if not run_number.isdecimal() or int(run_number) == 0:
            raise SystemExit("GITHUB_RUN_NUMBER must be a positive decimal integer for CI builds")
        major, minor, _patch = base_version.split(".")
        version = f"{major}.{minor}.{run_number}"
        if not VERSION_PATTERN.fullmatch(version):
            raise SystemExit(f"Computed CI version is invalid: {version!r}")
        kind = "ci"

    outputs = f"version={version}\nkind={kind}\n"
    output_path = os.environ.get("GITHUB_OUTPUT")
    if output_path:
        with open(output_path, "a", encoding="utf-8") as output:
            output.write(outputs)
    else:
        sys.stdout.write(outputs)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
