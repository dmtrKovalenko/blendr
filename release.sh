#!/bin/bash

VERSION="$1"

cargo install cargo-edit
cargo set-version "$VERSION"

git add Cargo.toml Cargo.lock

git commit -m "chore(release): v$VERSION"

git tag "v$VERSION"

git push origin "v$VERSION"

git push
