# Specification References

These references describe implemented RuntimeScaner behavior that should remain
stable as the tool evolves. Code and passing tests are the source of truth.

## Product Scope

RuntimeScaner is a Rust CLI for auditing Linux shared library requirements for
dynamically linked executables.

Implemented scope:

- Linux runtime inspection for the current machine architecture.
- Static `DT_NEEDED` extraction through `readelf -d`.
- Runtime loader observation through `LD_DEBUG=libs,files`.
- Server inventory extraction through `ldconfig -p`.
- Optional inventory scanning of explicit library directories.
- SONAME-based diffing.
- Explicit TOML ignore policy with exact and shell-style glob patterns.
- JSON output for all commands.
- Copying exact SONAME bundle candidates from supplied search directories.

Out of current implementation scope:

- Windows and macOS support.
- Cross-architecture execution.
- Downloading packages.
- Online package lookup.
- Recursive dependency collection for copied libraries.
- ABI compatibility guarantees across distributions.

## References

- [Runtime library audit](runtime-library-audit.md): command contracts,
  responsibility model, ignore policy, and error behavior.
- [Output schemas](output-schemas.md): JSON fields emitted by commands.
- [Validation coverage](validation-coverage.md): behavior covered by the current
  test suite.
