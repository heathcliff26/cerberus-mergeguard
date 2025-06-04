#!/bin/bash

set -e

base_dir="$(dirname "${BASH_SOURCE[0]}" | xargs realpath | xargs dirname)"

bin_dir="${base_dir}/dist"
name="$(yq -r '.package.name' "${base_dir}/Cargo.toml")"

[ -d "${bin_dir}" ] || mkdir -p "${bin_dir}"

CI_COMMIT_SHA="$(git rev-parse HEAD)"
export CI_COMMIT_SHA

cargo build --release

case "$(uname -m)" in
    x86_64 | amd64)
        arch="amd64"
        ;;
    aarch64 | arm64)
        arch="arm64"
        ;;
    *)
        arch="$(uname -m)"
        ;;
esac
mv "${base_dir}/target/release/${name}" "${bin_dir}/${name}-${arch}"
