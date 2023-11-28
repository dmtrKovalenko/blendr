#!/bin/bash

VERSION="$1"

if ! git diff --quiet; then
  echo "Error: There are unstaged changes in the repository."
  exit 1
fi

git checkout main
git pull

cargo install cargo-edit
cargo set-version "$VERSION"

git add Cargo.toml Cargo.lock

git commit -m "chore(release): v$VERSION"

git tag "v$VERSION"

git push origin "v$VERSION"

git push
