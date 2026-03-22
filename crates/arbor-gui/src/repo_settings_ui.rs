use super::*;

impl ArborWindow {
    pub(crate) fn open_repo_settings_modal(
        &mut self,
        repository_index: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(repository) = self.repositories.get(repository_index) else {
            return;
        };

        let group_key = repository.group_key.clone();
        let repo_root = repository.root.clone();

        // Load current UI settings
        let label = self
            .custom_repo_labels
            .get(&group_key)
            .cloned()
            .unwrap_or_default();
        let custom_url = self
            .custom_repo_urls
            .get(&group_key)
            .cloned()
            .unwrap_or_default();

        // Load current arbor.toml settings
        let repo_config = repo_config::load_repo_config(&repo_root);

        let branch_prefix_mode = repo_config.as_ref().and_then(|c| c.branch.prefix_mode);
        let branch_prefix = repo_config
            .as_ref()
            .and_then(|c| c.branch.prefix.clone())
            .unwrap_or_default();
        let agent_default_preset = repo_config
            .as_ref()
            .and_then(|c| c.agent.default_preset.clone())
            .unwrap_or_default();
        let agent_auto_checkpoint = repo_config
            .as_ref()
            .and_then(|c| c.agent.auto_checkpoint)
            .unwrap_or(false);
        let notifications_desktop = repo_config
            .as_ref()
            .and_then(|c| c.notifications.desktop)
            .unwrap_or(true);
        let notifications_webhook_url = repo_config
            .as_ref()
            .map(|c| c.notifications.webhook_urls.join(", "))
            .unwrap_or_default();

        let quick_launch_command = self
            .quick_launch_commands
            .get(&group_key)
            .cloned()
            .unwrap_or_default();

        self.repo_settings_modal = Some(RepoSettingsModal {
            repository_index,
            group_key,
            repo_root,
            tab: RepoSettingsTab::General,
            label_cursor: label.len(),
            label,
            custom_url_cursor: custom_url.len(),
            custom_url,
            quick_launch_command_cursor: quick_launch_command.len(),
            quick_launch_command,
            branch_prefix_mode,
            branch_prefix_cursor: branch_prefix.len(),
            branch_prefix,
            agent_default_preset_cursor: agent_default_preset.len(),
            agent_default_preset,
            agent_auto_checkpoint,
            notifications_desktop,
            notifications_webhook_url_cursor: notifications_webhook_url.len(),
            notifications_webhook_url,
            active_field: 0,
            error: None,
        });
        cx.notify();
    }

