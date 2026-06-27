# Runtime Library Audit Tool Specification

Status: Draft

## Objective

Define a Linux tool that helps package GUI applications for offline deployment by identifying runtime shared libraries that must be bundled with the application package.

The tool must support both `x86_64` and `aarch64` Linux environments. It is intended for workflows where development usually happens on `x86_64`, while deployment targets may be offline `aarch64` servers.

## Problem Statement

Static ELF dependency inspection is not sufficient for GUI applications. Libraries such as `winit`, `eframe`, OpenGL, X11, Wayland, and input-method stacks may load shared objects at runtime with `dlopen`. These libraries do not always appear in `readelf -d` or normal `ldd` output.

For offline servers, package managers cannot fetch missing dependencies. The application provider therefore needs a repeatable way to:

- list libraries required by the application,
- list libraries already available on the target server,
- compare both lists,
- remove libraries that are intentionally server-owned,
- produce a final bundle candidate list.

## Scope

In scope:

- Linux `x86_64` and `aarch64`.
- Dynamically linked ELF executables.
- Direct ELF dependencies from `DT_NEEDED`.
- Runtime-loaded libraries observed with the dynamic loader.
- Server inventory from `ldconfig`.
- Missing library detection by SONAME.
- Ignore lists for server-owned libraries.
- Machine-readable and human-readable outputs.

Out of scope:

- Windows and macOS support.
- Fully static binaries.
- Cross-architecture binary execution.
- Downloading OS packages.
- Resolving package names from online repositories.
- Guaranteeing ABI compatibility across arbitrary distributions.
- Bundling GPU drivers, Mesa DRI drivers, or glibc.

## Deployment Responsibility Model

The tool must support a clear responsibility split.

Server responsibility:

- Linux kernel and base OS.
- `glibc` runtime and ELF interpreter.
- Xorg or Wayland display server.
- GPU, DRM, Mesa, OpenGL, EGL, GLX, Vulkan, or vendor driver stack.
- Base display libraries already guaranteed by the server profile.

Application responsibility:

- The application executable.
- Runtime helper libraries required by the application stack but not guaranteed by the server profile.
- Optional bundled libraries placed in an application-private directory.

The tool must not automatically assume every missing library should be bundled. The ignore list is the policy boundary between server-owned and application-owned libraries.

## Commands

The implementation should expose these commands. Names are provisional.

### `required`

Produces the application-required library list.

Inputs:

- Path to executable.
- Optional command arguments.
- Optional environment variables.
- Optional timeout.
- Optional working directory.
- Optional output path.

Behavior:

- Read static dependencies from `readelf -d`.
- Run the executable with `LD_DEBUG=libs,files`.
- Capture dynamic loader diagnostics from stderr.
- Extract requested SONAMEs from `find library=...` lines.
- Extract loaded library paths from `calling init: ...` lines.
- Preserve failures such as `not found`, `cannot open`, and library load panics in diagnostics.
- De-duplicate and sort outputs.

Example:

```sh
runtime-lib-audit required \
  --exec ./target/release/dummygui \
  --env DISPLAY=:0 \
  --env WINIT_UNIX_BACKEND=x11 \
  --timeout 5s \
  --out required.json
```

### `inventory`

Produces the library list available on the current server.

Inputs:

- Optional output path.
- Optional additional library search directories.

Behavior:

- Read `ldconfig -p`.
- Extract available SONAMEs and paths.
- Optionally scan explicit directories for `.so` files and symlinks.
- De-duplicate and sort outputs.

Example:

```sh
runtime-lib-audit inventory --out server-inventory.json
```

### `diff`

Compares required libraries with a server inventory and emits missing libraries.

Inputs:

- Required list.
- Server inventory.
- Optional ignore list.
- Optional output path.

Behavior:

- Compare by SONAME, not by full path.
- Produce:
  - missing before ignore,
  - ignored missing libraries,
  - final missing bundle candidates.
- Preserve reason strings from the ignore list.

Example:

```sh
runtime-lib-audit diff \
  --required required.json \
  --inventory server-inventory.json \
  --ignore ignore.toml \
  --out missing.json
```

### `collect`

Optional command for collection environments.

Inputs:

- Final missing list.
- Library search paths.
- Output directory.

Behavior:

- Resolve each SONAME to an architecture-native file.
- Copy the resolved file with symlinks needed by its SONAME.
- Recurse into dependencies of collected files unless ignored.
- Report unresolved items.

Example:

```sh
runtime-lib-audit collect \
  --missing missing.json \
  --libdir ./package/usr/lib/dummygui/lib
```

## Output Schemas

The exact JSON shape may evolve, but it must include these fields.

### Required List

```json
{
  "schema_version": 1,
  "arch": "aarch64",
  "executable": "./dummygui",
  "command": ["./dummygui"],
  "environment": {
    "DISPLAY": ":0",
    "WINIT_UNIX_BACKEND": "x11"
  },
  "static_needed": [
    "libgcc_s.so.1",
    "libm.so.6",
    "libc.so.6"
  ],
  "runtime_requested": [
    "libXcursor.so.1",
    "libXi.so.6",
    "libxkbcommon-x11.so.0"
  ],
  "loaded_paths": [
    "/lib/aarch64-linux-gnu/libX11.so.6",
    "/usr/lib/aarch64-linux-gnu/libGL.so.1"
  ],
  "diagnostics": [
    "Library libxkbcommon-x11.so could not be loaded"
  ]
}
```

