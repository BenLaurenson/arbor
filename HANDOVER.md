# Agent Hub Feature Handover

## Project
Arbor GUI — a Rust desktop app using the GPUI framework (from Zed editor) for managing multiple AI coding agent terminals.

## Branch
`feat/agent-hub-sidebar` on `https://github.com/BenLaurenson/arbor.git` (remote: `mine`)

## What Was Built (35+ commits)

### Agent Session Cards in Sidebar
- Session slug extraction from Claude Code `.jsonl` files
- Active/inactive session display with green/red dots
- Click session → resumes in Hub terminal via `claude --resume <id>`
- Right-click context menu (Resume, Copy Session ID)
- Thin ruled-line branch separators (not full worktree rows)
- Hub-connected sessions show green dots in sidebar

### Agent Hub — Multi-Terminal Grid View
- **Grid layout**: Equal-sized terminal panes in auto-sizing grid (1=full, 2=1×2, 3=1×3, 4=2×2, 5-6=2×3)
- **Pagination**: Max 6 per page, page dots + arrows at bottom, auto-navigate on add
- **Hub tab**: Always-present first tab, default active on startup, can't be closed
- **Maximize**: Expand button on pane header for single-terminal fullscreen
- **Close**: × button on each pane header
- **Drag-and-drop**: Drag pane headers to swap positions
- **Context switching**: Clicking a hub pane syncs sidebar/right-pane to that worktree
- **PTY resize**: Canvas paint callback captures pane bounds, sends to terminal daemon
- **Zoom**: Cmd+=/-/0 via GPUI action bindings with `render_terminal_line_with_font_size()`
- **Persistence**: Hub grid layout saved to `~/.arbor/ui-state.json`
- **Text truncation**: Lines truncated to pane column count (char-boundary safe)
- **Full scrollback**: No line limit, full terminal history scrollable

## What Needs Fixing NOW

### 1. Terminal Text Selection is Broken
**Problem**: Text selection in hub panes is inaccurate — the cursor position doesn't match where the text actually is. Selection highlighting is offset.

**Root cause**: `terminal_grid_position_from_pointer()` in `terminal_interaction.rs` calculates grid position using bounds that don't match the actual rendered terminal content position. The hub pane has nested divs (header, canvas overlay, content div with padding) and the bounds from `hub_pane_bounds` are the outer container bounds, not the inner terminal content area.

**Fix needed**:
- Get the ACTUAL bounds of the terminal scroll container (the div with id `hub-terminal-scroll-{id}`), not the outer pane container
- Account for the pane header height (24px), padding (`px_2` = 16px horizontal, `pt_1` = 4px top)
- Use the correct scroll offset for each pane (currently uses `self.terminal_scroll_handle.offset()` which is the single-terminal view's offset, NOT the hub pane's scroll offset)

**Key files**:
- `crates/arbor-gui/src/terminal_interaction.rs` lines 122-240 — mouse handlers
- `crates/arbor-gui/src/terminal_rendering.rs` — `terminal_grid_position_from_pointer()` function
- `crates/arbor-gui/src/hub_rendering.rs` — pane structure and bounds capture

### 2. Performance is Sluggish
**Problem**: The hub view re-renders ALL terminal lines for ALL visible panes on every frame. With full scrollback enabled, this can be thousands of lines per pane.

**Fix needed**:
- Implement virtual scrolling — only render lines visible in the viewport
- Or limit rendering to the terminal's `rows` count (visible viewport) plus a small buffer
- The single-terminal view uses `ScrollHandle` with `track_scroll()` for efficient scroll rendering — hub panes need the same

### 3. Scroll Offset Per Pane
**Problem**: Each hub pane has its own scroll container but they all share the same scroll offset tracking. When you scroll in one pane, the selection in another pane uses the wrong offset.

**Fix needed**: Store per-pane scroll state. Either:
- Give each pane its own `ScrollHandle` and store in a `HashMap<u64, ScrollHandle>`
- Or track scroll offset in `hub_pane_grid_sizes` alongside the grid dimensions

## Architecture Overview

### Key Data Structures

```
HubGrid (hub_grid.rs)
├── terminals: Vec<u64>     — ordered list of terminal IDs
├── current_page: usize     — which page is visible
└── layout: HubLayoutMode   — Auto or SwimLanes

ArborWindow fields (types.rs):
├── hub_grid: HubGrid
├── hub_active_terminal_id: Option<u64>
├── hub_maximized_terminal: Option<u64>
├── hub_pane_grid_sizes: HashMap<u64, (rows, cols, pw, ph)>
├── hub_pane_bounds: HashMap<u64, Bounds<Pixels>>
├── hub_connected_session_ids: HashSet<String>
├── hub_terminal_to_session: HashMap<u64, String>
├── hub_tab_active: bool
└── terminal_font_scale: f32
```

### Rendering Flow
```
render_hub_view() [hub_rendering.rs]
  → grid_dimensions(page_terminals.len()) → (rows, cols)
  → for each cell: render_hub_terminal_pane(terminal_id)
    → Canvas callback captures pane bounds for PTY resize
    → styled_lines_for_session() → Vec<TerminalStyledLine>
    → Truncate lines to pane_cols
    → render_terminal_line_with_font_size() for each line
    → Lines rendered in scroll container with overflow_y_scroll
```

### Terminal Sync Flow
```
sync_running_terminals() [terminal_session.rs] — runs on timer
  → For hub terminals: reads hub_pane_grid_sizes for cols/rows
  → Hub terminals treated as is_active=true for sync frequency
  → Sends PTY resize via runtime.sync(session, is_active, grid_size)
```

### Key Patterns from Research
- **GPUI bounds**: Only valid during prepaint/paint phase. Use `canvas()` paint callback to capture bounds, store them, read in timer callbacks.
- **Terminal resize**: Zed uses custom Element with prepaint() for same-frame bounds. We use canvas() with one-frame delay (acceptable for PTY resize).
- **Text selection**: Needs accurate bounds of the terminal content area, not the pane container. Must account for header, padding, scroll offset.
- **AgentHub pattern**: Uses SwiftTerm which handles resize/selection natively. Arbor must do it manually via GPUI primitives.

## Reference Projects
- **AgentHub**: `/Users/ben/Projects/personal/AgentHub/` — SwiftUI + SwiftTerm native terminal
- **Agent Deck**: `/Users/ben/.claude/plugins/marketplaces/agent-deck/` — Go TUI with tmux backend

## Commands
```bash
just format    # auto-fix formatting
just lint      # clippy with -D warnings
just test      # all tests
just run       # launch the app

# Or directly:
export PATH="$HOME/.rustup/toolchains/nightly-2025-11-30-aarch64-apple-darwin/bin:$PATH"
cargo clippy --workspace -- -D warnings
cargo test
cargo run -p arbor-gui --features agent-chat
```

## Git
```bash
git remote: mine = https://github.com/BenLaurenson/arbor.git
git push mine feat/agent-hub-sidebar
```

## CLAUDE.md Rules
- No `unwrap()`/`expect()` in non-test code
- No `Co-Authored-By` trailers
- Use `just format && just lint && just test` before committing
- Conventional commits: `feat|fix|docs|style|refactor|test|chore(scope): description`
- Never shell out to external CLIs for GitHub API calls
- Use `pub(crate)` visibility for internal items
