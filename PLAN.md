# Planning

## Deferred
- Defer ratatui upgrade; current ratatui pulls `lru 0.12.5` which is affected by a soundness issue in `IterMut` (Stacked Borrows). Risk noted; revisit when ready to upgrade ratatui to a version that depends on `lru >= 0.16.3`.
