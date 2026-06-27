# Output Schemas

RuntimeScaner emits JSON with `schema_version: 1`.

## Required Report

Fields:

- `schema_version`: schema version number.
- `arch`: current machine architecture.
- `executable`: inspected executable path.
- `command`: executable path plus `--arg` values.
- `environment`: `--env` values passed to the executable.
- `static_needed`: static `DT_NEEDED` SONAMEs from `readelf -d`.
- `runtime_requested`: SONAMEs observed from `LD_DEBUG` `find library=...`
  lines.
- `loaded_paths`: paths observed from `LD_DEBUG` `calling init: ...` lines.
- `diagnostics`: captured loader/runtime diagnostics.

Example:

```json
{
  "schema_version": 1,
  "arch": "x86_64",
  "executable": "/bin/true",
  "command": [
    "/bin/true"
  ],
  "environment": {},
  "static_needed": [
    "libc.so.6"
  ],
  "runtime_requested": [
    "libc.so.6"
  ],
  "loaded_paths": [
    "/lib/x86_64-linux-gnu/libc.so.6",
    "/lib64/ld-linux-x86-64.so.2"
  ],
  "diagnostics": []
}
```

## Inventory Report

Fields:

- `schema_version`: schema version number.
- `arch`: current machine architecture.
- `libraries`: available libraries with `soname` and `path`.

Example:

```json
{
  "schema_version": 1,
  "arch": "x86_64",
  "libraries": [
    {
      "soname": "libX11.so.6",
      "path": "/lib/x86_64-linux-gnu/libX11.so.6"
    }
  ]
}
```

## Diff Report

Fields:

- `schema_version`: schema version number.
- `arch`: inventory architecture.
- `missing_before_ignore`: required SONAMEs absent from inventory before ignore
  policy is applied.
- `ignored`: missing SONAMEs ignored by policy, with `soname` and `reason`.
- `bundle_candidates`: missing SONAMEs that remain application-owned candidates.

Example:

```json
{
  "schema_version": 1,
  "arch": "x86_64",
  "missing_before_ignore": [
    "libGL.so.1",
    "libXcursor.so.1"
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

## Collect Report

Fields:

- `schema_version`: schema version number.
- `arch`: current machine architecture.
- `copied`: copied libraries with `soname` and destination `path`.
- `unresolved`: bundle candidates that could not be found in search
  directories.

Example:

```json
{
  "schema_version": 1,
  "arch": "x86_64",
  "copied": [
    {
      "soname": "libXcursor.so.1",
      "path": "./package/usr/lib/dummygui/lib/libXcursor.so.1"
    }
  ],
  "unresolved": []
}
```
