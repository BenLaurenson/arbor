use {super::*, gpui::relative};

/// Compute which drop zone the mouse is over within a pane's bounds.
/// Uses 25% edge threshold (Zed's default): outer 25% of each edge
/// triggers a directional split, center 50% triggers a swap/merge.
fn compute_drop_zone(mouse: gpui::Point<Pixels>, bounds: Bounds<Pixels>) -> HubDropZone {
    let x = mouse.x - bounds.origin.x;
    let y = mouse.y - bounds.origin.y;
    let w = bounds.size.width;
    let h = bounds.size.height;

    if w <= px(0.) || h <= px(0.) {
        return HubDropZone::Center;
    }

    let edge_threshold = 0.25;
    let edge_w = w * edge_threshold;
    let edge_h = h * edge_threshold;

    // Check if in center zone (not near any edge)
    if x > edge_w && x < w - edge_w && y > edge_h && y < h - edge_h {
        return HubDropZone::Center;
    }

    // Determine closest edge — use normalized distances
    let dist_left = x / w;
    let dist_right = (w - x) / w;
    let dist_top = y / h;
    let dist_bottom = (h - y) / h;

    let min = dist_left.min(dist_right).min(dist_top).min(dist_bottom);

    if min == dist_left {
        HubDropZone::Left
    } else if min == dist_right {
        HubDropZone::Right
    } else if min == dist_top {
        HubDropZone::Top
    } else {
        HubDropZone::Bottom
    }
}

impl ArborWindow {
    /// Remove terminals from the hub that no longer exist in self.terminals.
    pub(crate) fn hub_prune_stale_terminals(&mut self) {
        let stale_ids: Vec<u64> = self
            .hub_grid
            .terminals
            .iter()
            .copied()
            .filter(|id| !self.terminals.iter().any(|t| t.id == *id))
            .collect();
        for id in &stale_ids {
            self.hub_grid.remove_terminal(*id);
            self.hub_layout.remove_terminal(*id);
            self.hub_pane_scroll_handles.remove(id);
        }
    }

    /// Render the hub view as an equal-sized grid with pagination.
    pub(crate) fn render_hub_view(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = self.theme();

        // Maximized mode: single terminal fills the entire hub
        if let Some(max_tid) = self.hub_maximized_terminal {
            return div()
                .id("hub-view")
                .size_full()
                .min_w_0()
                .min_h_0()
                .overflow_hidden()
                .bg(rgb(theme.terminal_bg))
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .min_h_0()
                        .overflow_hidden()
                        .child(self.render_hub_terminal_pane(max_tid, cx)),
                );
        }

        let page_terminals = self.hub_grid.page_terminals().to_vec();
        let page_count = self.hub_grid.page_count();
        let current_page = self.hub_grid.current_page;
        let (rows, cols) = hub_grid::grid_dimensions(page_terminals.len());

        let mut hub = div()
            .id("hub-view")
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .bg(rgb(theme.terminal_bg))
            .flex()
            .flex_col();

        if page_terminals.is_empty() {
            hub = hub.child(
                div().flex_1().flex().items_center().justify_center().child(
                    div()
                        .text_sm()
                        .text_color(rgb(theme.text_disabled))
                        .child("Click a session to open it here"),
                ),
            );
        } else {
            // Grid of terminal panes — equal-sized rows and columns
            let mut grid = div().flex_1().flex().flex_col().gap(px(2.));

            for row in 0..rows {
                let mut row_div = div().flex_1().flex().flex_row().gap(px(2.)).min_h_0();

                for col in 0..cols {
                    let idx = row * cols + col;
                    if idx < page_terminals.len() {
                        row_div = row_div.child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .min_h_0()
                                .overflow_hidden()
                                .child(self.render_hub_terminal_pane(page_terminals[idx], cx)),
                        );
                    } else {
                        // Empty cell to maintain grid structure
                        row_div = row_div.child(div().flex_1().min_w_0().min_h_0());
                    }
                }

                grid = grid.child(row_div);
            }

