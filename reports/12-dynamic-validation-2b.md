# Phase 2b — Remaining documented-only items (DataGrip DB-261.24374.56)

**Date:** 2026-10-15  
**Prior:** [11-dynamic-validation.md](./11-dynamic-validation.md) (Phase 2a), [10-architecture-synthesis.md](./10-architecture-synthesis.md)  
**Constraint:** no secrets, no Keychain password reads, no new DB credentials entered by auditor.

---

## Executed vs documented-only

| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | Fresh connect + full schema sync / introspection trace | **Partial executed** | Live IDE + `RemoteJdbcServer` still up; Oct-15 connect logs; schema `storage_v2` on disk. **No** new `IntrospectionQueryContext` / `DataSourceSync` lines after 03:35 (debug categories off; no Refresh triggered this pass). |
| 2 | `SqlResolveInspection` → `RunIntrospection` evidence path | **Executed (bytecode)** | `javap` call chain + jar-index; Vineflower run (JBR) for jars. Interactive trigger still **documented**. |
| 3 | `DatabaseView` UI selection/sync validation procedure | **Documented** (+ API surface from `javap`) | No GUI harness this pass. |
| 4 | REA `observe_native_calls` / Frida attach | **Documented recipe only** | Attach would risk running session; JDBC already proven via logs/`lsof`. |
| 5 | PasswordSafe / Keychain locations + service-name pattern | **Executed (structure only)** | Bytecode + XML **attribute/element inventory** (no secret values). |

Helper: `scripts/validate-dynamic.sh all` (needs unsandboxed process list — sandbox `pgrep` fails).

---

## 1. Live process + connect evidence (item 1)

### 1.1 Processes (2026-10-15 ~03:42, `all` perms)

| PID | Role |
|-----|------|
| **26456** | `/Applications/DataGrip.app/Contents/MacOS/datagrip` |
| **27372** | `RemoteJdbcServer com.mysql.cj.jdbc.Driver` — LISTEN `127.0.0.1:31481` (registry), `127.0.0.1:52870` (service) |
| **27377** | `RemoteJdbcServer com.clickhouse.jdbc.ClickHouseDriver` — `127.0.0.1:18907` / `127.0.0.1:52886` |

Matches Phase 2a RMI port table in [11](./11-dynamic-validation.md).

### 1.2 Connect log (existing session — no password)

From `~/Library/Logs/JetBrains/DataGrip2026.1/idea.log`:

```text
2026-10-15 03:24:48 … DatabaseConnectionEstablisher - Connecting to: jdbc:mysql://localhost:3306
2026-10-15 03:24:48 … DatabaseConnectionEstablisher - Auth provider: user-pass
2026-10-15 03:24:48 … RemoteProcessSupport - Detected: MYSQL 8.4.3
2026-10-15 03:25:07 … DatabaseConnectionEstablisher - Connecting to: jdbc:clickhouse://…
2026-10-15 03:25:07 … Auth provider: user-pass
2026-10-15 03:35:08 … Detected: CLICKHOUSE null
```

### 1.3 Schema sync / introspection — what was / was not seen

| Signal | Result |
|--------|--------|
| `#c.i.d.d.b.i.IntrospectionQueryContext` on 2026-10-15 | **Absent** (last hits 2026-09-27: model tx reset / deferred tasks) |
| `DataSourceSync` / `DatabaseModelLoader` / `RunIntrospection` in idea.log | **Absent** at INFO (need debug categories — see §2.3) |
| On-disk introspection model | **Present** under project `.idea/dataSources/<uuid>/storage_v2/_src_/schema/*.meta|*.zip` — proves prior successful sync for connected DS |

**Full sync trace procedure (user-triggered, no auditor secrets):**

1. Append to user `datagrip.vmoptions` (restart IDE):

```text
-Didea.log.debug.categories=#com.intellij.database.introspection:debug,#com.intellij.database.dataSource.DataSourceSyncManager:debug,#com.intellij.database.dialects.base.introspector:debug,#com.intellij.database.util.DataSourceUtilKt:debug
```

2. Database tool window → select DS → **Refresh** (`DatabaseView.Refresh` / `RefreshModelAction`).
3. Grep log:

```bash
rg -i 'DataSourceSync|RunIntrospection|performManualIntrospection|IntrospectionQueryContext|DatabaseModelLoader|SyncIntrospection' \
  ~/Library/Logs/JetBrains/DataGrip2026.1/idea.log | tail -50
```

4. Optional breakpoints: `DataSourceSyncManager.tryPerformSync`, `RunIntrospection.runAsyncIntrospection`, dialect `*Introspector`.

---

## 2. SqlResolveInspection → RunIntrospection (item 2)

### 2.1 Class locations (jar-index)

