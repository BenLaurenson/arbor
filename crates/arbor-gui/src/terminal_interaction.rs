use super::*;

pub(crate) fn should_queue_terminal_input(session: &TerminalSession) -> bool {
    session.runtime.is_none() && session.is_initializing
}

pub(crate) fn terminal_input_unavailable_error(session: &TerminalSession) -> TerminalError {
    TerminalError::Pty(format!("terminal `{}` is not available", session.title))
}

impl ArborWindow {
    pub(crate) fn active_terminal(&self) -> Option<&TerminalSession> {
        let worktree_path = self.selected_worktree_path()?;
        let session_id = self.active_terminal_id_for_worktree(worktree_path)?;

        self.terminals.iter().find(|session| {
            session.id == session_id && session.worktree_path.as_path() == worktree_path
        })
    }

    pub(crate) fn write_input_to_terminal(
        &mut self,
        session_id: u64,
        input: &[u8],
    ) -> Result<(), TerminalError> {
        if input.is_empty() {
            return Ok(());
        }

        let Some(index) = self
            .terminals
            .iter()
            .position(|session| session.id == session_id)
        else {
            return Ok(());
        };

        if should_queue_terminal_input(&self.terminals[index]) {
            self.terminals[index].queued_input.extend_from_slice(input);
            return Ok(());
        }

        let Some(runtime) = self.terminals[index].runtime.clone() else {
            return Err(terminal_input_unavailable_error(&self.terminals[index]));
        };

        {
            let session = &self.terminals[index];
            runtime.write_input(session, input)?;
        }

        self.terminals[index].updated_at_unix_ms = current_unix_timestamp_millis();
        Ok(())
    }

    pub(crate) fn flush_queued_input_for_terminal(
        &mut self,
        session_id: u64,
    ) -> Result<(), TerminalError> {
        let Some(index) = self
            .terminals
            .iter()
            .position(|session| session.id == session_id)
        else {
            return Ok(());
        };
        if self.terminals[index].queued_input.is_empty() {
            return Ok(());
        }
        if should_queue_terminal_input(&self.terminals[index]) {
            return Ok(());
        }

        let Some(runtime) = self.terminals[index].runtime.clone() else {
            self.terminals[index].queued_input.clear();
            return Err(terminal_input_unavailable_error(&self.terminals[index]));
        };

        let queued_input = std::mem::take(&mut self.terminals[index].queued_input);
        let session = self.terminals[index].clone();
        runtime.write_input(&session, &queued_input)?;
        self.terminals[index].updated_at_unix_ms = current_unix_timestamp_millis();
        Ok(())
    }

    pub(crate) fn clear_terminal_selection(&mut self) {
        self.terminal_selection = None;
        self.terminal_selection_drag_anchor = None;
    }

    pub(crate) fn clear_terminal_selection_for_session(&mut self, session_id: u64) {
        if self
            .terminal_selection
            .as_ref()
            .is_some_and(|selection| selection.session_id == session_id)
        {
            self.clear_terminal_selection();
        }
    }

    pub(crate) fn terminal_display_lines_for_session(&self, session_id: u64) -> Vec<String> {
        let Some(session) = self
            .terminals
            .iter()
            .find(|session| session.id == session_id)
        else {
            return vec![String::new()];
        };

        terminal_display_lines(session)
    }

    pub(crate) fn terminal_selection_for_session(
        &self,
        session_id: u64,
    ) -> Option<&TerminalSelection> {
        self.terminal_selection
            .as_ref()
            .filter(|selection| selection.session_id == session_id)
    }

    /// Returns (content_bounds, scroll_offset) for a terminal pane.
    /// For hub panes: adjusts canvas bounds by padding (px_2 = 8px each side, pt_1 = 4px top)
    /// and reads per-pane scroll offset. Returns zero-size bounds if hub pane bounds haven't
    /// been captured yet (first frame), which causes grid position to return None safely.
    /// For standalone terminals: uses the single terminal scroll handle.
    fn terminal_content_bounds_and_scroll(
        &self,
        session_id: u64,
    ) -> (Bounds<Pixels>, gpui::Point<Pixels>) {
        if self.hub_tab_active {
            let Some(pane_bounds) = self.hub_pane_bounds.get(&session_id) else {
                // Bounds not captured yet (first frame) — return zero-size so
                // terminal_grid_position_from_pointer returns None safely.
                return (Bounds::default(), gpui::Point::default());
            };

            // hub_pane_bounds comes from the canvas inside hub-terminal-content-{id},
            // which is already below the 24px header. But the terminal content has
            // px_2 (8px each side) and pt_1 (4px top) padding.
            let h_pad = px(8.);
            let v_pad = px(4.);
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

        // Standalone terminal view
        let bounds = self.terminal_scroll_handle.bounds();
        let offset = self.terminal_scroll_handle.offset();
        (bounds, offset)
    }

    pub(crate) fn handle_terminal_output_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }

