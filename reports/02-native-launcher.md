# Native launcher — `Contents/MacOS/datagrip` (REA deep check)

**Path:** `/Applications/DataGrip.app/Contents/MacOS/datagrip`  
**On disk:** ~744 KB, Mach-O 64-bit executable **arm64**  
**SHA-256:** `ad4492be2278803d8c7397e32354361dd4e12ec70fc95344f46fb811746173fc`  
**Mach-O UUID:** `0A5A4D73-8606-3C6C-AAC1-9C303768C7E1`  
**Min OS (load command):** macOS 11.0 · **SDK:** 26.0 · **LD:** 1221.4  
**Entry file offset:** 2296 → Ghidra `entry` @ `0x1000008f8` → `xplat_launcher::main` @ `0x1000008f4`

## What it is

**Not Java.** Rust **`xplat_launcher`** (JetBrains cross-platform IDE bootstrap), shared across IntelliJ-family products.

| Ghidra metric | Value |
|---------------|-------|
| Procedures | 2092 |
| Defined strings | 2951 |
| `main_lib` body | ~8 KB (non-contiguous) |

## Call chain (observed)

```
entry (0x1000008f8)
  └─ xplat_launcher::main (0x1000008f4)   // 1 insn: branch
       └─ xplat_launcher::main_lib (0x100051ed0)   // all launch logic
```

`main` pseudocode (Ghidra):

```c
void xplat_launcher::main(void) {
  main_lib();
  return;
}
```

## Launch configuration (symbols + strings)

Implements trait `LaunchConfiguration`:

| Mode | Rust type |
|------|-----------|
| Normal desktop | `DefaultLaunchConfiguration` |
| Remote dev / gateway | `RemoteDevLaunchConfiguration` |

**Default path** (from symbols/strings):

- Load **`Resources/product-info.json`** (`ProductInfo`, serde_json)
- Find IDE home (symlink resolve, parent walk)
- JBR: env → user config → **bundled** `Contents/jbr` → `JDK_HOME` → `JAVA_HOME` → `…/Contents/Home`
- VM options: `datagrip.vmoptions` + user file
- Classpath, custom properties, args
- JNI: `JNI_CreateJavaVM`, `-Djava.class.path=`, `-Dsun.java.command=`
- macOS: CoreFoundation event loop + `run_jvm_and_event_loop` on worker thread

**Remote-dev path** (same binary; argv branch):

- Strings: `remote-dev-server`, `remote-devMode`, `REMOTE_DEV_*` env vars
- `get_known_intellij_commands`, `print_help`
- Property: `ide.started.from.remote.dev.launcher`

## Code signing (REA `inspect_signature`)

| Field | Value |
|-------|-------|
| Signed | yes |
| Identifier | `com.jetbrains.datagrip` |
| Team | `2ZEFAR8TH3` (JetBrains s.r.o.) |
| Hardened runtime | yes |
| Timestamp | 4 May 2026 |

**Entitlements (launcher binary):**

- `com.apple.security.cs.allow-jit`
- `com.apple.security.cs.allow-unsigned-executable-memory`
- `com.apple.security.cs.disable-library-validation`
- Apple Events, camera, mic, location, photos, calendars, address book

(JVM/JCEF need JIT + unsigned executable memory — expected for IDE.)

## Dynamic libraries (LC_LOAD_DYLIB)

- `libiconv.2.dylib`
- `CoreFoundation.framework`
- `libSystem.B.dylib`

~104 import symbols reported (partial inventory).

## Takeaway for Grip

- **Hook/patch surface:** `main_lib`, `DefaultLaunchConfiguration::*`, `xplat_launcher::java::*`, env vars (`DATAGRIP_*`, `JAVA_HOME`, `REMOTE_DEV_*`).
- **Not where SQL/DB logic lives** — that stays in JARs after JVM start.
- Full `main_lib` decompile retained in REA session evidence (`ev_e7ba29ae…`, ~65k UTF-16 pseudocode units); page with `inspect_analysis_view` if needed.

REA: Ghidra 12.1.4 · 2026-10-14