| Class | JAR |
|-------|-----|
| `com.intellij.sql.inspections.SqlResolveInspection` | `plugins/DatabaseTools/lib/modules/intellij.database.sql.impl.jar` |
| `…$RefreshSchemaFix` | same |
| `…$IntrospectSchemaFix` | same |
| `com.intellij.database.introspection.RunIntrospection` | `intellij.database.connectivity.jar` |
| `com.intellij.database.dataSource.DataSourceSyncManager` | `intellij.database.connectivity.jar` |
| `com.intellij.database.util.DataSourceUtilKt.performManualIntrospection` | `intellij.database.connectivity.jar` |

**FQCN correction vs Phase 2a guess:** inspection is `com.intellij.sql.inspections.SqlResolveInspection`, **not** `com.intellij.database.psi.SqlResolveInspection`.

### 2.2 Bytecode call chain (executed)

```text
SqlResolveInspection$RefreshSchemaFix.applyFix
  → IntrospectionTasks.prepareOneElementRefreshTask(uniqueId, BasicElement)
  → RunIntrospection.runAsyncIntrospection(project, LocalDataSource, IntrospectionTask)

SqlResolveInspection$IntrospectSchemaFix.applyFix
  → (popup or) setVisible(candidates)
  → DataSourceUtil.setPathsVisible(project, ds, paths, true)
  → DataSourceUiUtil.prepareScopeIntrospectionTask(uniqueId, oldScope, newScope)
  → RunIntrospection.runAsyncIntrospection(…)

RunIntrospection.runAsyncIntrospection(project, ds, tasks)
  → LoaderContext.selectTasks(project, ds, tasks)
  → AsyncTask { DataSourceUtilKt.performManualIntrospection(loaderContext) }

DataSourceUtilKt.performManualIntrospection
  → DataSourceSyncManager.getInstance().tryPerformSync(loaderContext, …)
```

Inner sync processor uses `DataSourceSyncManager$DbSyncProcessor$SyncIntrospectionSession` (extends introspection session — see jar-index nested classes).

### 2.3 Log categories to enable for runtime proof

| Category | Why |
|----------|-----|
| `#com.intellij.database.introspection` | `RunIntrospection` / session |
| `#com.intellij.database.dataSource.DataSourceSyncManager` | `tryPerform*` |
| `#com.intellij.database.dialects.base.introspector` | `IntrospectionQueryContext` |
| `#com.intellij.sql.inspections` (if present) | resolve inspection noise |

### 2.4 Exact interactive steps

1. Open SQL console bound to DS with **incomplete** schema cache (or uncheck schemas in introspection scope).
2. Type reference to unknown table → `SqlResolveInspection` highlight.
3. Alt-Enter → **Refresh schema…** (`RefreshSchemaFix`) or **Introspect schema…** (`IntrospectSchemaFix`).
4. Breakpoints: `RefreshSchemaFix.applyFix`, `RunIntrospection.runAsyncIntrospection`, `DataSourceSyncManager.tryPerformSync`.
5. Confirm log lines from §1.3 grep after fix applied.

Artifacts: `tmp/phase2b/*.javap.txt`, Vineflower under `tmp/phase2b/vf/` (JBR Java 25).

---

## 3. DatabaseView selection / sync procedure (item 3)

### 3.1 API surface (`javap` — executed)

`com.intellij.database.view.DatabaseView` (`intellij.database.impl.jar`):

| Method | Role |
|--------|------|
| `selectNode(BasicNode)` | Tree node focus |
| `selectElement(Object)` | Model element |
| `select(PsiElement, focus)` | Editor → tree |
| `select(project, collection, focus[, wait])` | Batch |
| `refreshViewNicely` / `refreshViewAndSelect` | Post-introspection UI |

`SelectInDatabaseView` (`selectIn` / `askProviders`) + action `DatabaseView.ScrollFromEditor` — see [09-ui-component-mapping.md](./09-ui-component-mapping.md) §5.

### 3.2 Manual validation procedure (documented)

1. Enable `#com.intellij.database.view:debug` if logger exists; else use debugger.
2. **Editor → tree:** place caret on table PSI in SQL console → **Navigate → Select In → Database** (or `ScrollFromEditor`). Expect `SelectInDatabaseView.selectIn` → `DatabaseView.select*`.
3. **Tree → model:** expand node after Refresh; thread dump / CPU sample should show `AsyncTreeModel` / loader off EDT, UI updates on EDT (`DatabaseViewPanel`).
4. **Post-sync:** after §1.3 Refresh, tree should pick up new nodes without IDE restart (`refreshViewNicely`).
5. Full `DatabaseView.select(PsiElement)` integration-test harness: **not in product bundle** (JetBrains platform tests only).

---

## 4. Frida / observe_native_calls (item 4) — attach recipe only

**Not attached this pass:** live session already validating JDBC via logs; `observe_native_calls` owns LLDB launch (would spawn new process); Frida attach to production IDE can pause JVM.

### 4.1 Safe recipe (when user OK with pause / new process)

