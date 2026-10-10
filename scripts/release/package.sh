#!/usr/bin/env bash
set -euo pipefail

# Produce a real binary archive only after the workspace can build its lidash binary.
: "${LIDB_VERSION:?Set LIDB_VERSION to a SemVer release version}"
: "${LIDB_ARCH:?Set LIDB_ARCH to x86_64 or aarch64}"
[[ "${LIDB_VERSION}" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] || { echo "Invalid release version" >&2; exit 2; }
[[ "${LIDB_ARCH}" == x86_64 || "${LIDB_ARCH}" == aarch64 ]] || { echo "Unsupported architecture" >&2; exit 2; }
[[ -x target/release/lidash ]] || { echo "Release blocked: target/release/lidash is missing" >&2; exit 2; }

OUT="dist"
PACKAGE="lidb-v${LIDB_VERSION}-linux-${LIDB_ARCH}"
mkdir -p "$OUT"
STAGING_DIR=$(mktemp -d target/package.XXXXXX)
trap 'rm -rf -- "$STAGING_DIR"' EXIT
mkdir -p "${STAGING_DIR}/${PACKAGE}/bin"
cp target/release/lidash "${STAGING_DIR}/${PACKAGE}/bin/"
cp LICENSE README.md "${STAGING_DIR}/${PACKAGE}/"
tar -C "$STAGING_DIR" -czf "${OUT}/${PACKAGE}.tar.gz" "${PACKAGE}"
(cd "$OUT" && sha256sum "${PACKAGE}.tar.gz" > "${PACKAGE}.tar.gz.sha256")
echo "Packaged ${OUT}/${PACKAGE}.tar.gz"
