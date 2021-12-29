use crossterm::event::KeyCode;
use tui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    terminal::Frame,
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Paragraph},
};
use unicode_width::UnicodeWidthStr;

pub enum InputFieldDisplayType {
    Echo,
    Replace(char),
}

pub struct InputFieldWidget {
    title: String,
    content: String,
    cursor: u16,
    display_type: InputFieldDisplayType,
}

impl InputFieldWidget {
    pub fn new(title: impl ToString, display_type: InputFieldDisplayType) -> Self {
        let title = title.to_string();

        Self {
            title,
            content: String::new(),
            cursor: 0,
            display_type,
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.content.len()
    }

    fn show_string(&self) -> String {
        use InputFieldDisplayType::{Echo, Replace};

        match self.display_type {
            Echo => self.content.clone(),
            Replace(character) => character.to_string().repeat(self.len()),
        }
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        debug_assert!(usize::from(self.cursor) <= self.len());
        self.cursor -= 1;
        self.content.remove(self.cursor.into());
    }

    fn delete(&mut self) {
        if usize::from(self.cursor) == self.len() {
            return;
        }

        debug_assert!(usize::from(self.cursor) <= self.len());
        self.content.remove(self.cursor.into());
    }

    fn insert(&mut self, character: char) {
        // Make sure the cursor doesn't overflow
        if self.len() == usize::from(u16::MAX) {
            return;
        }

        debug_assert!(usize::from(self.cursor) <= self.len());
        self.content.insert(self.cursor.into(), character);
        self.cursor += 1;
    }

    fn right(&mut self) {
        if usize::from(self.cursor) == self.len() {
            return;
        }

        self.cursor += 1;
    }

    fn left(&mut self) {
        if self.cursor == 0 {
            return;
        }

        self.cursor -= 1;
    }

    pub fn render(
        &self,
        frame: &mut Frame<impl tui::backend::Backend>,
        area: Rect,
        is_focused: bool,
    ) {
        let show_string = self.show_string();
        let widget = Paragraph::new(show_string.as_ref())
            .style(if is_focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(self.title.clone()),
            );

        frame.render_widget(widget, area);

        if is_focused {
            frame.set_cursor(area.x + self.content[..usize::from(self.cursor)].width() as u16 + 1, area.y + 1);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use InputFieldDisplayType::*;

    #[test]
    fn cursor_movement() {
        // TODO: Verify Unicode behaviour
        let mut input_field = InputFieldWidget::new("", Echo);
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
        let mut input_field = InputFieldWidget::new("", Echo);
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
