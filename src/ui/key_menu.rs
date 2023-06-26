use std::process::{Command, Output};

use crossterm::event::KeyCode;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::config::{
    get_color, get_key, get_modifiers, PowerControlConfig, SwitcherConfig, SwitcherVisibility,
};

#[derive(Clone)]
pub struct KeyMenuWidget {
    power_config: PowerControlConfig,
    switcher_config: SwitcherConfig,
}

impl KeyMenuWidget {
    pub fn new(power_config: PowerControlConfig, switcher_config: SwitcherConfig) -> Self {
        Self {
            power_config,
            switcher_config,
        }
    }
    fn shutdown_style(&self) -> Style {
        let mut style = Style::default().fg(get_color(&self.power_config.shutdown_hint_color));

        for modifier in get_modifiers(&self.power_config.shutdown_hint_modifiers) {
            style = style.add_modifier(modifier);
        }

        style
    }

    fn reboot_style(&self) -> Style {
        let mut style = Style::default().fg(get_color(&self.power_config.reboot_hint_color));

        for modifier in get_modifiers(&self.power_config.reboot_hint_modifiers) {
            style = style.add_modifier(modifier);
        }

        style
    }

    fn switcher_toggle_style(&self) -> Style {
        let mut style = Style::default().fg(get_color(&self.switcher_config.toggle_hint_color));

        for modifier in get_modifiers(&self.switcher_config.toggle_hint_modifiers) {
            style = style.add_modifier(modifier);
        }

        style
    }

    pub fn render(&self, frame: &mut Frame<impl ratatui::backend::Backend>, area: Rect) {
        let mut items = Vec::new();

        if self.power_config.allow_shutdown {
            items.push(Span::styled(
                self.power_config
                    .shutdown_hint
                    .replace("%key%", &self.power_config.shutdown_key),
                self.shutdown_style(),
            ));

            // Add margin
            items.push(Span::raw(" ".repeat(self.power_config.hint_margin.into())));
        }

        if self.power_config.allow_reboot {
            items.push(Span::styled(
                self.power_config
                    .reboot_hint
                    .replace("%key%", &self.power_config.reboot_key),
                self.reboot_style(),
            ));
        }

        // Since we only allow Fn keys, this should always match if it's set... because of this, an
        // invalid key is effectively the same as "hidden"
        if let SwitcherVisibility::Keybind(KeyCode::F(n)) = self.switcher_config.switcher_visibility
        {
            let right_widget = Paragraph::new(
                self.switcher_config
                    .toggle_hint
                    .replace("%key%", &format!("F{n}")),
            )
            .alignment(Alignment::Right)
            .style(self.switcher_toggle_style());
            frame.render_widget(right_widget, area);
        }

        let mut text = Text::raw("");
        text.lines.push(Line::from(items));
        let left_widget = Paragraph::new(text);

        frame.render_widget(left_widget, area);
    }

    pub(crate) fn key_press(&self, key_code: KeyCode) -> Option<super::ErrorStatusMessage> {
        // TODO: Properly handle StdIn
        if self.power_config.allow_shutdown && key_code == get_key(&self.power_config.shutdown_key)
        {
            let cmd_status = Command::new("bash")
                .arg("-c")
                .arg(self.power_config.shutdown_cmd.clone())
                .output();

            match cmd_status {
                Err(err) => {
                    log::error!("Failed to execute shutdown command: {:?}", err);
                    return Some(super::ErrorStatusMessage::FailedShutdown);
                }
                Ok(Output {
                    status,
                    stdout,
                    stderr,
                }) if !status.success() => {
                    log::error!("Error while executing shutdown command");
                    log::error!("STDOUT:\n{:?}", stdout);
                    log::error!("STDERR:\n{:?}", stderr);

                    return Some(super::ErrorStatusMessage::FailedShutdown);
                }
                _ => {}
            }
        }
        if self.power_config.allow_reboot && key_code == get_key(&self.power_config.reboot_key) {
            let cmd_status = Command::new("bash")
                .arg("-c")
                .arg(self.power_config.reboot_cmd.clone())
                .output();

            match cmd_status {
                Err(err) => {
                    log::error!("Failed to execute reboot command: {:?}", err);
                    return Some(super::ErrorStatusMessage::FailedReboot);
                }
                Ok(Output {
                    status,
                    stdout,
                    stderr,
                }) if !status.success() => {
                    log::error!("Error while executing reboot command");
                    log::error!("STDOUT:\n{:?}", stdout);
                    log::error!("STDERR:\n{:?}", stderr);

                    return Some(super::ErrorStatusMessage::FailedReboot);
                }
                _ => {}
            }
        }

        None
    }
}
