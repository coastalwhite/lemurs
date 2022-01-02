use crossterm::event::KeyCode;
use std::path::PathBuf;
use tui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    terminal::Frame,
    text::{Span, Spans, Text},
    widgets::{Block, Paragraph},
};

const NO_WINDOW_MANAGERS_STRING: &str = "No Window Managers Specified";
const NO_WINDOW_MANAGERS_STRING_COLOR: [Color; 2] = [Color::LightRed, Color::Red];
const WM_CUTOFF_WIDTH: usize = 8;
const PREV_NEXT_ARROWS: [&str; 2] = ["<", ">"];
const ARROWS_COLOR: [Color; 2] = [Color::DarkGray, Color::Yellow];
const PREV_NEXT_PADDING: usize = 1;
const PREV_NEXT_COLOR: [Color; 2] = [Color::DarkGray, Color::DarkGray];
const WM_PADDING: usize = 2;
const CURRENT_COLOR: [Color; 2] = [Color::Gray, Color::White];

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct WindowManager {
    pub title: String,
    pub initrc_path: PathBuf,
}

#[derive(Debug)]
struct WindowManagerSelector {
    selected: Option<usize>,
    window_managers: Vec<WindowManager>,
}

/// A widget used to select a specific window manager
pub struct WindowManagerSelectorWidget(WindowManagerSelector);

impl WindowManager {
    /// Create a new [`WindowManager`]
    pub fn new(title: impl ToString, initrc_path: PathBuf) -> WindowManager {
        let title = title.to_string();

        Self { title, initrc_path }
    }
}

impl WindowManagerSelector {
    /// Create a new [`WindowManagerSelector`]
    fn new(window_managers: Vec<WindowManager>) -> Self {
        Self {
            selected: if window_managers.is_empty() {
                None
            } else {
                Some(0)
            },
            window_managers,
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.window_managers.len()
    }

    #[inline]
    fn next_index(&self, index: usize) -> Option<usize> {
        let next_index = index + 1;

        if next_index == self.len() {
            None
        } else {
            Some(next_index)
        }
    }

    #[inline]
    fn prev_index(&self, index: usize) -> Option<usize> {
        if index == 0 {
            return None;
        }

        Some(index - 1)
    }

    fn go_next(&mut self) {
        match self.selected.map(|index| self.next_index(index)) {
            None | Some(None) => {}
            Some(val) => self.selected = val,
        }
    }

    fn go_prev(&mut self) {
        match self.selected.map(|index| self.prev_index(index)) {
            None | Some(None) => {}
            Some(val) => self.selected = val,
        }
    }

    fn next(&self) -> Option<&WindowManager> {
        self.selected.and_then(|index| {
            debug_assert!(self.len() > 0);
            match self.next_index(index) {
                Some(next_index) => self.window_managers.get(next_index),
                None => None,
            }
        })
    }

    fn prev(&self) -> Option<&WindowManager> {
        self.selected.and_then(|index| {
            debug_assert!(self.len() > 0);
            match self.prev_index(index) {
                Some(prev_index) => self.window_managers.get(prev_index),
                None => None,
            }
        })
    }

    pub fn current(&self) -> Option<&WindowManager> {
        self.selected.and_then(|index| {
            debug_assert!(self.len() > 0);
            self.window_managers.get(index)
        })
    }
}

impl WindowManagerSelectorWidget {
    pub fn new(window_managers: Vec<WindowManager>) -> Self {
        Self(WindowManagerSelector::new(window_managers))
    }

    fn do_show_neighbours(area_width: usize) -> bool {
        WM_CUTOFF_WIDTH * 3
            + PREV_NEXT_ARROWS[0].len()
            + PREV_NEXT_ARROWS[1].len()
            + PREV_NEXT_PADDING * 2
            + WM_PADDING * 2
            <= area_width
    }

    fn left(&mut self) {
        let Self(ref mut selector) = self;
        selector.go_prev();
    }

    fn right(&mut self) {
        let Self(ref mut selector) = self;
        selector.go_next();
    }

