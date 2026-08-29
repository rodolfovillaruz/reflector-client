# reflector-client

Fetches an instance IP from a reflector service and SSHes in.

## Install

### Prebuilt binary (recommended)

Using [cargo-binstall](https://github.com/cargo-bins/cargo-binstall). This crate is not
published to crates.io, so point binstall at the git repo — it reads the version from
`Cargo.toml` on the default branch and downloads the matching binary from the GitHub
release, with no Rust toolchain build:

```sh
cargo install cargo-binstall          # if you don't have it
cargo binstall --git https://github.com/rodolfovillaruz/reflector-client reflector-client
```

Prebuilt binaries are published for:

| Target                        | Platform                     |
| ----------------------------- | ---------------------------- |
| `x86_64-unknown-linux-gnu`    | Linux (x86-64)               |
| `x86_64-pc-windows-msvc`      | Windows (x86-64)             |
| `aarch64-linux-android`       | Android / Termux (arm64)     |
| `x86_64-linux-android`        | Android (x86-64)             |

To pin a specific release, add `--version <x.y.z>`, e.g.:

```sh
cargo binstall --git https://github.com/rodolfovillaruz/reflector-client --version 0.1.6 reflector-client
```

### From source

```sh
cargo install --git https://github.com/rodolfovillaruz/reflector-client
```

## Config

Requires `~/.config/reflector.json` (`%USERPROFILE%\.config\reflector.json` on Windows):

```json
{
  "url": "https://your-reflector-host/endpoint",
  "auth_token": "..."
}
```

## SSH host key checking

The reflector hands back a different IP over time, so SSH's normal host key
verification would fail on every address change. `reflector-client` invokes `ssh`
with `-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null`, so host keys
are neither verified nor recorded in `~/.ssh/known_hosts`.
