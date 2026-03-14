# Tasks: Rust Cross-Platform WebWallpaper CLI

**Input**: Design documents from `/specs/001-rust-webwallpaper-cli/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/cli.md
**Phase 1 Focus**: Windows implementation

**Tests**: Not explicitly requested - tests omitted. Add tests if TDD approach is desired.

**Organization**: Tasks grouped by user story for independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Project initialization and Rust project structure

- [x] T001 Initialize Cargo project with `cargo init` in repository root
- [x] T002 Configure Cargo.toml with dependencies from research.md (wry, tao, clap, interprocess, tiny_http, serde, serde_json, windows)
- [x] T003 [P] Create project directory structure per plan.md in src/
- [x] T004 [P] Configure rustfmt.toml for code formatting
- [x] T005 [P] Configure clippy.toml for linting rules

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**CRITICAL**: No user story work can begin until this phase is complete

- [x] T006 Implement CLI argument parsing with clap derive in src/cli.rs
- [x] T007 [P] Create Config struct and default configuration in src/config.rs
- [x] T008 [P] Create platform module structure in src/platform/mod.rs
- [x] T009 Create Display trait definition in src/display.rs
- [x] T010 Implement Windows EnumDisplayMonitors in src/platform/windows/display.rs
- [x] T011 [P] Create WallpaperInstance struct with serde serialization in src/config.rs
- [x] T012 [P] Implement instance directory management (create %TEMP%\webwallpaper) in src/config.rs
- [x] T013 Create entry point with CLI dispatch in src/main.rs

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Display Web Content as Wallpaper (Priority: P1)

**Goal**: Display a URL or local HTML file as fullscreen desktop wallpaper behind all windows

**Independent Test**: Run `webwallpaper ./test.html` and verify content appears as desktop background, click-through works

### Implementation for User Story 1

- [x] T014 [US1] Create Wallpaper trait definition in src/wallpaper.rs
- [x] T015 [US1] Implement local HTTP server with tiny_http in src/server.rs
- [x] T016 [US1] Add path validation and directory traversal protection in src/server.rs
- [x] T017 [US1] Create Windows webview window with wry/tao (frameless, fullscreen) in src/platform/windows/wallpaper.rs
- [x] T018 [US1] Apply WS_EX_TOOLWINDOW style (no taskbar) in src/platform/windows/wallpaper.rs
- [x] T019 [US1] Apply WS_EX_NOACTIVATE style (no focus) in src/platform/windows/wallpaper.rs
- [x] T020 [US1] Apply WS_EX_TRANSPARENT + WS_EX_LAYERED for click-through in src/platform/windows/wallpaper.rs
- [x] T021 [US1] Call SetLayeredWindowAttributes with alpha=252 in src/platform/windows/wallpaper.rs
- [x] T022 [US1] Implement SetWindowPos with HWND_BOTTOM for Z-order in src/platform/windows/wallpaper.rs
- [x] T023 [US1] Enable DPI awareness via SetProcessDpiAwareness in src/platform/windows/mod.rs
- [x] T024 [US1] Write instance JSON file on wallpaper start in src/config.rs
- [x] T025 [US1] Implement URL vs local path detection in src/main.rs
- [x] T026 [US1] Wire up CLI → server (if local) → webview flow in src/main.rs

**Checkpoint**: User Story 1 complete - single-monitor wallpaper on primary display works

---

## Phase 4: User Story 2 - Target Specific Display (Priority: P2)

**Goal**: Allow `--display NUM` to target a specific monitor, default to all monitors

**Independent Test**: Run `webwallpaper ./test.html --display 1` on multi-monitor setup and verify only display 1 shows wallpaper

### Implementation for User Story 2

- [x] T027 [US2] Add display index validation against enumerated monitors in src/main.rs
- [x] T028 [US2] Implement multi-monitor positioning using Display.x, Display.y in src/platform/windows/wallpaper.rs
- [x] T029 [US2] Implement wallpaper spawn per display (all displays when --display not specified) in src/main.rs
- [x] T030 [US2] Add "Display N does not exist" error with available displays list in src/main.rs
- [x] T031 [US2] Write separate instance files per display (display_0.json, display_1.json) in src/config.rs

**Checkpoint**: User Story 2 complete - multi-monitor support works

---

## Phase 5: User Story 3 - Stop Running Wallpaper Instances (Priority: P2)

**Goal**: Implement `--stop NUM` and `--stopall` to terminate running wallpaper instances

**Independent Test**: Start wallpaper with `webwallpaper ./test.html`, then run `webwallpaper --stopall` and verify process terminates

### Implementation for User Story 3

- [x] T032 [US3] Implement IPC named pipe listener in src/ipc.rs
- [x] T033 [US3] Define IPC protocol (STOP:0, STOP:ALL, PING) in src/ipc.rs
- [x] T034 [US3] Start IPC listener thread when wallpaper starts in src/platform/windows/wallpaper.rs
- [x] T035 [US3] Implement IPC client for sending stop commands in src/ipc.rs
- [x] T036 [US3] Implement --stop handler: connect to pipe, send STOP:N in src/main.rs
- [x] T037 [US3] Implement --stopall handler: read all instance files, send STOP:ALL in src/main.rs
- [x] T038 [US3] Handle graceful shutdown on IPC stop command in src/platform/windows/wallpaper.rs
- [x] T039 [US3] Delete instance JSON file on wallpaper stop in src/main.rs
- [x] T040 [US3] Handle Ctrl+C (SIGINT) for graceful cleanup in src/platform/windows/wallpaper.rs
- [x] T041 [US3] Add "No wallpaper running on display N" message in src/main.rs

**Checkpoint**: User Story 3 complete - stop commands work

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Error handling, logging, documentation

- [x] T042 [P] Implement WebView2 runtime detection and helpful error message in src/platform/windows/mod.rs
- [x] T043 [P] Add --verbose flag logging throughout application in src/main.rs
- [x] T044 [P] Implement consistent exit codes per contracts/cli.md in src/main.rs
- [x] T045 [P] Add port-in-use detection with hint to use --port in src/server.rs
- [x] T046 [P] Handle instance replacement (stop existing before starting new) in src/main.rs
- [x] T047 Validate quickstart.md scenarios work end-to-end
- [x] T048 Update README.md with usage instructions

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - start immediately
- **Foundational (Phase 2)**: Depends on Setup - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational - can start after Phase 2
- **User Story 2 (Phase 4)**: Depends on Foundational + benefits from US1 display code
- **User Story 3 (Phase 5)**: Depends on Foundational + needs instance tracking from US1
- **Polish (Phase 6)**: Depends on all user stories complete

### User Story Dependencies

| Story | Depends On | Can Run After |
|-------|------------|---------------|
| US1 (P1) | Phase 2 only | Phase 2 complete |
| US2 (P2) | Phase 2 + US1 display positioning | US1 T022 (window positioning) |
| US3 (P2) | Phase 2 + US1 instance tracking | US1 T024 (instance file) |

### Within Each User Story

- Core implementation before integration
- Window creation before styling
- Styling before Z-order positioning

### Parallel Opportunities

**Phase 1 (Setup):**
```
T003, T004, T005 can run in parallel (different files)
```

**Phase 2 (Foundational):**
```
T007, T008, T011, T012 can run in parallel (different files)
```

**US1 (Phase 3):**
```
T014, T015 can start in parallel
T017-T022 are sequential (same file, building on window)
```

**Polish (Phase 6):**
```
T042, T043, T044, T045, T046 can all run in parallel (different files/features)
```

---

## Parallel Example: Phase 2 Foundational

```bash
# These can run simultaneously:
Task: "Create Config struct in src/config.rs"
Task: "Create platform module structure in src/platform/mod.rs"
Task: "Create WallpaperInstance struct in src/config.rs"
Task: "Implement instance directory management in src/config.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (~30 min)
2. Complete Phase 2: Foundational (~2 hours)
3. Complete Phase 3: User Story 1 (~3-4 hours)
4. **STOP and VALIDATE**: Test single-display wallpaper works
5. Deploy/demo if ready - **BASIC WALLPAPER WORKS!**

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add User Story 1 → Test → **MVP: Single wallpaper works**
3. Add User Story 2 → Test → **Multi-monitor support**
4. Add User Story 3 → Test → **Stop command support**
5. Polish → Test → **Production ready**

### Estimated Effort

| Phase | Tasks | Est. Time |
|-------|-------|-----------|
| Setup | T001-T005 | 30 min |
| Foundational | T006-T013 | 2 hours |
| US1 | T014-T026 | 3-4 hours |
| US2 | T027-T031 | 1-2 hours |
| US3 | T032-T041 | 2-3 hours |
| Polish | T042-T048 | 1-2 hours |
| **Total** | **48 tasks** | **~10-14 hours** |

---

## Notes

- [P] tasks = different files, safe to parallelize
- [US1/US2/US3] labels track which user story each task serves
- US4 (Cross-Platform) is addressed by architecture; Linux/macOS implementation is future work
- Windows-specific code is isolated in `src/platform/windows/`
- Test locally with simple HTML file before using external URLs
- WebView2 is required on Windows - ensure it's installed before testing
