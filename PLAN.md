# Configuration Roadmap

## Current state
1. `currency` and `map` config sections now exist in code and are loaded in a backward-compatible way.
2. Config load/save now normalises city codes, currency codes, and map country codes, and validates duplicate city codes, timezones, currency codes, and map focus values.
3. Currency cycling is now derived from effective config instead of a fixed `CURRENCY_PAIRS` constant.
4. Canonical country and currency references now live in `src/reference.rs`, and both typed commands and picker search use the same alias matching rules.
5. Slash commands now support:
   - `/country <query>` or bare `/country`
   - `/focus <query>`
   - `/currency <from> -> <to>`
   - `/currency pin <query>`
   - `/currency sync on|off`
   - `/map <route|cities|countries|both>` or bare `/map`
6. Bare `/country`, `/currency`, and `/map` now open interactive picker overlays.
7. Focused map mode now honours configured map settings and shows a visible summary in the UI title/footer.

## Resume here
1. Build the full `/config` staged editor on top of the now-stable config and picker foundations.
2. Extend picker coverage to:
   - add/remove `map.focus_country_codes`
   - set `map.focus_city_code`
   - expose `currency.pinned_codes` removal and reorder
   - expose `currency.sync_with_cities` and pin management without typed commands
3. Decide whether `current_city` should remain the NZ/weather anchor long-term, or whether NZ weather needs its own explicit anchor field before broader geographic generalisation.
4. Keep weather NZ-only for now. If broadening weather later, split that work from map/currency customisation to avoid conflating concerns.
5. Preserve compatibility with legacy configs and keep old defaults unless users explicitly save overrides.

## Vision
1. Add an in-app `/config` menu that lets users review and edit world cities, currency behaviour, and map focus without touching raw TOML.
2. Keep compatibility with existing `~/.config/nzi-cli/config.toml`, including migration of old `NYC` entries to `BOS`.
3. Keep the same fast startup profile and preserve current defaults unless users intentionally change them.

## Problem to solve
1. The current model already supports customisation by editing the file directly, but users cannot safely discover or edit values in-app.
2. City, currency, and map options can drift out of sync when users add/remove tracked cities.
3. There is no dedicated flow for "set" and "reset" actions in the UI.
4. Map focal choices are now config-driven, but multi-value editing and broader discovery remain incomplete.

## Data design
1. Keep the TOML schema structure and add non-breaking optional blocks.
2. Extend config with new sections while preserving old fields:
   - `current_city`, `home_city`, `tracked_cities`, `display` remain mandatory for now.
   - add optional `ui`, `currency`, and `map` sections.
3. Derive currency lists and default pair cycling directly from active city list by default.
4. Represent map context with explicit fields:
   - `map.mode` (`route`, `cities`, `countries`, `both`)
   - `map.focus_city_code`
   - `map.focus_country_codes` as ISO-3166-1 alpha-3
5. Validate on read/write:
   - unique city codes (case-insensitive)
   - valid timezone parsing
   - known currency codes
   - known city/country codes where available
6. Current implementation status:
   - done: optional `currency` and `map` sections, effective defaults, and validation
   - done: canonical lookup layer in `src/reference.rs`
   - done: slash-command and picker-backed edits for focal country, currency pair, and map mode
   - remaining: staged `/config` editor and richer list-management flows

## In-app menu flow
1. Add command `/config` with an overlay panel over the existing UI.
2. Menu mode with 4 tabs:
   1. `Cities`
   2. `Currency`
   3. `Map`
   4. `Advanced`
3. All edits are staged in a working copy.
4. `s` confirms/apply.
5. `r` resets the active tab to defaults.
6. `Ctrl-r` or `a` resets all config sections to defaults.
7. `Esc` exits config mode without saving and returns to main panel.
8. `/reload` remains an explicit config-file refresh and stays available in main mode.
9. Keep the existing slash-command and picker path as a fast path even after `/config` exists.

## Cities tab
1. Show sections: `current_city`, `home_city`, `tracked_cities`.
2. Provide add/remove/edit actions for tracked cities.
3. Allow alias/label edits for `name`, `code`, `timezone`, `currency`, `country`.
4. Keep default city set as a reusable preset from `Config::default()`.
5. Include a helper action "set home = selected tracked city".
6. Enforce uniqueness by city code, and auto-fix legacy `NYC` entries to `BOS`.

## Currency tab
1. Show computed currency list from city set.
2. Expose optional manual override list for power users.
3. Add action to add/remove currency pins in the converter cycle.
4. Add "sync with cities" toggle:
   1. on = derive from current city set
   2. off = use manual override
