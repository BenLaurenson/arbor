# Hub Terminal Selection, Performance & Per-Pane Scroll Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix broken text selection in hub panes, add per-pane scroll handles, and implement viewport-limited rendering for performance.

**Architecture:** Three bugs are intertwined — selection uses wrong bounds (outer pane container instead of inner content area), wrong scroll offset (shared single-terminal handle instead of per-pane), and all lines render on every frame. Fix order: (1) per-pane scroll handles first (needed by both selection and rendering), (2) fix selection bounds calculation, (3) viewport-limited rendering.

**Tech Stack:** Rust, GPUI framework (from Zed editor), `ScrollHandle`, `Bounds<Pixels>`, `canvas()` paint callbacks.

---

### Task 1: Add `hub_pane_scroll_handles` HashMap to ArborWindow

**Files:**
- Modify: `crates/arbor-gui/src/types.rs:2466` (after `hub_pane_bounds`)
- Modify: `crates/arbor-gui/src/app_init.rs` (both `new()` constructors)

**Step 1: Add the field to ArborWindow**

In `types.rs`, after the `hub_pane_bounds` field (line 2466), add:

```rust
/// Per-hub-pane scroll handles for accurate scroll offset tracking.
pub(crate) hub_pane_scroll_handles: HashMap<u64, ScrollHandle>,
```

**Step 2: Initialize in both constructors in app_init.rs**

Find every place that initializes `hub_pane_bounds: HashMap::new()` and add after it:

```rust
hub_pane_scroll_handles: HashMap::new(),
```

There are two constructors (around lines 209-390 and 682-864). Both need the field.

**Step 3: Run format and lint**

Run: `just format && just lint`
Expected: PASS (no warnings)

**Step 4: Commit**

```
feat(gui): add per-pane scroll handle storage to ArborWindow
```

---

### Task 2: Create and wire ScrollHandle per hub pane

**Files:**
- Modify: `crates/arbor-gui/src/hub_rendering.rs:680-687` (the scroll container)

**Step 1: Get or create ScrollHandle for each pane**

In `render_hub_terminal_pane()`, before the scroll container div (around line 667), add logic to get-or-create the scroll handle. The function takes `&self` not `&mut self`, so we need to use an entry pattern. Since `render_hub_terminal_pane` takes `&self`, we can't mutate. Instead, pre-populate the handles in the caller or use a different approach.

**Approach:** Pre-create handles in `hub_add_terminal()` and `hub_resume_session()` (wherever terminals get added to the hub grid). Then in `render_hub_terminal_pane`, just read from the map.

Find every call to `self.hub_grid.add_terminal(id)` or equivalent and add after it:

```rust
self.hub_pane_scroll_handles
    .entry(terminal_id)
    .or_insert_with(ScrollHandle::new);
```

Also clean up in `hub_remove_terminal()`:

```rust
self.hub_pane_scroll_handles.remove(&terminal_id);
```

**Step 2: Wire the ScrollHandle with `.track_scroll()` in the hub pane scroll container**

In `hub_rendering.rs`, the scroll container div (around line 679-687) currently looks like:

```rust
div()
    .id(ElementId::Name(
        format!("hub-terminal-scroll-{terminal_id}").into(),
    ))
    .size_full()
    .min_w_0()
    .min_h_0()
    .overflow_x_hidden()
    .overflow_y_scroll()
    .scrollbar_width(px(TERMINAL_SCROLLBAR_WIDTH_PX))
```

After `.scrollbar_width(...)`, add `.track_scroll(scroll_handle)` where `scroll_handle` is obtained from the map at the top of `render_hub_terminal_pane`:

At the top of the function (after `let ime_text = ...`), add:

```rust
let scroll_handle = self
    .hub_pane_scroll_handles
    .get(&terminal_id)
    .cloned();
```

Then in the div chain, after `.scrollbar_width(px(TERMINAL_SCROLLBAR_WIDTH_PX))`:

```rust
.when_some(scroll_handle.clone(), |el, handle| el.track_scroll(&handle))
```

**Step 3: Run format and lint**

Run: `just format && just lint`
Expected: PASS

**Step 4: Run tests**

Run: `just test`
Expected: PASS

