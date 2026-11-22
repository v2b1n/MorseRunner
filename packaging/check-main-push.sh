#!/usr/bin/env bash
set -euo pipefail

before=$(jq -er '.before' "$GITHUB_EVENT_PATH")
if [[ "$before" =~ ^0+$ ]]; then
  before=$(git mktree </dev/null)
fi

# Count both paths of a rename, so a move across the docs boundary builds.
if git diff --quiet --no-renames "$before" "$GITHUB_SHA" -- . \
  ':(exclude,glob)**/*.md' ':(exclude,top)docs/**'; then
  # The push from a merge is the only automatic trigger for main releases.
  # Retry to allow the commit's pull request association to become visible.
  for delay in 0 2 5 10; do
    sleep "$delay"
    merged=$(gh api --paginate "repos/$GITHUB_REPOSITORY/commits/$GITHUB_SHA/pulls" |
      jq -rs --arg sha "$GITHUB_SHA" '
        any(.[][]; .base.ref == "main" and .merged_at != null and .merge_commit_sha == $sha)
      ')
    if [[ "$merged" == true ]]; then
      exit 0
    fi
  done

  echo "Skip the build for this documentation-only push."
  echo "skip=true" >> "$GITHUB_OUTPUT"
else
  status=$?
  if [[ "$status" != 1 ]]; then
    exit "$status"
  fi
fi
