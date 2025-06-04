#!/usr/bin/env bash

set -euo pipefail

crate="rustls-platform-verifier-android"
jq_filter=".packages[] | select(.name == \"$crate\") | .manifest_path"

metadata="$(cargo metadata --format-version 1 --filter-platform aarch64-linux-android --features rustls-platform-verifier)"

manifest_path="$(echo "$metadata" | jq -r "$jq_filter")"
echo "Manifest path: $manifest_path"

maven_dir="$(dirname "$manifest_path")/maven"
echo "$maven_dir"

filename="rustls-platform-verifier-android.tar.gz"
pushd "$maven_dir"
ls -l
tar czvf "$filename" *
popd

mkdir -p artifacts
mv "$maven_dir/$filename" artifacts/

ls -l
