#!/usr/bin/env bash
# Checks the build.rs rerun triggers on a throwaway repository holding the tracked
# files of the working tree, uncommitted edits included:
# a repeat build with no changes is fresh, with loose and with packed refs, and
# commits, tags and src/ edits reach `cave --version` without `cargo clean`.
set -euo pipefail

repo=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export CARGO_TARGET_DIR="$work/target"

mkdir "$work/tree"
git -C "$repo" ls-files -z | tar -C "$repo" --null -T - -c | tar -x -C "$work/tree"
cd "$work/tree"
g() { git -c user.name=check -c user.email=check@localhost "$@"; }
g init -q
g add -A
g commit -qm base
g tag -a v0.0.1 -m check

fail=0
build() { cargo build --offline -q; }

expect_fresh() {
    build
    local out
    out=$(cargo build --offline -v 2>&1)
    if grep -q '^ *Fresh cave ' <<<"$out"; then
        echo "ok    $1: repeat build is fresh"
    else
        echo "FAIL  $1: $(grep -m1 'Dirty cave' <<<"$out" | sed 's/.*): //')"
        fail=1
    fi
}

expect_version() {
    build
    local v
    v=$("$CARGO_TARGET_DIR/debug/cave" --version)
    if [[ $v =~ $2 ]]; then
        echo "ok    $1: $v"
    else
        echo "FAIL  $1: got '$v', want /$2/"
        fail=1
    fi
}

expect_fresh "loose refs, no packed-refs"
expect_version "tagged" 'v0\.0\.1-0-g[0-9a-f]+\)$'
echo "// edit" >>src/cli.rs
expect_version "src/ edit" 'v0\.0\.1-0-g[0-9a-f]+-dirty\)$'
g checkout -q src/cli.rs
expect_version "src/ restored" 'v0\.0\.1-0-g[0-9a-f]+\)$'
g commit -q --allow-empty -m one
expect_version "commit" 'v0\.0\.1-1-g[0-9a-f]+\)$'
g pack-refs --all
expect_fresh "packed refs"
g commit -q --allow-empty -m two
expect_version "commit over packed refs" 'v0\.0\.1-2-g[0-9a-f]+\)$'
g tag -a v0.0.2 -m check
expect_version "new tag" 'v0\.0\.2-0-g[0-9a-f]+\)$'
expect_fresh "after tag"

exit "$fail"
