#!/usr/bin/env bash
# shellcheck shell=bash
# Installation script for Linux
set -eu

err() {
  echo "$1" >&2
  exit 1
}

has_command() {
  if ! command -v $1; then
    err "Required command not found: $1"
  fi
}

usage() {
  echo "
Usage: ${0##*/} [options] [BUNDLE]
Install MonyaCode on Linux from a tar bundle.

Options:
  -h, --help          Display this help and exit.
  --build             Build the tar bundle before installation.
  --build-remote      Build the `remote_server` binary
  --prefix PREFIX     Install into PREFIX (default ~/.local).
  "
}

MONYACODE_BUILD_TARBALL=false
MONYACODE_BUILD_REMOTE=false
MONYACODE_INSTALL_PREFIX="$HOME/.local"
MONYACODE_BUNDLE_FILE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            exit 0
            ;;
        --build)
            MONYACODE_BUILD_TARBALL=true
            shift
            ;;
        --build-remote)
            MONYACODE_BUILD_REMOTE=true
            shift
            ;;
        --prefix)
            shift
            [[ $# -lt 1 ]] && err "Expected PREFIX"
            MONYACODE_INSTALL_PREFIX="$1"
            shift
            ;;
        --)
            shift
            break
            ;;
        -*)
            echo "Unknown option: $1" >&2
            help_info
            exit 1
            ;;
        *)
            if [[ $# -gt 1 ]]; then
              err "Too many arguments, expected [BUNDLE]"
            fi
            if [[ $# -eq 1 ]]; then
              MONYACODE_BUNDLE_FILE="$1"
              shift
            fi
            ;;
    esac
done

version=$(
  curl -s -X GET -H 'accept: application/json' https://api.github.com/repos/monyacode/monyacode/releases/latest \
    | grep -o '"tag_name":"[^"]\+"' \
    | sed -e 's,"tag_name":"\([^"]\+\)",\1,g'
)

host_line="$(rustc --version --verbose | grep "host")"
target_triple=${host_line#*: }
arch="$(echo $target_triple | awk -F - '{print $1}')"

target_dir="${CARGO_TARGET_DIR:-target}"

if [[ "$MONYACODE_BUILD_TARBALL" = "true" ]]; then
  no_build_flag=""
  if [ "$MONYACODE_BUILD_REMOTE" = false ]; then
    no_build_flag="--no-build-remote"
  fi

  ./script/bundle-linux --tarball $no_build_flag
  MONYACODE_BUNDLE_FILE="${target_dir}/release/monyacode-linux-$arch.tar.gz"
elif [ "$MONYACODE_BUNDLE_FILE" = "" ]; then
  MONYACODE_BUNDLE_FILE="monyacode-linux-$arch-$version.tar.gz"
  curl --skip-existing -L -O https://github.com/monyacode/monyacode/releases/download/"$version"/"$MONYACODE_BUNDLE_FILE"
fi
[[ ! -f "$MONYACODE_BUNDLE_FILE" ]] && err "$MONYACODE_BUNDLE_FILE not found, exiting..."

channel=stable
if tar ztf "$MONYACODE_BUNDLE_FILE" | head -1 | grep -q "dev"; then
  channel=dev
fi
appid="app.liten.MonyaCode"
suffix=""
if [ "$channel" != "stable" ]; then
  suffix="-$channel"
  appid="app.liten.MonyaCode-Dev"
fi
mkdir -p "$MONYACODE_INSTALL_PREFIX/monyacode$suffix.app"
mkdir -p "$MONYACODE_INSTALL_PREFIX/bin" "$MONYACODE_INSTALL_PREFIX/share/applications"
tar -xzf "$MONYACODE_BUNDLE_FILE" -C "$MONYACODE_INSTALL_PREFIX/"

ln -sf "$MONYACODE_INSTALL_PREFIX/monyacode$suffix.app/bin/monyacode" "$HOME/.local/bin/monyacode"

desktop_file_path="$MONYACODE_INSTALL_PREFIX/share/applications/${appid}.desktop"
src_dir="$MONYACODE_INSTALL_PREFIX/monyacode$suffix.app/share/applications"
cp "$src_dir/monyacode${suffix}.desktop" "${desktop_file_path}"

sed -i -e "s|Icon=monyacode|Icon=$MONYACODE_INSTALL_PREFIX/monyacode$suffix.app/share/icons/hicolor/512x512/apps/monyacode.png|g" "${desktop_file_path}"
sed -i -e "s|Exec=monyacode|Exec=$MONYACODE_INSTALL_PREFIX/monyacode$suffix.app/bin/monyacode|g" "${desktop_file_path}"

echo "Installation to $MONYACODE_INSTALL_PREFIX complete."

