# Offline Workflow

RuntimeScaner is intended for packaging workflows where a deployment target may
not be able to install missing packages from the network.

## 1. Record Application Requirements

On a development or collection machine, run the application in the mode that
loads the GUI backend and optional runtime libraries you expect to use:

```sh
runtimescaner required \
  --exec ./target/release/dummygui \
  --env DISPLAY=:0 \
  --env WINIT_UNIX_BACKEND=x11 \
  --timeout 5s \
  --out required.json
```

The report includes static dependencies, runtime-requested SONAMEs observed from
`LD_DEBUG`, loaded paths, and diagnostics.

## 2. Record Target Inventory

On the target server, or an environment that matches it closely, run:

```sh
runtimescaner inventory --out server-inventory.json
```

When the target has application-private library directories, include them with
`--lib-dir`.

## 3. Apply Ignore Policy

Keep server-owned libraries in a version-controlled ignore file:

```toml
[[ignore]]
pattern = "libGL*.so*"
reason = "OpenGL dispatch and GLX stack are server-owned"

[[ignore]]
pattern = "libc.so.6"
reason = "glibc is owned by the target OS"
```

Run the comparison:

```sh
runtimescaner diff \
  --required required.json \
  --inventory server-inventory.json \
  --ignore ignore.toml \
  --out missing.json
```

The diff output separates missing libraries before ignore, ignored missing
libraries with reasons, and final bundle candidates.

## 4. Copy Bundle Candidates

In an architecture-native collection environment, copy resolvable candidates:

```sh
runtimescaner collect \
  --missing missing.json \
  --search-dir /usr/lib/x86_64-linux-gnu \
  --libdir ./package/usr/lib/dummygui/lib \
  --out collect.json
```

The current implementation copies exact SONAME matches from the supplied search
directories and reports unresolved candidates. It does not recursively resolve
dependencies of copied libraries.

## 5. Validate Offline

After installing the package on the offline target server:

- Run `inventory`.
- Run `diff` with the required report and ignore policy.
- Confirm `bundle_candidates` is empty.
- Run the application with `LD_DEBUG=libs,files`.
- Treat `not found`, `cannot open`, and known runtime load errors as validation
  failures.
