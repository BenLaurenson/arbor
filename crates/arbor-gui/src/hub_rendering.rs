use {super::*, gpui::relative};

impl ArborWindow {
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
        _path: Vec<usize>,
        _ratio: f32,
        _cx: &mut Context<Self>,
    ) -> Div {
        let theme = self.theme();

        if is_horizontal {
            // Vertical divider (between left and right panes)
            div()
                .flex_none()
                .w(px(4.))
                .h_full()
                .cursor_col_resize()
                .flex()
                .items_center()
                .justify_center()
                .child(div().w(px(1.)).h_full().bg(rgb(theme.border)))
        } else {
            // Horizontal divider (between top and bottom panes)
            div()
                .flex_none()
                .h(px(4.))
                .w_full()
                .cursor_row_resize()
                .flex()
                .items_center()
                .justify_center()
                .child(div().h(px(1.)).w_full().bg(rgb(theme.border)))
        }
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
        cx.notify();
    }
}
