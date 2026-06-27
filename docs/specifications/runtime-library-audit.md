# Runtime Library Audit

This reference defines RuntimeScaner's implemented command behavior.

## Objective

RuntimeScaner helps package Linux GUI applications for offline deployment by
identifying runtime shared libraries that may need to be bundled with an
application package.

Static ELF dependency inspection is not enough for GUI applications. Libraries
such as `winit`, `eframe`, OpenGL, X11, Wayland, and input-method stacks may
load shared objects at runtime with `dlopen`, so some dependencies only appear
while the application runs.

## Responsibility Model

Server responsibility:

- Linux kernel and base OS.
- `glibc` runtime and ELF interpreter.
- Xorg or Wayland display server.
- GPU, DRM, Mesa, OpenGL, EGL, GLX, Vulkan, or vendor driver stack.
- Base display libraries guaranteed by the server profile.

Application responsibility:

- The application executable.
- Runtime helper libraries required by the application stack but not guaranteed
  by the server profile.
- Optional bundled libraries placed in an application-private directory.

RuntimeScaner does not assume every missing library should be bundled. The
ignore list is the policy boundary between server-owned and application-owned
libraries.

## Commands

### `required`

Produces the application-required library list.

Inputs:

- `--exec <path>`: executable to inspect.
- `--arg <value>`: argument passed to the executable. May be repeated.
- `--env NAME=VALUE`: environment variable passed to the executable. May be
  repeated.
- `--timeout <duration>`: timeout such as `5s` or `500ms`.
- `--cwd <dir>`: working directory for the executable.
- `--out <file>`: optional output path. Without this option, JSON is printed to
  stdout.

Behavior:

- Reports an error when the executable path does not exist.
- Reads static dependencies from `readelf -d`.
- Runs the executable with `LD_DEBUG=libs,files`.
- Captures dynamic loader diagnostics from stderr.
- Extracts requested SONAMEs from `find library=...` lines.
- Extracts loaded library paths from `calling init: ...` lines.
- Preserves loader failures such as `not found`, `cannot open`, known library
  load messages, panics, timeouts, and unsuccessful exit status in diagnostics.
- De-duplicates and sorts static dependencies, runtime requests, loaded paths,
  and diagnostics.
- Emits partial runtime observations even when the target exits unsuccessfully
  after loading libraries.

Example:

```sh
runtimescaner required \
  --exec ./target/release/dummygui \
  --env DISPLAY=:0 \
  --env WINIT_UNIX_BACKEND=x11 \
  --timeout 5s \
  --out required.json
```

### `inventory`

Produces the library list available on the current server.

Inputs:

- `--lib-dir <dir>` or `--libdir <dir>`: additional directory to scan for `.so`
  files and symlinks. May be repeated.
- `--out <file>`: optional output path.

Behavior:

- Reads `ldconfig -p`.
- Extracts available SONAMEs and paths.
- Scans explicit directories for file names ending in `.so` or containing
  `.so.`.
- De-duplicates and sorts by SONAME, then path.

Example:

```sh
runtimescaner inventory --out server-inventory.json
```

### `diff`

Compares required libraries with a server inventory and emits missing libraries.

Inputs:

- `--required <file>`: required report JSON.
- `--inventory <file>`: inventory report JSON.
- `--ignore <file>`: optional ignore policy TOML.
- `--allow-arch-mismatch`: permit cross-architecture SONAME comparison.
- `--out <file>`: optional output path.

Behavior:

- Compares by SONAME, not by full path.
- Rejects architecture mismatches unless `--allow-arch-mismatch` is present.
- Produces missing libraries before ignore, ignored missing libraries, and final
  bundle candidates.
- Preserves reason strings from the ignore list.

Example:

```sh
runtimescaner diff \
  --required required.json \
  --inventory server-inventory.json \
  --ignore ignore.toml \
  --out missing.json
```

### `collect`

Copies resolvable bundle candidates into an application-private directory.

Inputs:

- `--missing <file>`: diff report JSON.
- `--search-dir <dir>`: directory used to resolve bundle candidate SONAMEs. May
  be repeated.
- `--lib-dir <dir>` or `--libdir <dir>`: output directory.
- `--out <file>`: optional report output path.

Behavior:

- Creates the output directory when it does not exist.
- Resolves each bundle candidate by exact file name under the supplied search
  directories.
- Copies resolved files into the output directory using the candidate SONAME as
  the destination file name.
- Reports unresolved candidates.
- Does not recursively collect dependencies of copied libraries.

Example:

```sh
runtimescaner collect \
  --missing missing.json \
  --search-dir /usr/lib/x86_64-linux-gnu \
  --libdir ./package/usr/lib/dummygui/lib \
  --out collect.json
```

## Ignore List Format

Ignore lists are TOML documents containing repeated `[[ignore]]` tables:

```toml
[[ignore]]
pattern = "libGL*.so*"
reason = "OpenGL dispatch and GLX stack are server-owned"

[[ignore]]
pattern = "libc.so.6"
reason = "glibc is owned by the target OS"
```

`pattern` supports exact matches and shell-style `*` and `?` globs. `reason` is
copied into the diff report for ignored missing libraries.

## Error Behavior

The tool reports:

- executable not found,
- executable with no dynamic section as reported by `readelf`,
- `readelf` unavailable or unsuccessful,
- `ldconfig` unavailable or unsuccessful,
- target command timeout,
- target command unsuccessful exit,
- architecture mismatch between required list and server inventory,
- unreadable input files,
- unwritable output files,
- unresolved SONAMEs during collection.