        // Support both standalone terminal and hub terminal selection
        let session_id = if self.hub_tab_active {
            self.hub_active_terminal_id
        } else {
            self.active_terminal_id_for_selected_worktree()
        };
        let Some(session_id) = session_id else {
            return;
        };

        let lines = self.terminal_display_lines_for_session(session_id);
        let line_height = terminal_line_height_px(cx) * self.terminal_font_scale;
        let cell_width = terminal_cell_width_px(cx) * self.terminal_font_scale;
        let (scroll_bounds, scroll_offset) = self.terminal_content_bounds_and_scroll(session_id);
        let point = terminal_grid_position_from_pointer(
            event.position,
            scroll_bounds,
            scroll_offset,
            line_height,
            cell_width,
            lines.len(),
        );

        // Always focus the terminal on click, even if grid position can't be
        // computed (e.g. first frame before pane bounds are captured).
        window.focus(&self.terminal_focus);
        self.focus_terminal_on_next_render = false;

        let Some(point) = point else {
            cx.notify();
            return;
        };

        if event.click_count >= 3 {
            if let Some((start, end)) = terminal_line_bounds(&lines, point) {
                self.terminal_selection = Some(TerminalSelection {
                    session_id,
                    anchor: start,
                    head: end,
                });
            } else {
                self.clear_terminal_selection_for_session(session_id);
            }
            self.terminal_selection_drag_anchor = None;
        } else if event.click_count == 2 {
            if let Some((start, end)) = terminal_token_bounds(&lines, point) {
                self.terminal_selection = Some(TerminalSelection {
                    session_id,
                    anchor: start,
                    head: end,
                });
            } else {
                self.clear_terminal_selection_for_session(session_id);
            }
            self.terminal_selection_drag_anchor = None;
        } else {
            self.terminal_selection = Some(TerminalSelection {
                session_id,
                anchor: point,
                head: point,
            });
            self.terminal_selection_drag_anchor = Some(point);
        }