    fn cutoff_wm_title_with_padding(title: &str) -> (String, &str, String) {
        if title.len() >= WM_CUTOFF_WIDTH {
            return (String::new(), &title[..WM_CUTOFF_WIDTH], String::new());
        };

        let length_difference = WM_CUTOFF_WIDTH - title.len();
        let padding = " ".repeat(length_difference / 2);
        if length_difference % 2 == 0 {
            (padding.clone(), title, padding)
        } else {
            let right_padding = " ".repeat(1 + length_difference / 2);
            (padding, title, right_padding)
        }
    }

    #[inline]
    fn empty_style(is_focused: bool) -> Style {
        Style::default().fg(NO_WINDOW_MANAGERS_STRING_COLOR[if is_focused { 1 } else { 0 }])
    }

    #[inline]
    fn arrow_style(is_focused: bool) -> Style {
        Style::default().fg(ARROWS_COLOR[if is_focused { 1 } else { 0 }])
    }

    #[inline]
    fn neighbour_wm_style(is_focused: bool) -> Style {
        Style::default().fg(PREV_NEXT_COLOR[if is_focused { 1 } else { 0 }])
    }

    #[inline]
    fn current_wm_style(is_focused: bool) -> Style {
        Style::default()
            .fg(CURRENT_COLOR[if is_focused { 1 } else { 0 }])
            .add_modifier(Modifier::UNDERLINED)
    }

    #[inline]
    fn add_wm_title(
        items: &mut Vec<Span>,
        window_manager: &WindowManager,
        is_focused: bool,
        is_current: bool,
    ) {
        // TODO: Maybe if the strings empty, there should be no span generated
        let (left_padding, title, right_padding) =
            Self::cutoff_wm_title_with_padding(&window_manager.title);

        let style = if is_current {
            Self::current_wm_style(is_focused)
        } else {
            Self::neighbour_wm_style(is_focused)
        };

        items.push(Span::raw(left_padding));
        items.push(Span::styled(title.to_string(), style));
        items.push(Span::raw(right_padding));
    }

    pub fn render(
        &self,
        frame: &mut Frame<impl tui::backend::Backend>,
        area: Rect,
        is_focused: bool,
    ) {
        let Self(selector) = self;

        let mut spans = Vec::with_capacity(
            // Left + Right +
            // LeftPad + RightPad +
            // LeftWM(3) + RightWM(3) +
            // LeftWMPad + RightWMPad +
            // MiddleWM(3) = 15
            15,
        );
        if let Some(current) = selector.current() {
            let do_show_neighbours = Self::do_show_neighbours(area.width.into());

            // Showing left item
            if let Some(prev) = selector.prev() {
                spans.push(Span::styled(
                    PREV_NEXT_ARROWS[0],
                    Self::arrow_style(is_focused),
                )); // Left Arrow
                spans.push(Span::raw(" ".repeat(PREV_NEXT_PADDING))); // LeftPad

                if do_show_neighbours {
                    Self::add_wm_title(&mut spans, prev, is_focused, false); // LeftWM
                    spans.push(Span::raw(" ".repeat(WM_PADDING))); // LeftWMPad
                }
            } else {
                spans.push(Span::raw(" ".repeat(
                    PREV_NEXT_ARROWS[0].len() + PREV_NEXT_PADDING + WM_CUTOFF_WIDTH + WM_PADDING,
                )));
            }

            Self::add_wm_title(&mut spans, current, is_focused, true); // CurrentWM

            // Showing next item
            if let Some(next) = selector.next() {
                if do_show_neighbours {
                    spans.push(Span::raw(" ".repeat(WM_PADDING))); // RightWMPad
                    Self::add_wm_title(&mut spans, next, is_focused, false); // RightWM
                }

                spans.push(Span::raw(" ".repeat(PREV_NEXT_PADDING))); // RightPad
                spans.push(Span::styled(
                    PREV_NEXT_ARROWS[1],
                    Self::arrow_style(is_focused),
                )); // Right Arrow
            } else {
                spans.push(Span::raw(" ".repeat(
                    PREV_NEXT_ARROWS[0].len() + PREV_NEXT_PADDING + WM_CUTOFF_WIDTH + WM_PADDING,
                )));
            }
        } else {
            spans.push(Span::styled(
                NO_WINDOW_MANAGERS_STRING,
                Self::empty_style(is_focused),
            ));
        }

        let text = Text::from(Spans::from(spans));
        let widget = Paragraph::new(text)
            .block(Block::default())
            .alignment(Alignment::Center);

        frame.render_widget(widget, area);
    }

