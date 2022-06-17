use std::process::{Command, Output};

use crossterm::event::KeyCode;
use tui::layout::Rect;
use tui::style::Style;
use tui::text::{Span, Spans, Text};
use tui::widgets::Paragraph;
use tui::Frame;

use crate::config::{get_color, get_key, get_modifiers, PowerControlConfig};

pub struct PowerMenuWidget {
    config: PowerControlConfig,
}

impl PowerMenuWidget {
    pub fn new(config: PowerControlConfig) -> Self {
        Self { config }
    }
    fn shutdown_style(&self) -> Style {
        let mut style = Style::default().fg(get_color(&self.config.shutdown_hint_color));

        for modifier in get_modifiers(&self.config.shutdown_hint_modifiers) {
            style = style.add_modifier(modifier);
        }

        style
    }

    fn reboot_style(&self) -> Style {
        let mut style = Style::default().fg(get_color(&self.config.reboot_hint_color));

        for modifier in get_modifiers(&self.config.reboot_hint_modifiers) {
            style = style.add_modifier(modifier);
        }

        style
    }

    pub fn render(&mut self, frame: &mut Frame<impl tui::backend::Backend>, area: Rect) {
        let mut items = Vec::new();

        if self.config.allow_shutdown {
            items.push(Span::styled(
                self.config
                    .shutdown_hint
                    .replace("%key%", &self.config.shutdown_key),
                self.shutdown_style(),
            ));

            // Add margin
            items.push(Span::raw(" ".repeat(self.config.hint_margin.into())));
        }

        if self.config.allow_reboot {
            items.push(Span::styled(
                self.config
                    .reboot_hint
                    .replace("%key%", &self.config.reboot_key),
                self.reboot_style(),
            ));
        }

        let mut text = Text::raw("");
        text.lines.push(Spans(items));
        let widget = Paragraph::new(text);

        frame.render_widget(widget, area);
    }

    pub(crate) fn key_press(&mut self, key_code: KeyCode) -> Option<super::StatusMessage> {
        // TODO: Properly handle StdIn
        if self.config.allow_shutdown && key_code == get_key(&self.config.shutdown_key) {
            let cmd_status = Command::new("bash")
                .arg("-c")
                .arg(self.config.shutdown_cmd.clone())
                .output();

            match cmd_status {
                Err(err) => {
                    log::error!("Failed to execute shutdown command: {:?}", err);
                    return Some(super::StatusMessage::FailedShutdown);
                }
                Ok(Output {
                    status,
                    stdout,
                    stderr,
                }) if !status.success() => {
                    log::error!("Error while executing shutdown command");
                    log::error!("STDOUT:\n{:?}", stdout);
                    log::error!("STDERR:\n{:?}", stderr);

                    return Some(super::StatusMessage::FailedShutdown);
                }
                _ => {}
            }
        }
        if self.config.allow_reboot && key_code == get_key(&self.config.reboot_key) {
            let cmd_status = Command::new("bash")
                .arg("-c")
                .arg(self.config.reboot_cmd.clone())
                .output();

            match cmd_status {
                Err(err) => {
                    log::error!("Failed to execute reboot command: {:?}", err);
                    return Some(super::StatusMessage::FailedReboot);
                }
                Ok(Output {
                    status,
                    stdout,
                    stderr,
                }) if !status.success() => {
                    log::error!("Error while executing reboot command");
                    log::error!("STDOUT:\n{:?}", stdout);
                    log::error!("STDERR:\n{:?}", stderr);

                    return Some(super::StatusMessage::FailedReboot);
                }
                _ => {}
            }
        }

        None
    }
}
