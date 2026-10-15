# Phase 2c — UI dynamic validation runbook (DataGrip DB-261.24374.56)

**Date:** 2026-10-15  
**Prior:** [11-dynamic-validation.md](./11-dynamic-validation.md), [12-dynamic-validation-2b.md](./12-dynamic-validation-2b.md)  
**Constraint:** no secrets, no Keychain password reads, no Frida attach to live IDE.

This report closes the **documented-only** UI items from Phase 2b with: (1) live log scrape status, (2) jar-index action surface, (3) precise click paths the user must trigger, (4) script helpers. Interactive Refresh / Select In / Alt-Enter still need a human in the DataGrip window.

---

## Executive status

| # | Item (from 11/12) | Automated this pass | Still manual |
|---|-------------------|---------------------|--------------|
| 1 | DatabaseView selection / sync | jar-index + historical `Last Action: DatabaseView.PropertiesAction`; `SelectInDatabaseView` / `RefreshModelAction` present | Editor→tree Select In; post-Refresh `refreshViewNicely` |
| 2 | Refresh schema sync + debug logs | log scrape: **no** new `IntrospectionQueryContext` / `DataSourceSync` after 03:35; on-disk `storage_v2` proves prior sync | User Refresh with debug categories; re-run `ui-logs` |
| 3 | SqlResolve Alt-Enter path | bytecode chain already in [12](./12-dynamic-validation-2b.md); classes in jar-index | Open console → unknown table → Alt-Enter → Refresh/Introspect schema |

**Live session (2026-10-15 ~03:53):** `datagrip` **26456** + `RemoteJdbcServer` MySQL **27372** / ClickHouse **27377** still up; `idea.log` mtime **03:45:08**. No fresh Refresh / SqlResolve fix applied between 2b and this write-up. Project `hakki-backend` has `storage_v2` schema metas (prior sync).

---

## 1. Evidence captured without GUI clicks

### 1.1 Log markers (idea.log)

Path: `~/Library/Logs/JetBrains/DataGrip2026.1/idea.log`

| Pattern | Latest hit | Notes |
|---------|------------|-------|
| `IntrospectionQueryContext` | 2026-09-27 | Model tx reset / deferred tasks — not today's Refresh |
| `Last Action: DatabaseView.PropertiesAction` | 2026-09-27 | Proves DatabaseView action system fires into log on SEVERE paths |
| `DatabaseView took … ms` (ComponentStore save) | 2026-10-06 | Tool window persistence for project `hakki-backend` |
| `DataSourceSync` / `RunIntrospection` / `performManualIntrospection` | **Absent** at INFO | Need debug categories + user Refresh (§2) |
| JDBC connect / RemoteJdbcServer | 2026-10-15 03:24–03:35 | Already validated in [11](./11-dynamic-validation.md) / [12](./12-dynamic-validation-2b.md) |

Helper:

```bash
./scripts/validate-dynamic.sh ui-logs
```

### 1.2 Jar-index action / class surface

From `tmp/jar-index-database-modules.txt` (executed):

| Class / artifact | JAR |
|------------------|-----|
| `com.intellij.database.actions.RefreshModelAction` | `intellij.database.impl.jar` |
| `com.intellij.database.actions.SelectInDatabaseViewAction` | `intellij.database.impl.jar` |
| `com.intellij.database.view.SelectInDatabaseView` | `intellij.database.impl.jar` |
| `com.intellij.sql.inspections.SqlResolveInspection` (+ `$RefreshSchemaFix`, `$IntrospectSchemaFix`) | `intellij.database.sql.impl.jar` |

```bash
./scripts/validate-dynamic.sh index 'RefreshModelAction|SelectInDatabaseView|SqlResolveInspection'
```

### 1.3 On-disk schema cache (prior sync)

Projects that already connected show introspection material under:

```text
{project}/.idea/dataSources/<uuid>/storage_v2/
```

Presence of `storage_v2` / `*.meta` / schema zips = prior successful sync — **not** a substitute for a traced Refresh this session.

---

## 2. Click path A — Refresh schema (sync + debug)

Goal: produce `DataSourceSync` / `RunIntrospection` / `IntrospectionQueryContext` lines in `idea.log`.

### 2.1 One-time setup

1. **Help → Edit Custom VM Options** (user file under `~/Library/Application Support/JetBrains/DataGrip2026.1/datagrip.vmoptions`).
2. Append (single long `-Didea.log.debug.categories=…` line is OK):

```text
-Didea.log.debug.categories=#com.intellij.database.introspection:debug,#com.intellij.database.dataSource.DataSourceSyncManager:debug,#com.intellij.database.dialects.base.introspector:debug,#com.intellij.database.view:debug,#com.intellij.database.util.DataSourceUtilKt:debug
```