5. Show a warning when a tracked currency is missing from all active cities.
6. Current implementation note:
   - typed commands and picker now cover pair selection
   - typed commands cover sync toggle and pin add
   - pin removal and reorder are still missing

## Map tab
1. Show current map focus mode and active values.
2. For `route`, keep NZ-to-home default route line and allow alternate city pair.
3. For `cities`, show all active city markers.
4. For `countries`, show focal countries by ISO code with marker fallback.
5. For `both`, combine markers and route.
6. Add actions:
   1. set focus city
   2. add/remove focus country
   3. reset map section
7. Current implementation note:
   - mode and focal country are already configurable
   - focus city and multi-country management remain to be built

## Advanced tab
1. Add raw JSON/TOML diagnostics view for unsupported edits.
2. Include a one-click "open config path" shortcut.
3. Provide migration summary panel showing last migration applied.
4. Add "discard unsaved changes" and "backup before overwrite" actions.

## Implementation phases
1. Phase 1 (schema + menu shell):
   1. Add config structs for menu-backed optional sections.
   2. Add `/config`, tab rendering, and staged state.
   3. Add `Esc`, `s`, `r`, `Ctrl-r` key handling.
2. Phase 2 (city and currency wiring):
   1. Wire cities tab edits to config object.
   2. Derive currency cycle from active cities by default.
   3. Keep manual override only if enabled.
3. Phase 3 (map controls):
   1. Wire map mode and focus settings.
   2. Add combined marker/route preview panel.
4. Phase 4 (hardening):
   1. Validation and migration reporting.
   2. Tests for migration, dedupe, and menu state transitions.
   3. Update README usage and keybinding sections.
5. Pre-menu foundation now complete:
   1. schema, validation, and derived settings
   2. reference lookup/search layer
   3. command and picker controls for key map/currency operations

## Acceptance criteria
1. `/config` opens from normal mode and returns to main mode on demand.
2. Users can set and persist a new home city and tracked city set from the menu.
3. Users can reset a single tab or all settings.
4. Users can control map focus without editing TOML directly.
5. NYC legacy entries in old config files auto-migrate to BOS/Boston on first run.
6. Existing defaults remain unchanged until users explicitly save changes.

## Ticket list

## Delivery tracks

### Track A — MVP config menu
1. Goal: ship a simple, low-risk `/config` mode with set/reset/return and city-level edits.
2. Implement tickets: 1–6, 9, 12, 15, 16, 18.
3. Scope:
   - In-app command `/config` with overlay and `Esc` return.
   - Staged edit model with Apply (`s`), Reset tab (`r`), Reset all (`Ctrl+r`/`a`).
   - Cities tab enough to set home city, remove/add tracked cities, and align time/currency sources from that list.
   - Basic validation + migration guard before save.
   - Changelog + README update for menu availability.
4. Deferred in Track A:
   - advanced manual currency overrides
   - map focus modes beyond existing NZ route default
   - discard-confirmation nuances beyond simple escape behavior.

#### Track A execution checklist
1. Ticket 1 — app mode + staged draft
   - Command/area: `src/app.rs`
   - Acceptance:
     - `/config` command enters config mode.
     - A config draft is created from current runtime config.
     - Main-mode key handlers do not mutate draft.

2. Ticket 2 — config overlay scaffold
   - Command/area: `src/ui.rs`
   - Acceptance:
     - Config overlay appears with title and 4 tabs.
     - Overlay blocks focus movement to main panels.

3. Ticket 3 — config key handling
   - Command/area: `src/app.rs`
   - Acceptance:
     - `Esc` returns to main mode without applying draft.
     - `s` applies staged changes and saves.
     - `r` resets active tab.
     - `Ctrl+r` (or `a`) resets all config sections.

4. Ticket 4 — city draft model
   - Command/area: `src/config.rs`, `src/app.rs`
   - Acceptance:
     - `current_city`, `home_city`, `tracked_cities` can be staged independently.
     - `home_city` and tracked entries are preserved across staged ops.

5. Ticket 5 — cities tab edit actions
   - Command/area: `src/ui.rs`
   - Acceptance:
     - Add tracked city.
     - Remove tracked city.
     - Edit tracked city fields (`name`, `code`, `timezone`, `currency`, `country`).
     - Set home from tracked city.

6. Ticket 6 — save/apply integration
   - Command/area: `src/config.rs`, `src/app.rs`
   - Acceptance:
     - `s` persists config only when dirty.
     - `Config::load()` migration (`NYC` -> `BOS`) still applies after save/load roundtrip.
     - Time and currency flows re-bind from updated cities after apply.

