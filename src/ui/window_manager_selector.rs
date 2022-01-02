use crossterm::event::KeyCode;
use std::path::PathBuf;
use tui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    terminal::Frame,
    text::{Span, Spans, Text},
    widgets::{Block, Paragraph},
};

const NO_WINDOW_MANAGERS_STRING: &str = "No Window Managers Specified";
const NO_WINDOW_MANAGERS_STRING_COLOR: [Color; 2] = [Color::LightRed, Color::Red];
// const WINDOW_MANAGER_CUTOFF_WIDTH: usize = 16;
const PREV_NEXT_ARROWS: [&str; 2] = ["<", ">"];
const ARROWS_COLOR: [Color; 2] = [Color::DarkGray, Color::Yellow];
const PREV_NEXT_PADDING: usize = 1;
const PREV_NEXT_COLOR: [Color; 2] = [Color::DarkGray, Color::DarkGray];
const WM_PADDING: usize = 2;
const CURRENT_COLOR: [Color; 2] = [Color::Gray, Color::White];

// const MIN_WIDTH: usize = WINDOW_MANAGER_CUTOFF_WIDTH
//     + PREV_NEXT_ARROWS[0].len()
//     + PREV_NEXT_ARROWS[1].len()
//     + PREV_NEXT_PADDING * 2
//     + WM_PADDING * 2;

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
            selected: if window_managers.len() == 0 {
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

    // #[inline]
    // fn can_move(&self) -> bool {
    //     self.selected.map_or(false, |s| s == 0)
    // }

    #[inline]
    fn next_index(&self, index: usize) -> usize {
        (index + 1) % self.len()
    }

    #[inline]
    fn prev_index(&self, index: usize) -> usize {
        if index == 0 {
            self.len() - 1
        } else {
            index - 1
        }
    }

    fn go_next(&mut self) {
        self.selected = self.selected.map(|index| self.next_index(index));
    }

    fn go_prev(&mut self) {
        self.selected = self.selected.map(|index| self.prev_index(index));
    }

    fn next(&self) -> Option<&WindowManager> {
        self.selected.map_or(None, |index| {
            debug_assert!(self.len() > 0);
            self.window_managers.get(self.next_index(index))
        })
    }

    fn prev(&self) -> Option<&WindowManager> {
        self.selected.map_or(None, |index| {
            debug_assert!(self.len() > 0);
            self.window_managers.get(self.prev_index(index))
        })
    }

    pub fn current(&self) -> Option<&WindowManager> {
        self.selected.map_or(None, |index| {
            debug_assert!(self.len() > 0);
            self.window_managers.get(index)
        })
    }
}

impl WindowManagerSelectorWidget {
    pub fn new(window_managers: Vec<WindowManager>) -> Self {
        Self(WindowManagerSelector::new(window_managers))
    }

    fn wm_width(window_manager: &WindowManager) -> usize {
        // TODO: Take into account not Monospace fonts
        window_manager.title.len()
    }

    /// Get the character-width of the configuration with three window managers available
    fn find_width(prev_width: usize, current_width: usize, next_width: usize) -> usize {
        prev_width
            + current_width
            + next_width
            + PREV_NEXT_ARROWS[0].len()
            + PREV_NEXT_ARROWS[1].len()
            + PREV_NEXT_PADDING * 2
            + WM_PADDING * 2
    }

    fn do_show_prev(
        area_width: usize,
        prev_width: usize,
        current_width: usize,
        next_width: Option<usize>,
    ) -> bool {
        debug_assert!(area_width >= Self::find_width(0, current_width, 0));

        Self::find_width(prev_width, current_width, next_width.unwrap_or(0)) <= area_width
    }

    fn do_show_next(
        area_width: usize,
        left_width: Option<usize>,
        middle_width: usize,
        right_width: usize,
    ) -> bool {
        debug_assert!(area_width >= Self::find_width(0, middle_width, 0));

        Self::find_width(left_width.unwrap_or(0), middle_width, right_width) <= area_width
    }

    fn left(&mut self) {
        let Self(ref mut selector) = self;
        selector.go_prev();
    }

    fn right(&mut self) {
        let Self(ref mut selector) = self;
        selector.go_next();
    }

    pub fn render(
        &self,
        frame: &mut Frame<impl tui::backend::Backend>,
        area: Rect,
        is_focused: bool,
    ) {
        // TLDR: This code is a complete mess. Issue #1 should almost completely rewrite this code
        // so refactoring here doesn't make a lot of sense.

        let Self(selector) = self;

        // TODO: Optimize these calls
        let current = selector.current();
        let prev = selector.prev();
        let next = selector.next();

        let is_focused = if is_focused { 1 } else { 0 };

        let mut msg = Vec::with_capacity(
            // Left + Right +
            // LeftPad + RightPad +
            // LeftWM + RightWM +
            // LeftWMPad + RightWMPad +
            // MiddleWM = 9
            9,
        );
        if let Some(current) = current {
            msg.push(Span::styled(
                PREV_NEXT_ARROWS[0],
                Style::default().fg(ARROWS_COLOR[is_focused]),
            )); // Left Arrow
            msg.push(Span::raw(" ".repeat(PREV_NEXT_PADDING))); // LeftPad
            if let Some(prev) = prev {
                if Self::do_show_prev(
                    area.width.into(),
                    Self::wm_width(prev),
                    Self::wm_width(current),
                    next.map(|next| Self::wm_width(next)),
                ) {
                    msg.push(Span::styled(
                        &prev.title,
                        Style::default().fg(PREV_NEXT_COLOR[is_focused]),
                    )); // LeftWM
                    msg.push(Span::raw(" ".repeat(WM_PADDING))); // LeftWMPad
                }
            }

            msg.push(Span::styled(
                &current.title,
                Style::default().fg(CURRENT_COLOR[is_focused]),
            )); // CurrentWM

            if let Some(next) = next {
                if Self::do_show_next(
                    area.width.into(),
                    prev.map(|prev| Self::wm_width(prev)),
                    Self::wm_width(current),
                    Self::wm_width(next),
                ) {
                    msg.push(Span::raw(" ".repeat(WM_PADDING))); // RightWMPad
                    msg.push(Span::styled(
                        &next.title,
                        Style::default().fg(PREV_NEXT_COLOR[is_focused]),
                    )); // RightWM
                }
            }
            msg.push(Span::raw(" ".repeat(PREV_NEXT_PADDING))); // RightPad
            msg.push(Span::styled(
                PREV_NEXT_ARROWS[1],
                Style::default().fg(ARROWS_COLOR[is_focused]),
            )); // Right Arrow
        } else {
            msg.push(Span::styled(
                NO_WINDOW_MANAGERS_STRING,
                Style::default().fg(NO_WINDOW_MANAGERS_STRING_COLOR[is_focused]),
            ));
        }
        let text = Text::from(Spans::from(msg));
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
            assert_eq!(selector.current(), Some(&wm2));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm1));

            let mut selector = WindowManagerSelector::new(vec![wm1.clone(), wm2.clone()]);
            assert_eq!(selector.current(), Some(&wm1));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm2));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm1));

            let mut selector = WindowManagerSelector::new(vec![wm1.clone(), wm2.clone()]);
            assert_eq!(selector.current(), Some(&wm1));
            selector.go_prev();
            assert_eq!(selector.current(), Some(&wm2));
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
            assert_eq!(selector.current(), Some(&wm4));

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
            assert_eq!(selector.current(), Some(&wm1));
        }
    }
}
