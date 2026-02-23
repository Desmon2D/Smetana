# Task 03: Replace hand-rolled date formatting with `chrono`

**Phase:** 1 (drop-in replacement, no dependencies on other tasks)
**Estimated savings:** ~40 lines
**Depends on:** Nothing

## Goal

Replace 44 lines of hand-rolled calendar math (`days_to_ymd`, `is_leap`, `format_system_time`) and the unnecessary `ProjectListAction` single-variant enum in `src/app/project_list.rs` with a 3-line `chrono` call. Also fixes a bug: current code shows UTC times instead of local time.

## Code Review

### Current implementation: `src/app/project_list.rs`

**Date functions (lines 12-55, 44 lines):**
- `format_system_time(t: SystemTime) -> String` (13 lines) — converts Unix epoch to `dd.mm.yyyy HH:MM`
- `days_to_ymd(days: u64) -> (u64, u64, u64)` (24 lines) — epoch days to year/month/day via loop
- `is_leap(y: u64) -> bool` (1 line)

**Bug:** Uses raw `duration_since(UNIX_EPOCH).as_secs()` divided by 86400 for hours/minutes. This gives UTC time. Users in UTC+3 (Moscow, primary target audience) see times 3 hours behind.

**Call site:** Only `project_list.rs:124`: `let date_str = format_system_time(entry.modified);`

**`ProjectListAction` enum (lines 8-10):**
```rust
enum ProjectListAction {
    Open(usize),
}
```
Single variant, used once. The "Delete" action already bypasses this enum (uses `self.confirm_delete = Some(i)` directly). This inconsistency confirms it's an early over-abstraction, not a deliberate extension point.

### Alternatives considered

| Approach | Verdict |
|----------|---------|
| `chrono` with `clock` feature | **Selected** — de facto standard, strftime familiar, fixes timezone |
| `time` crate | Rejected — `current_local_offset()` has soundness issues on some platforms |
| `jiff` crate | Rejected — pulls IANA tzdb, disproportionate for "show local time" |
| Fix UTC offset manually via Win32 FFI | Rejected — unsafe FFI complexity not worth avoiding a well-maintained crate |
| Just condense the existing code | Rejected — saves ~20 lines but still shows UTC |

## Changes

### 1. Add dependency to `Cargo.toml`

```toml
chrono = { version = "0.4", default-features = false, features = ["clock"] }
```

The `clock` feature gives `Local::now()` and `SystemTime` conversion. Default features include deprecated `oldtime` interop — not needed.

### 2. Replace the three functions in `src/app/project_list.rs`

Delete lines 12-55 (`format_system_time`, `days_to_ymd`, `is_leap`).

Replace with:

```rust
fn format_system_time(t: std::time::SystemTime) -> String {
    let dt: chrono::DateTime<chrono::Local> = t.into();
    dt.format("%d.%m.%Y %H:%M").to_string()
}
```

### 3. Remove `ProjectListAction` enum and simplify usage

Delete lines 8-10 (enum definition).

In `show_project_list()`, change:
```rust
let mut action: Option<ProjectListAction> = None;
// ...
action = Some(ProjectListAction::Open(i));
// ...
if let Some(a) = action {
    match a {
        ProjectListAction::Open(i) => {
            let path = self.project_entries[i].path.clone();
            self.open_project_from_path(&path);
        }
    }
}
```

To:
```rust
let mut open_idx: Option<usize> = None;
// ...
open_idx = Some(i);
// ...
if let Some(i) = open_idx {
    let path = self.project_entries[i].path.clone();
    self.open_project_from_path(&path);
}
```

## Verification

```bash
cargo build
cargo run  # check project list shows correct local times in dd.mm.yyyy HH:MM format
```
