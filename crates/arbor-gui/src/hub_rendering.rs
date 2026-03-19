use {super::*, gpui::relative};

impl ArborWindow {
    /// Remove terminals from the hub layout that no longer exist in self.terminals.
    #[allow(dead_code)]
    pub(crate) fn hub_prune_stale_terminals(&mut self) {
        let stale_ids: Vec<u64> = self
            .hub_layout
            .terminal_ids()
            .into_iter()
            .filter(|id| !self.terminals.iter().any(|t| t.id == *id))
            .collect();
        for id in stale_ids {
            self.hub_layout.remove_terminal(id);
        }
    }

    /// Render the full hub view — a recursive split layout of terminal panes.
    pub(crate) fn render_hub_view(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = self.theme();
        let layout = self.hub_layout.clone();

        div()
            .id("hub-view")
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .bg(rgb(theme.terminal_bg))
            .child(self.render_hub_node(&layout, Vec::new(), cx))
    }

    /// Recursively render a node in the hub split tree.
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

        // Build styled lines
        let selection = self.terminal_selection_for_session(session.id);
        let ime_text = self.ime_marked_text.as_deref();
        let styled_lines =
            styled_lines_for_session(session, theme, is_focused, selection, ime_text);
        let mono_font = terminal_mono_font(cx);
        let cell_width = terminal_cell_width_px(cx);
        let line_height = terminal_line_height_px(cx);

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
            .on_click(cx.listener(move |this, _, window, cx| {
                this.hub_focus_terminal(tid, window, cx);
            }))
            // Pane header bar
            .child(
                div()
                    .w_full()
                    .flex_none()
                    .h(px(24.))
                    .px_2()
                    .flex()
                    .items_center()
                    .gap(px(6.))
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
                    ),
            )
            // Terminal output
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .min_w_0()
                    .min_h_0()
                    .overflow_hidden()
                    .font(mono_font.clone())
                    .text_size(px(TERMINAL_FONT_SIZE_PX))
                    .line_height(px(line_height))
                    .px_2()
                    .pt_1()
                    .flex()
                    .flex_col()
                    .gap_0()
                    .child(
                        div()
                            .id(ElementId::Name(
                                format!("hub-terminal-scroll-{terminal_id}").into(),
                            ))
                            .flex_1()
                            .w_full()
                            .min_w_0()
                            .min_h_0()
                            .overflow_x_hidden()
                            .overflow_y_scroll()
                            .scrollbar_width(px(TERMINAL_SCROLLBAR_WIDTH_PX))
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .flex_none()
                                    .flex()
                                    .flex_col()
                                    .gap_0()
                                    .children(styled_lines.into_iter().map(|line| {
                                        render_terminal_line(
                                            line,
                                            theme,
                                            cell_width,
                                            line_height,
                                            mono_font.clone(),
                                        )
                                    })),
                            ),
                    ),
            )
    }

    /// Render a draggable divider between hub split panes.
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

    /// Handle drag events on hub dividers to resize split ratios.
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
        if self.hub_active_terminal_id == Some(terminal_id) {
            return;
        }
        self.hub_active_terminal_id = Some(terminal_id);

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
        if self.hub_layout.contains_terminal(terminal_id) {
            self.hub_active_terminal_id = Some(terminal_id);
            cx.notify();
            return;
        }
        self.hub_layout.add_terminal(terminal_id);
        self.hub_active_terminal_id = Some(terminal_id);
        self.sync_hub_layout_store(cx);
        cx.notify();
    }
}
