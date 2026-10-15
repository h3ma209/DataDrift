# Plugin JAR — `database-plugin.jar`

**Path:** `Contents/plugins/DatabaseTools/lib/database-plugin.jar`  
**SHA-256:** `2e646602c19d2add009f9333425b8cba05f2756d2c249936d6e43870dd2090da`  
**REA:** `open_binary` (zip) + `inspect_artifact`

## Archive stats

| Metric | Value |
|--------|-------|
| Graph nodes | 5037 |
| ZIP entries (occurrences) | 5077 |
| On-disk size | ~45.6 MB |

## Descriptor

| Entry | Declared size |
|-------|---------------|
| `META-INF/plugin.xml` | 356,850 B |

Large `plugin.xml` → extensive extension-point registration (datasources, introspection, UI, run configurations, etc.). **Content not parsed in this REA pass** (inventory only).

## Role

**Core database IDE plugin** (`DatabaseTools` module): connectivity, schema introspection, data editor, consoles, dialects, tooling shared across JetBrains DB products. DataGrip loads this plus `datagrip-impl` for product-specific shell.

## Next steps for deeper REA

1. JEB/JADX MCP — decompile selected `com.intellij.database.*` / driver classes.
2. `inspect_artifact` on nested JARs inside plugin `lib/` if present in graph pages.
3. Runtime: `observe_native_calls` / network capture during connect.
