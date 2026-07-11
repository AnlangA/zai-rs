#!/usr/bin/env bash
# scripts/bootstrap-tools.sh
#
# Install the exact, pinned tool versions the zai-rs 0.5.0 plan requires.
# Plan P00.11. Never reads "latest" — every version is fixed so a future run
# reproduces the same environment.
#
# Usage:
#   ./scripts/bootstrap-tools.sh              # install every tool below
#   ./scripts/bootstrap-tools.sh cargo-audit  # install just one
#
# Rust tools are installed with `cargo +1.88.0 install --locked --version '=x.y.z'`
# (rust-toolchain.toml pins the default toolchain to 1.88.0). gitleaks is
# downloaded as a prebuilt GitHub release binary and SHA-256 verified. On an
# unsupported OS/arch the script fails rather than guessing.
set -euo pipefail

# Rust-based tools: name -> exact version.
RUST_TOOLS=(
  "cargo-audit 0.22.2"
  "cargo-deny 0.20.2"
  "cargo-llvm-cov 0.8.7"
  "cargo-fuzz 0.13.2"
  "cargo-cyclonedx 0.5.9"
  "cargo-nextest 0.9.114"
  "mdbook 0.5.4"
  "lychee 0.24.2"
)

# gitleaks is distributed as a prebuilt binary; only these two targets are
# supported. Other platforms fail the script.
install_gitleaks() {
  local version="8.30.1"
  local os arch archive sha256 url tmpdir
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Linux)  case "$arch" in x86_64) archive="gitleaks_${version}_linux_x64.tar.gz"; sha256="551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb" ;; *) echo "gitleaks: unsupported arch $arch on Linux" >&2; exit 1 ;; esac ;;
    Darwin) case "$arch" in arm64) archive="gitleaks_${version}_darwin_arm64.tar.gz"; sha256="b40ab0ae55c505963e365f271a8d3846efbc170aa17f2607f13df610a9aeb6a5" ;; *) echo "gitleaks: unsupported arch $arch on macOS" >&2; exit 1 ;; esac ;;
    *) echo "gitleaks: unsupported OS $os" >&2; exit 1 ;;
  esac
  url="https://github.com/gitleaks/gitleaks/releases/download/v${version}/${archive}"
  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' RETURN
  echo "downloading $archive"
  curl -fsSL -o "$tmpdir/$archive" "$url"
  local got
  got="$(shasum -a 256 "$tmpdir/$archive" | cut -d' ' -f1)"
  if [ "$got" != "$sha256" ]; then
    echo "gitleaks SHA-256 mismatch: got $got, expected $sha256" >&2
    exit 1
  fi
  tar -xzf "$tmpdir/$archive" -C "$tmpdir"
  install -m 0755 "$tmpdir/gitleaks" "${CARGO_HOME:-$HOME/.cargo}/bin/gitleaks"
  echo "installed gitleaks $version ($got)"
}

install_rust_tool() {
  local name="$1" version="$2"
  # Skip if the exact version is already present.
  if command -v "$name" >/dev/null 2>&1 && "$name" --version 2>/dev/null | grep -qF "$version"; then
    echo "$name $version already installed"
    return 0
  fi
  cargo +1.88.0 install --locked --version "=$version" "$name"
}

main() {
  local requested="${1:-all}"
  if [ "$requested" = "gitleaks" ]; then
    install_gitleaks
    return 0
  fi
  for entry in "${RUST_TOOLS[@]}"; do
    set -- $entry
    if [ "$requested" = "all" ] || [ "$requested" = "$1" ]; then
      install_rust_tool "$1" "$2"
    fi
  done
  if [ "$requested" = "all" ]; then
    install_gitleaks
  fi
}

main "$@"
