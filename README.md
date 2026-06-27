# RuntimeScaner

RuntimeScaner is a Linux CLI for auditing shared libraries needed by dynamically
linked applications. It combines static ELF dependency inspection, dynamic loader
runtime observations, server inventory data, and explicit ignore policy to help
prepare offline deployment bundles.

## Run Locally

Prerequisites:

- Rust stable toolchain
- `vorbere`
- Linux tools used by commands: `readelf` and `ldconfig`

Create a required-library report with:

```sh
cargo run -- required --exec /bin/true --timeout 1s
```

## Development

Useful commands:

```sh
vorbere run check
vorbere run test
vorbere run build
```

The validation commands above match the local checks expected before opening a
pull request.

## Documentation

- [User guides](docs/user-guides/README.md): day-to-day CLI workflows.
- [Specification references](docs/specifications/README.md): implemented
  behavior, output schemas, and validation coverage.
