---
'@astryxdesign/core': patch
---

[feat] Timestamp: rename the recently added `system_unix` format to `unix_seconds`. The value is absolute Unix time in whole seconds since the epoch — not a wall-clock `system_*` rendering — so it does not belong to the `system_*` family; the explicit unit name also leaves room for a future `unix_millis`. Behavior is unchanged (zone-independent epoch seconds). This renames a format value that only just shipped, before it has consumers.

@freddymeta
