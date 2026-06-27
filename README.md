# RuntimeScaner

This repository is the current scaffold for the RuntimeScaner CLI.
The binary is still minimal and currently prints Hello, world!, but the project metadata and task wiring are aligned to this repository.

## Setup

- Rust stable
- `vorbere` for task shortcuts

Build the project once after cloning:

```bash
vorbere run build
```

## Common Commands

- Run: `vorbere run run`
- Test: `vorbere run test`
- Format: `vorbere run fmt`
- Lint: `vorbere run clippy`
- CI-equivalent checks: `vorbere run ci`

## Project Structure

- `src/main.rs`: current RuntimeScaner CLI entry point
- `tests/`: smoke test for the compiled binary
- `vorbere.yaml`: local development tasks

## Expected Output

```text
Hello, world!
```
