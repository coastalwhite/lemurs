use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
    Frame,
};
use image::{ImageReader, GenericImageView, imageops::FilterType};
use log::error;

use crate::config::{get_color, BackgroundConfig};

#[derive(Clone)]
pub struct BackgroundWidget {
    config: BackgroundConfig,
}

struct BackgroundImageWidget<'a> {
    config: &'a BackgroundConfig,
}

impl<'a> Widget for BackgroundImageWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.config.image.is_empty() {
            return;
        }

        match ImageReader::open(&self.config.image) {
            Ok(reader) => match reader.decode() {
                Ok(img) => {
                    let resized = img.resize_exact(area.width as u32, area.height as u32, FilterType::Nearest);
                    
                    for x in 0..area.width {
                        for y in 0..area.height {
                            let pixel = resized.get_pixel(x as u32, y as u32);
                            let [r, g, b, _] = pixel.0;
                            if let Some(cell) = buf.cell_mut((area.x + x, area.y + y)) {
                                cell.set_bg(Color::Rgb(r, g, b));
                            }
                        }
                    }
                },
                Err(err) => {
                     error!("Failed to decode background image '{}': {}", self.config.image, err);
                }
            },
            Err(err) => {
                error!("Failed to open background image '{}': {}", self.config.image, err);
            }
        }
    }
}

impl BackgroundWidget {
    pub fn new(config: BackgroundConfig) -> Self {
        Self { config }
    }

    pub fn render(&self, frame: &mut Frame) {
        if !self.config.show_background {
            return;
        }

        let area = frame.area();
        let has_image = !self.config.image.is_empty();

        if has_image {
             frame.render_widget(BackgroundImageWidget { config: &self.config }, area);
        } else {
             let block = Block::default().style(self.background_style());
             let block = if self.config.style.show_border {
                block
                    .borders(Borders::ALL)
                    .border_style(self.border_style())
            } else {
                block
            };
            frame.render_widget(block, area);
        }

        if has_image && self.config.style.show_border {
             // Image rendered, just draw borders on top
             let block = Block::default()
                .borders(Borders::ALL)
                .border_style(self.border_style())
                .style(Style::default()); // Transparent bg
             frame.render_widget(block, area);
        }
    }

    fn background_style(&self) -> Style {
        Style::default().bg(get_color(&self.config.style.color))
    }
    fn border_style(&self) -> Style {
        Style::default().fg(get_color(&self.config.style.border_color))
    }
}
