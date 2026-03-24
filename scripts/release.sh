#!/bin/bash

set -e

VERSION=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')
echo "Releasing version: $VERSION"

# Run tests
echo "Running tests..."
cargo test

# Check if tag exists
if git rev-parse "v$VERSION" >/dev/null 2>&1; then
    echo "Tag v$VERSION already exists"
    exit 1
fi

# Create tag
git tag "v$VERSION"
git push origin "v$VERSION"

echo "Release triggered! Check GitHub Actions for progress."
