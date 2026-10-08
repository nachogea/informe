[← Documentation](README.md)

# Distribution status

The first public offering is an **experimental source release** under the MIT
license. A public source repository and a dependable downloadable Mac app have
separate acceptance criteria.

## Available now

- Source builds on the verified Apple Silicon / macOS 26 target.
- CLI launch from any directory, validation, schema/capability discovery, and
  handoff to one running viewer with document tabs.
- A local `.app` bundler, unsigned archive builder, signing/notarization script,
  and local Homebrew cask experiment.
- A portable generation skill, installed separately from the runtime.

See the [README](../README.md) for the supported source installation and
[packaging guide](../packaging/README.md) for development archives. There is no
supported network Homebrew installation or signed downloadable release yet.

## Public source release

- [x] MIT license and contributor instructions.
- [x] Clear source-build quick start and generation workflow.
- [x] Dashboard, report, and plan examples with illustrative data.
- [x] Documented format limits, experimental review status, and measurement boundaries.
- [x] CI checks and unsigned packaging verification.
- [x] Original editable logo/icon assets and bundled macOS application icon.
- [x] Finalize the public project name and apply it consistently.
- [x] Refresh the README screenshot after the viewer and identity pass.
- [x] Review tracked files and Git history for material intended to stay private.
- [x] Use a GitHub noreply author email for the public history.
- [x] Publish the clean snapshot from a fresh GitHub repository.

The docs and packaging scripts do not change repository visibility. Public
source release does not imply that unsigned CI archives are supported installers.
The pre-publication review scanned tracked text for credential patterns and
personal paths and inspected screenshots. The public repository starts with one
commit, excluding earlier screenshots and private commit-author metadata. Earlier
development history is preserved in a separate private archive repository.
Historical benchmark fixtures remain as sample data. Commit IDs in the older QA
and benchmark records refer to development history before the public snapshot;
they are retained as historical provenance and do not resolve in the public history.

## Signed macOS release

1. Fix the release name, bundle ID, CLI name, icon, and version. Preserve a migration
   path for existing commands, recent paths, and review data if the identity changes.
2. Run tests, Clippy, example/schema validation, and the release build in CI.
3. Package the app, schema, examples, and matching skill resources.
4. Sign with Developer ID, notarize with Apple, and staple the ticket using the
   [packaging scripts](../packaging/README.md).
5. Publish an immutable versioned archive and SHA-256 checksum.
6. Verify installation, upgrade, uninstall, launch, copy, reload, and accessibility
   on a clean Apple Silicon Mac without a Rust toolchain.
7. Create an owned Homebrew tap with a cask that installs the app and CLI. Verify
   public download URLs and the cask on a clean machine before advertising a command.

Declare the minimum macOS version that has actually been verified. Intel and
other operating systems need separate builds and compatibility checks.

The [Cask Cookbook](https://docs.brew.sh/Cask-Cookbook) and
[Apple notarization guidance](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
are references for release maintainers. Homebrew can handle upgrades initially;
an in-app updater is outside this release scope.

## Agent integration

Homebrew should install the runtime and its resources. Skill installation remains
an explicit separate step; packaging must not silently edit agent configuration.
The [agent workflow](agent-workflow.md) describes the implemented file-based loop.
A plugin package can follow once the local skill and installation experience are
stable. A model API or MCP server inside the viewer is not required for this loop.
