#!/usr/bin/env python3
"""Create the immutable changelog and source metadata for one CI run."""

from __future__ import annotations

import hashlib
import html
import json
import os
from pathlib import Path
import re
import subprocess
import urllib.error
import urllib.request


VERSION_PATTERN = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)")
SHA_PATTERN = re.compile(r"[0-9a-f]{40}")


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True).strip()


def latest_stable_tag() -> str | None:
    repository = os.environ["GITHUB_REPOSITORY"]
    request = urllib.request.Request(
        f"https://api.github.com/repos/{repository}/releases/latest",
        headers={
            "Authorization": f"Bearer {os.environ['GH_TOKEN']}",
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            release = json.load(response)
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return None
        raise SystemExit(f"GitHub latest-release lookup failed with HTTP {error.code}") from None
    except Exception as error:
        raise SystemExit(f"GitHub latest-release lookup failed: {type(error).__name__}") from None

    if release.get("draft") or release.get("prerelease"):
        raise SystemExit("GitHub's latest release endpoint returned a non-stable release")
    tag = release.get("tag_name", "")
    if not re.fullmatch(r"v" + VERSION_PATTERN.pattern, tag):
        raise SystemExit(f"Latest stable release has an invalid version tag: {tag!r}")
    return tag


def main() -> None:
    version = os.environ.get("MORSE_RUNNER_BUILD_VERSION", "")
    run_number = os.environ.get("GITHUB_RUN_NUMBER", "")
    source_sha = os.environ.get("GITHUB_SHA", "").lower()
    event_name = os.environ.get("GITHUB_EVENT_NAME", "")
    if not VERSION_PATTERN.fullmatch(version):
        raise SystemExit("MORSE_RUNNER_BUILD_VERSION must be MAJOR.MINOR.PATCH")
    if not run_number.isdecimal() or int(run_number) < 1:
        raise SystemExit("GITHUB_RUN_NUMBER must be a positive integer")
    if not SHA_PATTERN.fullmatch(source_sha):
        raise SystemExit("GITHUB_SHA must identify the source commit")
    if version.rsplit(".", 1)[1] != run_number:
        raise SystemExit("Build version patch must equal GITHUB_RUN_NUMBER")

    if event_name == "pull_request":
        base_sha = os.environ.get("PULL_REQUEST_BASE_SHA", "").lower()
        base_tag = ""
    elif event_name == "push" and os.environ.get("GITHUB_REF") == "refs/heads/main":
        base_tag = latest_stable_tag() or ""
        if not base_tag:
            manifest = Path("Cargo.toml").read_text(encoding="utf-8")
            base_version_match = re.search(r'^version = "([^\"]+)"$', manifest, re.M)
            if not base_version_match:
                raise SystemExit("Cargo.toml does not define package.version")
            base_tag = f"v{base_version_match.group(1)}"
        if not re.fullmatch(r"v" + VERSION_PATTERN.pattern, base_tag):
            raise SystemExit(f"Baseline release tag is not a numeric version: {base_tag!r}")
        subprocess.run(
            ["git", "fetch", "--no-tags", "origin", f"refs/tags/{base_tag}:refs/tags/{base_tag}"],
            check=True,
        )
        base_sha = git("rev-parse", f"{base_tag}^{{commit}}").lower()
    else:
        raise SystemExit("Build metadata is supported only for pull requests and pushes to main")

    if not SHA_PATTERN.fullmatch(base_sha):
        raise SystemExit("The build must have a valid base commit")
    if git("rev-parse", "HEAD").lower() != source_sha:
        raise SystemExit("Checked-out source does not match GITHUB_SHA")
    subprocess.run(["git", "merge-base", "--is-ancestor", base_sha, source_sha], check=True)

    subjects = subprocess.check_output(
        ["git", "log", "--reverse", "--no-merges", "--format=%s", f"{base_sha}..{source_sha}"],
        text=True,
    ).splitlines()
    changes: list[str] = []
    seen: set[str] = set()
    for subject in subjects:
        subject = " ".join(subject.split())
        if not subject:
            continue
        subject = html.escape(subject, quote=False).replace("`", "'")
        subject = subject.replace("[", "\\[").replace("]", "\\]")
        if subject not in seen:
            changes.append(subject)
            seen.add(subject)
    if not changes:
        raise SystemExit("No non-merge commit subjects were found since the baseline")

    build_date = git("show", "-s", "--format=%cs", source_sha)
    if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", build_date):
        raise SystemExit("Could not determine the source commit date")
    changelog = (
        "# Changelog\n\n"
        f"## [{version}] - {build_date}\n\n"
        "### Changes\n\n"
        + "".join(f"- {change}\n" for change in changes)
        + "\n### Build verification\n\n"
        "- GitHub Artifact Attestations cover all Linux and Windows packages.\n"
    )
    changelog_bytes = changelog.encode("utf-8")
    changelog_hash = hashlib.sha256(changelog_bytes).hexdigest()
    build_info = {
        "version": version,
        "kind": "ci",
        "run_number": int(run_number),
        "workflow": os.environ.get("GITHUB_WORKFLOW", ""),
        "source_ref": "main" if event_name == "push" else "pull_request",
        "source_sha": source_sha,
        "base_tag": base_tag or None,
        "base_sha": base_sha,
        "build_date": build_date,
        "changelog_sha256": changelog_hash,
        "github_artifact_attestations": True,
    }

    output = Path("dist/metadata")
    output.mkdir(parents=True, exist_ok=True)
    (output / "CHANGELOG.md").write_bytes(changelog_bytes)
    (output / "BUILD-INFO.json").write_text(
        json.dumps(build_info, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    output_file = os.environ.get("GITHUB_OUTPUT")
    if output_file:
        with open(output_file, "a", encoding="utf-8") as stream:
            stream.write(f"version={version}\nsource_sha={source_sha}\nbase_sha={base_sha}\n")


if __name__ == "__main__":
    main()
