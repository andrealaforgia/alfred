#!/usr/bin/env bash
# Acceptance test for step 01-01: Cargo workspace scaffolding
# Verifies:
#   1. Workspace root Cargo.toml exists and defines 5 crates
#   2. Each crate has Cargo.toml and src/ with entry point
#   3. alfred-core depends on ropey (and nothing else external)
#   4. alfred-tui depends on alfred-core, crossterm, ratatui
#   5. alfred-bin depends on all workspace crates
#   6. alfred-core does NOT depend on alfred-tui, alfred-lisp, or alfred-plugin
#   7. cargo build --workspace succeeds

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FAIL=0

fail() {
    echo "FAIL: $1"
    FAIL=1
}

pass() {
    echo "PASS: $1"
}

# 1. Workspace root Cargo.toml
if [ -f "$PROJECT_ROOT/Cargo.toml" ]; then
    pass "Root Cargo.toml exists"
    for crate in alfred-core alfred-lisp alfred-plugin alfred-tui alfred-bin; do
        if grep -q "crates/$crate" "$PROJECT_ROOT/Cargo.toml"; then
            pass "Workspace member: $crate"
        else
            fail "Workspace missing member: $crate"
        fi
    done
else
    fail "Root Cargo.toml does not exist"
fi

# 2. Each crate has Cargo.toml and source entry point
for crate in alfred-core alfred-lisp alfred-plugin alfred-tui; do
    if [ -f "$PROJECT_ROOT/crates/$crate/Cargo.toml" ] && [ -f "$PROJECT_ROOT/crates/$crate/src/lib.rs" ]; then
        pass "$crate has Cargo.toml and src/lib.rs"
    else
        fail "$crate missing Cargo.toml or src/lib.rs"
    fi
done

if [ -f "$PROJECT_ROOT/crates/alfred-bin/Cargo.toml" ] && [ -f "$PROJECT_ROOT/crates/alfred-bin/src/main.rs" ]; then
    pass "alfred-bin has Cargo.toml and src/main.rs"
else
    fail "alfred-bin missing Cargo.toml or src/main.rs"
fi

# 3. alfred-core depends on ropey
if [ -f "$PROJECT_ROOT/crates/alfred-core/Cargo.toml" ]; then
    if grep -q 'ropey' "$PROJECT_ROOT/crates/alfred-core/Cargo.toml"; then
        pass "alfred-core depends on ropey"
    else
        fail "alfred-core does not depend on ropey"
    fi
fi

# 4. alfred-tui depends on alfred-core, crossterm, ratatui
if [ -f "$PROJECT_ROOT/crates/alfred-tui/Cargo.toml" ]; then
    for dep in alfred-core crossterm ratatui; do
        if grep -q "$dep" "$PROJECT_ROOT/crates/alfred-tui/Cargo.toml"; then
            pass "alfred-tui depends on $dep"
        else
            fail "alfred-tui does not depend on $dep"
        fi
    done
fi

# 5. alfred-bin depends on all workspace crates
if [ -f "$PROJECT_ROOT/crates/alfred-bin/Cargo.toml" ]; then
    for dep in alfred-core alfred-lisp alfred-plugin alfred-tui; do
        if grep -q "$dep" "$PROJECT_ROOT/crates/alfred-bin/Cargo.toml"; then
            pass "alfred-bin depends on $dep"
        else
            fail "alfred-bin does not depend on $dep"
        fi
    done
fi

# 6. alfred-core does NOT depend on alfred-tui, alfred-lisp, or alfred-plugin
if [ -f "$PROJECT_ROOT/crates/alfred-core/Cargo.toml" ]; then
    for forbidden in alfred-tui alfred-lisp alfred-plugin; do
        if grep -q "$forbidden" "$PROJECT_ROOT/crates/alfred-core/Cargo.toml"; then
            fail "alfred-core has forbidden dependency on $forbidden"
        else
            pass "alfred-core does not depend on $forbidden"
        fi
    done
fi

# 7. cargo build --workspace succeeds
echo ""
echo "--- Building workspace ---"
if (cd "$PROJECT_ROOT" && cargo build --workspace 2>&1); then
    pass "cargo build --workspace succeeds"
else
    fail "cargo build --workspace failed"
fi

echo ""
if [ "$FAIL" -eq 0 ]; then
    echo "ALL ACCEPTANCE TESTS PASSED"
    exit 0
else
    echo "SOME ACCEPTANCE TESTS FAILED"
    exit 1
fi
