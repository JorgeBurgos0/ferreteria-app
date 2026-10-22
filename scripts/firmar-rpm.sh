#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 || ! $1 =~ ^([[:xdigit:]]{40}|[[:xdigit:]]{64})$ ]]; then
  printf 'Uso: %s HUELLA_GPG_COMPLETA\n' "$0" >&2
  exit 2
fi

for program in cmp gpg rpmsign rpmkeys sha256sum; do
  if ! command -v "$program" >/dev/null 2>&1; then
    printf 'Falta el comando %s. En Fedora instala rpm-sign con: sudo dnf install rpm-sign\n' "$program" >&2
    exit 1
  fi
done

fingerprint=${1^^}
if ! gpg --list-secret-keys --with-colons "$fingerprint" | grep -q '^sec:'; then
  printf 'No se encontró una clave privada para %s en tu llavero GPG.\n' "$fingerprint" >&2
  exit 1
fi

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
rpm_name=ferreteria-app-0.1.0-5.x86_64.rpm
source_rpm="$repo_dir/artifacts/$rpm_name"
output_dir="$repo_dir/artifacts/signed"
signed_rpm="$output_dir/$rpm_name"
public_key="$output_dir/RPM-GPG-KEY-ferreteria-app.asc"
checksum="$output_dir/$rpm_name.sha256"

if [[ ! -f $source_rpm ]]; then
  printf 'No existe el RPM original: %s\n' "$source_rpm" >&2
  exit 1
fi
if [[ -e $signed_rpm || -e $checksum ]]; then
  printf 'Ya hay archivos de firma en %s; revísalos antes de volver a firmar.\n' "$output_dir" >&2
  exit 1
fi

mkdir -p -- "$output_dir"
work_dir=$(mktemp -d "$output_dir/.firmar-XXXXXX")
verify_db=$(mktemp -d /tmp/ferreteria-rpmdb.XXXXXX)
trap 'rm -rf -- "$work_dir" "$verify_db"' EXIT

rpmkeys --checksig "$source_rpm"
cp -- "$source_rpm" "$work_dir/$rpm_name"
rpmsign --addsign --key-id "$fingerprint" "$work_dir/$rpm_name"
gpg --armor --export "$fingerprint" > "$work_dir/RPM-GPG-KEY-ferreteria-app.asc"
if [[ ! -s $work_dir/RPM-GPG-KEY-ferreteria-app.asc ]]; then
  printf 'No se pudo exportar la clave pública.\n' >&2
  exit 1
fi
if [[ -e $public_key ]] && ! cmp -s -- "$work_dir/RPM-GPG-KEY-ferreteria-app.asc" "$public_key"; then
  printf 'La clave pública existente no coincide con la clave solicitada.\n' >&2
  exit 1
fi

# Verificar sin importar la clave al sistema ni necesitar privilegios de administrador.
rpmkeys --dbpath "$verify_db" --import "$work_dir/RPM-GPG-KEY-ferreteria-app.asc"
LC_ALL=C rpmkeys --dbpath "$verify_db" --checksig --verbose "$work_dir/$rpm_name" | tee "$work_dir/verificacion.txt"
if ! grep -Eiq 'signature.*: OK' "$work_dir/verificacion.txt"; then
  printf 'No apareció una firma válida en la comprobación del RPM.\n' >&2
  exit 1
fi

mv -- "$work_dir/$rpm_name" "$signed_rpm"
if [[ ! -e $public_key ]]; then
  mv -- "$work_dir/RPM-GPG-KEY-ferreteria-app.asc" "$public_key"
fi
(cd -- "$output_dir" && sha256sum "$rpm_name" > "$rpm_name.sha256")
printf '\nRPM firmado: %s\nClave pública: %s\nSHA-256: %s\nHuella: %s\n' \
  "$signed_rpm" "$public_key" "$checksum" "$fingerprint"
