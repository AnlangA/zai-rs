#!/usr/bin/env bash
# Verify the actual crates.io archive without freezing an exact file snapshot.
set -euo pipefail

MAX_FILES=400
MAX_ARCHIVE_BYTES=$((2 * 1024 * 1024))
MAX_UNCOMPRESSED_TAR_BYTES=$((8 * 1024 * 1024))

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

if [[ "$#" -gt 1 ]]; then
  echo "usage: $0 [path/to/zai-rs-VERSION.crate]" >&2
  exit 2
fi

version="$(cargo pkgid -p zai-rs --locked | sed 's/.*[#@]//')"
if [[ -z "${version}" ]]; then
  echo "ERROR: could not resolve the zai-rs package version" >&2
  exit 1
fi

if [[ "$#" -eq 0 ]]; then
  cargo package -p zai-rs --allow-dirty --locked --no-verify
  archive="target/package/zai-rs-${version}.crate"
else
  archive="$1"
fi
if [[ ! -f "${archive}" ]]; then
  echo "ERROR: package archive was not created: ${archive}" >&2
  exit 1
fi

archive_bytes="$(wc -c <"${archive}")"
tar_bytes="$(gzip -cd -- "${archive}" | wc -c)"
if ((archive_bytes > MAX_ARCHIVE_BYTES)); then
  echo "ERROR: package archive is ${archive_bytes} bytes (policy maximum ${MAX_ARCHIVE_BYTES})" >&2
  exit 1
fi
if ((tar_bytes > MAX_UNCOMPRESSED_TAR_BYTES)); then
  echo "ERROR: uncompressed package tar is ${tar_bytes} bytes (policy maximum ${MAX_UNCOMPRESSED_TAR_BYTES})" >&2
  exit 1
fi

list_file="$(mktemp)"
trap 'rm -f "${list_file}"' EXIT
tar -tzf "${archive}" >"${list_file}"

# Cargo packages regular files only. Reject links, devices, or unexpected
# directory entries so an allowlisted pathname cannot hide a different type.
if ! tar -tvzf "${archive}" | awk 'substr($1, 1, 1) != "-" { exit 1 }'; then
  echo "ERROR: package archive contains a non-regular tar entry" >&2
  exit 1
fi

prefix="zai-rs-${version}/"
required=(
  Cargo.lock
  Cargo.toml
  Cargo.toml.orig
  LICENSE
  README.md
  SECURITY.md
  spec/contracts/operations.json
  src/lib.rs
)

for path in "${required[@]}"; do
  if ! grep -Fqx -- "${prefix}${path}" "${list_file}"; then
    echo "ERROR: required package entry is missing: ${path}" >&2
    exit 1
  fi
done

file_count=0
while IFS= read -r entry; do
  if [[ "${entry}" != "${prefix}"* ]]; then
    echo "ERROR: archive entry is outside ${prefix}: ${entry}" >&2
    exit 1
  fi
  path="${entry#"${prefix}"}"
  if [[ -z "${path}" ]]; then
    echo "ERROR: package archive contains an empty relative path" >&2
    exit 1
  fi

  case "${path}" in
    /* | ../* | */../* | */.. | *\\*)
      echo "ERROR: unsafe package path: ${path}" >&2
      exit 1
      ;;
  esac

  top_level="${path%%/*}"
  case "${top_level}" in
    .cargo_vcs_info.json | Cargo.lock | Cargo.toml | Cargo.toml.orig | LICENSE | README.md | README.en.md | SECURITY.md | benches | docs | examples | spec | src | tests)
      ;;
    *)
      echo "ERROR: package entry is outside the approved top-level surface: ${path}" >&2
      exit 1
      ;;
  esac

  case "${path}" in
    .github/* | data/* | fuzz/* | scripts/* | spec/upstream/* | target/*)
      echo "ERROR: repository-only content entered the package: ${path}" >&2
      exit 1
      ;;
  esac

  case "${path}" in
    spec/contracts/*)
      ;;
    spec/*)
      echo "ERROR: only frozen public contracts may be packaged from spec/: ${path}" >&2
      exit 1
      ;;
  esac

  file_count=$((file_count + 1))
done <"${list_file}"

if ((file_count > MAX_FILES)); then
  echo "ERROR: package contains ${file_count} files (policy maximum ${MAX_FILES})" >&2
  exit 1
fi

echo "Package content policy passed: ${file_count} files, ${archive_bytes} compressed bytes, ${tar_bytes} tar bytes"
