# Info.plist — bundle metadata & TCC

**REA operation:** `inspect_plist`  
**Path:** `/Applications/DataGrip.app/Contents/Info.plist`

## Identity

| Key | Value |
|-----|-------|
| CFBundleIdentifier | `com.jetbrains.datagrip` |
| CFBundleName | DataGrip |
| CFBundleShortVersionString | 2026.1.3 |
| CFBundleVersion | DB-261.24374.56 |
| CFBundleExecutable | datagrip |
| LSArchitecturePriority | arm64 |
| LSRequiresNativeExecution | YES |
| LSMinimumSystemVersion | 10.13 |
| LSApplicationCategoryType | public.app-category.developer-tools |

## Privacy / usage descriptions (macOS TCC)

| Purpose | User-facing string (abridged) |
|---------|-------------------------------|
| Local network | SSH, DB connectivity, broadcast |
| Desktop / Documents / Downloads | Folder access for embedded apps |
| Network / removable volumes | File access on volumes |
| Camera / Microphone / Bluetooth / Location | Generic “application in DataGrip requests…” |
| Apple Events | Send Apple events |

## Notes

- Plist parsed to JSON by REA (`plutil`); no execution.
- Nested helper bundles (JCEF, etc.) have separate plists — not expanded in this report.
