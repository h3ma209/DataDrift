# Overview — DataGrip static layout

**Target:** `/Applications/DataGrip.app`  
**Root SHA-256:** `7156117f58f6450e76bebf4e3f2c1858dbf5f638c21398fd8c83ba346e7ecabf`  
**Evidence:** REA `inspect_artifact` (directory), 2026-10-14

## Stack (observed)

```
macOS .app bundle
├── Contents/MacOS/datagrip          → Rust/JNI launcher (Mach-O arm64)
├── Contents/jbr/                      → Bundled JetBrains Runtime (JBR)
│   └── .../Home/bin/java              → JVM entry (Mach-O)
├── Contents/lib/app.jar               → Platform bootstrap (~23 MB ZIP)
└── Contents/plugins/
    ├── datagrip-impl/lib/datagrip-impl.jar     → DG IDE shell (~235 KB)
    └── DatabaseTools/lib/database-plugin.jar   → DB engine UI (~45 MB)
```

## Launch path (inferred from launcher strings)

1. Native `datagrip` resolves JBR: env → user config → **bundled runtime** → `JDK_HOME` → `JAVA_HOME`.
2. Reads VM options (`datagrip.vmoptions`, user overrides).
3. `JNI_CreateJavaVM`, sets `-Djava.class.path=…`, runs Java `main` on primordial thread + CoreFoundation event loop.

## Inventory scale

| Artifact | Graph nodes | ZIP/archive entries |
|----------|-------------|---------------------|
| `DataGrip.app` | 2018 | 2297 occurrences |
| `datagrip-impl.jar` | 66 | 66 entries |
| `database-plugin.jar` | 5037 | 5077 entries |

## Key hashes

| File | SHA-256 |
|------|---------|
| `Contents/MacOS/datagrip` | `ad4492be2278803d8c7397e32354361dd4e12ec70fc95344f46fb811746173fc` |
| `datagrip-impl.jar` | `e74f78a180b459dfcca507cfa36fe97a1a6e4e1cd384e3bc70a7600c6d8b0589` |
| `database-plugin.jar` | `2e646602c19d2add009f9333425b8cba05f2756d2c249936d6e43870dd2090da` |
| `Contents/lib/app.jar` | (in bundle inventory, ~23.5 MB container) |

## Subsystem reports (architecture audit)

**Status:** Subsystems **1–4 complete** (static pass, build **DB-261.24374.56**). **Phase 2 dynamic validation** — [11-dynamic-validation.md](./11-dynamic-validation.md) (2a) + [12-dynamic-validation-2b.md](./12-dynamic-validation-2b.md) (2b) + [13-ui-validation.md](./13-ui-validation.md) (2c UI runbook; no secrets). UI/session bytecode is in `plugins/DatabaseTools/lib/modules/intellij.database.{impl,core.impl,connectivity}.jar`. Cross-ref: `tmp/jar-index.txt` (`Contents/lib/` only) and **`tmp/jar-index-database-modules.txt`** (51 DatabaseTools module JARs → class/path lookup). Helper: `scripts/validate-dynamic.sh` (`ui-logs` / `ui-runbook`).

| Report | Topic |
|--------|--------|
| [16-datagrip-driver-url-templates.md](./16-datagrip-driver-url-templates.md) | **Driver URLs** — extracted DG `*-drivers.xml` → Grip native vendor map |
| [15-native-drivers.md](./15-native-drivers.md) | **Grip recreation** — pure-Rust drivers (PG/MySQL/Maria/SQLite/CH/Cassandra/Mongo/Redis/Redshift) |
| [14-datagrip-reference-guide.md](./14-datagrip-reference-guide.md) | **Product guide** — DataGrip workflow / windows / menus map for Grip recreation |
| [13-ui-validation.md](./13-ui-validation.md) | **Phase 2c** — UI validation runbook (Refresh / Select In / SqlResolve click paths + log scrapers) |
| [12-dynamic-validation-2b.md](./12-dynamic-validation-2b.md) | **Phase 2b** — resolve→`RunIntrospection` bytecode, sync procedure, PasswordSafe service-name, UI/Frida recipes |
| [11-dynamic-validation.md](./11-dynamic-validation.md) | **Phase 2a** — RMI/RemoteJdbcServer log+`lsof` proof, JVM/log breakpoints, REA/Frida notes |
| [10-architecture-synthesis.md](./10-architecture-synthesis.md) | **Cross-subsystem** — launch → connect → introspect → UI (mermaid + index) |
| [07-core-architecture-lifecycle.md](./07-core-architecture-lifecycle.md) | Subsystem 1 — entry, bootstrap JARs, plugin registry, `ApplicationLoader` / `ComponentManager` |
| [02-jdbc-connections.md](./02-jdbc-connections.md) | Subsystem 2 — `JdbcClassLoader` / child JVM drivers, URL editors, `PasswordSafe` + Keychain |
| [08-query-parsing-introspection.md](./08-query-parsing-introspection.md) | Subsystem 3 — Grammar-Kit `SqlLexer`/`SqlParser`/`*GeneratedParser`, dialect EPs, native `*IntroQueries` + JDBC `DatabaseMetaDataWrapper`, `DatabaseIntrospectionSession` |
| [09-ui-component-mapping.md](./09-ui-component-mapping.md) | Subsystem 4 — `DatabaseToolWindowFactory`, `DatabaseViewPanel`, `JdbcConsole` ↔ `DatabaseSession`, Services view, action `DataKey`s |

See sibling reports (`01`–`06`) for bundle/JAR inventory detail.

**Rust:** all reports here embedded in workspace crate [`grip`](../grip/) — `grip::datagrip::reports()` / `report_by_id` (see repo [README](../README.md)).
