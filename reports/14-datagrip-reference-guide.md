# DataGrip: Complete Reference Guide

A detailed walkthrough of how DataGrip works: the workflow, windows, toolbars, right-click menus, features and settings. Menu names and some option labels shift slightly between versions, so treat this as a map and expect small differences in your build.

**Role in Grip:** product map for the recreation shell (`datadrift-ui`). Companion to architecture reports `07`–`13`.

---

## 1. What DataGrip Is

DataGrip is JetBrains' standalone database IDE. It is built on the same IntelliJ platform as IntelliJ IDEA, PyCharm and WebStorm, which is why its look, keymaps, search behavior, settings and plugin system feel familiar if you have used any of them. It connects to relational databases (PostgreSQL, MySQL, MariaDB, SQL Server, Oracle, SQLite, DB2, Redshift, Snowflake, BigQuery, ClickHouse, Cassandra, H2, HSQLDB, Sybase, Vertica, Exasol, Greenplum, Azure and others) and also to some non-relational stores such as MongoDB.

Its core idea is that the IDE introspects your database (reads its metadata: schemas, tables, columns, keys, indexes, routines, views, sequences, triggers, types) and keeps a local model of it. Almost every smart feature (completion, error detection, navigation, refactoring, diagrams, DDL generation) works off that local model instead of asking the server each time.

## 2. The Overall Workflow

A typical session in DataGrip flows like this:

1. **Create a data source.** Host, port, user, password, database, JDBC driver (download if missing).
2. **Test the connection.** Credentials and network.
3. **Introspection.** Metadata for selected schemas → Database tool window.
4. **Open a console or SQL file.** Scratch console attached to DS, or project `.sql` mapped to DS.
5. **Write code with assistance.** Completion, inspections, formatting, parameter hints.
6. **Run.** Ctrl+Enter runs statement under caret. Results in Services → Output / Result tabs.
7. **Inspect and edit results.** Grid is also an editor: edit cells, add rows, filter, sort, export, submit.
8. **Commit or roll back** if manual transaction mode.
9. **Manage structure.** Table editor, diagrams, DDL, schema compare, refactorings.
10. **Move data.** Import/export, dump/restore.
11. **Version your work.** SQL in project/Git; Local History without VCS.

## 3. The Main Window Layout

### 3.1 Title bar and main menu

File, Edit, View, Navigate, Code, Refactor, Run, Tools, VCS/Git, Window, Help. Project name; VCS/run widgets in newer UI.

### 3.2 Navigation bar

Optional breadcrumb of current file in project (View → Appearance).

### 3.3 Left tool windows

- **Database** (often Alt+1): explorer tree — heart of the app.
- **Project / Files**: `.sql`, console backing files, scratches.
- **Structure**: outline of current file (statements, CTEs, procedures).
- Bookmarks, Commit, Pull Requests (when Git plugins set up).

### 3.4 Editor area

Center tabs: consoles, SQL files, table editors, diagrams, diffs, data editor.

### 3.5 Bottom tool windows

- **Services** (often Alt+8): sessions, result tabs, output.
- Output, Terminal, Problems, Event Log, TODO, VCS, Database Changes.

### 3.6 Right tool windows

Database Changes, AI Assistant, Notifications, etc. (setup-dependent).

### 3.7 Status bar

Line:column, encoding, indent, current schema/DS for file, Git branch, memory, introspection progress.

## 4. Data Sources and Connections

### 4.1 What a data source is

Saved connection config — project (`.idea/dataSources.xml`) or global. Credentials in IDE credential store / keychain, not project files.

### 4.2 Creating a data source

Database tool window → **+** → Data Source / Driver / from URL / from Path / DDL Data Source / Import / Driver.

### 4.3 Data Sources and Drivers dialog

Ctrl+Alt+Shift+S (approx). Left: Project / Global / DDL / Drivers. Right tabs:

**General:** Name, connection type/read-only, Host, Port, User, Password, Database, URL, auth modes, Save password policy, Driver, Test Connection.

**Options:** keep-alive, auto-disconnect, auto sync, startup script, read-only, reuse sessions, auto-commit, transaction control, JDBC metadata vs native introspect, timeouts, single session mode, Advanced driver props.

**SSH/SSL:** SSH tunnel configs; SSL CA/client cert/key/modes.

**Schemas:** tick schemas to introspect; All / Current / search / refresh from server.

**Advanced:** VM options, JDBC properties, timezone, sslmode, application name.

**Drivers:** jars, Class, URL templates, custom drivers.

