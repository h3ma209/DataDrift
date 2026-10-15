# DataGrip REA analysis (Grip)

Static + dynamic reverse-engineering pass on **DataGrip 2026.1.3** (`DB-261.24374.56`) at `/Applications/DataGrip.app`, produced with **REA** and bytecode inspection.

Rust workspace:

| Crate | Role |
|-------|------|
| [`grip`](grip/) | Knowledge lib — embeds `reports/*.md`, exposes `grip::datagrip::reports()` |
| [`grip-ui`](grip-ui/) | **Dioxus desktop** shell — modern UI + pure-Rust drivers |

```bash
cargo check          # workspace
cargo test -p grip   # report catalog tests
cargo run -p grip-ui # Dioxus desktop (webview)
```

## Reports (markdown assets)

Canonical source: [`reports/`](reports/). Embedded at compile time by the `grip` crate.

### Index & inventory

| Path | Description |
|------|-------------|
| [reports/00-overview.md](reports/00-overview.md) | Index and architecture sketch |
| [reports/01-app-inventory.md](reports/01-app-inventory.md) | Full `.app` bundle graph |
| [reports/02-native-launcher.md](reports/02-native-launcher.md) | Mach-O launcher + Ghidra overview |
| [reports/03-info-plist.md](reports/03-info-plist.md) | `Info.plist` (permissions, bundle ID) |
| [reports/04-datagrip-impl-jar.md](reports/04-datagrip-impl-jar.md) | DataGrip-specific IDE plugin JAR |
| [reports/05-database-plugin-jar.md](reports/05-database-plugin-jar.md) | DatabaseTools plugin JAR |
| [reports/06-jdbc-connection-contents-lib.md](reports/06-jdbc-connection-contents-lib.md) | JDBC/connect scope in `Contents/lib/` only |

### Subsystem audit (1–4) + synthesis + dynamic

| Path | Subsystem | Description |
|------|-----------|-------------|
| [reports/07-core-architecture-lifecycle.md](reports/07-core-architecture-lifecycle.md) | 1 | Entry, bootstrap, plugin registry, lifecycle |
| [reports/02-jdbc-connections.md](reports/02-jdbc-connections.md) | 2 | JDBC child JVM, URL editors, PasswordSafe |
| [reports/08-query-parsing-introspection.md](reports/08-query-parsing-introspection.md) | 3 | SQL parse / dialect / introspection |
| [reports/09-ui-component-mapping.md](reports/09-ui-component-mapping.md) | 4 | Database tool window, consoles, actions |
| [reports/10-architecture-synthesis.md](reports/10-architecture-synthesis.md) | — | Cross-subsystem narrative |
| [reports/11-dynamic-validation.md](reports/11-dynamic-validation.md) | — | Phase 2a runtime validation |
| [reports/12-dynamic-validation-2b.md](reports/12-dynamic-validation-2b.md) | — | Phase 2b remaining items |
| [reports/13-ui-validation.md](reports/13-ui-validation.md) | — | Phase 2c UI validation runbook |
| [reports/14-datagrip-reference-guide.md](reports/14-datagrip-reference-guide.md) | — | Product UX reference map |

Evidence bundle: [rea-evidence/session-evidence.json](rea-evidence/session-evidence.json) (~17 MB).

## Rust API (`grip::datagrip`)

```rust
use grip::datagrip::{report_by_id, reports, ReportKind};

for r in reports() {
    println!("{} [{}] — {} bytes", r.id, r.file, r.len());
}

let jdbc = report_by_id("02-jdbc-connections").unwrap();
assert_eq!(jdbc.subsystem, Some("2"));
assert_eq!(jdbc.kind, ReportKind::Subsystem);
let _body: &str = jdbc.markdown;
```

Non-embedded helpers (repo-relative constants in `grip::datagrip::paths`):

| Path | Notes |
|------|--------|
| `scripts/validate-dynamic.sh` | Phase-2 log / process / index helpers |
| `tmp/jar-index.txt` | Full lib JAR class index (~18 MB, **gitignored**) |
| `tmp/jar-index-database-modules.txt` | DatabaseTools modules index (~1.7 MB, **gitignored**) |
| `tmp/phase2b/` | Optional javap/Vineflower scratch (**gitignored**) |

```bash
./scripts/validate-dynamic.sh all
./scripts/validate-dynamic.sh index RemoteJdbcServer
./scripts/validate-dynamic.sh ui-logs
./scripts/validate-dynamic.sh ui-runbook
```

## Grip UI

Two modes (toolbar):

| Mode | Role |
|------|------|
| **DB shell** | Data sources, JDBC dialog, explorer tree, table tabs, SQL console (default) |
| **Audit reports** | Browse embedded `grip::datagrip::reports()` markdown (incl. Phase 2c) |

Native drivers (`postgres` / `mysql` / `rusqlite`). Persists under `~/.grip/datasources.json`.

## Limits

- Inventory JARs are ZIP/class-index based; full JEB decompile not assumed.
- Dynamic notes avoid secrets / Keychain password reads.
- Large `tmp/` artifacts are local-only (see `.gitignore`).

Generated: 2026-10-15.
