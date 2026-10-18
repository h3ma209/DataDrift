# Phase 2 — Dynamic / runtime validation (DataGrip DB-261.24374.56)

**Date:** 2026-10-15  
**Bundle:** `/Applications/DataGrip.app`  
**Prior static synthesis:** [10-architecture-synthesis.md](./10-architecture-synthesis.md)

This pass validates Subsystems **1–4** using **live process + log evidence** where possible, without DB credentials, Keychain reads, or new connections initiated by the auditor.

---

## Executive summary

| Hypothesis (static) | Runtime result |
|---------------------|----------------|
| IDE boot → `PathClassLoader`, `AppStarter`, DatabaseTools plugin | **Confirmed** in `idea.log` (2026-10-15 session): full JVM option line includes `-Djava.system.class.loader=com.intellij.util.lang.PathClassLoader`, `-Didea.platform.prefix=DataGrip`, bundled plugin list includes **Database Tools and SQL** |
| User JDBC drivers run in **child JVM** (`RemoteJdbcServer`) | **Confirmed**: two `java … RemoteJdbcServer` processes (MySQL, ClickHouse); `RemoteProcessSupport` logged exact command lines |
| RMI stub on IDE side, registry + service ports on child | **Confirmed**: log lines `Port/ServicesPort/ID` match `lsof` LISTEN on `127.0.0.1` |
| `DatabaseConnectionEstablisher` logs URL + auth **mode** (not password) | **Confirmed**: `Connecting to: jdbc:mysql://localhost:3306`, `Auth provider: user-pass` |
| Driver JARs under config `jdbc-drivers/` + shim | **Confirmed**: `Using classpath: […/jdbc-drivers/…, intellij.database.jdbcConsole.shim.jar]` |
| Introspection touches `IntrospectionQueryContext` | **Partial**: historical log lines for model tx reset / deferred tasks; no fresh sync captured this pass |
| `PasswordSafe` never logs cleartext secrets | **Confirmed** (absence): connect logs show auth provider only |

---

## What was executed (this session)

| Action | Tool | Outcome |
|--------|------|---------|
| REA MCP auth | `user-rea` / `mcp_auth` | Succeeded (server was in error state before auth) |
| Frida device list | `list_frida_devices` | `local`, `socket`, `barebone` available |
| Frida process list | `list_frida_processes` (`device_id=local`) | Succeeded (full process catalog; no attach/instrument) |
| Live PIDs | `pgrep` / `ps` / `lsof` | Main `datagrip` **26456**; child JDBC **27372** (MySQL), **27377** (ClickHouse) |
| Log scrape | `rg` on `~/Library/Logs/JetBrains/DataGrip2026.1/idea.log` | JDBC/RMI/connect/introspection markers (see §4) |
| Bytecode sanity | `javap` | `RemoteJdbcServer` extends `RemoteJdbcServerBase` → `com.intellij.execution.rmi.RemoteServer` |
| Helper script | [../scripts/validate-dynamic.sh](../scripts/validate-dynamic.sh) | `all` subcommand run successfully |

**Not executed (by design or blocker):**

- `observe_native_calls` / `observe_native_ui` — would spawn or require window/PID binding; JDBC validation already observable via logs + `lsof`
- Fresh **Test Connection** / full **schema sync** — would need user DB reachability; relied on **existing** session log entries (2026-10-15 03:24–03:35)
- `SqlResolveInspection` → introspection trigger — Phase 2a: procedure only; **2b bytecode proof** in [12](./12-dynamic-validation-2b.md) (`com.intellij.sql.inspections.SqlResolveInspection`)
- Keychain / `PasswordSafe.get` inspection — no secret access
- GUI / integration-test harness for `DatabaseView` — documented only

---

## 1. Subsystem 1 — lifecycle (runtime hooks)

### 1.1 Log file & config paths

