#!/usr/bin/env bash
# Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
#
# Installs actionlint and shellcheck (pinned versions, SHA-256 verified) into the job's
# temp folder and puts them on PATH for the next steps. Works on Linux and on Windows
# (Git Bash) runners, without docker. actionlint lints the shell in `run:` steps only
# when shellcheck is on PATH, so both always come together.
set -euo pipefail

ACTIONLINT=1.7.12
SHELLCHECK=0.11.0
dir="${RUNNER_TEMP:?}/bedrock-linters"
mkdir -p "$dir"

fetch() { # url file sha256
  curl -fsSL -o "$dir/$2" "$1"
  echo "$3  $dir/$2" | sha256sum -c - >/dev/null
}

if [ "${RUNNER_OS:-Linux}" = "Windows" ]; then
  fetch "https://github.com/rhysd/actionlint/releases/download/v$ACTIONLINT/actionlint_${ACTIONLINT}_windows_amd64.zip" \
    actionlint.zip 6e7241b51e6817ea6a047693d8e6fed13b31819c9a0dd6c5a726e1592d22f6e9
  unzip -oq "$dir/actionlint.zip" actionlint.exe -d "$dir"
  fetch "https://github.com/koalaman/shellcheck/releases/download/v$SHELLCHECK/shellcheck-v$SHELLCHECK.zip" \
    shellcheck.zip 8a4e35ab0b331c85d73567b12f2a444df187f483e5079ceffa6bda1faa2e740e
  unzip -oq "$dir/shellcheck.zip" shellcheck.exe -d "$dir"
else
  fetch "https://github.com/rhysd/actionlint/releases/download/v$ACTIONLINT/actionlint_${ACTIONLINT}_linux_amd64.tar.gz" \
    actionlint.tar.gz 8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8
  tar -xzf "$dir/actionlint.tar.gz" -C "$dir" actionlint
  fetch "https://github.com/koalaman/shellcheck/releases/download/v$SHELLCHECK/shellcheck-v$SHELLCHECK.linux.x86_64.tar.gz" \
    shellcheck.tar.gz b7af85e41cc99489dcc21d66c6d5f3685138f06d34651e6d34b42ec6d54fe6f6
  tar -xzf "$dir/shellcheck.tar.gz" -C "$dir" --strip-components=1 "shellcheck-v$SHELLCHECK/shellcheck"
fi

if [ -n "${GITHUB_PATH:-}" ]; then echo "$dir" >> "$GITHUB_PATH"; fi
"$dir/actionlint" -version | head -1
"$dir/shellcheck" --version | sed -n 2p