            hub = hub.child(grid);
        }

        // Page navigation bar: ◀ ● ○ ○ ▶  Page 1/3
        if page_count > 1 {
            let mut page_bar = div()
                .flex_none()
                .h(px(24.))
                .w_full()
                .px_2()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(8.))
                .bg(rgb(theme.panel_bg));

            // Previous page button
            page_bar = page_bar.child(
                div()
                    .id("hub-page-prev")
                    .cursor_pointer()
                    .text_xs()
                    .text_color(rgb(theme.text_muted))
                    .hover(|this| this.text_color(rgb(theme.text_primary)))
                    .child("\u{25c0}") // ◀
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.hub_grid.prev_page();
                        cx.notify();
                    })),
            );

            // Page dots
            for page in 0..page_count {
                let is_current = page == current_page;
                page_bar = page_bar.child(
                    div()
                        .id(ElementId::Name(format!("hub-page-dot-{page}").into()))
                        .cursor_pointer()
                        .w(px(if is_current {
                            8.
                        } else {
                            6.
                        }))
                        .h(px(if is_current {
                            8.
                        } else {
                            6.
                        }))
                        .rounded_full()
                        .bg(rgb(if is_current {
                            theme.accent
                        } else {
                            theme.text_disabled
                        }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.hub_grid.go_to_page(page);
                            cx.notify();
                        })),
                );
            }

            // Next page button
            page_bar = page_bar.child(
                div()
                    .id("hub-page-next")
                    .cursor_pointer()
                    .text_xs()
                    .text_color(rgb(theme.text_muted))
                    .hover(|this| this.text_color(rgb(theme.text_primary)))
                    .child("\u{25b6}") // ▶
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.hub_grid.next_page();
                        cx.notify();
                    })),
            );

            // Page count label
            page_bar = page_bar.child(
                div()
                    .text_xs()
                    .text_color(rgb(theme.text_disabled))
                    .child(format!("{}/{}", current_page + 1, page_count)),
            );

            hub = hub.child(page_bar);
        }

        hub
    }

    /// Recursively render a node in the hub split tree (legacy, used by old layout).
    #[allow(dead_code)]
    fn render_hub_node(
        &self,
        pane: &hub_layout::HubPane,
        path: Vec<usize>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme();

        match pane {
            hub_layout::HubPane::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let ratio_val = *ratio;
                let is_horizontal = *direction == hub_layout::SplitDirection::Horizontal;
                let mut first_path = path.clone();
                first_path.push(0);
                let mut second_path = path.clone();
                second_path.push(1);
                let drag_path = path.clone();

                let first_child = self.render_hub_node(first, first_path, cx);
                let second_child = self.render_hub_node(second, second_path, cx);

                // Divider between panes
                let divider = self.render_hub_divider(is_horizontal, drag_path, ratio_val, cx);

                if is_horizontal {
                    div()
                        .size_full()
                        .min_w_0()
                        .min_h_0()
                        .flex()
                        .flex_row()
                        .overflow_hidden()
                        .child(
                            div()
                                .min_w_0()
                                .min_h_0()
                                .overflow_hidden()
                                .flex_basis(relative(ratio_val))
                                .flex_shrink()
                                .flex_grow()
                                .child(first_child),
                        )
                        .child(divider)
                        .child(
                            div()
                                .min_w_0()
                                .min_h_0()
                                .overflow_hidden()
                                .flex_basis(relative(1.0 - ratio_val))
                                .flex_shrink()
                                .flex_grow()
                                .child(second_child),
                        )
                        .into_any_element()
                } else {
                    div()
                        .size_full()
                        .min_w_0()
                        .min_h_0()
                        .flex()
                        .flex_col()
                        .overflow_hidden()
                        .child(
                            div()
                                .min_w_0()
                                .min_h_0()
                                .overflow_hidden()
                                .flex_basis(relative(ratio_val))
                                .flex_shrink()
                                .flex_grow()
                                .child(first_child),
                        )
                        .child(divider)
                        .child(
                            div()
                                .min_w_0()
                                .min_h_0()
                                .overflow_hidden()
                                .flex_basis(relative(1.0 - ratio_val))
                                .flex_shrink()
                                .flex_grow()
                                .child(second_child),
                        )
                        .into_any_element()
                }
            },

            hub_layout::HubPane::Terminal(terminal_id) => self
                .render_hub_terminal_pane(*terminal_id, cx)
                .into_any_element(),

            hub_layout::HubPane::Empty => div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(theme.text_disabled))
                        .child("Click a session to open it here"),
                )
                .into_any_element(),
        }
    }

    /// Render a single terminal pane within the hub.
    fn render_hub_terminal_pane(&self, terminal_id: u64, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = self.theme();
        let is_focused = self.hub_active_terminal_id == Some(terminal_id);

        let Some(session) = self.terminals.iter().find(|t| t.id == terminal_id) else {
            // Terminal not found — show placeholder
            return div()
                .id(ElementId::Name(
                    format!("hub-pane-gone-{terminal_id}").into(),
                ))
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(theme.text_disabled))
                        .child("Terminal session ended"),
                );
        };

        // Build styled lines, truncated to pane width
        let selection = self.terminal_selection_for_session(session.id);
        let ime_text = self.ime_marked_text.as_deref();
        let scroll_handle = self.hub_pane_scroll_handles.get(&terminal_id).cloned();
        let all_lines = styled_lines_for_session(session, theme, is_focused, selection, ime_text);
        let pane_cols = self
            .hub_pane_grid_sizes
            .get(&terminal_id)
            .map(|(_, cols, ..)| *cols as usize)
            .unwrap_or(120);
        let styled_lines: Vec<_> = all_lines
            .into_iter()
            .map(|mut line| {
                // Truncate cells to pane column count to prevent overflow
                line.cells.truncate(pane_cols);
                line.runs = line
                    .runs
                    .into_iter()
                    .map(|mut run| {
                        // Truncate by character count, not byte count
                        let char_count: usize = run.text.chars().count();
                        if char_count > pane_cols {
                            run.text = run.text.chars().take(pane_cols).collect();
                        }
                        run
                    })
                    .collect();
                line
            })
            .collect();
        let mono_font = terminal_mono_font(cx);
        let scale = self.terminal_font_scale;
        let cell_width = terminal_cell_width_px(cx) * scale;
        let line_height = terminal_line_height_px(cx) * scale;
        let font_size = TERMINAL_FONT_SIZE_PX * scale;

        // Pane header: shows session name/branch + worktree
        let pane_title = session
            .last_command
            .as_deref()
            .or(Some(&session.title))
            .unwrap_or("terminal")
            .to_owned();
        let worktree_label = session
            .worktree_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_owned();

        let border_color = if is_focused {
            theme.accent
        } else {
            theme.border
        };

        let tid = terminal_id;
        let tid_ctx = terminal_id;

        div()
            .id(ElementId::Name(format!("hub-pane-{terminal_id}").into()))
            .size_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .border_1()
            .border_color(rgb(border_color))
            .rounded_sm()
            .cursor_pointer()
            .relative()
            .on_click(cx.listener(move |this, _, window, cx| {
                this.hub_focus_terminal(tid, window, cx);
            }))
            // Drop target: accept DraggedHubPane payloads with zone detection
            .on_drop(cx.listener(move |this, dragged: &DraggedHubPane, _, cx| {
                this.hub_handle_drop(terminal_id, dragged, cx);
            }))
            .on_drag_move(cx.listener(
                move |this, event: &DragMoveEvent<DraggedHubPane>, _, cx| {
                    let drag = event.drag(cx);
                    if drag.terminal_id == terminal_id {
                        if this.hub_drop_target.is_some() {
                            this.hub_drop_target = None;
                            cx.notify();
                        }
                        return;
                    }
                    let mouse = event.event.position;
                    let zone = if let Some(bounds) = this.hub_pane_bounds.get(&terminal_id) {
                        compute_drop_zone(mouse, *bounds)
                    } else {
                        HubDropZone::Center
                    };
                    let changed = this
                        .hub_drop_target
                        .as_ref()
                        .map(|t| (t.terminal_id, t.zone))
                        != Some((terminal_id, zone));
                    if changed {
                        this.hub_drop_target = Some(HubDropTarget {
                            terminal_id,
                            zone,
                        });
                        cx.notify();
                    }
                },
            ))
            // Drop zone overlay
            .when_some(
                self.hub_drop_target
                    .as_ref()
                    .filter(|dt| dt.terminal_id == terminal_id)
                    .map(|dt| dt.zone),
                |this, zone| {
                    this.child(
                        div()
                            .absolute()
                            .top(relative(match zone {
                                HubDropZone::Bottom => 0.5,
                                _ => 0.0,
                            }))
                            .left(relative(match zone {
                                HubDropZone::Right => 0.5,
                                _ => 0.0,
                            }))
                            .w(relative(match zone {
                                HubDropZone::Left | HubDropZone::Right => 0.5,
                                _ => 1.0,
                            }))
                            .h(relative(match zone {
                                HubDropZone::Top | HubDropZone::Bottom => 0.5,
                                _ => 1.0,
                            }))
                            .bg(gpui::rgba(0x61afef33))
                            .border_2()
                            .border_color(gpui::rgba(0x61afef99))
                            .rounded_sm(),
                    )
                },
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.hub_pane_context_menu = Some(HubPaneContextMenu {
                        terminal_id: tid_ctx,
                        position: event.position,
                    });
                    cx.notify();
                }),
            )
            // Pane header bar — draggable for rearranging
            .child(
                div()
                    .id(ElementId::Name(
                        format!("hub-pane-header-{terminal_id}").into(),
                    ))
                    .w_full()
                    .flex_none()
                    .h(px(24.))
                    .px_2()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .cursor_grab()
                    .on_drag(
                        DraggedHubPane { terminal_id },
                        |dragged, _, _, cx| {
                            cx.stop_propagation();
                            cx.new(|_| dragged.clone())
                        },
                    )
                    .bg(rgb(if is_focused {
                        theme.panel_active_bg
                    } else {
                        theme.panel_bg
                    }))
                    .border_b_1()
                    .border_color(rgb(theme.border))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(if is_focused {
                                theme.text_primary
                            } else {
                                theme.text_muted
                            }))
                            .child(pane_title),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_xs()
                            .text_color(rgb(theme.text_disabled))
                            .child(worktree_label),
                    )
                    // Maximize/minimize button
                    .child({
                        let max_tid = terminal_id;
                        let is_maximized = self.hub_maximized_terminal == Some(terminal_id);
                        div()
                            .id(ElementId::Name(
                                format!("hub-pane-max-{terminal_id}").into(),
                            ))
                            .cursor_pointer()
                            .flex_none()
                            .text_xs()
                            .text_color(rgb(theme.text_disabled))
                            .hover(|this| this.text_color(rgb(theme.text_primary)))
                            .child(if is_maximized {
                                "\u{f066}" // compress icon
                            } else {
                                "\u{f065}" // expand icon
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this.hub_maximized_terminal == Some(max_tid) {
                                    this.hub_maximized_terminal = None;
                                } else {
                                    this.hub_maximized_terminal = Some(max_tid);
                                }
                                cx.notify();
                                cx.stop_propagation();
                            }))
                    })
                    // Close pane button on header
                    .child({
                        let close_tid = terminal_id;
                        div()
                            .id(ElementId::Name(
                                format!("hub-pane-close-{terminal_id}").into(),
                            ))
                            .cursor_pointer()
                            .flex_none()
                            .text_xs()
                            .text_color(rgb(theme.text_disabled))
                            .hover(|this| this.text_color(rgb(0xeb6f92)))
                            .child("\u{00d7}") // ×
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.hub_remove_terminal(close_tid, cx);
                                cx.stop_propagation();
                            }))
                    }),
            )
            // Terminal output with canvas bounds measurement for PTY resize
            .child({
                let entity = cx.entity().downgrade();
                let cw = cell_width;
                let lh = line_height;

                div()
                    .id(ElementId::Name(
                        format!("hub-terminal-content-{terminal_id}").into(),
                    ))
                    .flex_1()
                    .w_full()
                    .min_w_0()
                    .min_h_0()
                    .overflow_hidden()
                    .relative()
                    // Canvas measures the VIEWPORT (fixed parent), not scroll content
                    .child(
                        canvas(
                            move |bounds, _window, cx| {
                                let width = (bounds.size.width.to_f64() as f32
                                    - TERMINAL_SCROLLBAR_WIDTH_PX
                                    - 16.0) // px_2 padding
                                    .max(1.0);
                                let height =
                                    (bounds.size.height.to_f64() as f32 - 4.0).max(1.0);
                                if let Some((rows, cols)) =
                                    terminal_grid_size_for_viewport(width, height, cw, lh)
                                {
                                    let pw =
                                        width.floor().clamp(1., f32::from(u16::MAX)) as u16;
                                    let ph =
                                        height.floor().clamp(1., f32::from(u16::MAX)) as u16;
                                    if let Some(entity) = entity.upgrade() {
                                        entity.update(
                                            cx,
                                            |this: &mut ArborWindow, _cx| {
                                                this.hub_pane_grid_sizes.insert(
                                                    terminal_id,
                                                    (rows, cols, pw, ph),
                                                );
                                                this.hub_pane_bounds
                                                    .insert(terminal_id, bounds);
                                            },
                                        );
                                    }
                                }
                            },
                            |_, _, _, _| {},
                        )
                        .size_full()
                        .absolute()
                        .inset_0(),
                    )
                    // Terminal content on top of the measurement canvas
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .overflow_hidden()
                            .font(mono_font.clone())
                            .text_size(px(font_size))
                            .line_height(px(line_height))
                            .px_2()
                            .pt_1()
                            .cursor_text()
                            .child(
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
                                    .when_some(scroll_handle, |el, handle| {
                                        el.track_scroll(&handle)
                                    })
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(
                                            Self::handle_terminal_output_mouse_down,
                                        ),
                                    )
                                    .on_mouse_move(cx.listener(
                                        Self::handle_terminal_output_mouse_move,
                                    ))
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(
                                            Self::handle_terminal_output_mouse_up,
                                        ),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .min_w_0()
                                            .flex_none()
                                            .flex()
                                            .flex_col()
                                            .gap_0()
                                            .children(styled_lines.into_iter().map(
                                                |line| {
                                                    render_terminal_line_with_font_size(
                                                        line,
                                                        theme,
                                                        cell_width,
                                                        line_height,
                                                        mono_font.clone(),
                                                        font_size,
                                                    )
                                                },
                                            )),
                                    ),
                            ),
                    )
            })
    }

    #[allow(dead_code)]
    fn render_hub_divider(
        &self,
        is_horizontal: bool,
        path: Vec<usize>,
        _ratio: f32,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let theme = self.theme();
        let divider_id = ElementId::Name(
            format!(
                "hub-divider-{}",
                path.iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join("-")
            )
            .into(),
        );
        let drag_path = path.clone();

        if is_horizontal {
            div()
                .id(divider_id)
                .flex_none()
                .w(px(4.))
                .h_full()
                .cursor_col_resize()
                .flex()
                .items_center()
                .justify_center()
                .on_drag(
                    DraggedHubDivider {
                        path: drag_path,
                        is_horizontal: true,
                    },
                    |dragged, _, _, cx| {
                        cx.stop_propagation();
                        cx.new(|_| dragged.clone())
                    },
                )
                .on_drag_move(cx.listener(
                    move |this, event: &DragMoveEvent<DraggedHubDivider>, _, cx| {
                        this.handle_hub_divider_drag(event, cx);
                    },
                ))
                .child(div().w(px(1.)).h_full().bg(rgb(theme.border)))
        } else {
            div()
                .id(divider_id)
                .flex_none()
                .h(px(4.))
                .w_full()
                .cursor_row_resize()
                .flex()
                .items_center()
                .justify_center()
                .on_drag(
                    DraggedHubDivider {
                        path,
                        is_horizontal: false,
                    },
                    |dragged, _, _, cx| {
                        cx.stop_propagation();
                        cx.new(|_| dragged.clone())
                    },
                )
                .on_drag_move(cx.listener(
                    move |this, event: &DragMoveEvent<DraggedHubDivider>, _, cx| {
                        this.handle_hub_divider_drag(event, cx);
                    },
                ))
                .child(div().h(px(1.)).w_full().bg(rgb(theme.border)))
        }
    }

    #[allow(dead_code)]
    fn handle_hub_divider_drag(
        &mut self,
        event: &DragMoveEvent<DraggedHubDivider>,
        cx: &mut Context<Self>,
    ) {
        let drag = event.drag(cx);
        // Use mouse position relative to the center pane to compute new ratio.
        // The ratio is clamped in set_ratio_at_path to [0.15, 0.85].
        let mouse_pos = event.event.position;
        let ratio = if drag.is_horizontal {
            let left_edge = px(self.left_pane_width + PANE_RESIZE_HANDLE_WIDTH);
            let right_edge = px(self.left_pane_width + PANE_RESIZE_HANDLE_WIDTH + 800.0);
            let center_width = right_edge - left_edge;
            if center_width > px(0.) {
                ((mouse_pos.x - left_edge) / center_width).clamp(0.15, 0.85)
            } else {
                0.5
            }
        } else {
            let top_edge = px(60.0); // top bar height
            let center_height = px(600.0); // approximate
            if center_height > px(0.) {
                ((mouse_pos.y - top_edge) / center_height).clamp(0.15, 0.85)
            } else {
                0.5
            }
        };

        self.hub_layout.set_ratio_at_path(&drag.path, ratio);
        cx.notify();
    }

    /// Focus a terminal in the hub — updates active terminal and syncs worktree context.
    pub(crate) fn hub_focus_terminal(
        &mut self,
        terminal_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.hub_active_terminal_id = Some(terminal_id);

        // Navigate to the page containing this terminal
        if let Some(page) = self.hub_grid.page_for_terminal(terminal_id) {
            self.hub_grid.current_page = page;
        }

        // Sync worktree context to match the focused terminal
        if let Some(session) = self.terminals.iter().find(|t| t.id == terminal_id) {
            let worktree_path = session.worktree_path.clone();
            if let Some(index) = self.worktrees.iter().position(|w| w.path == worktree_path)
                && self.active_worktree_index != Some(index)
            {
                self.select_worktree(index, window, cx);
            }
        }

        window.focus(&self.terminal_focus);
        cx.notify();
    }

    /// Add a terminal to the hub layout. If the terminal is already present,
    /// focus it instead.
    pub(crate) fn hub_add_terminal(&mut self, terminal_id: u64, cx: &mut Context<Self>) {
        if self.hub_grid.contains_terminal(terminal_id) {
            self.hub_active_terminal_id = Some(terminal_id);
            // Navigate to the page containing this terminal
            if let Some(page) = self.hub_grid.page_for_terminal(terminal_id) {
                self.hub_grid.current_page = page;
            }
            cx.notify();
            return;
        }
        self.hub_grid.add_terminal(terminal_id);
        self.hub_pane_scroll_handles.entry(terminal_id).or_default();
        self.hub_layout.add_terminal(terminal_id); // keep old tree in sync for now
        self.hub_active_terminal_id = Some(terminal_id);
        self.sync_hub_layout_store(cx);
        cx.notify();
    }

    /// Remove a terminal from the hub, close the terminal session.
    pub(crate) fn hub_remove_terminal(&mut self, terminal_id: u64, cx: &mut Context<Self>) {
        // Remove from connected session tracking
        if let Some(session_id) = self.hub_terminal_to_session.remove(&terminal_id) {
            self.hub_connected_session_ids.remove(&session_id);
        }
        self.hub_pane_grid_sizes.remove(&terminal_id);
        self.hub_pane_bounds.remove(&terminal_id);
        self.hub_pane_scroll_handles.remove(&terminal_id);
        self.hub_grid.remove_terminal(terminal_id);
        self.hub_layout.remove_terminal(terminal_id);
        // Also close the actual terminal session
        self.close_terminal_session_by_id(terminal_id);
        if self.hub_active_terminal_id == Some(terminal_id) {
            self.hub_active_terminal_id = self.hub_grid.page_terminals().first().copied();
        }
        self.sync_hub_layout_store(cx);
        self.sync_daemon_session_store(cx);
        cx.notify();
    }

    /// Render the hub pane context menu (right-click on a terminal pane).
    pub(crate) fn render_hub_pane_context_menu(&mut self, cx: &mut Context<Self>) -> Div {
        let Some(menu) = self.hub_pane_context_menu.as_ref() else {
            return div();
        };

        let theme = self.theme();
        let position = menu.position;
        let terminal_id = menu.terminal_id;

        let menu_item = |id: &'static str, icon: &'static str, label: &'static str, color: u32| {
            div()
                .id(id)
                .h(px(30.))
                .mx(px(4.))
                .px(px(8.))
                .rounded_sm()
                .cursor_pointer()
                .hover(|this| this.bg(rgb(theme.panel_active_bg)))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_size(px(14.))
                        .text_color(rgb(color))
                        .child(icon),
                )
                .child(div().text_size(px(13.)).text_color(rgb(color)).child(label))
        };

        div()
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.hub_pane_context_menu = None;
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _, _, cx| {
                    this.hub_pane_context_menu = None;
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .child(
                div()
                    .absolute()
                    .left(position.x)
                    .top(position.y)
                    .w(px(180.))
                    .py(px(4.))
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(theme.border))
                    .bg(rgb(theme.chrome_bg))
                    .on_mouse_move(|_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                    // Split Right
                    .child(
                        menu_item(
                            "hub-ctx-split-right",
                            "\u{f105}", // arrow right
                            "Split Right",
                            theme.text_primary,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.hub_pane_context_menu = None;
                            this.hub_split_pane(
                                terminal_id,
                                hub_layout::DropZone::Right,
                                cx,
                            );
                        })),
                    )
                    // Split Down
                    .child(
                        menu_item(
                            "hub-ctx-split-down",
                            "\u{f107}", // arrow down
                            "Split Down",
                            theme.text_primary,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.hub_pane_context_menu = None;
                            this.hub_split_pane(
                                terminal_id,
                                hub_layout::DropZone::Bottom,
                                cx,
                            );
                        })),
                    )
                    // Divider
                    .child(div().h(px(1.)).mx(px(8.)).my(px(4.)).bg(rgb(theme.border)))
                    // Close Pane
                    .child(
                        menu_item(
                            "hub-ctx-close",
                            "\u{f00d}", // x icon
                            "Close Pane",
                            0xeb6f92,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.hub_pane_context_menu = None;
                            this.hub_remove_terminal(terminal_id, cx);
                        })),
                    ),
            )
    }

    /// Split a hub pane, creating an empty slot next to the target terminal.
    fn hub_split_pane(
        &mut self,
        terminal_id: u64,
        zone: hub_layout::DropZone,
        cx: &mut Context<Self>,
    ) {
        self.hub_layout.split_with_empty(terminal_id, zone);
        self.sync_hub_layout_store(cx);
        cx.notify();
    }

    /// Handle a drop of a DraggedHubPane onto a target pane.
    /// Uses the detected drop zone to determine split direction.
    fn hub_handle_drop(
        &mut self,
        target_id: u64,
        dragged: &DraggedHubPane,
        cx: &mut Context<Self>,
    ) {
        let source_id = dragged.terminal_id;
        self.hub_drop_target = None;
        if source_id == target_id {
            cx.notify();
            return;
        }

        // Swap positions in the grid
        self.hub_grid.swap_terminals(source_id, target_id);
        self.hub_active_terminal_id = Some(source_id);
        self.sync_hub_layout_store(cx);
        cx.notify();
    }
}
