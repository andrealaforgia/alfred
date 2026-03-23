#!/bin/sh
# Sets up git hooks for the Alfred project.
# Run once after cloning: ./scripts/setup-hooks.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
HOOKS_DIR="$REPO_ROOT/.git/hooks"

ln -sf ../../scripts/pre-commit "$HOOKS_DIR/pre-commit"
echo "Git hooks installed. Pre-commit hook will run fmt, clippy, and tests."
