# Validation Coverage

The current test suite covers:

- CLI help lists the implemented commands.
- `required` emits static `DT_NEEDED` libraries for a dynamically linked
  executable.
- `LD_DEBUG` parsing extracts requested libraries and loaded paths.
- `ldconfig -p` parsing extracts SONAME-to-path entries.
- `diff` reports missing SONAMEs.
- `diff` separates ignored missing libraries from bundle candidates.
- `diff` rejects architecture mismatches by default.
- Ignore TOML parsing supports repeated `[[ignore]]` entries.
- Generated required-report JSON can be parsed by the built-in reader.
- `collect` copies exact SONAME matches from a supplied search directory.
- `collect` reports unresolved bundle candidates.

The tests intentionally avoid external services and use local executables,
temporary directories, and static JSON fixtures.
