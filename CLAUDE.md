# Agent instructions

## GitHub Actions versions

Keep every `uses:` in `.github/workflows/` on a major version whose action
runtime is Node 24 or newer. GitHub emits a "Node.js 20 is deprecated"
annotation on any run that invokes an action still targeting the Node 20
runtime, and those annotations should stay empty.

When adding or bumping an action:

- Pin to a major tag (e.g. `actions/checkout@v5`), not a floating branch or a
  full SHA.
- Before committing, confirm the chosen major runs on Node 24. Check the
  action's `action.yml` (`runs.using: node24`) or its latest release notes.
- If a run produces a Node 20 deprecation annotation, bump the offending
  action to the next major that ships a Node 24 runtime and verify the
  workflow still passes.

Current known-good majors: `actions/checkout@v5`, `actions/upload-artifact@v7`,
`actions/download-artifact@v8`, `softprops/action-gh-release@v3`,
`taiki-e/install-action@v2`, `Swatinem/rust-cache@v2`,
`dtolnay/rust-toolchain@stable`.

## Releases

`Cargo.toml` `version` must match the pushed `vX.Y.Z` tag — the release
workflow's `verify-version` job fails otherwise. Bump the version, update the
pinned example in `README.md`, tag, and push.
