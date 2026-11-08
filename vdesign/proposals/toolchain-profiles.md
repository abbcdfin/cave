# Proposal: toolchains (`--with NAME`)

Status: ACCEPTED 2026-11-08 as Phase 6 (`vdesign/plan.md`). Decisions recorded in `docs/architecture_design.md` › Toolchains, which governs where this text differs.

## Problem

Profiles describe agents: selected by the agent command's basename, they mount the agent's own state. A project's toolchain is a second, independent axis. A Flutter project under `cave claude` needs the Flutter SDK, the Android SDK, Gradle and pub caches, and a debug signing key, and none of these is visible today.

The only way to supply them is static: `[defaults]` in `~/.config/cave/profiles.toml` (every session gets them, including projects that are not Flutter), or a shell alias of `--dir` / `--ro-dir`. Both hard-code machine-specific paths, neither checks that the toolchain is actually there, and neither is reusable for the next toolchain.

## Proposal

### 1. Selection: `--with NAME`

`cave --with flutter claude` adds the toolchain `flutter` to the session. Repeatable (`--with flutter --with node`); a cave option, so it comes before the agent command. An unknown `NAME` is fatal.

Toolchains live in their own table, separate from agent profiles (decided 2026-11-08): `[profiles.*]` describes agents and is chosen by basename (or the `--profile NAME` candidate); `[toolchains.*]` describes a project's tools and is chosen only with `--with`. The user file layers over built-in toolchains the same way it layers over built-in profiles. Running a toolchain command directly takes `cave --with flutter flutter build apk`; `cave flutter build apk` alone gets the unknown-agent warning.

Mount order becomes: engine defaults → agent profile → `--with` toolchains in command-line order → working directory → `--dir` / `--ro-dir`. Explicit options still win.

### 2. Toolchain entries

A toolchain takes `runtime` as agent profiles do, plus the additions below. Agent profiles are unchanged.

| Need | Today | Proposed |
|---|---|---|
| RW path that must never be created (an SDK) | `state` creates missing paths | `rw_cache`, same semantics as in `[defaults]`: mounted if it exists, never created |
| Paths outside `$HOME` (`/opt/flutter`) | profile paths are `$HOME`-relative | a leading `/` means host-absolute; otherwise `$HOME`-relative, as now |
| Machine-specific locations | literal paths only | expansion: `${VAR}`, `${VAR:-default}`, `${bin:NAME}` |
| Fail loudly when the toolchain is absent | — | `requires = [...]`: any entry that does not resolve is fatal |
| A host service the toolchain depends on | — | `check_tcp = [{ addr, hint }]`: warn at startup if nothing listens |

`${bin:NAME}` is the canonical path of `NAME` found on `PATH` (symlinks resolved, so `/usr/bin/flutter → /opt/flutter/bin/flutter` yields the SDK). Expanded paths are normalized lexically before use, so `${bin:flutter}/../..` is the SDK root. Expansion never executes anything.

An entry outside `requires` that does not resolve (unset variable, missing path) is skipped with one stderr warning naming the toolchain and the entry.

### 3. Built-in `flutter` toolchain

```toml
[toolchains.flutter]
requires = ["${bin:flutter}"]
rw_cache = [
    "${bin:flutter}/../..",   # SDK: flutter writes bin/cache (lock file, engine artifacts) on every run
    ".pub-cache",
    ".gradle/caches",
    ".gradle/wrapper",
    ".dartServer",
]
runtime = [
    "${ANDROID_HOME:-Android/Sdk}",   # RO: install missing components on the host
    ".android/debug.keystore",         # stable debug signature across sessions
    ".config/flutter",
]
check_tcp = [
    { addr = "127.0.0.1:5037", hint = "start the host adb server: ADB_REJECT_KILL_SERVER=1 adb start-server" },
]
```

`~/.android/adbkey` and the rest of `~/.android` stay hidden; files Gradle or adb write there land in the private `$HOME` tmpfs. `JAVA_HOME` (`/opt/android-studio/jbr` on the dogfood host) and `/usr/bin/adb` are already visible through `ro_system`.

### 4. Device debugging through the host adb server

The host owns the device: a USB phone or an emulator, set up outside cave. cave does not unshare the network, so `adb` inside the sandbox reaches the host server on `localhost:5037`. Verified 2026-11-08 from inside cave with no changes: `adb devices -l` listed the host's OnePlus 8T (`KB2000`) and `adb shell getprop` answered. `flutter run`'s install, port forwards and VM-service connection all go through that server, and forwarded ports listen on host `localhost`, which the sandbox shares.

Two failure modes, both from the shared network:

- **No host server.** `adb` in cave starts its own on the host's port 5037: no USB devices (cave's `/dev` has none), a fresh key, and it dies with the session while host tools talk to it. `check_tcp` warns at startup.
- **`adb kill-server` from a clumsy agent.** The next `adb` call restarts the server inside the sandbox, with the same result. Host-side guard: start the server with `ADB_REJECT_KILL_SERVER=1` (present in the host's adb 37.0.0; refusal not yet tested).

### 5. Purity and diagnostics

Expansion is a pure function over a lookup (environment and `PATH`), unit-testable without a real toolchain. `main.rs` performs the lookups and passes resolved toolchains into `build_mount_plan`, as it does with the config and `SSH_AUTH_SOCK`. `cave --doctor --with flutter` prints each entry, what it resolved to, and the `check_tcp` results. Resolved paths go through the same `/` and `$HOME` warnings as `--dir`.

## Out of scope

- **iOS.** Flutter builds and debugs iOS only on macOS with Xcode; cave is Linux-only.
- **Desktop and web targets that open windows** (`-d linux`, `-d chrome`). They need a display socket, a separate boundary decision: X11 lets a client read and type into other windows. `flutter run -d web-server` works without one.
- **Emulator or USB inside the sandbox.** Both need devices (`/dev/kvm`, `/dev/bus/usb`) cave does not expose; the host provides them.

## Alternatives considered

- **Static `[defaults]` in the user file.** Works today and stays the fallback, but every session gets the SDK, paths are hard-coded, and nothing checks the toolchain.
- **Flutter-specific Rust code.** Rejected: a special case in the engine, repeated for the next toolchain. The engine stays generic; Flutter is data.
- **Asking the toolchain (`flutter config --machine`) at startup.** Rejected: seconds of startup against the < 100 ms budget, and it runs a host program outside the sandbox before the sandbox exists.
- **Shell alias of `--dir` / `--ro-dir`.** No `requires`, no adb check, not reusable.
- **One table for agents and toolchains.** Rejected 2026-11-08: the two purposes would mix. `cave flutter build apk` would select `flutter` as its agent profile by basename, and `--with claude` would add an agent's state to another agent's session.

## Costs

- The Flutter SDK is writable: a clumsy agent can damage it. It is reinstallable, so it is classed as a cache.
- Expansion is a new mechanism. It is limited to three forms and never runs commands.
- `~/.gradle/gradle.properties` can hold signing passwords, so only `.gradle/caches` and `.gradle/wrapper` are mounted; Gradle's other files in `~/.gradle` are per-session.

## Done when

`vdesign/plan.md` › Phase 6.