    pub fn key_press(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.left();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.right();
            }
            _ => {}
        }
    }

    pub fn selected(&self) -> Option<&WindowManager> {
        let WindowManagerSelectorWidget(ref selector) = self;
        selector.current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod window_manager_selector {
        use super::*;
        #[test]
        fn empty_creation() {
            // On an empty selector the go_next and go_prev should do nothing.

            let mut selector = WindowManagerSelector::new(vec![]);
            assert_eq!(selector.current(), None);
            selector.go_next();
            assert_eq!(selector.current(), None);
            selector.go_prev();
            assert_eq!(selector.current(), None);

            let mut selector = WindowManagerSelector::new(vec![]);
            assert_eq!(selector.current(), None);
            selector.go_prev();
            assert_eq!(selector.current(), None);
            selector.go_next();
            assert_eq!(selector.current(), None);
        }

        #[test]
        fn single_creation() {
            let wm = WindowManager::new("abc", "/abc".into());

            let mut selector = WindowManagerSelector::new(vec![wm.clone()]);
            assert_eq!(selector.current(), Some(&wm));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm));
            selector.go_prev();
            assert_eq!(selector.current(), Some(&wm));

            let mut selector = WindowManagerSelector::new(vec![wm.clone()]);
            assert_eq!(selector.current(), Some(&wm));
            selector.go_prev();
            assert_eq!(selector.current(), Some(&wm));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm));

            let mut selector = WindowManagerSelector::new(vec![wm.clone()]);
            assert_eq!(selector.current(), Some(&wm));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm));

            let mut selector = WindowManagerSelector::new(vec![wm.clone()]);
            assert_eq!(selector.current(), Some(&wm));
            selector.go_prev();
            assert_eq!(selector.current(), Some(&wm));
            selector.go_prev();
            assert_eq!(selector.current(), Some(&wm));
        }

        #[test]
        fn multiple_creation() {
            let wm1 = WindowManager::new("abc", "/abc".into());
            let wm2 = WindowManager::new("def", "/def".into());

            let mut selector = WindowManagerSelector::new(vec![wm1.clone(), wm2.clone()]);
            assert_eq!(selector.current(), Some(&wm1));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm2));
            selector.go_prev();
            assert_eq!(selector.current(), Some(&wm1));

            let mut selector = WindowManagerSelector::new(vec![wm1.clone(), wm2.clone()]);
            assert_eq!(selector.current(), Some(&wm1));
            selector.go_prev();
            assert_eq!(selector.current(), Some(&wm1));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm2));

            let mut selector = WindowManagerSelector::new(vec![wm1.clone(), wm2.clone()]);
            assert_eq!(selector.current(), Some(&wm1));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm2));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm2));

            let mut selector = WindowManagerSelector::new(vec![wm1.clone(), wm2.clone()]);
            assert_eq!(selector.current(), Some(&wm1));
            selector.go_prev();
            assert_eq!(selector.current(), Some(&wm1));

            let wm3 = WindowManager::new("ghi", "/ghi".into());
            let wm4 = WindowManager::new("jkl", "/jkl".into());

            let mut selector = WindowManagerSelector::new(vec![
                wm1.clone(),
                wm2.clone(),
                wm3.clone(),
                wm4.clone(),
            ]);
            assert_eq!(selector.current(), Some(&wm1));
            selector.go_prev();
            assert_eq!(selector.current(), Some(&wm1));

            let mut selector = WindowManagerSelector::new(vec![
                wm1.clone(),
                wm2.clone(),
                wm3.clone(),
                wm4.clone(),
            ]);
            assert_eq!(selector.current(), Some(&wm1));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm2));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm3));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm4));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm4));
        }
    }
}