### Server Inventory

```json
{
  "schema_version": 1,
  "arch": "aarch64",
  "libraries": [
    {
      "soname": "libX11.so.6",
      "path": "/lib/aarch64-linux-gnu/libX11.so.6"
    }
  ]
}
```

### Diff Output

```json
{
  "schema_version": 1,
  "arch": "aarch64",
  "missing_before_ignore": [
    "libXcursor.so.1",
    "libGL.so.1"
  ],
  "ignored": [
    {
      "soname": "libGL.so.1",
      "reason": "server-owned GPU/OpenGL stack"
    }
  ],
  "bundle_candidates": [
    "libXcursor.so.1"
  ]
}
```

## Ignore List Format

The ignore list must be explicit and version-controlled.

Example TOML:

```toml
[[ignore]]
pattern = "ld-linux-aarch64.so.1"
reason = "ELF interpreter is owned by the target OS"

[[ignore]]
pattern = "libc.so.6"
reason = "glibc is owned by the target OS"

[[ignore]]
pattern = "libm.so.6"
reason = "glibc math library is owned by the target OS"

[[ignore]]
pattern = "libgcc_s.so.1"
reason = "compiler runtime is expected from the base OS"

[[ignore]]
pattern = "libGL*.so*"
reason = "OpenGL dispatch and GLX stack are server-owned"

[[ignore]]
pattern = "libEGL*.so*"
reason = "EGL stack is server-owned"

[[ignore]]
pattern = "libGLES*.so*"
reason = "GLES stack is server-owned"

[[ignore]]
pattern = "libdrm*.so*"
reason = "DRM stack is server-owned"

[[ignore]]
pattern = "libgbm.so*"
reason = "GBM stack is server-owned"

[[ignore]]
pattern = "libvulkan*.so*"
reason = "Vulkan loader and drivers are server-owned"

[[ignore]]
pattern = "libgallium*.so*"
reason = "Mesa driver implementation is server-owned"

[[ignore]]
pattern = "libLLVM*.so*"
reason = "Mesa/compiler backend dependency is server-owned"
```

Pattern matching may use shell-style glob syntax. Exact matching must be supported.

## Recommended Workflow

### Development on `x86_64`

Use the `required` command on the local development machine to produce an initial SONAME list. This list is useful because SONAMEs are generally architecture-independent.

This list is not final. GPU, Mesa, Wayland, X11, and input-method behavior may differ by architecture and server configuration.

### Collection on `aarch64`

Use an `aarch64` collection environment to resolve final library files. The collection environment should be close to the target server distribution and version.

The collection environment may have network access. The offline target server does not need network access.

### Target Server Validation

On the offline target server:

- run `inventory`,
- run `diff` with the required list and ignore list,
- confirm `bundle_candidates` is empty after installing the application package,
- run the application with `LD_DEBUG=libs,files`,
- fail validation if `not found`, `cannot open`, or known runtime load errors appear.

## Example for the Current GUI Application

For an `eframe` / `egui` / `winit` X11 application, likely application-owned bundle candidates include:

```text
libXcursor.so.1
libXfixes.so.3
libXi.so.6
libxkbcommon-x11.so.0
libxkbcommon.so.0
libxcb-xkb.so.1
```

Likely server-owned libraries include:

```text
libc.so.6
libm.so.6
libgcc_s.so.1
ld-linux-aarch64.so.1
libGL.so.1
libGLX.so.0
libGLdispatch.so.0
libGLX_mesa.so.0
libdrm.so.2
libgallium*.so
libLLVM*.so
```

This example is not a hardcoded rule. The implementation must derive the required list from runtime observation and apply the ignore list as policy.

## Error Handling

The tool must report:

- executable not found,
- executable is not dynamically linked,
- `readelf` unavailable,
- `ldconfig` unavailable,
- target command timed out,
- target command exited unsuccessfully,
- `LD_DEBUG` output could not be parsed,
- architecture mismatch between required list and server inventory,
- unresolved SONAME during collection.

The `required` command must still emit partial results when the application exits with an error after loading some libraries.

## Acceptance Criteria

- Given a dynamically linked executable, `required` emits static `DT_NEEDED` libraries.
- Given a GUI executable that uses `dlopen`, `required` emits runtime-requested libraries observed through `LD_DEBUG`.
- Given a server with `ldconfig`, `inventory` emits SONAME-to-path entries.
- Given required and inventory inputs, `diff` emits missing SONAMEs.
- Given an ignore list, `diff` separates ignored missing libraries from bundle candidates.
- Given `x86_64` required output and an `aarch64` inventory, `diff` reports architecture mismatch unless explicitly overridden.
- Given an offline target server, validation can be performed without network access.
- Given missing app-owned libraries, `collect` can copy architecture-native `.so` files into a package library directory.

## Open Questions

- Should the initial implementation be a standalone Rust CLI in this repository or a separate packaging utility?
- Should outputs default to JSON only, or include text/table output by default?
- Should `collect` recurse through dependencies automatically, or require an explicit second `required` pass over collected libraries?
- Should Wayland and X11 be separate profiles with different default ignore lists?
- Should package generation be part of this tool, or remain a separate task?
