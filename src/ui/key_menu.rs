use std::process::{Command, Output};

use crossterm::event::KeyCode;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::config::{
    get_color, get_key, get_modifiers, PowerControl, PowerControlConfig, SwitcherConfig,
    SwitcherVisibility,
};

#[derive(Clone)]
pub struct KeyMenuWidget {
    power_config: PowerControlConfig,
    switcher_config: SwitcherConfig,
}

impl PowerControl {
    fn style(&self) -> Style {
        let mut style = Style::default().fg(get_color(&self.hint_color));

        for modifier in get_modifiers(&self.hint_modifiers) {
            style = style.add_modifier(modifier);
        }

        style
    }
}

impl KeyMenuWidget {
    pub fn new(power_config: PowerControlConfig, switcher_config: SwitcherConfig) -> Self {
        Self {
            power_config,
            switcher_config,
        }
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

        let mut entries: Vec<_> = self.power_config.entries.values().collect();
        entries.sort_by_key(|v| v.index);

        for power_control in entries {
            items.push(Span::styled(
                power_control.hint.replace("%key%", &power_control.key),
                power_control.style(),
            ));

            // Add margin
            items.push(Span::raw(" ".repeat(self.power_config.hint_margin.into())));
        }

        let left_widget = Paragraph::new(Line::from(items));
        frame.render_widget(left_widget, area);

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
    }

    pub(crate) fn key_press(&self, key_code: KeyCode) -> Option<super::ErrorStatusMessage> {
        // TODO: Properly handle StdIn
        for (name, power_control) in &self.power_config.entries {
            if power_control.allow && key_code == get_key(&power_control.key) {
                let cmd_status = Command::new("bash")
                    .arg("-c")
                    .arg(power_control.cmd.clone())
                    .output();

                match cmd_status {
                    Err(err) => {
                        log::error!("Failed to execute shutdown command: {:?}", err);
                        return Some(super::ErrorStatusMessage::FailedPowerControl(name.clone()));
                    }
                    Ok(Output {
                        status,
                        stdout,
                        stderr,
                    }) if !status.success() => {
                        log::error!("Error while executing {name} command");
                        log::error!("STDOUT:\n{:?}", stdout);
                        log::error!("STDERR:\n{:?}", stderr);

                        return Some(super::ErrorStatusMessage::FailedPowerControl(name.clone()));
                    }
                    _ => {}
                }
            }
        }

        None
    }
}