    pub(crate) fn render_repo_settings_modal(&mut self, cx: &mut Context<Self>) -> Div {
        let Some(modal) = self.repo_settings_modal.as_ref() else {
            return div();
        };

        let theme = self.theme();
        let tab = modal.tab;
        let repo_label = self
            .repositories
            .get(modal.repository_index)
            .map(|r| r.label.clone())
            .unwrap_or_default();
        let group_key = modal.group_key.clone();
        let has_custom_icon = self.custom_repo_icons.contains_key(&group_key);
        let error = modal.error.clone();

        div().absolute().inset_0().child(modal_backdrop()).child(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.repo_settings_modal = None;
                        cx.stop_propagation();
                        cx.notify();
                    }),
                )
                .child(
                    div()
                            .w(px(500.))
                            .max_h(px(520.))
                            .flex_none()
                            .overflow_hidden()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(theme.border))
                            .bg(rgb(theme.sidebar_bg))
                            .p_4()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                cx.stop_propagation();
                            })
                            // Title row
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(theme.text_primary))
                                            .child(format!("Settings \u{2014} {repo_label}")),
                                    )
                                    .child(
                                        div()
                                            .id("repo-settings-close")
                                            .cursor_pointer()
                                            .text_sm()
                                            .text_color(rgb(theme.text_muted))
                                            .hover(|this| {
                                                this.text_color(rgb(theme.text_primary))
                                            })
                                            .child("\u{00d7}")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.repo_settings_modal = None;
                                                cx.notify();
                                            })),
                                    ),
                            )
                            // Tab bar
                            .child(
                                div()
                                    .flex()
                                    .border_b_1()
                                    .border_color(rgb(theme.border))
                                    .child(self.repo_settings_tab_button(
                                        "General",
                                        RepoSettingsTab::General,
                                        tab,
                                        cx,
                                    ))
                                    .child(self.repo_settings_tab_button(
                                        "Branch",
                                        RepoSettingsTab::Branch,
                                        tab,
                                        cx,
                                    ))
                                    .child(self.repo_settings_tab_button(
                                        "Agent",
                                        RepoSettingsTab::Agent,
                                        tab,
                                        cx,
                                    ))
                                    .child(self.repo_settings_tab_button(
                                        "Notifications",
                                        RepoSettingsTab::Notifications,
                                        tab,
                                        cx,
                                    )),
                            )
                            // Error
                            .when_some(error, |this, error| {
                                this.child(
                                    div()
                                        .rounded_sm()
                                        .border_1()
                                        .border_color(rgb(0xa44949))
                                        .bg(rgb(0x4d2a2a))
                                        .px_2()
                                        .py_1()
                                        .text_xs()
                                        .text_color(rgb(0xffd7d7))
                                        .child(error),
                                )
                            })
                            // Tab content
                            .child(self.render_repo_settings_tab_content(has_custom_icon, cx))
                            // Buttons
                            .child(
                                div()
                                    .flex()
                                    .justify_end()
                                    .gap_2()
                                    .child(
                                        action_button(
                                            theme,
                                            "repo-settings-cancel",
                                            "Cancel",
                                            ActionButtonStyle::Secondary,
                                            true,
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.repo_settings_modal = None;
                                            cx.notify();
                                        })),
                                    )
                                    .child(
                                        action_button(
                                            theme,
                                            "repo-settings-save",
                                            "Save",
                                            ActionButtonStyle::Primary,
                                            true,
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.save_repo_settings(cx);
                                        })),
                                    ),
                            ),
                ),
        )
    }

    fn repo_settings_tab_button(
        &self,
        label: &str,
        target_tab: RepoSettingsTab,
        current_tab: RepoSettingsTab,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let theme = self.theme();
        let is_active = current_tab == target_tab;
        div()
            .id(gpui::SharedString::from(format!(
                "repo-settings-tab-{label}"
            )))
            .px_3()
            .py_1()
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .cursor_pointer()
            .text_color(rgb(if is_active {
                theme.text_primary
            } else {
                theme.text_muted
            }))
            .when(is_active, |this| {
                this.border_b_2().border_color(rgb(theme.accent))
            })
            .hover(|this| this.text_color(rgb(theme.text_primary)))
            .child(label.to_owned())
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(modal) = this.repo_settings_modal.as_mut() {
                    modal.tab = target_tab;
                    modal.active_field = 0;
                }
                cx.notify();
            }))
    }

    fn render_repo_settings_tab_content(
        &mut self,
        has_custom_icon: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let Some(modal) = self.repo_settings_modal.as_ref() else {
            return div();
        };
        let theme = self.theme();

        match modal.tab {
            RepoSettingsTab::General => self.render_repo_settings_general(has_custom_icon, cx),
            RepoSettingsTab::Branch => self.render_repo_settings_branch(theme, cx),
            RepoSettingsTab::Agent => self.render_repo_settings_agent(theme, cx),
            RepoSettingsTab::Notifications => self.render_repo_settings_notifications(theme, cx),
        }
    }

    fn render_repo_settings_general(
        &mut self,
        has_custom_icon: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let Some(modal) = self.repo_settings_modal.as_ref() else {
            return div();
        };
        let theme = self.theme();
        let group_key = modal.group_key.clone();

        let section_card = |child: Div| {
            child
                .rounded_sm()
                .border_1()
                .border_color(rgb(theme.border))
                .bg(rgb(theme.panel_bg))
                .p_3()
                .flex()
                .flex_col()
                .gap_2()
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            // Label
            .child(section_card(div()).child(
                modal_input_field(
                    theme,
                    "repo-settings-label",
                    "Display Label",
                    &modal.label,
                    modal.label_cursor,
                    "Default (from remote)",
                    modal.active_field == 0,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(modal) = this.repo_settings_modal.as_mut() {
                        modal.active_field = 0;
                    }
                    cx.notify();
                })),
            ))
            // Custom URL
            .child(section_card(div()).child(
                modal_input_field(
                    theme,
                    "repo-settings-url",
                    "Icon Link URL",
                    &modal.custom_url,
                    modal.custom_url_cursor,
                    "Default (GitHub)",
                    modal.active_field == 1,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(modal) = this.repo_settings_modal.as_mut() {
                        modal.active_field = 1;
                    }
                    cx.notify();
                })),
            ))
            // Quick Launch Command
            .child(section_card(div()).child(
                modal_input_field(
                    theme,
                    "repo-settings-quick-launch",
                    "Quick Launch Command (+)",
                    &modal.quick_launch_command,
                    modal.quick_launch_command_cursor,
                    "claude --dangerously-skip-permissions",
                    modal.active_field == 2,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(modal) = this.repo_settings_modal.as_mut() {
                        modal.active_field = 2;
                    }
                    cx.notify();
                })),
            ))
            // Icon section
            .child(
                section_card(div())
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(theme.text_muted))
                            .child("Repository Icon"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            // Current icon preview
                            .child(
                                if let Some(custom) = self.custom_repo_icons.get(&group_key) {
                                    div()
                                        .size(px(40.))
                                        .rounded_full()
                                        .overflow_hidden()
                                        .border_1()
                                        .border_color(rgb(theme.border))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            img(PathBuf::from(&custom.path))
                                                .size(px(40. * custom.scale)),
                                        )
                                } else {
                                    div()
                                        .size(px(40.))
                                        .rounded_full()
                                        .border_1()
                                        .border_color(rgb(theme.border))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .font_family(FONT_MONO)
                                        .text_size(px(18.))
                                        .text_color(rgb(theme.text_muted))
                                        .child("\u{f09b}")
                                },
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .id("repo-settings-change-icon")
                                            .cursor_pointer()
                                            .text_xs()
                                            .text_color(rgb(theme.accent))
                                            .hover(|this| this.opacity(0.8))
                                            .child("Change icon\u{2026}")
                                            .on_click({
                                                let group_key = group_key.clone();
                                                cx.listener(move |this, _, _, cx| {
                                                    this.open_repo_icon_file_picker(
                                                        group_key.clone(),
                                                        cx,
                                                    );
                                                })
                                            }),
                                    )
                                    .when(has_custom_icon, |this| {
                                        this.child(
                                            div()
                                                .id("repo-settings-reset-icon")
                                                .cursor_pointer()
                                                .text_xs()
                                                .text_color(rgb(theme.text_muted))
                                                .hover(|this| {
                                                    this.text_color(rgb(theme.text_primary))
                                                })
                                                .child("Reset to default")
                                                .on_click({
                                                    let group_key = group_key.clone();
                                                    cx.listener(move |this, _, _, cx| {
                                                        this.custom_repo_icons
                                                            .remove(&group_key);
                                                        this.sync_custom_repo_icons_store(cx);
                                                        cx.notify();
                                                    })
                                                }),
                                        )
                                    }),
                            ),
                    ),
            )
    }

    fn render_repo_settings_branch(&mut self, theme: ThemePalette, cx: &mut Context<Self>) -> Div {
        let Some(modal) = self.repo_settings_modal.as_ref() else {
            return div();
        };

        let section_card = |child: Div| {
            child
                .rounded_sm()
                .border_1()
                .border_color(rgb(theme.border))
                .bg(rgb(theme.panel_bg))
                .p_3()
                .flex()
                .flex_col()
                .gap_2()
        };

        let prefix_mode = modal.branch_prefix_mode;

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                section_card(div())
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(theme.text_muted))
                            .child("Branch Prefix Mode"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(theme.text_disabled))
                            .child("How branch names are prefixed when creating worktrees"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(self.branch_mode_option(
                                "None",
                                prefix_mode.is_none()
                                    || prefix_mode == Some(repo_config::RepoBranchPrefixMode::None),
                                theme,
                            ))
                            .child(self.branch_mode_option(
                                "Git Author",
                                prefix_mode == Some(repo_config::RepoBranchPrefixMode::GitAuthor),
                                theme,
                            ))
                            .child(self.branch_mode_option(
                                "GitHub User",
                                prefix_mode == Some(repo_config::RepoBranchPrefixMode::GithubUser),
                                theme,
                            ))
                            .child(self.branch_mode_option(
                                "Custom",
                                prefix_mode == Some(repo_config::RepoBranchPrefixMode::Custom),
                                theme,
                            )),
                    ),
            )
            .child(
                section_card(div()).child(
                    modal_input_field(
                        theme,
                        "repo-settings-branch-prefix",
                        "Custom Prefix",
                        &modal.branch_prefix,
                        modal.branch_prefix_cursor,
                        "e.g. your-name",
                        modal.active_field == 0,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(modal) = this.repo_settings_modal.as_mut() {
                            modal.active_field = 0;
                        }
                        cx.notify();
                    })),
                ),
            )
    }

    fn branch_mode_option(&self, label: &str, is_selected: bool, theme: ThemePalette) -> Div {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .size(px(14.))
                    .rounded_full()
                    .border_1()
                    .border_color(rgb(if is_selected {
                        theme.accent
                    } else {
                        theme.border
                    }))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(is_selected, |this| {
                        this.child(div().size(px(8.)).rounded_full().bg(rgb(theme.accent)))
                    }),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(if is_selected {
                        theme.text_primary
                    } else {
                        theme.text_muted
                    }))
                    .child(label.to_owned()),
            )
    }

    fn render_repo_settings_agent(&mut self, theme: ThemePalette, cx: &mut Context<Self>) -> Div {
        let Some(modal) = self.repo_settings_modal.as_ref() else {
            return div();
        };

        let section_card = |child: Div| {
            child
                .rounded_sm()
                .border_1()
                .border_color(rgb(theme.border))
                .bg(rgb(theme.panel_bg))
                .p_3()
                .flex()
                .flex_col()
                .gap_2()
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                section_card(div()).child(
                    modal_input_field(
                        theme,
                        "repo-settings-agent-preset",
                        "Default Agent Preset",
                        &modal.agent_default_preset,
                        modal.agent_default_preset_cursor,
                        "e.g. claude",
                        modal.active_field == 0,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(modal) = this.repo_settings_modal.as_mut() {
                            modal.active_field = 0;
                        }
                        cx.notify();
                    })),
                ),
            )
            .child(
                section_card(div()).child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .size(px(14.))
                                .rounded_sm()
                                .border_1()
                                .border_color(rgb(if modal.agent_auto_checkpoint {
                                    theme.accent
                                } else {
                                    theme.border
                                }))
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(modal.agent_auto_checkpoint, |this| {
                                    this.child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(theme.accent))
                                            .child("\u{2713}"),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(theme.text_primary))
                                .child("Auto-checkpoint on agent completion"),
                        ),
                ),
            )
    }

    fn render_repo_settings_notifications(
        &mut self,
        theme: ThemePalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let Some(modal) = self.repo_settings_modal.as_ref() else {
            return div();
        };

        let section_card = |child: Div| {
            child
                .rounded_sm()
                .border_1()
                .border_color(rgb(theme.border))
                .bg(rgb(theme.panel_bg))
                .p_3()
                .flex()
                .flex_col()
                .gap_2()
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                section_card(div()).child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .size(px(14.))
                                .rounded_sm()
                                .border_1()
                                .border_color(rgb(if modal.notifications_desktop {
                                    theme.accent
                                } else {
                                    theme.border
                                }))
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(modal.notifications_desktop, |this| {
                                    this.child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(theme.accent))
                                            .child("\u{2713}"),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(theme.text_primary))
                                .child("Desktop notifications"),
                        ),
                ),
            )
            .child(
                section_card(div()).child(
                    modal_input_field(
                        theme,
                        "repo-settings-webhook",
                        "Webhook URLs",
                        &modal.notifications_webhook_url,
                        modal.notifications_webhook_url_cursor,
                        "https://example.com/hook",
                        modal.active_field == 0,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(modal) = this.repo_settings_modal.as_mut() {
                            modal.active_field = 0;
                        }
                        cx.notify();
                    })),
                ),
            )
    }

    pub(crate) fn save_repo_settings(&mut self, cx: &mut Context<Self>) {
        let Some(modal) = self.repo_settings_modal.take() else {
            return;
        };

        // Save UI settings (ui-state.json)
        if modal.label.trim().is_empty() {
            self.custom_repo_labels.remove(&modal.group_key);
        } else {
            self.custom_repo_labels
                .insert(modal.group_key.clone(), modal.label.trim().to_owned());
        }

        if modal.custom_url.trim().is_empty() {
            self.custom_repo_urls.remove(&modal.group_key);
        } else {
            self.custom_repo_urls
                .insert(modal.group_key.clone(), modal.custom_url.trim().to_owned());
        }

        if modal.quick_launch_command.trim().is_empty() {
            self.quick_launch_commands.remove(&modal.group_key);
        } else {
            self.quick_launch_commands.insert(
                modal.group_key.clone(),
                modal.quick_launch_command.trim().to_owned(),
            );
        }

        self.sync_repo_ui_settings_store(cx);

        // Save arbor.toml settings
        let config_path = repo_config::repo_config_path(&modal.repo_root);
        if let Err(e) = self.write_arbor_toml(&config_path, &modal) {
            self.notice = Some(format!("Failed to save arbor.toml: {e}"));
        }

        cx.notify();
    }

    fn write_arbor_toml(
        &self,
        config_path: &Path,
        modal: &RepoSettingsModal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Read existing content or start fresh
        let existing = fs::read_to_string(config_path).unwrap_or_default();
        let mut doc = existing
            .parse::<toml_edit::DocumentMut>()
            .unwrap_or_default();

        // Branch section
        let branch = doc
            .entry("branch")
            .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
        if let Some(branch_table) = branch.as_table_mut() {
            if let Some(mode) = modal.branch_prefix_mode {
                let mode_str = match mode {
                    repo_config::RepoBranchPrefixMode::None => "none",
                    repo_config::RepoBranchPrefixMode::GitAuthor => "git-author",
                    repo_config::RepoBranchPrefixMode::GithubUser => "github-user",
                    repo_config::RepoBranchPrefixMode::Custom => "custom",
                };
                branch_table["prefix_mode"] = toml_edit::value(mode_str);
            }
            if !modal.branch_prefix.trim().is_empty() {
                branch_table["prefix"] = toml_edit::value(modal.branch_prefix.trim());
            } else {
                branch_table.remove("prefix");
            }
        }

        // Agent section
        let agent = doc
            .entry("agent")
            .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
        if let Some(agent_table) = agent.as_table_mut() {
            if !modal.agent_default_preset.trim().is_empty() {
                agent_table["default_preset"] = toml_edit::value(modal.agent_default_preset.trim());
            } else {
                agent_table.remove("default_preset");
            }
            agent_table["auto_checkpoint"] = toml_edit::value(modal.agent_auto_checkpoint);
        }

        // Notifications section
        let notifications = doc
            .entry("notifications")
            .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
        if let Some(notif_table) = notifications.as_table_mut() {
            notif_table["desktop"] = toml_edit::value(modal.notifications_desktop);
            let urls: Vec<&str> = modal
                .notifications_webhook_url
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            if urls.is_empty() {
                notif_table.remove("webhook_urls");
            } else {
                let mut arr = toml_edit::Array::new();
                for url in urls {
                    arr.push(url);
                }
                notif_table["webhook_urls"] = toml_edit::value(arr);
            }
        }

        fs::write(config_path, doc.to_string())?;
        Ok(())
    }
}
