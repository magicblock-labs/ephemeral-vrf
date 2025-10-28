#!/bin/bash

set -e

# Step 1: Read the version from Cargo.toml
version=$(grep '^version = ' ../Cargo.toml | head -n 1 | sed 's/version = "\(.*\)"/\1/')

if [ -z "$version" ]; then
    echo "Version not found in Cargo.toml"
    exit 1
fi

echo "Aligning for version: $version"

# GNU/BSD compat
sedi=(-i'')
case "$(uname)" in
  # For macOS, use two parameters
  Darwin*) sedi=(-i '')
esac

# Update the version in packages/npm-package/package.json.tmpl
jq --arg version "$version" '.version = $version' packages/npm-package/package.json.tmpl > temp.json && mv temp.json packages/npm-package/package.json.tmpl

# Check if the any changes have been made to the specified files, if running with --check
if [[ "$1" == "--check" ]]; then
    files_to_check=(
        "packages/npm-package/package.json.tmpl"
    )

    for file in "${files_to_check[@]}"; do
        # Check if the file has changed from the previous commit
        if git diff --name-only | grep -q "$file"; then
            echo "Error: version not aligned for $file. Align the version, commit and try again."
            exit 1
        fi
    done
    exit 0
fi