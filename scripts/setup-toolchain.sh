#!/usr/bin/env bash
# Installs the tools needed to build Walkmark into `.toolchain/` inside this
# folder, so nothing depends on system-wide installs or on temp directories
# that a cleaner may empty. Safe to run again: it only adds what is missing.
#
#   scripts/setup-toolchain.sh              Rust, pnpm, project dependencies, Chromium for the UI tests
#   scripts/setup-toolchain.sh --windows    plus what a Windows build from macOS needs
#
# Then, in each shell:  source scripts/dev-env.sh
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tc="$root/.toolchain"
export CARGO_HOME="$tc/cargo" RUSTUP_HOME="$tc/rustup"
mkdir -p "$tc"

with_windows=0
for arg in "$@"; do
  case "$arg" in
    --windows) with_windows=1 ;;
    *) echo "Unknown option: $arg" >&2; exit 2 ;;
  esac
done

case "$(uname -m)" in
  arm64 | aarch64) rust_host="aarch64-apple-darwin" ;;
  x86_64) rust_host="x86_64-apple-darwin" ;;
  *) echo "Unsupported CPU: $(uname -m)" >&2; exit 1 ;;
esac

# --- Rust --------------------------------------------------------------------
if [ ! -x "$CARGO_HOME/bin/cargo" ]; then
  echo "==> Installing Rust into $tc"
  curl -sSf -o "$tc/rustup-init" "https://static.rust-lang.org/rustup/dist/$rust_host/rustup-init"
  chmod +x "$tc/rustup-init"
  "$tc/rustup-init" -y --no-modify-path --profile minimal \
    -c clippy -c rustfmt \
    -t aarch64-pc-windows-msvc -t x86_64-pc-windows-msvc
  rm -f "$tc/rustup-init"
else
  echo "==> Rust already installed"
fi

# --- pnpm (the version package.json asks for) ----------------------------------
pnpm_version="$(node -p "require('$root/package.json').packageManager.split('@')[1]")"
if [ ! -x "$tc/pnpm/node_modules/.bin/pnpm" ]; then
  echo "==> Installing pnpm $pnpm_version"
  npm install --prefix "$tc/pnpm" --no-audit --no-fund "pnpm@$pnpm_version"
else
  echo "==> pnpm already installed"
fi

# --- Project dependencies and the browser the UI tests use ---------------------
# shellcheck disable=SC1091
source "$root/scripts/dev-env.sh"
echo "==> Installing project dependencies"
# node_modules is rebuilt from the lockfile when the store moves; no prompt needed.
(cd "$root" && pnpm install --frozen-lockfile --config.confirmModulesPurge=false)
echo "==> Installing Chromium for the UI tests"
(cd "$root" && pnpm exec playwright install chromium)

# --- Windows builds from macOS ---------------------------------------------------
# llvm   provides llvm-rc (Windows resources) and clang-cl (C code in dependencies);
#        there is no standalone macOS download, so it comes from Homebrew.
# nsis   (makensis) builds the Windows installer, the only format Tauri can
#        produce when cross-compiling.
# cargo-xwin goes into .toolchain/ like the rest.
if [ "$with_windows" = 1 ]; then
  if ! command -v brew > /dev/null; then
    echo "Homebrew is needed for llvm and makensis: https://brew.sh" >&2
    exit 1
  fi
  for formula in llvm makensis; do
    if ! brew list --versions "$formula" > /dev/null 2>&1; then
      echo "==> brew install $formula"
      HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_ENV_HINTS=1 brew install "$formula"
    else
      echo "==> $formula already installed"
    fi
  done
  source "$root/scripts/dev-env.sh"
  if ! command -v cargo-xwin > /dev/null; then
    echo "==> Installing cargo-xwin"
    cargo install cargo-xwin --locked
  else
    echo "==> cargo-xwin already installed"
  fi
  cat <<'NOTE'

Windows build, from the repository root, after `source scripts/dev-env.sh`:

  cargo xwin check --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc
  pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis

The first `cargo xwin` run downloads Microsoft's Windows SDK and C runtime, and
by running it you accept Microsoft's licence for them. This script does not do
that for you.
NOTE
fi
echo "Done. Next: source scripts/dev-env.sh"
