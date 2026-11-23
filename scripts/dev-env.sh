# Source this file (do not run it) to build Walkmark with the tools that
# `scripts/setup-toolchain.sh` installed into `.toolchain/`:
#
#   source scripts/dev-env.sh
#
# Tight on disk? Also export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 to
# keep debug builds small.
_walkmark_root="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")/.." && pwd)"
_walkmark_tc="$_walkmark_root/.toolchain"

export CARGO_HOME="$_walkmark_tc/cargo"
export RUSTUP_HOME="$_walkmark_tc/rustup"
export PATH="$CARGO_HOME/bin:$_walkmark_tc/pnpm/node_modules/.bin:$PATH"
# Keep pnpm's store and Playwright's Chromium here too, out of ~/Library/Caches.
export npm_config_store_dir="$_walkmark_tc/pnpm-store"
export PLAYWRIGHT_BROWSERS_PATH="$_walkmark_tc/browsers"
# Homebrew's LLVM (llvm-rc, clang-cl), only present after `setup-toolchain.sh --windows`.
# It is keg-only, so it is not on the PATH unless added here.
[ -d /opt/homebrew/opt/llvm/bin ] && export PATH="/opt/homebrew/opt/llvm/bin:$PATH"

unset _walkmark_root _walkmark_tc
