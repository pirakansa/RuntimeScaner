# RuntimeScaner

RuntimeScaner is a Linux CLI for auditing shared libraries needed by dynamically
linked applications. It combines static ELF dependency inspection, dynamic loader
runtime observations, server inventory data, and explicit ignore policy to help
build offline deployment bundles.

The detailed behavior and schemas are described in
[docs/runtime-library-audit-spec.md](docs/runtime-library-audit-spec.md).

## Setup

- Rust stable
- `vorbere` for task shortcuts
- Linux tools used by commands: `readelf` and `ldconfig`

Build the project once after cloning:

```bash
vorbere run build
```

## Quick Start

Create a required-library report for an executable:

```bash
cargo run -- required \
  --exec ./target/release/dummygui \
  --env DISPLAY=:0 \
  --timeout 5s \
  --out required.json
```

Create a server inventory and compare it with the required list:

```bash
cargo run -- inventory --out server-inventory.json
cargo run -- diff \
  --required required.json \
  --inventory server-inventory.json \
  --ignore ignore.toml \
  --out missing.json
```

Copy resolvable bundle candidates from local search directories:

```bash
cargo run -- collect \
  --missing missing.json \
  --search-dir /usr/lib/x86_64-linux-gnu \
  --libdir ./package/usr/lib/dummygui/lib
```

## Common Commands

- Build: `vorbere run build`
- Test: `vorbere run test`
- Format: `vorbere run fmt`
- Lint and formatting checks: `vorbere run check`
- CI-equivalent checks: `vorbere run ci`
