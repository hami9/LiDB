#!/usr/bin/env bash
set -euo pipefail

# Produce a real binary archive only after the workspace can build its lidash binary.
: "${LIDB_VERSION:?Set LIDB_VERSION to a SemVer release version}"
: "${LIDB_ARCH:?Set LIDB_ARCH to x86_64 or aarch64}"
[[ "${LIDB_ARCH}" == x86_64 || "${LIDB_ARCH}" == aarch64 ]] || { echo "Unsupported architecture" >&2; exit 2; }
[[ -x target/release/lidash ]] || { echo "Release blocked: target/release/lidash is missing" >&2; exit 2; }

OUT="dist"
PACKAGE="lidb-v${LIDB_VERSION}-linux-${LIDB_ARCH}"
mkdir -p "$OUT" "target/package/${PACKAGE}/bin"
cp target/release/lidash "target/package/${PACKAGE}/bin/"
for optional_binary in lidashd lidash-helper; do
  if [[ -x "target/release/${optional_binary}" ]]; then
    cp "target/release/${optional_binary}" "target/package/${PACKAGE}/bin/"
  fi
done
cp LICENSE README.md "target/package/${PACKAGE}/"
tar -C target/package -czf "${OUT}/${PACKAGE}.tar.gz" "${PACKAGE}"
(cd "$OUT" && sha256sum "${PACKAGE}.tar.gz" > "${PACKAGE}.tar.gz.sha256")
echo "Packaged ${OUT}/${PACKAGE}.tar.gz"
