[← README](../README.md)

# macOS packaging

These are contributor tools run from a source checkout. Source builds are
verified on Apple Silicon with macOS 26. Signed downloads and a supported
network Homebrew installation are not available yet.

## Build a local application

```sh
sh scripts/bundle-macos.sh release
open "target/release/Informe.app"
```

The bundle includes the release executable, application icon, schema, and an
illustrative dashboard. Use **Open…** inside the app to select another document.
It is a local development bundle, not a signed distribution build. The default
sample and schema are bundled so that moving the app does not require the checkout.
Use `debug` instead of `release` for a development build.

The icon comes from `assets/brand/AppIcon.icns`. To rebuild it after changing the
SVG, run `sh scripts/generate-icons.sh`; see [brand assets](../assets/brand/README.md).

## Build an archive

```sh
sh scripts/package-macos.sh
```

Output is `target/dist/informe-VERSION-macos-ARCH.tar.gz` with a sibling
SHA-256 checksum. The archive contains the app, examples, schema, docs, and the
matching generation skill. Skill installation is a separate user action;
packaging does not modify agent settings. CI builds an **unsigned test archive**,
not a signed end-user release.

For signing, set `SIGNING_IDENTITY` and `NOTARY_PROFILE` for credentials already
stored in Keychain. With both set, packaging calls `scripts/sign-macos.sh` and
creates the archive only after verification, notarization, and stapling succeed.
Signing requires the maintainer's Apple Developer credentials; no credentials
are distributed in the repository.

## Local Homebrew experiment

For maintainers testing a locally built archive:

```sh
sh scripts/install-local-cask.sh \
  target/dist/informe-0.1.0-macos-arm64.tar.gz
```

Use the filename produced by your build. The script generates a local cask that
installs the app and links its CLI. It is **not a supported installation path**:
the previous unsigned pilot installed, but macOS killed the installed executable
on the version check. The cask was then uninstalled. No Gatekeeper settings were
changed. Signing/notarization and clean-machine verification remain necessary.

A public tap needs published, versioned assets with verified URLs/checksums and
a successful installation rehearsal. Repository visibility alone does not make
a Homebrew command available. See [distribution status](../docs/distribution.md)
for release gates.

Before advertising a downloadable release, verify install, upgrade, uninstall,
launch, copying, reload, and accessibility on a clean Mac without Rust. Local
relocation checks and CI success do not establish clean-machine compatibility.
