use ratatui::{
    style::Style,
    widgets::{Block, Borders},
    Frame,
};

use crate::config::{get_color, PanelConfig};

#[derive(Clone)]
pub struct PanelWidget {
    config: PanelConfig,
}

impl PanelWidget {
    pub fn new(config: PanelConfig) -> Self {
        Self { config }
    }

    pub fn render(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        if !self.config.show_panel {
            return;
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.border_style())
            .style(self.style());

        frame.render_widget(block, area);
    }

    fn style(&self) -> Style {
        Style::default().bg(get_color(&self.config.color))
    }

    fn border_style(&self) -> Style {
        Style::default().fg(get_color(&self.config.border_color))
    }
}
