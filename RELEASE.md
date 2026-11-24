# Publishing a release

## 1. Bump version

Update version in all three places:

- `package.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml` (`[package].version`)

## 2. Update changelog

Add a section titled `## X.Y.Z — date` to `CHANGELOG.md`. Its text becomes the release notes.

## 3. Push to main

Nothing else to do. The [Release workflow](.github/workflows/release.yml) runs on every push to `main`. When the version has no `vX.Y.Z` tag yet, it creates the tag, builds macOS (Apple Silicon + Intel), Windows (x64 + ARM64), and Linux installers, and attaches them to a new GitHub Release.

The workflow fails, without releasing, if the three version numbers differ or the changelog has no section for the version. A push with an already-tagged version does nothing. Run it by hand from the Actions tab (**Run workflow**) if a build needs repeating.

## 4. Local build (one platform)

```bash
pnpm install
pnpm tauri build
```

Artifacts land in `src-tauri/target/release/bundle/`:

| Platform | Typical files |
| --- | --- |
| macOS arm64 | `dmg/Walkmark_<version>_aarch64.dmg` |
| macOS x64 | `dmg/Walkmark_<version>_x64.dmg` |
| Windows x64 | `nsis/Walkmark_<version>_x64-setup.exe` |
| Windows ARM64 | `nsis/Walkmark_<version>_arm64-setup.exe` |
| Linux | `appimage/Walkmark_<version>_amd64.AppImage`, `deb/walkmark_<version>_amd64.deb` |

## 5. Verify downloads

Open the [latest release](https://github.com/plynko/walkmark-desktop/releases/latest) and confirm that every installer listed above is attached.

## Code signing (optional)

Installers are unsigned by default. For public distribution without Gatekeeper warnings, configure Apple Developer signing and notarization in Tauri’s bundle settings before tagging.
