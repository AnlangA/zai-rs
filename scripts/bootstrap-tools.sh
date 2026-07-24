#!/usr/bin/env bash
# scripts/bootstrap-tools.sh
#
# Install the exact tool versions used by this repository's quality checks.
# Every version is pinned so repeated runs produce the same environment.
#
# Usage:
#   ./scripts/bootstrap-tools.sh              # install every tool below
#   ./scripts/bootstrap-tools.sh cargo-audit  # install just one
#   ./scripts/bootstrap-tools.sh --help
#
# Rust tools are installed with an explicitly pinned bootstrap toolchain and
# exact package version. Most still build on the repository MSRV; tools whose
# own MSRV is newer declare that independently below. gitleaks is downloaded
# as a prebuilt GitHub release binary and SHA-256 verified. On an unsupported
# OS/arch the script fails rather than guessing.
set -euo pipefail

# Rust-based tools: name -> exact version -> bootstrap toolchain.
RUST_TOOLS=(
  "cargo-audit 0.22.2 1.88.0"
  "cargo-deny 0.20.2 1.88.0"
  "cargo-llvm-cov 0.8.7 1.88.0"
  "cargo-fuzz 0.13.2 1.97.1"
  "cargo-cyclonedx 0.5.9 1.88.0"
  "cargo-nextest 0.9.114 1.88.0"
  "mdbook 0.5.4 1.88.0"
  "lychee 0.24.2 1.88.0"
  "cargo-semver-checks 0.49.0 1.97.1"
)

# gitleaks is distributed as a prebuilt binary for common Linux/macOS targets.
# Other platforms fail explicitly. A subshell-local EXIT trap keeps temporary
# archives from surviving either success or failure.
install_gitleaks() (
  local version="8.30.1"
  local os arch archive sha256 url tmpdir cargo_bin got
  if command -v gitleaks >/dev/null 2>&1 && [ "$(gitleaks version 2>/dev/null)" = "$version" ]; then
    echo "gitleaks $version already installed"
    return 0
  fi

  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Linux)
      case "$arch" in
        x86_64)
          archive="gitleaks_${version}_linux_x64.tar.gz"
          sha256="551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb"
          ;;
        aarch64 | arm64)
          archive="gitleaks_${version}_linux_arm64.tar.gz"
          sha256="e4a487ee7ccd7d3a7f7ec08657610aa3606637dab924210b3aee62570fb4b080"
          ;;
        *)
          echo "gitleaks: unsupported arch $arch on Linux" >&2
          exit 1
          ;;
      esac
      ;;
    Darwin)
      case "$arch" in
        arm64)
          archive="gitleaks_${version}_darwin_arm64.tar.gz"
          sha256="b40ab0ae55c505963e365f271a8d3846efbc170aa17f2607f13df610a9aeb6a5"
          ;;
        x86_64)
          archive="gitleaks_${version}_darwin_x64.tar.gz"
          sha256="dfe101a4db2255fc85120ac7f3d25e4342c3c20cf749f2c20a18081af1952709"
          ;;
        *)
          echo "gitleaks: unsupported arch $arch on macOS" >&2
          exit 1
          ;;
      esac
      ;;
    *)
      echo "gitleaks: unsupported OS $os" >&2
      exit 1
      ;;
  esac

  url="https://github.com/gitleaks/gitleaks/releases/download/v${version}/${archive}"
  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT
  echo "downloading $archive"
  curl --proto '=https' --tlsv1.2 --retry 3 -fsSL -o "$tmpdir/$archive" "$url"
  got="$(sha256_file "$tmpdir/$archive")"
  if [ "$got" != "$sha256" ]; then
    echo "gitleaks SHA-256 mismatch: got $got, expected $sha256" >&2
    exit 1
  fi
  tar -xzf "$tmpdir/$archive" -C "$tmpdir"
  cargo_bin="${CARGO_HOME:-$HOME/.cargo}/bin"
  mkdir -p "$cargo_bin"
  install -m 0755 "$tmpdir/gitleaks" "$cargo_bin/gitleaks"
  echo "installed gitleaks $version ($got)"
)

sha256_file() {
  local file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
  else
    echo "neither sha256sum nor shasum is available" >&2
    return 1
  fi
}

tool_version() {
  local name="$1"
  case "$name" in
    cargo-llvm-cov) cargo llvm-cov --version ;;
    cargo-fuzz) cargo fuzz --version ;;
    cargo-cyclonedx) cargo cyclonedx --version ;;
    cargo-nextest) cargo nextest --version ;;
    *) "$name" --version ;;
  esac
}

install_rust_tool() {
  local name="$1" version="$2" toolchain="$3"
  local version_pattern="${version//./\\.}"
  # Skip if the exact version is already present.
  if command -v "$name" >/dev/null 2>&1 \
    && tool_version "$name" 2>/dev/null \
      | grep -Eq "(^|[^[:digit:]])${version_pattern}([^[:digit:]]|$)"; then
    echo "$name $version already installed"
    return 0
  fi
  if ! cargo +"$toolchain" --version >/dev/null 2>&1; then
    echo "$name $version requires the Rust $toolchain toolchain for installation." >&2
    echo "Install it first with: rustup toolchain install $toolchain" >&2
    return 1
  fi
  cargo +"$toolchain" install --locked --version "=$version" "$name"
}

usage() {
  cat <<'EOF'
Usage: ./scripts/bootstrap-tools.sh [all|TOOL|--help]

Install all pinned quality tools (the default), or one named tool.
Installation requires each tool's pinned Rust toolchain; missing toolchains
produce an explicit rustup command instead of silently changing the host.

Tools: cargo-audit, cargo-deny, cargo-llvm-cov, cargo-fuzz,
       cargo-cyclonedx, cargo-nextest, cargo-semver-checks, mdbook,
       lychee, gitleaks
EOF
}

main() {
  if [ "$#" -gt 1 ]; then
    usage >&2
    return 2
  fi
  local requested="${1:-all}"
  local matched=false
  local entry name version toolchain
  if [ "$requested" = "--help" ] || [ "$requested" = "-h" ]; then
    usage
    return 0
  fi
  if [ "$requested" = "gitleaks" ]; then
    install_gitleaks
    return 0
  fi
  for entry in "${RUST_TOOLS[@]}"; do
    read -r name version toolchain <<< "$entry"
    if [ "$requested" = "all" ] || [ "$requested" = "$name" ]; then
      matched=true
      install_rust_tool "$name" "$version" "$toolchain"
    fi
  done
  if [ "$requested" = "all" ]; then
    install_gitleaks
  elif [ "$matched" = false ]; then
    echo "unknown tool: $requested" >&2
    return 2
  fi
}

main "$@"
