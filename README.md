# RuntimeScaner

RuntimeScaner is a Linux CLI for auditing shared libraries needed by dynamically
linked applications. It combines static ELF dependency inspection, dynamic loader
runtime observations, server inventory data, and explicit ignore policy to help
prepare offline deployment bundles.

## Quick Start

Prerequisites:

- RuntimeScaner binary available in `PATH`
- Linux tools used by commands: `readelf` and `ldconfig`

Create a required-library report with:

```sh
runtimescaner required --exec /bin/true --timeout 1s
```

## Development

Useful commands:

```sh
vorbere run check
vorbere run test
vorbere run build
vorbere run linux-amd64-archive
vorbere run linux-arm64-archive
```

The validation commands above match the local checks expected before opening a
pull request.

## Documentation

- [User guides](docs/user-guides/README.md): day-to-day CLI workflows.
- [Specification references](docs/specifications/README.md): implemented
  behavior, output schemas, and validation coverage.
