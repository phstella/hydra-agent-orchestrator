#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FRONTEND_DIR="${ROOT_DIR}/crates/hydra-app/frontend"
APP_MANIFEST="${ROOT_DIR}/crates/hydra-app/Cargo.toml"

MODE="release"
TAG_NAME=""

usage() {
  cat <<'EOF'
Usage: scripts/clean-build.sh [--debug|--release] [--tag <name> | <name>] [--help]

Performs a clean rebuild for hydra-app:
1) cargo clean (hydra-app crate target)
2) remove frontend build caches
3) frontend production build
4) cargo build for hydra-app

Options:
  --debug         Build with debug profile
  --release       Build with release profile (default)
  --tag <name>    Create a git tag at current HEAD after successful build
  <name>          Shorthand for --tag <name>
  --help          Show this help text
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --debug)
      MODE="debug"
      shift
      ;;
    --release)
      MODE="release"
      shift
      ;;
    --tag)
      if [[ $# -lt 2 ]]; then
        echo "error: --tag requires a value" >&2
        exit 1
      fi
      TAG_NAME="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      if [[ "${1}" == -* ]]; then
        echo "error: unknown option: $1" >&2
        usage
        exit 1
      fi
      if [[ -n "${TAG_NAME}" ]]; then
        echo "error: multiple tag values provided" >&2
        exit 1
      fi
      TAG_NAME="$1"
      shift
      ;;
  esac
done

echo "[clean-build] root: ${ROOT_DIR}"
echo "[clean-build] profile: ${MODE}"

echo "[clean-build] cargo clean (hydra-app)"
cargo clean --manifest-path "${APP_MANIFEST}"

echo "[clean-build] clearing frontend caches"
rm -rf \
  "${FRONTEND_DIR}/dist" \
  "${FRONTEND_DIR}/node_modules/.vite" \
  "${FRONTEND_DIR}/.vite"

echo "[clean-build] frontend build"
(
  cd "${FRONTEND_DIR}"
  npm run build
)

echo "[clean-build] cargo build (hydra-app)"
if [[ "${MODE}" == "release" ]]; then
  cargo build --manifest-path "${APP_MANIFEST}" --release
else
  cargo build --manifest-path "${APP_MANIFEST}"
fi

if [[ -n "${TAG_NAME}" ]]; then
  echo "[clean-build] creating git tag: ${TAG_NAME}"
  git -C "${ROOT_DIR}" tag "${TAG_NAME}"
fi

echo "[clean-build] done"
