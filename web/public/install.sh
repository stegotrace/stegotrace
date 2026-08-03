#!/bin/sh
set -eu

version="${STEGOTRACE_VERSION:-latest}"
install_dir="${STEGOTRACE_INSTALL_DIR:-${HOME}/.local/bin}"
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
mkdir -p "$install_dir"
install -m 0755 "${temporary}/stegotrace" "${install_dir}/stegotrace"

printf '%s\n' "StegoTrace instalado en ${install_dir}/stegotrace"
case ":${PATH}:" in
  *":${install_dir}:"*) ;;
  *) printf '%s\n' "Añade ${install_dir} a PATH para ejecutar: stegotrace doctor" ;;
esac
