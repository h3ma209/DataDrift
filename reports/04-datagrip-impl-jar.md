# Plugin JAR — `datagrip-impl.jar`

**Path:** `Contents/plugins/datagrip-impl/lib/datagrip-impl.jar`  
**SHA-256:** `e74f78a180b459dfcca507cfa36fe97a1a6e4e1cd384e3bc70a7600c6d8b0589`  
**REA:** `open_binary` (zip) + `inspect_artifact` + inventory view

## Archive stats

| Metric | Value |
|--------|-------|
| ZIP entries | 66 |
| Uncompressed jar size | ~235 KB |
| Kotlin module | `META-INF/intellij.datagrip.impl.kotlin_module` |

## Notable non-class resources

| Entry | Size (declared) |
|-------|-----------------|
| `META-INF/plugin.xml` | 21,030 B |
| `META-INF/DGMainMenuActions.xml` | 8,368 B |
| `p-com.intellij.database.ide-searchableOptions.json` | 2,760 B |
| `intellij.platform.commercial.verifier.xml` | 216 B |
| `__index__` | 5,391 B |

## Class packages (sample — full list in REA evidence)

Primary package: **`com.intellij.database.ide`**

Representative types:

- `DataGripPluginConfigurator` (+ nested Companion / PatchKeymap)
- `DataGripProjectView`, `DataGripProjectMigration`, `DataGripDropHandler`
- `DataGripTabCustomisationStrategy`, `DataGripExternalResourceUrls`
- Actions: `NewSqlFileAction`, `NewProjectAction`, `DGRunActionGroup`, SQL intention delegates
- Hierarchy: `SqlCallHierarchyBrowser`, `SqlCallHierarchyProvider`
- `com.intellij.ultimate.PluginVerifier`

## Role

**DataGrip-specific IDE layer** on top of shared DatabaseTools — project model, menus, tabs, SQL editor affordances, commercial verifier hooks. Not the core JDBC/datagrid engine (see `database-plugin.jar`).
