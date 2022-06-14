use crossterm::event::KeyCode;
use std::cmp::min;
use tui::{
    layout::{Rect, Margin},
    style::Style,
    terminal::Frame,
    text::Span,
    widgets::{Block, Borders, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use crate::config::{get_color, UsernameFieldConfig};

/// The type of the input field display. How are the characters which are typed displayed?
pub enum InputFieldDisplayType {
    /// Show the characters that were typed
    Echo,
    /// Always statically show a selected character
    Replace(String),
}

pub struct InputFieldWidget {
    content: String,
    /// Horizontal position of the cursor
    cursor: u16,
    /// Horizontal scroll
    scroll: u16,
    width: u16,
    display_type: InputFieldDisplayType,
    config: UsernameFieldConfig,
}

impl InputFieldWidget {
    /// Creates a new input field widget
    pub fn new(display_type: InputFieldDisplayType, config: UsernameFieldConfig) -> Self {
        Self {
            content: String::new(),
            cursor: 0,
            scroll: 0,
            width: 8, // Give it some initial width
            display_type,
            config,
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.content.len()
    }

    /// Returns what the displayed string should be
    fn show_string(&self) -> String {
        use InputFieldDisplayType::{Echo, Replace};

        let substr = &self.content
            [usize::from(self.scroll)..min(usize::from(self.scroll + self.width), self.len())];

        match &self.display_type {
            Echo => substr.to_string(),
            Replace(character) => character.clone().repeat(substr.len()),
        }
    }

    fn backspace(&mut self) {
        if self.cursor == 0 && self.scroll == 0 {
            return;
        }

        let index = usize::from(self.cursor + self.scroll - 1);

        if self.cursor > 0 {
            self.cursor -= 1;
        } else if self.scroll > 0 {
            self.scroll -= 1;
        }
        self.content.remove(index);
    }

    fn delete(&mut self) {
        let index = usize::from(self.cursor + self.scroll);
        if index >= self.len() {
            return;
        }

        self.content.remove(index);
    }

    fn insert(&mut self, character: char) {
        // Make sure the cursor doesn't overflow
        if self.len() == usize::from(u16::MAX) {
            return;
        }

        debug_assert!(usize::from(self.cursor) <= self.len());
        self.content
            .insert(usize::from(self.cursor + self.scroll), character);
        if self.cursor == self.width - 1 {
            self.scroll += 1;
        } else {
            self.cursor += 1;
        }
    }

    fn right(&mut self) {
        if usize::from(self.cursor + self.scroll) >= self.len() {
            return;
        }

        if self.cursor == self.width - 1 {
            self.scroll += 1;
        } else {
            self.cursor += 1;
        }
    }

    fn left(&mut self) {
        if self.cursor == 0 {
            if self.scroll > 0 {
                self.scroll -= 1;
            }

            return;
        }

        self.cursor -= 1;
    }

    pub fn clear(&mut self) {
        self.cursor = 0;
        self.scroll = 0;
        self.content = String::new();
    }

    fn get_text_style(&self, is_focused: bool) -> Style {
        if is_focused {
            Style::default().fg(get_color(&self.config.content_color_focused))
        } else {
            Style::default().fg(get_color(&self.config.content_color))
        }
    }

    fn get_block(&self, is_focused: bool) -> Block {
        let (title_style, border_style) = if is_focused {
            (
                Style::default().fg(get_color(&self.config.title_color_focused)),
                Style::default().fg(get_color(&self.config.border_color_focused)),
            )
        } else {
            (
                Style::default().fg(get_color(&self.config.title_color)),
                Style::default().fg(get_color(&self.config.border_color)),
            )
        };

        let block = Block::default();

        let block = if self.config.show_title {
            block.title(Span::styled(self.config.title.clone(), title_style))
        } else {
            block
        };

        let block = if self.config.show_border {
            block.borders(Borders::ALL).style(border_style)
        } else {
            block
        };

        block
    }

    /// Constraint the area to the given configuration
    fn constraint_area(&self, mut area: Rect) -> Rect {
        let config = &self.config;

        // Check whether a maximum width has been set
        if config.use_max_width {
             if config.max_width < area.width {
                 // Center the area
                 area.x = (area.width - config.max_width) / 2;
                 area.width = config.max_width;
             }
        }

        area
    }

    pub fn render(
        &mut self,
        frame: &mut Frame<impl tui::backend::Backend>,
        area: Rect,
        is_focused: bool,
    ) {
        let area = self.constraint_area(area);
        let block = self.get_block(is_focused);
        let inner = block.inner(area);

        // Get width of text field minus borders (2)
        self.width = inner.width;

        let show_string = self.show_string();
        let widget = Paragraph::new(show_string.as_ref())
            .style(self.get_text_style(is_focused))
            .block(self.get_block(is_focused));

        frame.render_widget(widget, area);

        if is_focused {
            let Rect { x, y, .. } = inner;
            frame.set_cursor(
                x + self.content[usize::from(self.scroll)..usize::from(self.cursor + self.scroll)]
                    .width() as u16,
                y,
            );
        }
    }

    pub fn key_press(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete(),

            KeyCode::Left => self.left(),
            KeyCode::Right => self.right(),

            KeyCode::Char(c) => self.insert(c),
            _ => {}
        }
    }

    /// Get the real content of the input field
    pub fn get_content(&self) -> String {
        self.content.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use InputFieldDisplayType::*;

    #[test]
    fn cursor_movement() {
        // TODO: Verify Unicode behaviour
        let mut input_field = InputFieldWidget::new(Echo, Config::default().username_field);
        assert_eq!(input_field.cursor, 0);
        input_field.insert('x');
        assert_eq!(input_field.cursor, 1);
        input_field.insert('x');
        assert_eq!(input_field.cursor, 2);
        input_field.insert('x');
        assert_eq!(input_field.cursor, 3);
        input_field.insert('x');
        assert_eq!(input_field.cursor, 4);
        input_field.right();
        assert_eq!(input_field.cursor, 4);
        input_field.left();
        assert_eq!(input_field.cursor, 3);
        input_field.left();
        assert_eq!(input_field.cursor, 2);
        input_field.left();
        assert_eq!(input_field.cursor, 1);
        input_field.left();
        assert_eq!(input_field.cursor, 0);
        input_field.left();
        assert_eq!(input_field.cursor, 0);
        input_field.right();
        assert_eq!(input_field.cursor, 1);
        input_field.backspace();
        assert_eq!(input_field.cursor, 0);
    }

    #[test]
    fn integration() {
        let mut input_field = InputFieldWidget::new(Echo, Config::default().username_field);
        assert_eq!(&input_field.show_string(), "");
        input_field.backspace();
        assert_eq!(&input_field.show_string(), "");
        input_field.insert('x');
        assert_eq!(&input_field.show_string(), "x");
        input_field.insert('y');
        assert_eq!(&input_field.show_string(), "xy");
        input_field.backspace();
        assert_eq!(&input_field.show_string(), "x");
        input_field.insert('y');
        assert_eq!(&input_field.show_string(), "xy");
        input_field.left();
        assert_eq!(&input_field.show_string(), "xy");
        input_field.backspace();
        assert_eq!(&input_field.show_string(), "y");
        input_field.backspace();
        assert_eq!(&input_field.show_string(), "y");
        input_field.right();
        assert_eq!(&input_field.show_string(), "y");
        input_field.backspace();
        assert_eq!(&input_field.show_string(), "");
    }
}