| Item | Path |
|------|------|
| Main log | `~/Library/Logs/JetBrains/DataGrip2026.1/idea.log` |
| Path selector | `-Didea.paths.selector=DataGrip2026.1` (logged at startup) |
| Bundled VM options | `/Applications/DataGrip.app/Contents/bin/datagrip.vmoptions` |
| User VM options (override) | `~/Library/Application Support/JetBrains/DataGrip2026.1/datagrip.vmoptions` (create via **Help → Edit Custom VM Options**) |

### 1.2 JVM flags worth adding for validation

Append to **user** `datagrip.vmoptions` (restart IDE):

```text
-Didea.log.debug.categories=#com.intellij.platform.ide.bootstrap.ApplicationLoader:debug,#com.intellij.ide.plugins.PluginManagerCore:debug,#com.intellij.execution.rmi.RemoteProcessSupport:debug,#com.intellij.database.dataSource.DatabaseConnectionEstablisher:debug,#com.intellij.database.introspection:debug,#com.intellij.database.dialects.base.introspector:debug
-Dsun.rmi.transport.logLevel=VERBOSE
-Dsun.rmi.client.logLevel=VERBOSE
-Dsun.rmi.server.logLevel=VERBOSE
```

Notes:

- Log abbreviations in `idea.log` map to FQCNs (e.g. `#c.i.e.r.RemoteProcessSupport` → `com.intellij.execution.rmi.RemoteProcessSupport`).
- `-ea` is already enabled in bundled options (useful for `assert` in platform code).
- `-Djdk.attach.allowAttachSelf=true` is already set — enables JDI attach to same JVM (see §6).

### 1.3 Breakpoints / span names (IDE debugger)

| Class | JAR (see jar-index) | When to hit |
|-------|---------------------|-------------|
| `com.intellij.idea.Main` | `app.jar` | Process entry |
| `com.intellij.platform.ide.bootstrap.StartupUtil` | `intellij.platform.lang.impl.jar` | Parallel boot |
| `com.intellij.platform.ide.bootstrap.ApplicationLoader` | `intellij.platform.lang.impl.jar` | `loadApp`, service registration |
| `com.intellij.ide.plugins.PluginManagerCore` | `intellij.platform.core.impl.jar` | Plugin set ready |
| `com.intellij.ide.plugins.PluginDescriptorLoader` | `intellij.platform.core.impl.jar` | Descriptor load |

**Expected log (no debug flags):** `#c.i.p.i.b.AppStarter - Loaded bundled plugins: … Database Tools and SQL …`

---

## 2. Subsystem 2 — JDBC / remote driver JVM

### 2.1 Static → runtime mapping

Child process main class (observed):

```text
com.intellij.database.remote.RemoteJdbcServer <driver.class.FQN>
```

Example (from log + `ps`, 2026-10-15):

```text
…/bin/java -Djava.rmi.server.hostname=127.0.0.1 -Duser.timezone=UTC -Xmx750m -Xms256m …
  -classpath …/util_rt.jar:…/intellij.database.jdbcConsole.jar:…
  com.intellij.database.remote.RemoteJdbcServer com.mysql.cj.jdbc.Driver
```

### 2.2 RMI ports (registry vs service)

`RemoteProcessSupport` logs:

```text
Port/ServicesPort/ID: <registryPort>/<servicePort>/<exportId>
Started remote process on: 127.0.0.1:<registryPort> with service port <servicePort> and id = RemoteDriverFactoryImpl…
```

**Observed (MySQL child PID 27372):**

| Role | Port |
|------|------|
| RMI registry (typical) | `127.0.0.1:31481` |
| Exported service | `127.0.0.1:52870` |

**Observed (ClickHouse child PID 27377):** registry `18907`, service `52886` (from log at spawn; ports differ per process).

Verify live:

```bash
/Users/hema/Desktop/tmp/scripts/validate-dynamic.sh jdbc-children
/Users/hema/Desktop/tmp/scripts/validate-dynamic.sh rmi-ports
```

### 2.3 IDE-side breakpoint classes (connect pipeline)

