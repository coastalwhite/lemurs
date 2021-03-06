use std::process::Command;

use crossterm::event::KeyCode;
use tui::layout::Rect;
use tui::style::Style;
use tui::text::{Span, Spans, Text};
use tui::widgets::Paragraph;
use tui::Frame;

use crate::config::{get_color, get_key, get_modifiers, PowerOptionsConfig};

pub struct PowerMenuWidget {
    config: PowerOptionsConfig,
}

impl PowerMenuWidget {
    pub fn new(config: PowerOptionsConfig) -> Self {
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

    pub fn key_press(&mut self, key_code: KeyCode) {
        if self.config.allow_shutdown && key_code == get_key(&self.config.shutdown_key) {
            Command::new("bash")
                .arg("-c")
                .arg(self.config.shutdown_cmd.clone())
                .status()
                .expect("Unable to shutdown");
        }
        if self.config.allow_reboot && key_code == get_key(&self.config.reboot_key) {
            Command::new("bash")
                .arg("-c")
                .arg(self.config.reboot_cmd.clone())
                .status()
                .expect("Unable to reboot");
        }
    }
}