### 4.4 Connecting and disconnecting

Expand/double-click to connect. Disconnect toolbar/menu. Green active indicator. Multiple sessions per DS unless Single session mode.

### 4.5 Introspection and synchronization

Synchronize / Refresh (Ctrl+F5). Background. Levels vary by DB. Stale tree → refresh first. JDBC metadata or native catalog queries.

## 5. The Database Tool Window

### 5.1 The tree

Data source → database/catalog → schema → Tables / Views / Materialized Views / Sequences / Routines / Packages / Triggers / Types / Extensions / … → under table: Columns, Keys, FKs, Indexes, Triggers, Checks. Server Objects, Roles, Tablespaces. Icons for PK/FK/nullability/state.

### 5.2 Toolbar

New (+), Synchronize, Schemas chooser, Data Source Properties, Jump to Query Console, Table Editor (F4), Disconnect, Stop, Group/Collapse, Scroll from/to Source, Filter, Settings, speed-search typing.

### 5.3–5.7 Context menus

**Data source:** New, console, Run SQL Script, dump/restore, diagrams, refactor, Properties, Disconnect, Remove, colors/groups, Find Usages.

**Schema:** New objects, console with schema, set current schema, export/DDL/diagrams, Drop, Compare, Refresh.

**Table:** New column/index/key, Open data editor (double-click), F4 structure, Generate DDL/SQL templates, Import/Export, Copy table, diagrams, Compare, Truncate/Drop, Quick Documentation.

**Column:** Modify, Rename, Drop, Find Usages, FK diagrams.

**Views/routines/sequences/triggers:** Jump to Source, Run/Debug (limited), Refresh materialized views.

## 6. Consoles, SQL Files and Scratches

### 6.1 Query console

Editor tied to one DS (and session). Saved under Consoles, not project. Many per DS. Ctrl+Shift+F10 / New → Query Console.

### 6.2 SQL files

Project `.sql` under VCS. Attach Data Source + schema. Dialect per file/project.

### 6.3 Scratches

Ctrl+Shift+Alt+Insert — outside project.

### 6.4 Editor context widgets

Data source selector, Schema selector, Session selector, Transaction mode (Auto/Manual) + Commit/Rollback, Playground.

## 7. Running SQL

### 7.1 Execution

- **Execute (Ctrl+Enter):** statement under caret or selection; popup if ambiguous.
- Execute to File for huge sets.
- Run whole file (Ctrl+Shift+F10 context).
- Cancel (Ctrl+F2).
- Explain Plan / Analyze.

### 7.2 Parameters

`?`, `:name`, `$1`, `${var}` — prompt dialog; patterns configurable.

### 7.3 Transactions

Auto vs Manual; Commit/Rollback; isolation/read-only; grid edits follow mode.

### 7.4 Delimiters

`;`, `GO`, `/`, MySQL `DELIMITER`; blank-line splitting optional. Gutter run icons.

### 7.5 Gutter

Run triangles, line numbers, fold, VCS, breakpoints, bookmarks.

### 7.6 Query History

Local History; per-console/DS execution history (Ctrl+Alt+Shift+H approx); Services keeps result tabs.

## 8. Services Tool Window and Result Tabs

Left tree: data sources → consoles/files → runs. Right: content.

Toolbar: Rerun, Stop, Pin/Close tabs, Commit/Rollback, Jump to Source/Console, Disconnect, Group, Filter, Settings.

Content tabs:

- **Output:** statements, timing, row counts, warnings, errors (clickable line links).
- **Result 1…N:** grids per SELECT; pin/rename.
- Plan/Statistics for explain.

---

## Grip implementation notes

| Guide area | Grip status (target) |
|------------|----------------------|
| DS General + Test | Shell dialog + native drivers |
| Introspect tree | Explorer schemas/tables |
| Console + Ctrl+Enter | Console tab + statement execute |
| Editor DS/schema/Tx widgets | Editor header bar |
| Services Output / Result | Bottom Services split |
| Table data editor | Table tabs + WHERE/ORDER + in-place edit |
| Grid edit / insert / delete / Submit | `grid.rs` — F2/double-click, +Row/−Row, Submit DML |
| Column resize / reorder / sort | Header drag edge, drag reorder, click sort |
| Cell / table context menus | Filter WHERE, Copy, SQL Scripts, DDL stub |
| Drag-and-drop table → console | Explorer hold-drag → console drop zone |
| Full dump / diagrams / schema compare | Not yet |

*Source: user-supplied DataGrip product walkthrough, adapted for Grip docs 2026-10-15.*
