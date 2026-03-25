#!/usr/bin/env bash

set -euo pipefail

usage() {
    cat <<'EOF'
Usage:
  bash tools/ci/check_commit_message.sh --message "feat(typeck): add effect inference"
  bash tools/ci/check_commit_message.sh --from-file .git/COMMIT_EDITMSG

Allowed format:
  <type>(<scope>)?: <description>
  <type>!:(breaking change)

Allowed types:
  feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert
EOF
}

message=""

case "${1:-}" in
    --message)
        message="${2:-}"
        ;;
    --from-file)
        file="${2:-}"
        if [[ -z "$file" || ! -f "$file" ]]; then
            echo "error: commit message file not found: ${file:-<empty>}" >&2
            exit 2
        fi
        message="$(head -n1 "$file")"
        ;;
    -h|--help|help)
        usage
        exit 0
        ;;
    *)
        usage >&2
        exit 2
        ;;
esac

if [[ -z "$message" ]]; then
    echo "error: commit message is empty" >&2
    exit 2
fi

# Conventional Commits with optional scope and optional breaking marker.
pattern='^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9._/-]+\))?(!)?: .{1,}$'

if [[ "$message" =~ $pattern ]]; then
    echo "[ok] Conventional Commit message accepted: $message"
    exit 0
fi

echo "[error] Invalid commit message: $message" >&2
echo "Expected Conventional Commits format (type(scope): description)." >&2
exit 1
