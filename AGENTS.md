# Repository Guidelines

## Project Structure & Module Organization

Walkmark is a Tauri 2 desktop app with a Svelte 5/TypeScript frontend and Rust backend.
- `src/lib/components/` contains feature components; `ui/` holds shared primitives.
- `src/lib/api.ts` wraps Tauri commands; `store.svelte.ts` manages application state.
- `src-tauri/src/` contains IPC handlers (`commands.rs`), capture, AI providers, export, and storage modules.
- Rust integration tests live in `src-tauri/tests/`; unit tests sit alongside implementation code.
- `src-tauri/assets/` and `src-tauri/icons/` hold bundled assets. `docs/` contains documentation and screenshots; `scripts/` contains development utilities.

Read `docs/architecture.md` before changing capture, AI, or export behavior.

## Build, Test, and Development Commands

Use pnpm 11.5.0 from `package.json`, Node.js 22 (matching CI), Rust, and your platform’s Tauri prerequisites.

- `pnpm install --frozen-lockfile`: install frontend dependencies from the lockfile.
- `pnpm app`: launch the desktop app with frontend hot reload and Rust rebuilds.
- `pnpm dev`: run the Vite frontend server only.
- `pnpm typecheck`: check Svelte and TypeScript.
- `pnpm build`: build the frontend.
- `pnpm app:build`: build native application bundles.
- `pnpm test:rust`: run Rust unit and integration tests.
- `cargo fmt --manifest-path src-tauri/Cargo.toml`: format Rust.
- `cargo clippy --manifest-path src-tauri/Cargo.toml`: lint Rust.

## Coding Style & Naming Conventions

Match surrounding code: two-space frontend indentation, double-quoted TypeScript strings, and semicolons; use rustfmt’s four-space Rust indentation. Use PascalCase for feature components, camelCase for TypeScript functions, and snake_case for Rust functions/modules. Follow existing lowercase naming in `components/ui/`.

Use Svelte 5 runes and Tailwind utilities. Keep changes focused. No frontend formatter or lint script is configured.

## Testing Guidelines

Use Rust’s built-in test framework with descriptive snake_case test names. Add regression coverage for parsing, export formatting, and storage; no numerical coverage threshold is configured. `wire.rs` tests provider HTTP against local stubs. Live Mistral tests require `RUN_LIVE_TESTS=1`, local credentials, and a recorded frame; leave them opt-in.

Run typechecking and Rust tests before submitting. Manually verify affected recording, writing, and export flows.

## Commit & Pull Request Guidelines

Recent commits use short imperative subjects, such as “Run CI on the same pnpm version as local dev.” Follow that style.

Branch from `main`; discuss large features in an issue first. Describe what changed and why, record manual testing and platform, and ensure CI passes. Update `docs/` when behavior changes; those documents also feed the marketing site.
