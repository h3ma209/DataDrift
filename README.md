# DataGrip REA analysis (Grip)

Static reverse-engineering pass on **DataGrip 2026.1.3** at `/Applications/DataGrip.app`, produced with **REA** (rea-agents 6.5.0).

## Outputs

| Path | Description |
|------|-------------|
| [reports/00-overview.md](reports/00-overview.md) | Index and architecture sketch |
| [reports/01-app-inventory.md](reports/01-app-inventory.md) | Full `.app` bundle graph (`inspect_artifact`) |
| [reports/02-native-launcher.md](reports/02-native-launcher.md) | Mach-O launcher + Ghidra overview |
| [reports/03-info-plist.md](reports/03-info-plist.md) | `Info.plist` (permissions, bundle ID) |
| [reports/04-datagrip-impl-jar.md](reports/04-datagrip-impl-jar.md) | DataGrip-specific IDE plugin JAR |
| [reports/05-database-plugin-jar.md](reports/05-database-plugin-jar.md) | DatabaseTools plugin JAR |
| [reports/06-jdbc-connection-contents-lib.md](reports/06-jdbc-connection-contents-lib.md) | JDBC/connect scope in `Contents/lib/` only (REA) |
| [rea-evidence/session-evidence.json](rea-evidence/session-evidence.json) | Full REA session evidence bundle (17 records, ~17 MB) |

## Limits (this pass)

- **No JVM bytecode decompile** — JARs inventoried as ZIP; Java needs JEB/JADX (not configured in REA here).
- **No runtime** — app not launched; no JDBC/network capture.
- **Launcher only for native deep dive** — Ghidra bound to `Contents/MacOS/datagrip`, not every nested Mach-O in JBR/JCEF.

## Suggested next REA probes

1. `open_binary` + Ghidra on `Contents/jbr/Contents/Home/lib/server/libjvm.dylib` (JVM internals).
2. Configure `REA_JEB_MCP_JAR` and decompile hot classes from `database-plugin.jar`.
3. `observe_native_calls` while connecting to a DB (runtime).

## Grip UI (Rust)

**DataGrip-style shell** (not a report viewer): data sources, JDBC URL dialog, test/connect, schema tree, SQL console, services log. Native drivers (`postgres` / `mysql` / `rusqlite`); REA markdown stays in `reports/` only.

```bash
cd grip-ui
cargo run --release
```

Data sources persist under `~/.grip/datasources.json`.

Generated: 2026-10-14.