7. Ticket 9 — validation on apply
   - Command/area: `src/config.rs`
   - Acceptance:
     - Duplicate city codes are rejected.
     - Invalid timezone strings are rejected.
     - Empty required city fields are rejected.
     - Migration and dedupe keep a valid runtime fallback when errors are fixed.

8. Ticket 12 — currency list sync from cities
   - Command/area: `src/app.rs`
   - Acceptance:
     - Time/currency displays always reflect active city list.
     - `home_city` currency change propagates to conversion defaults.

9. Ticket 15 — docs and changelog updates for MVP
   - Command/area: `CHANGELOG.md`, `README.md`
   - Acceptance:
     - Changelog top entry includes `/config` and migration-safe behavior.
     - README documents `/config`, `s`, `r`, `Esc` and reset flow.

10. Ticket 16 — tests for MVP core
    - Command/area: `src/app.rs`, `src/config.rs`
    - Acceptance:
      - Test for config mode transitions.
      - Test for apply/discard.
      - Test for NYC->BOS migration remains idempotent.

11. Ticket 18 — final verification pass
    - Command/area: `README.md`, `PLAN.md`
    - Acceptance:
      - README examples match implemented keys.
      - Plan status reflects executed Track A tasks.

#### Track A done criteria
1. Users can enter config mode with `/config`.
2. Users can add/remove/edit tracked cities and set home city from tracked.
3. Users can apply or discard changes reliably (`s` / `Esc`).
4. NYC->BOS migration remains active and idempotent.
5. No functional regression in startup, default config, or runtime display.

### Track B — Full config experience
1. Goal: complete feature set with currency overrides, map focus controls, and full admin conveniences.
2. Implement tickets: 10, 11, 13, 14, 17, 19, plus any blocked Track A refinements from pre-flight.
3. Scope:
   - Full currency override/override-sync workflow.
   - Map section with route/cities/countries/both modes and focus city/country control.
   - Advanced diagnostics and backup workflow.
   - Expanded validation and migration reporting.
4. Acceptance for Track B:
   - map-focused and currency-focused usage without `/edit`.
   - config workflow supports power-user cases without manual TOML edits.

#### Track B execution checklist
1. Ticket 10 — currency config model
   - Command/area: `src/config.rs`
   - Acceptance:
     - Optional `currency` section exists with sync flag and explicit override list.
     - Deserialisation with missing `currency` section remains defaulted.
   - Status:
     - done

2. Ticket 11 — currency tab edit actions
   - Command/area: `src/ui.rs`, `src/app.rs`
   - Acceptance:
     - Manual add/remove of currencies is possible in override mode.
     - Sync toggle switches effective list source.
     - Currency list reorder or replace operations are deterministic.
   - Status:
     - part done: pair selection, sync toggle, and pin add exist through commands/picker
     - remaining: pin removal, reorder, and richer in-app list editing

3. Ticket 13 — map config model
   - Command/area: `src/config.rs`
   - Acceptance:
     - `map.mode`, `map.focus_city_code`, `map.focus_country_codes` are serialised/deserialised.
     - Defaults preserve existing NZ-to-home route behaviour.
     - Invalid country/city focus values are rejected or normalised.
   - Status:
     - mostly done
     - note: `focal_country_code` was also added as a practical default anchor

4. Ticket 14 — map controls
   - Command/area: `src/ui.rs`, `src/app.rs`
   - Acceptance:
     - Users can set map mode (`route|cities|countries|both`).
     - Users can set focus city.
     - Users can add/remove focus countries.
     - Preview reflects the current mode and focus selections.
   - Status:
     - part done: map mode and focal country can be set through commands/picker and preview reflects mode
     - remaining: focus city and multi-country add/remove controls

5. Ticket 17 — map focus serialisation tests
   - Command/area: `src/config.rs`
   - Acceptance:
     - Test roundtrip for defaults.
     - Test legacy config without map block.
     - Test malformed values yield safe fallback and non-fatal migration note.
   - Status:
     - part done: validation coverage exists for malformed focus values
     - remaining: explicit serialisation roundtrip and legacy-map-block tests

6. Ticket 19 — docs for advanced flow
   - Command/area: `CHANGELOG.md`, `README.md`
   - Acceptance:
     - Changelog entry for Track B launch includes currency override and map focus capabilities.
     - README includes examples of map mode and currency override usage.
   - Status:
     - in progress with this update

