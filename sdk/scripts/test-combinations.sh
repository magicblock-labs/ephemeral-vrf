#!/bin/bash

set -euo pipefail

color=true
build_count=0

if [[ "${1:-}" == "--no-color" ]]; then
    color=false
    shift
fi

if [[ $# -gt 0 ]]; then
    echo "usage: $0 [--no-color]" >&2
    exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

print_command() {
    local command="$1"
    local line="------------------------------------------------------------"

    build_count=$((build_count + 1))

    echo
    if [[ "$color" == true ]]; then
        printf '\033[32m%s\n==> [%02d] %s\n%s\033[0m\n' "$line" "$build_count" "$command" "$line"
    else
        printf '%s\n==> [%02d] %s\n%s\n' "$line" "$build_count" "$command" "$line"
    fi
    echo
}

build() {
    local features="${1:-}"

    if [[ -z "$features" ]]; then
        print_command "cargo build -p ephemeral-vrf-sdk"
        cargo build -p ephemeral-vrf-sdk
    else
        print_command "cargo build -p ephemeral-vrf-sdk --features $features"
        cargo build -p ephemeral-vrf-sdk --features "$features"
    fi
}

build_no_default() {
    local features="${1:-}"

    if [[ -z "$features" ]]; then
        print_command "cargo build -p ephemeral-vrf-sdk --no-default-features"
        cargo build -p ephemeral-vrf-sdk --no-default-features
    else
        print_command "cargo build -p ephemeral-vrf-sdk --no-default-features --features $features"
        cargo build -p ephemeral-vrf-sdk --no-default-features --features "$features"
    fi
}

build
build_no_default

build "backward-compat"
build_no_default "backward-compat"

build "anchor"
build_no_default "anchor"

build "anchor-compat"
build_no_default "anchor-compat"

# Unsupported by design: `anchor` targets current Anchor/Solana and must not be
# combined with `backward-compat`. Use `anchor-compat` for older Anchor support.
# build "anchor,backward-compat"
# build "anchor-modern,anchor-compat"