        cx.notify();
    }

    pub(crate) fn handle_terminal_output_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.pressed_button != Some(MouseButton::Left) {
            return;
        }

        let session_id = if self.hub_tab_active {
            self.hub_active_terminal_id
        } else {
            self.active_terminal_id_for_selected_worktree()
        };
        let Some(session_id) = session_id else {
            return;
        };
        let Some(anchor) = self.terminal_selection_drag_anchor else {
            return;
        };

        let lines = self.terminal_display_lines_for_session(session_id);
        let line_height = terminal_line_height_px(cx) * self.terminal_font_scale;
        let cell_width = terminal_cell_width_px(cx) * self.terminal_font_scale;
        let (scroll_bounds, scroll_offset) = self.terminal_content_bounds_and_scroll(session_id);
        let Some(head) = terminal_grid_position_from_pointer(
            event.position,
            scroll_bounds,
            scroll_offset,
            line_height,
            cell_width,
            lines.len(),
        ) else {
            return;
        };

        self.terminal_selection = Some(TerminalSelection {
            session_id,
            anchor,
            head,
        });
        cx.notify();
    }

    pub(crate) fn handle_terminal_output_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        if event.button == MouseButton::Left {
            self.terminal_selection_drag_anchor = None;
        }
    }

    pub(crate) fn track_terminal_command_input(&mut self, session_id: u64, keystroke: &Keystroke) {
        let Some(session) = self
            .terminals
            .iter_mut()
            .find(|session| session.id == session_id)
        else {
            return;
        };

        track_terminal_command_keystroke(session, keystroke);
    }

    pub(crate) fn copy_terminal_content_to_clipboard(
        &mut self,
        session_id: u64,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self
            .terminals
            .iter()
            .find(|session| session.id == session_id)
        else {
            return;
        };

        let clipboard_text =
            if let Some(selection) = self.terminal_selection_for_session(session_id) {
                let selected = terminal_selection_text(
                    &self.terminal_display_lines_for_session(session_id),
                    selection,
                );
                if !selected.is_empty() {
                    selected
                } else if !session.pending_command.trim().is_empty() {
                    session.pending_command.clone()
                } else {
                    session.output.clone()
                }
            } else if !session.pending_command.trim().is_empty() {
                session.pending_command.clone()
            } else {
                session.output.clone()
            };
        if clipboard_text.is_empty() {
            return;
        }

        cx.write_to_clipboard(ClipboardItem::new_string(clipboard_text));
    }

    pub(crate) fn append_pasted_text_to_pending_command(&mut self, session_id: u64, text: &str) {
        let Some(session) = self
            .terminals
            .iter_mut()
            .find(|session| session.id == session_id)
        else {
            return;
        };

        session.pending_command.push_str(text);
    }

    pub(crate) fn paste_clipboard_into_terminal(
        &mut self,
        session_id: u64,
        cx: &mut Context<Self>,
    ) {
        let Some(clipboard_item) = cx.read_from_clipboard() else {
            return;
        };
        let Some(text) = clipboard_item.text() else {
            return;
        };
        if text.is_empty() {
            return;
        }

        self.append_pasted_text_to_pending_command(session_id, &text);
        if let Err(error) = self.write_input_to_terminal(session_id, text.as_bytes()) {
            self.notice = Some(format!("failed to paste into terminal: {error}"));
        }
    }

    pub(crate) fn handle_terminal_key_down(
        &mut self,
        event: &KeyDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.quit_overlay_until.is_some() {
            // The quit overlay is modal — suppress terminal input entirely so
            // the key event propagates up to handle_global_key_down which
            // handles Enter/Escape for the overlay.
            return;
        }

        if self.new_tab_menu_position.is_some() {
            if event.keystroke.key.as_str() == "escape" {
                self.new_tab_menu_position = None;
                cx.stop_propagation();
                cx.notify();
            }
            return;
        }

        if self.command_palette_modal.is_some() {
            if event.keystroke.key.as_str() == "escape" {
                self.close_command_palette(cx);
                cx.stop_propagation();
                return;
            }
            // Keep all other keys out of the terminal while the palette is open
            // so they continue to route through the global modal handler.
            return;
        }

        if self.right_pane_search_active
            || self.worktree_notes_active
            || self.create_modal.is_some()
            || self.settings_modal.is_some()
            || self.github_auth_modal.is_some()
            || self.delete_modal.is_some()
            || self.manage_hosts_modal.is_some()
            || self.manage_presets_modal.is_some()
            || self.manage_repo_presets_modal.is_some()
            || self.daemon_auth_modal.is_some()
            || self.start_daemon_modal
            || self.connect_to_host_modal.is_some()
            || self.show_theme_picker
            || self.repo_settings_modal.is_some()
            || self.repo_icon_preview.is_some()
        {
            return;
        }

        let active_tab = self.active_center_tab_for_selected_worktree();

        // Handle file view editing before terminal input
        if matches!(active_tab, Some(CenterTab::FileView(_))) {
            if self.handle_file_view_key_down(event, cx) {
                cx.stop_propagation();
            }
            return;
        }

        // Handle agent chat input — only stop propagation for keys that were
        // actually consumed (Enter, Backspace, arrows, etc.).  Regular character
        // keys must flow through to the IME pipeline so that
        // `replace_text_in_range` inserts the typed text into the chat input.
        if let Some(CenterTab::AgentChat(local_id)) = active_tab {
            if self.handle_agent_chat_key_down(local_id, event, cx) {
                cx.stop_propagation();
            }
            return;
        }

        let active_terminal_id = match active_tab {
            Some(CenterTab::Terminal(id)) => id,
            Some(CenterTab::Hub) => {
                let Some(id) = self.hub_active_terminal_id else {
                    return;
                };
                id
            },
            _ => return,
        };

        if let Some(command) = terminal_keys::platform_command_for_keystroke(&event.keystroke) {
            match command {
                terminal_keys::TerminalPlatformCommand::Copy => {
                    self.copy_terminal_content_to_clipboard(active_terminal_id, cx);
                },
                terminal_keys::TerminalPlatformCommand::Paste => {
                    self.paste_clipboard_into_terminal(active_terminal_id, cx);
                },
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }

        self.clear_terminal_selection_for_session(active_terminal_id);

        let terminal_modes = self
            .terminals
            .iter()
            .find(|session| session.id == active_terminal_id)
            .map(|session| session.modes)
            .unwrap_or_default();

        let Some(input) =
            terminal_keys::terminal_bytes_from_keystroke(&event.keystroke, terminal_modes)
        else {
            // No bytes for this key — let the event propagate to the IME /
            // InputHandler so composed characters arrive via
            // `replace_text_in_range`.
            return;
        };

        self.track_terminal_command_input(active_terminal_id, &event.keystroke);
        if let Err(error) = self.write_input_to_terminal(active_terminal_id, &input) {
            self.notice = Some(format!("failed to write to terminal: {error}"));
        }
        cx.stop_propagation();
        cx.notify();
    }

    pub(crate) fn focus_terminal_panel(
        &mut self,
        _: &MouseDownEvent,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.right_pane_search_active = false;
        window.focus(&self.terminal_focus);
        self.focus_terminal_on_next_render = false;
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use {super::*, crate::daemon_runtime::session_with_styled_line, std::sync::Arc};

    #[test]
    fn terminal_input_buffers_only_while_session_is_initializing() {
        let mut session = session_with_styled_line("prompt", 0xffffff, 0x000000, None);

        session.is_initializing = true;
        assert!(should_queue_terminal_input(&session));

        session.is_initializing = false;
        assert!(!should_queue_terminal_input(&session));

        session.is_initializing = true;
        session.runtime = Some(Arc::new(daemon_runtime::tests::daemon_runtime_for_test()));
        assert!(!should_queue_terminal_input(&session));
    }
}