**Step 5: Commit**

```
feat(gui): wire per-pane ScrollHandle for hub terminal panes
```

---

### Task 3: Write failing test for grid position calculation with pane offsets

**Files:**
- Modify: `crates/arbor-gui/src/terminal_rendering.rs` (test module at bottom)

**Step 1: Write the failing test**

Add to the existing `mod tests` block:

```rust
#[test]
fn grid_position_accounts_for_header_and_padding_offsets() {
    // Simulate a hub pane whose content area starts 28px below the pane
    // top (24px header + 4px pt_1 padding) and 16px from the left (px_2).
    let content_bounds = Bounds {
        origin: gpui::Point {
            x: px(116.),  // 100px pane left + 16px px_2 padding
            y: px(228.),  // 200px pane top + 24px header + 4px pt_1 padding
        },
        size: gpui::Size {
            width: px(400.),
            height: px(300.),
        },
    };
    let scroll_offset = gpui::Point {
        x: px(0.),
        y: px(0.),  // Not scrolled
    };
    let line_height = 19.0_f32;
    let cell_width = 9.0_f32;
    let line_count = 50;

    // Click at pixel (125, 247) — which is 9px into the content area
    // horizontally and 19px into the content area vertically.
    // That should be line 1 (floor(19/19)=1), column 1 (floor(9/9)=1).
    let pos = gpui::Point {
        x: px(125.),
        y: px(247.),
    };

    let result = terminal_grid_position_from_pointer(
        pos,
        content_bounds,
        scroll_offset,
        line_height,
        cell_width,
        line_count,
    );

    assert_eq!(
        result,
        Some(TerminalGridPosition { line: 1, column: 1 })
    );
}

#[test]
fn grid_position_with_scroll_offset() {
    let content_bounds = Bounds {
        origin: gpui::Point {
            x: px(16.),
            y: px(28.),
        },
        size: gpui::Size {
            width: px(400.),
            height: px(300.),
        },
    };
    // Scrolled down 190px (10 lines at 19px each)
    let scroll_offset = gpui::Point {
        x: px(0.),
        y: px(-190.),
    };
    let line_height = 19.0_f32;
    let cell_width = 9.0_f32;
    let line_count = 100;

    // Click at the very top of the content area
    let pos = gpui::Point {
        x: px(25.),  // 9px into content → column 1
        y: px(28.),  // At content top
    };

    let result = terminal_grid_position_from_pointer(
        pos,
        content_bounds,
        scroll_offset,
        line_height,
        cell_width,
        line_count,
    );

    // With -190px scroll offset, local_y=0, content_y = 0 - (-190) = 190
    // line = floor(190/19) = 10
    assert_eq!(
        result,
        Some(TerminalGridPosition {
            line: 10,
            column: 1
        })
    );
}
```

**Step 2: Run test to verify it passes (these test the existing function which already works correctly for correct bounds)**

Run: `just test -- terminal_rendering::tests::grid_position`
Expected: PASS (the function logic is correct — the bug is that callers pass wrong bounds, not that the function is wrong)

**Step 3: Commit**

```
test(gui): add grid position calculation tests for pane offsets and scroll
```

---

### Task 4: Fix mouse handlers to use per-pane content bounds and scroll offset

**Files:**
- Modify: `crates/arbor-gui/src/terminal_interaction.rs:122-249`