#### Track B done criteria
1. Currency override mode can be fully controlled in-app.
2. Map focus is user-configurable without raw config editing.
3. Route/cities/countries/both render modes function and persist.
4. Advanced diagnostics show migration/validation state to the user.
5. No regressions relative to Track A baseline behavior.

### Phase 1 — Menu shell and safe state
1. [ ] Add config UI mode state to app
   - File: `src/app.rs`
   - Add `Mode::Config` and a staged config draft model.
   - Add command parsing for `/config` in command handling.
2. [ ] Add `/config` overlay rendering skeleton
   - File: `src/ui.rs`
   - Add overlay layout with title, tab bar placeholders, and footer hints.
   - Ensure existing panels do not handle focus movement while in config mode.
3. [ ] Add keybindings for menu flow
   - File: `src/app.rs`
   - Implement:
     - `Esc` to discard staged edits and return.
     - `s` to validate + apply staged config.
     - `r` to reset current tab defaults.
     - `Ctrl+r` (or dedicated `a`) to reset all sections to defaults.
4. [ ] Add unsaved-change guard
   - File: `src/app.rs`, `src/ui.rs`
   - If dirty and `Esc`, show confirmation line before exit (or discard explicitly via one command).

### Phase 2 — City editing + migration-safe persistence
5. [ ] Add Config draft data structures and edit ops
   - File: `src/config.rs`, `src/app.rs`
   - Keep existing schema unchanged for load/save compatibility.
   - Add functions to stage city edits against `current_city`, `home_city`, `tracked_cities`.
6. [ ] Implement Cities tab editing UI
   - File: `src/ui.rs`
   - Show and edit:
     - current city
     - home city
     - tracked cities list add/remove/update
   - Add action to set selected tracked city as home.
7. [ ] Add helper operations for city selection
   - File: `src/app.rs`
   - Implement selection index and move/edit commands for tracked list.
8. [ ] Enforce validation on staged city edits
   - File: `src/config.rs`, `src/app.rs`
   - Validate:
     - unique city codes (case-insensitive)
     - timezone parseable by `chrono-tz`
     - non-empty required fields
   - Reuse migration path from NYC->BOS and dedupe logic.
9. [ ] Ensure apply path writes defaults correctly
   - File: `src/config.rs`, `src/app.rs`
   - On `s`, merge staged config into runtime config, call `save()`, and refresh dependent services.

### Phase 3 — Currency derivation and overrides
10. [ ] Add optional currency config section
    - File: `src/config.rs`
    - Add optional `currency` settings and sync flag:
      - derive currencies from active cities by default.
      - optional manual override list.
11. [ ] Implement Currency tab
    - File: `src/ui.rs`, `src/app.rs`
    - Add add/remove/swap list operations for currencies in override mode.
    - Show sync status with city set.
12. [ ] Wire converter pair defaults to chosen currency list
    - File: `src/app.rs`, `src/exchange.rs`
    - Ensure currency conversion cycling and defaults use active effective list.

### Phase 4 — Map focus controls
13. [ ] Add map options section in config
    - File: `src/config.rs`
    - Add map config fields:
      - `mode`
      - `focus_city_code`
      - `focus_country_codes`
    - Keep backward-compatible fallback values.
14. [ ] Implement Map tab controls
    - File: `src/ui.rs`, `src/app.rs`
    - Actions:
      - set focus city
      - add/remove focus countries
      - change map mode (`route|cities|countries|both`)
      - reset map section
15. [ ] Update map rendering inputs
    - File: `src/ui.rs`, `src/app.rs`
    - Map marker/line source reads from active map config, with NZ-route default retained.

### Phase 5 — Polishing, docs, and tests
16. [ ] Add tests for menu state transitions and save/apply flow
    - File: `src/config.rs`, `src/app.rs`
    - Cover:
      - draft editing
      - reset tab/all
      - apply/discard paths
      - NYC migration remains idempotent
17. [ ] Add tests for map focus config serialisation
    - File: `src/config.rs`
    - Cover defaults + deserialise legacy + malformed inputs.
18. [ ] Update README command and keybinding docs
    - File: `README.md`
    - Add `/config` usage and keymap: `Esc`, `s`, `r`, `Ctrl+r`.
19. [ ] Update changelog for `/config` feature launch
    - File: `CHANGELOG.md`
    - New top entry with migration + menu release summary.

### Pre-flight checklist (before implementation)
1. [ ] Agree on exact keybinding for full reset (`Ctrl+r` vs `a`) and conflict with existing shortcuts.
2. [ ] Decide whether map section uses additive or replace semantics for tracked city markers.
3. [ ] Decide if timezone/currency validation should block save with hard error or keep staged warning state.
