#!/bin/sh
# protcount installer
#
#   curl -LsSf https://github.com/Dipayan26/protcount/releases/latest/download/install.sh | sh
#
# Downloads the prebuilt binary for this machine and installs it to
# ~/.local/bin (override with PROTCOUNT_INSTALL_DIR).

set -eu

REPO="Dipayan26/protcount"
BINARY="protcount"
INSTALL_DIR="${PROTCOUNT_INSTALL_DIR:-$HOME/.local/bin}"

say() { printf '%s\n' "$*"; }
err() { printf 'error: %s\n' "$*" >&2; exit 1; }

need() {
    command -v "$1" >/dev/null 2>&1 || err "required command not found: $1"
}

need uname
need tar
need mktemp

# Prefer curl, fall back to wget.
if command -v curl >/dev/null 2>&1; then
    fetch() { curl -sSfL "$1" -o "$2"; }
    # Final URL after following redirects — used to discover the latest tag.
    resolve_url() { curl -sIL -o /dev/null -w '%{url_effective}' "$1"; }
elif command -v wget >/dev/null 2>&1; then
    fetch() { wget -qO "$2" "$1"; }
    resolve_url() {
        wget -qS --spider --max-redirect=10 "$1" 2>&1 \
            | awk '/[Ll]ocation:/ { last = $2 } END { print last }'
    }
else
    err "need either curl or wget"
fi

# --- work out the target triple for this machine ---------------------
os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
    Linux)  os_part="unknown-linux-musl" ;;
    Darwin) os_part="apple-darwin" ;;
    *)      err "unsupported OS: $os (Windows users: download the .zip from the releases page)" ;;
esac

case "$arch" in
    x86_64|amd64)  arch_part="x86_64" ;;
    aarch64|arm64) arch_part="aarch64" ;;
    *)             err "unsupported architecture: $arch" ;;
esac

TARGET="${arch_part}-${os_part}"

# --- find the latest released version --------------------------------
# PROTCOUNT_VERSION pins a specific tag, e.g. PROTCOUNT_VERSION=v0.1.0
if [ -n "${PROTCOUNT_VERSION:-}" ]; then
    VERSION="$PROTCOUNT_VERSION"
else
    say "Looking up the latest release..."
    # /releases/latest redirects to /releases/tag/<tag>, so the tag is just
    # the last path segment. This deliberately avoids api.github.com, which
    # is rate limited to 60 requests/hour per IP and fails on shared networks.
    LATEST_URL="$(resolve_url "https://github.com/${REPO}/releases/latest")"
    VERSION="${LATEST_URL##*/}"
fi

case "$VERSION" in
    v*) ;;
    *)  err "could not determine the latest release (got '${VERSION}')" ;;
esac

NUMBER="${VERSION#v}"
ASSET="${BINARY}-${NUMBER}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"

# --- download and unpack ---------------------------------------------
TMP="$(mktemp -d)"
# Clean up the temp dir however we exit.
trap 'rm -rf "$TMP"' EXIT

say "Downloading ${BINARY} ${VERSION} for ${TARGET}..."
fetch "$URL" "$TMP/$ASSET" || err "download failed: $URL"

tar xzf "$TMP/$ASSET" -C "$TMP"

mkdir -p "$INSTALL_DIR"
install -m 755 "$TMP/${BINARY}-${NUMBER}-${TARGET}/${BINARY}" "$INSTALL_DIR/${BINARY}"

say "Installed ${BINARY} ${VERSION} to ${INSTALL_DIR}/${BINARY}"

# --- remind the user about PATH if needed ----------------------------
case ":${PATH}:" in
    *":${INSTALL_DIR}:"*)
        say "Run '${BINARY} --help' to get started."
        ;;
    *)
        say ""
        say "${INSTALL_DIR} is not on your PATH. Add this to your shell profile:"
        say "    export PATH=\"${INSTALL_DIR}:\$PATH\""
        ;;
esac