This is the core fix. The mouse handlers currently use `hub_pane_bounds` (outer container) and `self.terminal_scroll_handle.offset()` (single-terminal view's offset). They need to:
1. Use the per-pane `ScrollHandle` for scroll offset
2. Offset the pane bounds by header height (24px) and padding (px_2=16px horizontal, pt_1=4px top)

**Step 1: Create a helper to get hub pane content bounds and scroll offset**

Add a new method to `impl ArborWindow` in `terminal_interaction.rs` (before `handle_terminal_output_mouse_down`):

```rust
/// Returns (content_bounds, scroll_offset) for a hub terminal pane,
/// accounting for the pane header (24px) and padding (px_2 + pt_1).
/// Falls back to the standalone terminal scroll handle for non-hub views.
fn terminal_content_bounds_and_scroll(
    &self,
    session_id: u64,
) -> (Bounds<Pixels>, gpui::Point<Pixels>) {
    if self.hub_tab_active {
        if let Some(pane_bounds) = self.hub_pane_bounds.get(&session_id) {
            // The pane bounds are the outer container. The content area is:
            // - 24px below for the header
            // - 16px (px_2) horizontal padding (8px each side)
            // - 4px (pt_1) top padding inside the content div
            let header_height = px(24.);
            let h_padding = px(16.);  // px_2 = 8px * 2
            let v_padding = px(4.);   // pt_1
            let content_origin = gpui::Point {
                x: pane_bounds.origin.x + h_padding / 2.0,
                y: pane_bounds.origin.y + header_height + v_padding,
            };
            let content_size = gpui::Size {
                width: pane_bounds.size.width - h_padding,
                height: pane_bounds.size.height - header_height - v_padding,
            };
            let content_bounds = Bounds {
                origin: content_origin,
                size: content_size,
            };

            let scroll_offset = self
                .hub_pane_scroll_handles
                .get(&session_id)
                .map(|h| h.offset())
                .unwrap_or_default();

            return (content_bounds, scroll_offset);
        }
    }

    // Standalone terminal view
    let bounds = self.terminal_scroll_handle.bounds();
    let offset = self.terminal_scroll_handle.offset();
    (bounds, offset)
}
```

**Step 2: Update `handle_terminal_output_mouse_down` to use the helper**

Replace lines 146-151 (the `scroll_bounds` and `scroll_offset` bindings):

```rust
// Old:
let scroll_bounds = self
    .hub_pane_bounds
    .get(&session_id)
    .copied()
    .unwrap_or_else(|| self.terminal_scroll_handle.bounds());
let scroll_offset = self.terminal_scroll_handle.offset();

// New:
let (scroll_bounds, scroll_offset) =
    self.terminal_content_bounds_and_scroll(session_id);
```

**Step 3: Update `handle_terminal_output_mouse_move` the same way**

Replace lines 226-231:

```rust
// Old:
let scroll_bounds = self
    .hub_pane_bounds
    .get(&session_id)
    .copied()
    .unwrap_or_else(|| self.terminal_scroll_handle.bounds());
let scroll_offset = self.terminal_scroll_handle.offset();

// New:
let (scroll_bounds, scroll_offset) =
    self.terminal_content_bounds_and_scroll(session_id);
```

**Step 4: Run format and lint**

Run: `just format && just lint`
Expected: PASS

**Step 5: Run tests**

Run: `just test`
Expected: PASS

**Step 6: Commit**

```
fix(gui): use per-pane content bounds and scroll offset for hub text selection
```

---

### Task 5: Capture content-area bounds instead of outer pane bounds

**Files:**
- Modify: `crates/arbor-gui/src/hub_rendering.rs:630-658`

The canvas callback currently captures `bounds` (the outer content div bounds). This is the measurement for PTY resize — it's fine for that. But `hub_pane_bounds` is used by the mouse handlers and stores the outer bounds.

**Better approach:** Instead of offsetting in the mouse handler (Task 4), capture the *content area* bounds directly. The canvas is inside the content div (after the header), so `bounds` from the canvas already excludes the header. But the content div has padding (`px_2`, `pt_1`) that the canvas is inside of (since it's `absolute().inset_0()`).

Wait — looking more carefully at the layout:

```
hub-pane-{id}          ← outer container (what hub_pane_bounds stores)
  ├── header (h=24px)
  └── hub-terminal-content-{id} (flex-1, overflow_hidden, relative)
       ├── canvas (absolute, inset-0)  ← bounds = content div bounds (EXCLUDES header)
       └── content div (absolute, inset-0, px_2, pt_1)
            └── scroll div (hub-terminal-scroll-{id})
                 └── line container
```

So the `canvas` `bounds` are the content area (minus header) but *before* the px_2/pt_1 padding. The mouse events fire on the scroll div which is *inside* the padded div.

**The fix in Task 4's helper is correct.** The `hub_pane_bounds` stores what the canvas reports (the content div bounds, which excludes the header but includes the padding area). So we only need to offset by the padding, not the header.

**Step 1: Re-examine what bounds the canvas captures**

The canvas is a child of `hub-terminal-content-{id}` which is `flex_1()` (fills space below the header). So `bounds` from the canvas = content area **below the header**. But we store it as `hub_pane_bounds`.

Update Task 4's helper: the header offset is already excluded by the canvas bounds. Only padding needs to be accounted for.

**Step 2: Fix the helper from Task 4**

The helper should be:

```rust
fn terminal_content_bounds_and_scroll(
    &self,
    session_id: u64,
) -> (Bounds<Pixels>, gpui::Point<Pixels>) {
    if self.hub_tab_active {
        if let Some(pane_bounds) = self.hub_pane_bounds.get(&session_id) {
            // hub_pane_bounds comes from the canvas inside hub-terminal-content-{id},
            // which is already below the 24px header. But the terminal content has
            // px_2 (8px each side) and pt_1 (4px top) padding.
            let h_pad = px(8.);   // px_2 means 8px each side
            let v_pad = px(4.);   // pt_1
            let content_origin = gpui::Point {
                x: pane_bounds.origin.x + h_pad,
                y: pane_bounds.origin.y + v_pad,
            };
            let content_size = gpui::Size {
                width: pane_bounds.size.width - h_pad * 2.0,
                height: pane_bounds.size.height - v_pad,
            };
            let content_bounds = Bounds {
                origin: content_origin,
                size: content_size,
            };

            let scroll_offset = self
                .hub_pane_scroll_handles
                .get(&session_id)
                .map(|h| h.offset())
                .unwrap_or_default();

            return (content_bounds, scroll_offset);
        }
    }

    let bounds = self.terminal_scroll_handle.bounds();
    let offset = self.terminal_scroll_handle.offset();
    (bounds, offset)
}
```

This is a correction to Task 4 — use this version instead.

**Step 3: No separate commit needed — this replaces Task 4's helper implementation.**

---

### Task 6: Implement viewport-limited rendering for hub panes

**Files:**
- Modify: `crates/arbor-gui/src/hub_rendering.rs:362-390` and `704-723`

**Step 1: Write test for viewport line slicing**

Add to `terminal_rendering.rs` tests:

```rust
#[test]
fn viewport_line_range_clamps_to_line_count() {
    // 300px viewport / 19px line height = ~15 visible lines
    // With 2-line buffer on each side = 19 lines total
    let viewport_height = 300.0_f32;
    let line_height = 19.0_f32;
    let scroll_offset_y = 0.0_f32;  // Not scrolled
    let total_lines = 50_usize;

    let (start, end) =
        visible_line_range(viewport_height, line_height, scroll_offset_y, total_lines);
    assert_eq!(start, 0);
    // ceil(300/19) = 16 visible + 2 buffer = 18, but start=0 so end = 18
    assert!(end <= 20);  // reasonable upper bound
    assert!(end < total_lines);
}

#[test]
fn viewport_line_range_with_scroll_offset() {
    let viewport_height = 300.0_f32;
    let line_height = 19.0_f32;
    // Scrolled down 190px (10 lines)
    let scroll_offset_y = -190.0_f32;
    let total_lines = 100_usize;

    let (start, end) =
        visible_line_range(viewport_height, line_height, scroll_offset_y, total_lines);
    // First visible line should be around 10
    assert!(start >= 8);   // 10 - 2 buffer
    assert!(start <= 10);
    // Last visible should be around 10 + 16 + 2 = 28
    assert!(end >= 26);
    assert!(end <= 30);
}
```

**Step 2: Run tests to verify they fail**

Run: `just test -- visible_line_range`
Expected: FAIL — function doesn't exist yet

**Step 3: Implement `visible_line_range` function**

Add to `terminal_rendering.rs` (before the test module):

```rust
/// Calculate the range of lines visible in the viewport, plus a buffer.
/// `scroll_offset_y` is negative when scrolled down (GPUI convention).
/// Returns (start_line, end_line) as a half-open range.
pub(crate) fn visible_line_range(
    viewport_height: f32,
    line_height: f32,
    scroll_offset_y: f32,
    total_lines: usize,
) -> (usize, usize) {
    if line_height <= 0. || total_lines == 0 {
        return (0, 0);
    }

    let buffer_lines = 2_usize;
    // scroll_offset_y is negative when scrolled down
    let first_visible = ((-scroll_offset_y) / line_height).floor() as usize;
    let visible_count = (viewport_height / line_height).ceil() as usize + 1;
    let start = first_visible.saturating_sub(buffer_lines);
    let end = (first_visible + visible_count + buffer_lines).min(total_lines);

    (start, end)
}
```

**Step 4: Run tests to verify they pass**

Run: `just test -- visible_line_range`
Expected: PASS

**Step 5: Commit**

```
feat(gui): add visible_line_range for viewport-limited terminal rendering
```

---

### Task 7: Use viewport-limited rendering in hub panes

**Files:**
- Modify: `crates/arbor-gui/src/hub_rendering.rs:362-390` and `704-723`

**Step 1: Get scroll offset and viewport height for line slicing**

In `render_hub_terminal_pane()`, after computing `styled_lines`, get the scroll offset:

```rust
let pane_scroll_offset = self
    .hub_pane_scroll_handles
    .get(&terminal_id)
    .map(|h| f32::from(h.offset().y))
    .unwrap_or(0.);
let viewport_height = self
    .hub_pane_grid_sizes
    .get(&terminal_id)
    .map(|(_, _, _, ph)| *ph as f32)
    .unwrap_or(300.);
let (vis_start, vis_end) =
    visible_line_range(viewport_height, line_height, pane_scroll_offset, styled_lines.len());
```

**Step 2: Slice lines and add spacer for scrollbar accuracy**

Replace the children rendering (around line 704-723) which currently renders all `styled_lines`:

Instead of:
```rust
.children(styled_lines.into_iter().map(|line| { ... }))
```

Use:
```rust
// Spacer for lines above viewport (preserves scroll position)
.child(
    div()
        .flex_none()
        .w_full()
        .h(px(vis_start as f32 * line_height))
)
.children(
    styled_lines
        .into_iter()
        .skip(vis_start)
        .take(vis_end - vis_start)
        .map(|line| {
            render_terminal_line_with_font_size(
                line,
                theme,
                cell_width,
                line_height,
                mono_font.clone(),
                font_size,
            )
        }),
)
// Spacer for lines below viewport (preserves scrollbar size)
.child(
    div()
        .flex_none()
        .w_full()
        .h(px(total_lines.saturating_sub(vis_end) as f32 * line_height))
)
```

Where `total_lines` is `styled_lines.len()` captured before the `into_iter()`.

**Step 3: Run format and lint**

Run: `just format && just lint`
Expected: PASS

**Step 4: Run tests**

Run: `just test`
Expected: PASS

**Step 5: Commit**

```
perf(gui): viewport-limited rendering for hub terminal panes
```

---

### Task 8: Clean up stale scroll handles on terminal removal

**Files:**
- Modify: `crates/arbor-gui/src/hub_rendering.rs` (find `hub_prune_stale_terminals`)

**Step 1: Add cleanup for scroll handles in `hub_prune_stale_terminals`**

In `hub_prune_stale_terminals()`, after the existing cleanup loop, add:

```rust
for id in &stale_ids {
    self.hub_grid.remove_terminal(*id);
    self.hub_layout.remove_terminal(*id);
    self.hub_pane_scroll_handles.remove(id);  // ADD THIS
}
```

Also check `hub_remove_terminal` (search for it) and add cleanup there too.

**Step 2: Run format, lint, and test**

Run: `just format && just lint && just test`
Expected: PASS

**Step 3: Commit**

```
fix(gui): clean up per-pane scroll handles when hub terminals are removed
```

---

### Task 9: Final verification

**Step 1: Run the full CI suite**

Run: `just ci`
Expected: PASS — format, lint, and all tests pass.

**Step 2: Visual verification**

Run: `just run`

Test manually:
1. Open Agent Hub with multiple terminal panes
2. Click and drag to select text in a pane — cursor should align with text
3. Scroll one pane, then select text in another — selection should not be offset
4. Scroll down in a long-output pane — scrolling should be smooth (viewport-limited rendering)
5. Close panes — no panics or stale state

**Step 3: Push**

```bash
git push mine feat/agent-hub-sidebar
```
