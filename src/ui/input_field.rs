use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::Style,
    terminal::Frame,
    text::Span,
    widgets::{Block, Borders, Paragraph},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::config::{get_color, InputFieldStyle};

/// The type of the input field display. How are the characters which are typed displayed?
#[derive(Clone)]
pub enum InputFieldDisplayType {
    /// Show the characters that were typed
    Echo,
    /// Always statically show a selected character
    Replace(String),
}

#[derive(Clone)]
pub struct InputFieldWidget {
    content: String,
    /// Horizontal position of the cursor
    cursor: u16,
    /// Horizontal scroll in UTF-8 characters
    scroll: u16,

    /// Width of the InputField in cells
    width: u16,
    display_type: InputFieldDisplayType,
    style: InputFieldStyle,
}

fn get_byte_offset_of_char_offset(s: &str, offset: usize) -> usize {
    s.char_indices().nth(offset).map_or(s.len(), |(i, _)| i)
}

impl InputFieldWidget {
    /// Creates a new input field widget
    pub fn new(
        display_type: InputFieldDisplayType,
        style: InputFieldStyle,
        preset_content: String,
    ) -> Self {
        // Calculate the initial cursor position from the preset_content
        let initial_cursor_position = preset_content
            .len()
            .try_into() // Convert from usize to u16
            .unwrap_or(0u16);

        Self {
            content: preset_content,
            cursor: initial_cursor_position,
            scroll: 0,
            width: 8, // Give it some initial width
            display_type,
            style,
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.content.len()
    }

    /// Return what string is currently shown to the user for an Echo type field
    fn show_echo(&self) -> String {
        let scroll = usize::from(self.scroll);
        let width = usize::from(self.width);

        let start_index = get_byte_offset_of_char_offset(&self.content, scroll);

        // The end_index is the end of the last index that fit within the box
        let mut cell_width = 0;
        let end_index = self
            .content
            .char_indices()
            .skip(scroll)
            .find(|(_, c)| {
                let Some(char_width) = c.width() else {
                    panic!("Input field cannot contain null byte (\\x00)");
                };

                if cell_width + char_width > width {
                    return true;
                }

                cell_width += char_width;
                false
            })
            .map_or(self.content.len(), |(i, _)| i);

        self.content[start_index..end_index].to_string()
    }

    fn show_replace(&self, replacement: &str) -> String {
        let scroll = usize::from(self.scroll);
        let width = usize::from(self.width);

        let replacement_width = replacement.width();

        let cell_width = self.content.chars().skip(scroll).count();
        let cell_width = usize::min(width, cell_width);
        let cell_width = cell_width / replacement_width;

        replacement.repeat(cell_width)
    }

    /// Returns what the displayed string should be
    fn show_string(&self) -> String {
        use InputFieldDisplayType::{Echo, Replace};

        match &self.display_type {
            Echo => self.show_echo(),
            Replace(s) => self.show_replace(s),
        }
    }

    fn backspace(&mut self) {
        let cursor = usize::from(self.cursor);
        let scroll = usize::from(self.scroll);

        if cursor == 0 && scroll == 0 {
            return;
        }

        let index = get_byte_offset_of_char_offset(&self.content, cursor + scroll - 1);

        self.left();
        self.content.remove(index);
    }

    fn delete(&mut self) {
        let cursor = usize::from(self.cursor);
        let scroll = usize::from(self.scroll);

        let Some((index, _)) = self.content.char_indices().nth(cursor + scroll) else {
            return;
        };

        self.content.remove(index);
    }

    fn insert(&mut self, character: char) {
        let Some(character_width) = character.width() else {
            // Don't handle null bytes
            return;
        };

        // Make sure the cursor doesn't overflow
        if usize::from(u16::MAX) - character_width < self.len() {
            return;
        }

        let cursor = usize::from(self.cursor);
        let scroll = usize::from(self.scroll);

        debug_assert!(cursor <= self.len());
        let index = self
            .content
            .char_indices()
            .nth(cursor + scroll)
            .map_or(self.content.len(), |(i, _)| i);

        self.content.insert(index, character);

        if self.cursor == self.width - 1 {
            self.scroll += 1;
        } else {
            self.cursor += 1;
        }
    }

    #[inline]
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

    #[inline]
    fn left(&mut self) {
        if self.cursor == 0 && self.scroll == 0 {
            return;
        }

        if self.cursor > 0 {
            self.cursor -= 1;
        }

        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    pub fn clear(&mut self) {
        self.cursor = 0;
        self.scroll = 0;
        self.content = String::new();
    }

    pub fn clear_before(&mut self) {
        let byte_offset =
            get_byte_offset_of_char_offset(&self.content, (self.cursor + self.scroll) as usize);
        self.content = self.content[byte_offset..].to_string();

        self.cursor = 0;
        self.scroll = 0;
    }

    pub fn clear_after(&mut self) {
        let byte_offset =
            get_byte_offset_of_char_offset(&self.content, (self.cursor + self.scroll) as usize);
        self.content.truncate(byte_offset);
    }

    pub fn move_to_begin(&mut self) {
        self.cursor = 0;
        self.scroll = 0;
    }

    pub fn move_to_end(&mut self) {
        self.cursor = (self.content.len() % (self.width as usize)) as u16;
        self.scroll = if self.content.len() > (self.width as usize) {
            (self.content.len() - (self.width as usize)) as u16
        } else {
            0
        }
    }

    fn get_text_style(&self, is_focused: bool) -> Style {
        if is_focused {
            Style::default().fg(get_color(&self.style.content_color_focused))
        } else {
            Style::default().fg(get_color(&self.style.content_color))
        }
    }

    fn get_block(&self, is_focused: bool) -> Block<'_> {
        let (title_style, border_style) = if is_focused {
            (
                Style::default().fg(get_color(&self.style.title_color_focused)),
                Style::default().fg(get_color(&self.style.border_color_focused)),
            )
        } else {
            (
                Style::default().fg(get_color(&self.style.title_color)),
                Style::default().fg(get_color(&self.style.border_color)),
            )
        };

        let block = Block::default();

        let block = if self.style.show_title {
            block.title(Span::styled(self.style.title.clone(), title_style))
        } else {
            block
        };

        let block = if self.style.show_border {
            block.borders(Borders::ALL).style(border_style)
        } else {
            block
        };

        block
    }

    /// Constraint the area to the given configuration
    fn constraint_area(&self, mut area: Rect) -> Rect {
        let style = &self.style;

        // Check whether a maximum width has been set
        if style.use_max_width && style.max_width < area.width {
            // Center the area
            area.x = (area.width - style.max_width) / 2;
            area.width = style.max_width;
        }

        area
    }

    pub fn render(
        &mut self,
        frame: &mut Frame<impl ratatui::backend::Backend>,
        area: Rect,
        is_focused: bool,
    ) {
        let area = self.constraint_area(area);
        let block = self.get_block(is_focused);
        let inner = block.inner(area);

        // Get width of text field minus borders (2)
        self.width = inner.width;

        let show_string = self.show_string();

        if is_focused {
            let Rect { x, y, .. } = inner;
            let cursor_offset = get_byte_offset_of_char_offset(&show_string, self.cursor.into());
            frame.set_cursor(x + show_string[..cursor_offset].width() as u16, y);
        }

        let widget = Paragraph::new(show_string)
            .style(self.get_text_style(is_focused))
            .block(self.get_block(is_focused));

        frame.render_widget(widget, area);
    }

    pub(crate) fn key_press(
        &mut self,
        key_code: KeyCode,
        modifiers: KeyModifiers,
    ) -> Option<super::ErrorStatusMessage> {
        match (key_code, modifiers) {
            (KeyCode::Backspace, _) | (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                self.backspace()
            }
            (KeyCode::Delete, _) | (KeyCode::Char('d'), KeyModifiers::CONTROL) => self.delete(),

            (KeyCode::Left, _) | (KeyCode::Char('b'), KeyModifiers::CONTROL) => self.left(),
            (KeyCode::Right, _) | (KeyCode::Char('f'), KeyModifiers::CONTROL) => self.right(),

            (KeyCode::Char('l'), KeyModifiers::CONTROL) => self.clear(),
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => self.move_to_begin(),
            (KeyCode::Char('e'), KeyModifiers::CONTROL) => self.move_to_end(),
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => self.clear_before(),
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => self.clear_after(),

            (KeyCode::Char(c), _) => self.insert(c),
            _ => {}
        }

        None
    }

    /// Get the real content of the input field
    pub fn get_content(&self) -> String {
        self.content.clone()
    }

    pub fn set_content(&mut self, content: &str) {
        self.cursor = content.len() as u16;
        self.content = content.to_string();
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
        let mut input_field = InputFieldWidget::new(
            Echo,
            Config::default().username_field.style,
            String::default(),
        );
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
        let mut input_field = InputFieldWidget::new(
            Echo,
            Config::default().username_field.style,
            String::default(),
        );

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
        input_field.insert('🐵');
        assert_eq!(&input_field.show_string(), "xy🐵");
        input_field.left();
        input_field.left();
        input_field.insert('🐒');
        assert_eq!(&input_field.show_string(), "x🐒y🐵");
        input_field.right();
        input_field.backspace();
        assert_eq!(&input_field.show_string(), "x🐒🐵");
        input_field.backspace();
        assert_eq!(&input_field.show_string(), "x🐵");
        input_field.delete();
        assert_eq!(&input_field.show_string(), "x");
        input_field.insert('y');
        assert_eq!(&input_field.show_string(), "xy");
        input_field.left();
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
