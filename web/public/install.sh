#!/bin/sh
set -eu

version="${STEGOTRACE_VERSION:-latest}"
install_dir="${STEGOTRACE_INSTALL_DIR:-/usr/local/bin}"
distribution="https://stegotrace.guillermozubikarai.dev/cli"

if [ "$version" = "latest" ]; then
  version="v0.2.0"
fi

if [ "$(uname -s)" != "Darwin" ]; then
  printf '%s\n' "StegoTrace publica binarios para macOS arm64/x86_64." >&2
  exit 1
fi

if [ "$(sysctl -n hw.optional.arm64 2>/dev/null || printf '0')" = "1" ]; then
  target="aarch64-apple-darwin"
elif [ "$(uname -m)" = "x86_64" ]; then
  target="x86_64-apple-darwin"
else
  printf '%s\n' "Arquitectura de Mac no compatible." >&2
  exit 1
fi

base="${distribution}/${version}"
archive="stegotrace-${target}.tar.gz"
temporary="$(mktemp -d "${TMPDIR:-/tmp}/stegotrace.XXXXXX")"
trap 'rm -rf "$temporary"' EXIT INT TERM

curl --proto '=https' --tlsv1.2 --fail --location --silent --show-error "${base}/${archive}" --output "${temporary}/${archive}"
curl --proto '=https' --tlsv1.2 --fail --location --silent --show-error "${base}/${archive}.sha256" --output "${temporary}/${archive}.sha256"
(cd "$temporary" && shasum -a 256 -c "${archive}.sha256")
tar -xzf "${temporary}/${archive}" -C "$temporary"
if mkdir -p "$install_dir" 2>/dev/null && [ -w "$install_dir" ]; then
  install -m 0755 "${temporary}/stegotrace" "${install_dir}/stegotrace"
else
  printf '%s\n' "macOS pedirá tu contraseña para instalar StegoTrace en ${install_dir}."
  sudo mkdir -p "$install_dir"
  sudo install -m 0755 "${temporary}/stegotrace" "${install_dir}/stegotrace"
fi

"${install_dir}/stegotrace" --json doctor >/dev/null

printf '%s\n' "StegoTrace instalado y verificado en ${install_dir}/stegotrace"
case ":${PATH}:" in
  *":${install_dir}:"*) printf '%s\n' "Ejecuta: stegotrace doctor" ;;
  *) printf '%s\n' "Ejecuta: ${install_dir}/stegotrace doctor" ;;
esac