3. Restart DataGrip.

### 2.2 Trigger

1. Open **Database** tool window.
2. Select a live data source (MySQL / ClickHouse already used this session is fine).
3. Toolbar **Refresh** (action id `DatabaseView.Refresh` / class `RefreshModelAction`) — or context menu **Refresh**.
4. Wait until progress finishes (tree may flash / expand).

### 2.3 Verify

```bash
./scripts/validate-dynamic.sh ui-logs
# or:
rg -i 'DataSourceSync|RunIntrospection|performManualIntrospection|IntrospectionQueryContext|SyncIntrospection|RefreshModel' \
  ~/Library/Logs/JetBrains/DataGrip2026.1/idea.log | tail -50
```

**Pass criteria:** at least one post-restart line mentioning sync/introspection for the DS; tree shows schemas without IDE restart.

Optional debugger breakpoints (same as [12](./12-dynamic-validation-2b.md) §1.3 / §2.4):

- `DataSourceSyncManager.tryPerformSync`
- `RunIntrospection.runAsyncIntrospection`
- `DatabaseView.refreshViewNicely`

---

## 3. Click path B — DatabaseView selection / sync with editor

Goal: prove editor ↔ tree selection wiring (`SelectInDatabaseView`).

1. Open SQL console bound to a DS that has tables in the tree.
2. Place caret on a **resolved** table identifier in SQL.
3. **Navigate → Select In → Database** (or action `DatabaseView.ScrollFromEditor` / `SelectInDatabaseViewAction`).
4. Expect Database tool window focuses matching node (`DatabaseView.select*` / `selectElement`).
5. Reverse: select table in tree → open data / console; confirm no freeze (loader off EDT — sample if needed).

**Log:** often quiet at INFO. Prefer debugger on `SelectInDatabaseView.selectIn` / `DatabaseView.selectNode`. After Refresh (§2), confirm new nodes appear via `refreshViewNicely` without restart.

Historical soft signal: `Last Action: DatabaseView.PropertiesAction` (2026-09-27) shows DatabaseView actions are registered and recorded as last-action on errors.

---

## 4. Click path C — SqlResolve Alt-Enter → RunIntrospection

Goal: interactive proof of bytecode chain already documented in [12](./12-dynamic-validation-2b.md) §2.

1. Prefer incomplete introspection scope: DS Properties → Schemas → uncheck some schemas, Apply; **or** use a table name not yet in cache.
2. SQL console: `SELECT * FROM totally_missing_table_xyz;`
3. Wait for `SqlResolveInspection` highlight (unresolved reference).
4. **Alt-Enter** (⌥⏎) → **Refresh schema…** (`RefreshSchemaFix`) or **Introspect schema…** (`IntrospectSchemaFix`).
5. Accept fix; watch progress.
6. Grep log (same as §2.3). Breakpoints: `RefreshSchemaFix.applyFix`, `RunIntrospection.runAsyncIntrospection`.

**FQCN:** `com.intellij.sql.inspections.SqlResolveInspection` (not `com.intellij.database.psi.*`).

---

## 5. What we intentionally did not do

| Action | Why |
|--------|-----|
| Frida / `observe_native_calls` attach to live IDE | Pause risk; JDBC already proven via logs/`lsof` |
| Keychain / PasswordSafe secret reads | Out of scope (structure in [12](./12-dynamic-validation-2b.md) §5) |
| GUI automation / accessibility drive of DataGrip | No harness; click paths above instead |
| Entering new DB credentials | Use existing session only |

---

## 6. Grip tooling for this report

| Tool | Role |
|------|------|
| `cargo run -p grip-ui` → **Audit reports** tab | Browse embedded markdown (including this file once recompiled) |
| `./scripts/validate-dynamic.sh ui-logs` | Scrape UI/sync-related idea.log lines |
| `./scripts/validate-dynamic.sh ui-runbook` | Print short click-path checklist to stdout |
| `./scripts/validate-dynamic.sh index …` | Confirm action/inspection classes still in jar-index |

---

## 7. Related

- [09-ui-component-mapping.md](./09-ui-component-mapping.md) — DatabaseView static map  
- [11-dynamic-validation.md](./11-dynamic-validation.md) — Phase 2a JDBC/RMI  
- [12-dynamic-validation-2b.md](./12-dynamic-validation-2b.md) — bytecode + PasswordSafe  

---

*Phase 2c: UI validation evidence is log/index/on-disk + precise manual click paths. Re-run `ui-logs` after user Refresh / Alt-Enter to promote rows to **Executed**.*
