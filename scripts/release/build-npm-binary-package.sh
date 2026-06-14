#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <rust-target> <npm-package-name>" >&2
  exit 2
fi

target="$1"
package_name="$2"
cargo_target_dir="${CARGO_TARGET_DIR:-target}"

case "$target" in
  aarch64-apple-darwin)
    package_dir="moonlight-darwin-arm64"
    binary_name="moonlight"
    ;;
  x86_64-apple-darwin)
    package_dir="moonlight-darwin-x64"
    binary_name="moonlight"
    ;;
  aarch64-unknown-linux-gnu)
    package_dir="moonlight-linux-arm64-gnu"
    binary_name="moonlight"
    ;;
  x86_64-unknown-linux-gnu)
    package_dir="moonlight-linux-x64-gnu"
    binary_name="moonlight"
    ;;
  x86_64-pc-windows-msvc)
    package_dir="moonlight-win32-x64-msvc"
    binary_name="moonlight.exe"
    ;;
  *)
    echo "unsupported Rust target: $target" >&2
    exit 2
    ;;
esac

expected_package="@moritzbrantner/${package_dir}"
if [[ "$package_name" != "$expected_package" ]]; then
  echo "package $package_name does not match target $target; expected $expected_package" >&2
  exit 2
fi

source_binary="${cargo_target_dir}/${target}/release/${binary_name}"
if [[ ! -f "$source_binary" ]]; then
  echo "release binary not found: $source_binary" >&2
  exit 1
fi

package_path="packages/npm/platforms/${package_dir}"
install -d "${package_path}/bin"
install -m 755 "$source_binary" "${package_path}/bin/${binary_name}"

echo "copied $source_binary to ${package_path}/bin/${binary_name}"