| Class | Module JAR |
|-------|----------------|
| `com.intellij.database.dataSource.DatabaseConnectionManager` | `intellij.database.connectivity.jar` |
| `com.intellij.database.dataSource.DatabaseConnectionEstablisher` | `intellij.database.connectivity.jar` |
| `com.intellij.database.console.JdbcDriverManagerImpl` | `intellij.database.connectivity.jar` |
| `com.intellij.execution.rmi.RemoteProcessSupport` | `util_rt.jar` |
| `com.intellij.database.remote.jdbc.impl.RemoteDriverImpl` | `intellij.database.jdbcConsole.jar` (child-side stack traces appear in IDE log via stderr forwarding) |

### 2.4 Environment variables

| Variable | Effect |
|----------|--------|
| `IDEA_PATHS_SELECTOR` | Log/config dir suffix (default `DataGrip2026.1`) |
| `JAVA_HOME` / `JDK_HOME` | Override JBR for IDE (launcher resolution order in [02-native-launcher.md](./02-native-launcher.md)) |
| `datagrip.driver.path` | `-Ddatagrip.driver.path=…` → alternate JDBC driver download root ([02-jdbc-connections.md](./02-jdbc-connections.md)) |

Child JVM inherits **`ConsoleRunConfiguration`** VM params from IDE; extra `-D` for driver debugging are usually set on the **data source driver** “VM options” in UI or driver XML — not required for architecture proof.

### 2.5 Credential boundary check (no secrets)

Procedure:

1. Enable `#com.intellij.database.access:debug` (optional) and `#com.intellij.credentialStore:debug` (verbose — avoid sharing logs publicly).
2. Connect to a datasource; confirm `DatabaseConnectionEstablisher` logs **URL** and **auth provider** only.
3. Inspect `{config}/options/dataSources.xml` — usernames may appear; **password fields must be absent** (PasswordSafe / Keychain).

---

## 3. Subsystem 3 — introspection & SQL resolve

### 3.1 Runtime log categories

| Abbrev in log | FQCN |
|---------------|------|
| `#c.i.d.d.b.i.IntrospectionQueryContext` | `com.intellij.database.dialects.base.introspector.IntrospectionQueryContext` |

Historical lines (same `idea.log`):

- `Resetting the model tx ids`
- `There are N deferred tasks but none of them can be processed with the current connection`

### 3.2 Procedure — confirm introspection on sync

1. Debug flags: `#com.intellij.database.introspection:debug`, `#com.intellij.database.dataSource.DataSourceSyncManager:debug`.
2. In Database tool window: **Refresh** on a connected datasource (user must have working connection).
3. Breakpoints:
   - `com.intellij.database.introspection.DatabaseIntrospectionSession.run`
   - `com.intellij.database.introspection.DBIntrospector.introspect`
   - Dialect impl e.g. `com.intellij.database.dialects.postgresbase.introspector.PgBaseIntrospector` (adjust for DBMS)
4. Expect topic publish: `DatabaseModelLoader` started/finished (grep log for `DatabaseModelLoader` if debug enabled).

### 3.3 Procedure — SQL resolve → introspection (synthesis item #2)

1. Open SQL console bound to a datasource with incomplete schema cache.
2. Reference an unknown table/column to trigger resolve inspection.
3. Breakpoint / watch: `com.intellij.sql.inspections.SqlResolveInspection` (corrected in [12](./12-dynamic-validation-2b.md); `validate-dynamic.sh index SqlResolve`).
4. Confirm downstream call into `DataSourceSyncManager` / `RunIntrospection` task (static target from [08-query-parsing-introspection.md](./08-query-parsing-introspection.md)).

**Status this pass:** not triggered interactively.

---

## 4. Subsystem 4 — UI bind (minimal runtime)

Without UI automation:

1. `#com.intellij.database.view:debug` (if available) or breakpoint on `com.intellij.database.view.DatabaseView` refresh handlers.
2. Manual: expand tree node → verify concurrent model read (`DbModelRegistry`) — CPU/thread dump shows `DatabaseViewPanel` / `AsyncTreeModel` on EDT vs background loaders.

