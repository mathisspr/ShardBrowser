#!/usr/bin/env bash
set -euo pipefail

git config user.name "yt-sync"
git config user.email "yt-sync@users.noreply.github.com"
git remote add upstream "${UPSTREAM_URL:-https://github.com/ProxyShard/ShardBrowser.git}"
git fetch upstream main
if git merge-base --is-ancestor upstream/main HEAD; then
  echo "Upstream unchanged."
  echo "changed=no" >> "$GITHUB_OUTPUT"
  exit 0
fi

merge_status=0
git merge --no-commit --no-ff upstream/main || merge_status=$?
# Resolve workflow conflicts before checking the remaining conflicts. The fork
# owns its workflows; GITHUB_TOKEN cannot push upstream workflow changes.
git restore --source=HEAD --staged --worktree -- .github/workflows
conflicts=$(git diff --name-only --diff-filter=U)
if [ -n "$conflicts" ]; then
  {
    echo '## Upstream merge needs manual resolution'
    printf '\nUpstream: `%s`\n\n' "$(git rev-parse upstream/main)"
    printf '%s\n' "$conflicts" | while IFS= read -r path; do printf -- '- `%s`\n' "$path"; done
    echo
    echo 'No changes were pushed. Resolve these conflicts on the youtube branch.'
  } >> "$GITHUB_STEP_SUMMARY"
  git merge --abort
  echo '::error::Upstream merge conflicts; see the run summary for affected files.'
  exit 1
fi
# A non-conflict merge failure must never be committed or reported as success.
if [ "$merge_status" -ne 0 ] && ! git rev-parse -q --verify MERGE_HEAD >/dev/null; then
  echo '::error::Upstream merge failed before a merge commit could be prepared.'
  exit 1
fi
git commit --no-edit
git push origin youtube
echo "changed=yes" >> "$GITHUB_OUTPUT"