```bash
# Device
# REA: list_frida_devices → device_id=local
# REA: list_frida_processes(device_id=local) → filter datagrip / RemoteJdbcServer

# Prefer child JVM (smaller blast radius) for JDBC:
# instrument_with_frida / Frida session on RemoteJdbcServer PID
# Hook (illustrative — write script yourself):
#   com.intellij.database.remote.jdbc.impl.RemoteDriverImpl.connect
#   java.sql.Driver.connect

# Native launcher only (Subsystem 1):
# open_binary Contents/MacOS/datagrip
# observe_native_calls on cold start with --help / dry args — NOT against already-running PID
```

JDI alternative (already in [11](./11-dynamic-validation.md) §6): `jstack` / attach debug port on IDE; driver child needs `-agentlib:jdwp` via driver VM options.

---

## 5. PasswordSafe / Keychain — locations + service-name (item 5)

**No secret material read.** Structure / bytecode only.

### 5.1 Storage locations (macOS DataGrip 2026.1)

| Store | Path / mechanism |
|-------|------------------|
| Config root | `~/Library/Application Support/JetBrains/DataGrip2026.1/` |
| DS metadata (no password) | Project `.idea/dataSources.xml` + `.idea/dataSources/<uuid>.xml` |
| Local DS extras | `.idea/dataSources.local.xml` — `<user-name>`, `<secret-storage>` mode |
| Schema cache | `.idea/dataSources/<uuid>/storage_v2/` |
| Drivers | `{config}/jdbc-drivers/`, `{config}/options/databaseDrivers.xml` |
| PasswordSafe facade | `com.intellij.ide.passwordSafe.PasswordSafe` → `PasswordSafeImpl` / `BasePasswordSafe` |
| macOS Keychain backend | `KeyChainCredentialStore` → `SecKeychainFindGenericPassword` / `SecKeychainAddGenericPassword` |
| File / KeePass-style backend | `AesEncryptionSupport` / KeePass DB under config when provider ≠ Keychain (optional `c.kdbx` pattern — see [02](./02-jdbc-connections.md) / [06](./06-jdbc-connection-contents-lib.md); **not opened this pass**) |

### 5.2 Observed project boundary (structure)

On open project’s `.idea/dataSources.local.xml`:

- Element `<secret-storage>` present on all DS; observed mode value **`master_key`** (PasswordSafe master-key / file provider — not cleartext).
- Element `<user-name>` present; **no** `<password>` element; file contains **no** `password=` attributes.
- Shared `dataSources.xml` holds jdbc-url / driver-ref / uuid — **no password fields**.

### 5.3 Service-name pattern (bytecode — executed)

`CredentialAttributesKt.SERVICE_NAME_PREFIX = "IntelliJ Platform"`

`generateServiceName(subsystem, key)` concat recipe:

```text
"IntelliJ Platform " + subsystem + " — " + key
```

(`StringConcatFactory` recipe `IntelliJ Platform \u0001 — \u0001`)

`DbCredentialManager.createCredentialAttributes` (`intellij.database.core.impl.jar`):

```text
subsystem = "DB"                                    // constant
key      = (suffix ?: "") + dataSource.uniqueId     // recipe \u0001\u0001
serviceName = generateServiceName(subsystem, key)
// => "IntelliJ Platform DB — " + optionalSuffix + uniqueId
```

**Note:** [02-jdbc-connections.md](./02-jdbc-connections.md) §3.3 described `subsystem = "DB" + suffix`. Phase 2b bytecode shows subsystem fixed `"DB"`; **suffix prepended to key** instead. Prefer this report for exact pattern.

Example (uuid only, empty suffix):

```text
IntelliJ Platform DB — <dataSource.uniqueId>
```

Keychain item (when Keychain provider active): generic password, **service** = that string; account often empty/null for DB password blobs (Credentials constructed with `userName=null` in `setPassword`).

**Do not** run `security dump-keychain` / `find-generic-password -g` for validation — metadata pattern above is enough.

---

## 6. Tooling notes

| Tool | Result |
|------|--------|
| `scripts/validate-dynamic.sh all` | OK with unsandboxed shell; sandbox blocks `pgrep` |
| `javap` | System `/usr/bin/javap` (JBR has no `javap`) |
| Vineflower `tmp/vineflower.jar` | Needs **JBR** `java` (classfile 61+); OpenJDK 11 fails |
| REA Frida list | Available; attach skipped |
| `observe_native_calls` | Skipped (launch ownership) |

---

## 7. Related

- [11-dynamic-validation.md](./11-dynamic-validation.md) — Phase 2a live JDBC/RMI  
- [08-query-parsing-introspection.md](./08-query-parsing-introspection.md) — sync session static  
- [09-ui-component-mapping.md](./09-ui-component-mapping.md) — DatabaseView  
- [02-jdbc-connections.md](./02-jdbc-connections.md) / [06-jdbc-connection-contents-lib.md](./06-jdbc-connection-contents-lib.md) — PasswordSafe  

---

*Phase 2b: hybrid bytecode + live process/log structure validation; interactive sync/resolve/UI still need user Refresh / Alt-Enter / Select In.*
