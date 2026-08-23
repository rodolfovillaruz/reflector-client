# reflector-client

Fetches an instance IP from a reflector service and SSHes in.

## Install

Via [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) (not published to crates.io, so use `--git`):

```sh
cargo binstall --git https://github.com/rodolfovillaruz/reflector-client reflector-client
```

Or build from source:

```sh
cargo install --git https://github.com/rodolfovillaruz/reflector-client
```

## Config

Requires `~/.config/reflector.json`:

```json
{
  "url": "https://your-reflector-host/endpoint",
  "auth_token": "..."
}
```