Full `DatabaseView.select(PsiElement)` harness: **documented only** (platform integration tests in JetBrains repo, not shipped in product bundle).

---

## 5. Sample log evidence (executed read-only scrape)

Source: `~/Library/Logs/JetBrains/DataGrip2026.1/idea.log` (2026-10-15)

```text
INFO - #c.i.e.r.RemoteProcessSupport - … com.intellij.database.remote.RemoteJdbcServer com.mysql.cj.jdbc.Driver
INFO - #c.i.e.r.RemoteProcessSupport - Port/ServicesPort/ID: 31481/52870/RemoteDriverFactoryImpl68c1d4ab
INFO - #c.i.e.r.RemoteProcessSupport - Using classpath: […/jdbc-drivers/MySQL ConnectorJ/…/mysql-connector-j-9.7.0.jar, …/intellij.database.jdbcConsole.shim.jar]
INFO - #c.i.d.d.DatabaseConnectionEstablisher - Connecting to: jdbc:mysql://localhost:3306
INFO - #c.i.d.d.DatabaseConnectionEstablisher - Auth provider: user-pass
```

Child stderr forwarded through `RemoteProcessSupport` (RMI stack + driver warnings) — confirms **`RemoteDriverImpl.connect`** executes in child, dispatched via `UnicastServerRef`.

---

## 6. JDI attach (optional deep dive)

Main IDE built-in debug port (observed on running instance): `127.0.0.1:63342` (`lsof` on `datagrip` PID). Use **Run → Attach to Process** or:

```bash
jcmd <datagrip-pid> VM.flags
jstack <datagrip-pid> | rg 'RemoteProcessSupport|DatabaseConnection'
```

Child JVMs: attach with `jstack` on `RemoteJdbcServer` PID while connected; breakpoint `RemoteDriverFactoryImpl.getDriver()` requires agent attach (`-agentlib:jdwp` not present by default — add via driver VM options for child-only debugging).

---

## 7. REA MCP notes

| Tool | Result |
|------|--------|
| Initial discovery | Namespace `user-rea` in **error** until `mcp_auth` |
| After auth | `list_frida_devices`, `list_frida_processes` OK |
| `observe_native_calls` | Not run (LLDB-owned launch; low value vs JDBC log trail) |

For native launcher validation (Subsystem 1 native half), see [02-native-launcher.md](./02-native-launcher.md) + optional REA `observe_native_calls` on `Contents/MacOS/datagrip` with `--help` or dry run.

---

## 8. Helper script

[../scripts/validate-dynamic.sh](../scripts/validate-dynamic.sh):

```bash
./scripts/validate-dynamic.sh index RemoteJdbcServer
./scripts/validate-dynamic.sh logs
./scripts/validate-dynamic.sh all
```

Indexes: `tmp/jar-index-database-modules.txt`, `tmp/jar-index.txt`.

---

## 9. Related reports

- [12-dynamic-validation-2b.md](./12-dynamic-validation-2b.md) — **Phase 2b** continuation (resolve→introspect bytecode, PasswordSafe pattern, UI/Frida recipes)
- [10-architecture-synthesis.md](./10-architecture-synthesis.md) — suggested checks origin
- [07-core-architecture-lifecycle.md](./07-core-architecture-lifecycle.md) — boot classes
- [02-jdbc-connections.md](./02-jdbc-connections.md) — connect pseudocode
- [08-query-parsing-introspection.md](./08-query-parsing-introspection.md) — introspection session
- [09-ui-component-mapping.md](./09-ui-component-mapping.md) — UI services

---

## Phase 2b pointer

Continuation of documented-only items: **[12-dynamic-validation-2b.md](./12-dynamic-validation-2b.md)**.

---

*Phase 2a complete — hybrid executed + documented procedures; no user secrets collected. See 12 for Phase 2b.*
