# Validation Coverage

The current test suite covers:

- CLI help lists the implemented commands.
- `required` emits static `DT_NEEDED` libraries for a dynamically linked
  executable.
- `LD_DEBUG` parsing extracts requested libraries and loaded paths.
- `ldconfig -p` parsing extracts SONAME-to-path entries.
- Explicit inventory library directory scanning includes `.so` files and
  symlinks while ignoring unrelated files.
- CLI `inventory --libdir` includes explicit library directory entries.
- `diff` reports missing SONAMEs.
- `diff` separates ignored missing libraries from bundle candidates.
- `diff` rejects architecture mismatches by default.
- `diff --allow-arch-mismatch` permits cross-architecture SONAME comparison.
- Ignore TOML parsing supports repeated `[[ignore]]` entries.
- Ignore matching supports exact matches, `*`, and `?` glob patterns.
- Generated required-report JSON can be parsed by the built-in reader.
- Generated inventory-report JSON can be parsed by the built-in reader.
- Generated diff-report JSON can be parsed for bundle candidates by the built-in
  reader.
- `collect` copies exact SONAME matches from a supplied search directory.
- `collect` reports unresolved bundle candidates.

The tests intentionally avoid external services and use local executables,
temporary directories, and static JSON fixtures.
