<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" alt="Walkmark logo" width="128" />
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="MIT License" /></a>
  <a href="https://github.com/plynko/walkmark-desktop/releases/latest"><img src="https://img.shields.io/github/v/release/plynko/walkmark-desktop" alt="Latest release" /></a>
</p>

# Walkmark

> **This is a fork** of [e-vicius/walkmark-desktop](https://github.com/e-vicius/walkmark-desktop), maintained at [plynko/walkmark-desktop](https://github.com/plynko/walkmark-desktop). It has its own releases, and the downloads below come from this fork, not from the original project. See [what is different](#what-is-different-in-this-fork).

Turn a workflow you do once into step-by-step documentation someone else can follow.

Walkmark is a native desktop app. You pick a screen or window, do the task, and it captures each meaningful step with a screenshot. When you are ready, an AI model writes the instructions. Export as Markdown, a single HTML file, Word, or PDF.

<p align="center">
  <img src="docs/screenshots/readme-editor.png" alt="Walkmark document editor with AI-written steps and export" width="900" />
</p>

**Download:** [Latest release of this fork](https://github.com/plynko/walkmark-desktop/releases/latest)  
**Original project:** [e-vicius/walkmark-desktop](https://github.com/e-vicius/walkmark-desktop) · [walkmark.app](https://walkmark.app) · [Docs](https://walkmark.app/docs)

## Download

Every file is in the [latest release](https://github.com/plynko/walkmark-desktop/releases/latest). The version number is part of the file name.

| Platform | File |
| --- | --- |
| **macOS** (Apple Silicon) | `Walkmark_<version>_aarch64.dmg` |
| **macOS** (Intel) | `Walkmark_<version>_x64.dmg` |
| **Windows** (x64) | `Walkmark_<version>_x64-setup.exe` |
| **Windows** (ARM64) | `Walkmark_<version>_arm64-setup.exe` |
| **Linux** | `Walkmark_<version>_amd64.AppImage` or `Walkmark_<version>_amd64.deb` |

See [all releases](https://github.com/plynko/walkmark-desktop/releases) for older versions.

Requires macOS 11+, Windows 10+, or a recent Linux desktop with screen capture.

**macOS note:** the apps are not signed or notarized. The first time, open Walkmark with right-click, then **Open**.

## What is different in this fork

The original project is at version 0.1.2. This fork is at 0.2 and adds:

- **Word export** (.docx) with real headings and lists, annotated screenshots, and a details line under the title.
- **Image annotations:** a Tint rectangle and an Arrow, both recolourable, plus cropping and a click cursor mark. All are embedded in exported images.
- **Export from the library:** a download button on each document card, and a details view.
- **Export wording per language:** the fixed words in exports (headings, "Step", "Before you start") follow the guide's language and can be edited in Settings.
- **Stop writing** interrupts requests that are already running.
- **Operating system TLS** instead of a bundled crypto library, so the Rust code also compiles for Windows from macOS.
- **Automatic releases:** pushing a new version to `main` builds the installers and publishes the release. See [RELEASE.md](RELEASE.md).

The full list is in the [changelog](CHANGELOG.md).

## Open source vs Cloud

Walkmark Cloud is offered by the original project, not by this fork. Everything here works without it.

| | **Open source (this repo)** | **Walkmark Cloud** (original project) |
| --- | --- | --- |
| Status | Available now on [GitHub](https://github.com/plynko/walkmark-desktop) | [Waitlist](https://walkmark.app/#cloud), not open yet |
| Price | Free, MIT licence, no subscription | Pricing at launch |
| AI writing | Ollama or LM Studio on your machine, or bring your own API keys | Hosted models in the same desktop app, no API keys |
| Your data | Recording, editing, and export stay on your computer | Sharable guide links when Cloud launches |
| Account | None required | Sign in when Cloud launches |

The open source app is fully usable without Cloud: projects and screenshots stay on disk, recording and export work offline, and your keys never pass through any server of ours when you write with a local model or your own provider.

## How it works

1. **Record**: Walkmark samples the screen, detects meaningful changes, and saves a frame per step. You can mark moments by hand.
2. **Review**: Reorder, merge, blur sensitive areas, annotate, swap frames, edit titles.
3. **Write**: Your chosen model turns screenshots into instructions. Steps you edited yourself are left alone. [How AI writing works →](docs/ai-writing.md)
4. **Export**: Markdown (+ images folder), self-contained HTML, editable Word, or print-ready PDF.

## Privacy

Recording, editing, and export stay on your machine. AI writing is opt-in only: local models keep writing on your hardware, or you connect your own API keys. [Full privacy guide →](docs/privacy.md)

## Documentation

Guides are in [`docs/`](docs/README.md).

**Using Walkmark:** [Getting started](docs/getting-started.md) · [Recording](docs/recording.md) · [Editing](docs/editing.md) · [Settings](docs/settings.md) · [AI writing](docs/ai-writing.md) · [Local models](docs/local-models.md) · [Export](docs/export.md) · [Shortcuts](docs/keyboard-shortcuts.md) · [Troubleshooting](docs/troubleshooting.md) · [Privacy](docs/privacy.md)

**Developers:** [Build from source](docs/build-from-source.md) · [Architecture](docs/architecture.md) · [Contributing](CONTRIBUTING.md) · [Releasing](RELEASE.md)

## Build from source

```bash
git clone https://github.com/plynko/walkmark-desktop.git
cd walkmark-desktop
pnpm install
pnpm tauri dev      # development
pnpm tauri build    # installers in src-tauri/target/release/bundle/
```

See [docs/build-from-source.md](docs/build-from-source.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

## Licence

MIT. See [LICENSE](LICENSE), which is unchanged from the original project.
