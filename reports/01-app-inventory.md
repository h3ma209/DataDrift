# App bundle inventory — `DataGrip.app`

**REA operation:** `inspect_artifact` (`detail: summary`)  
**Evidence ID:** `ev_5382275732cf8f3d5bb8019fc8df160460c6502734f446989f4196a8b31d3654`

## Summary

| Metric | Value |
|--------|-------|
| Format | `directory` (macOS `.app`) |
| Manifest nodes | 2018 |
| Occurrences | 2297 |
| Graph edges | 2130 |
| Integrity contradictions | 0 |
| Entry kinds | 1584 files, 547 directories, 166 symlinks |

## Pinned critical paths (verified in bundle graph)

| Logical path | Kind | Format | Declared size |
|--------------|------|--------|---------------|
| `Contents/jbr/Contents/Home/bin/java` | executable | mach-o | 69,936 B |
| `Contents/lib/app.jar` | container | zip | 23,475,242 B |
| `Contents/plugins/DatabaseTools/lib/database-plugin.jar` | container | zip | 45,583,428 B |
| `Contents/plugins/datagrip-impl/lib/datagrip-impl.jar` | container | zip | 235,034 B |

## REA next probes (not run in this pass)

- `analyze_javascript_application` — if hunting Electron/ASAR (not primary for DataGrip).
- `open_binary` on nested Mach-O (JBR, JCEF helpers).
- `inspect_plist` on nested bundles (done for root `Info.plist` separately).
- `inspect_managed_artifact` — PE/CLI only; **not applicable** to Java JARs.

## Limitations

Static inventory only. Does not prove runtime behavior, loaded plugins order, or update-channel differences.
