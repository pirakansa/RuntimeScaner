# Getting Started

This guide explains the shortest path to running RuntimeScaner locally.

## Prerequisites

- Linux
- RuntimeScaner binary available in `PATH`
- `readelf`
- `ldconfig`

## Verify The Binary

```sh
runtimescaner --help
```

## Create A Required-Library Report

Run `required` against a dynamically linked executable:

```sh
runtimescaner required \
  --exec /bin/true \
  --timeout 1s \
  --out required.json
```

The command reads static `DT_NEEDED` dependencies with `readelf -d`, then runs
the executable with `LD_DEBUG=libs,files` and records runtime loader activity.

Use `--arg` for command arguments, `--env NAME=VALUE` for environment variables,
and `--cwd` when the executable must run from a specific working directory.

## Create A Server Inventory

```sh
runtimescaner inventory --out server-inventory.json
```

The command reads `ldconfig -p`. Add `--lib-dir <dir>` for extra directories
that should be scanned for `.so` files and symlinks.

## Compare Reports

```sh
runtimescaner diff \
  --required required.json \
  --inventory server-inventory.json \
  --out missing.json
```

By default, `diff` rejects mismatched architectures between the required report
and the inventory. Use `--allow-arch-mismatch` only when you intentionally want
to compare SONAMEs across architectures.
