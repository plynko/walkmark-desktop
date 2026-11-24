# Build from source

Compile Walkmark locally for development, custom builds, or platforms without a prebuilt release.

## Requirements

| Tool | Version |
| --- | --- |
| **Rust** | 1.82+ |
| **Node.js** | 18+ |
| **pnpm** | Latest (via `corepack enable`) |
| **Tauri prerequisites** | [v2.tauri.app/start/prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS |

**Platforms:** macOS 11+, Windows 10+ (x64 or ARM64), Linux with working screen capture (X11 or Wayland portal).

HTTPS to the AI providers uses the operating system's own TLS (Schannel on Windows, Security.framework on macOS, OpenSSL on Linux), so no C code is compiled for it. On Linux install the OpenSSL headers (`libssl-dev` on Debian and Ubuntu) next to the other Tauri prerequisites.

## Self-contained toolchain (optional)

If you would rather not install Rust and pnpm system-wide, `scripts/setup-toolchain.sh` installs them, the project dependencies and the Chromium used by the UI tests into `.toolchain/` inside the repository (ignored by git). It only adds what is missing, so it is safe to run again.

```bash
scripts/setup-toolchain.sh        # once
source scripts/dev-env.sh         # in each shell; puts cargo, pnpm and the rest on PATH
```

### Building for Windows from macOS

```bash
scripts/setup-toolchain.sh --windows   # adds llvm and makensis (Homebrew) and cargo-xwin
source scripts/dev-env.sh
cargo xwin check --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc
pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis
```

The first `cargo xwin` run downloads Microsoft's Windows SDK and C runtime; by running it you accept Microsoft's licence for them. Only the NSIS installer can be produced when cross-compiling. Use `aarch64-pc-windows-msvc` for ARM64. The Rust code has been checked to compile for both targets; running the result on Windows has not been tested.

## Clone and install

```bash
git clone https://github.com/plynko/walkmark-desktop.git
cd walkmark
pnpm install
```

## Development

```bash
pnpm tauri dev
```

Hot-reloads the Svelte frontend; Rust changes rebuild the backend. Use **New guide** and a short recording to verify capture on your machine.

### Useful scripts

```bash
pnpm typecheck          # Svelte/TS check
pnpm build              # Frontend production build only
```

## Release build

```bash
pnpm tauri build
```

Artifacts appear under `src-tauri/target/release/bundle/` (native target) or `src-tauri/target/<triple>/release/bundle/` when cross-compiling.

### Cross-compilation examples

```bash
# macOS Apple Silicon
pnpm tauri build --target aarch64-apple-darwin

# macOS Intel
pnpm tauri build --target x86_64-apple-darwin

# Windows x64
pnpm tauri build --target x86_64-pc-windows-msvc

# Windows ARM64 (NSIS only — no MSI on this target)
pnpm tauri build --target aarch64-pc-windows-msvc --bundles nsis
```

### Output artifacts

| OS | Bundles |
| --- | --- |
| macOS | `.app`, `.dmg` |
| Windows x64 / ARM64 | NSIS `.exe` installer |
| Linux | `.deb`, `.AppImage` |

Release filenames follow `Walkmark_<version>_<arch>.<ext>` — see `.github/workflows/release.yml` for CI matrix.

## Tests

```bash
pnpm typecheck
pnpm test:ui    # first run: pnpm exec playwright install chromium
cargo test --manifest-path src-tauri/Cargo.toml
```

| Test | Purpose |
| --- | --- |
| `tests/wire.rs` | Provider HTTP against a local stub server |
| `tests/live_mistral.rs` | Optional live Mistral test — needs `MISTRAL_API_KEY` |
| Unit tests in `storage`, `catalog`, export | Persistence and formatting |

## Project layout

```
src/                      Svelte 5 UI (runes, TypeScript, Tailwind)
  lib/api.ts              Tauri invoke wrappers
  lib/store.svelte.ts     App state, events, dialogs
  lib/components/         Library, editor, settings, export
src-tauri/src/
  commands.rs             IPC command handlers
  capture/                Recording session, input, detection
  ai/                     Prompts, generation, catalog
  export/                 Markdown, HTML, Word, PDF
  storage.rs              Projects, settings, credentials
docs/                     User and developer documentation
```

Deep dive: [Architecture](architecture.md).

## Release process (maintainers)

Documented in [RELEASE.md](https://github.com/plynko/walkmark-desktop/blob/main/RELEASE.md). Summary:

1. Bump version in `src-tauri/tauri.conf.json`, `package.json` and `src-tauri/Cargo.toml`.
2. Add the version's section to [CHANGELOG.md](https://github.com/plynko/walkmark-desktop/blob/main/CHANGELOG.md).
3. Push to `main` — GitHub Actions tags the version, builds the platform artifacts and publishes the release.

## Contributing

Bug reports and pull requests welcome on [GitHub](https://github.com/plynko/walkmark-desktop). See [CONTRIBUTING.md](https://github.com/plynko/walkmark-desktop/blob/main/CONTRIBUTING.md).

Licence: MIT.
