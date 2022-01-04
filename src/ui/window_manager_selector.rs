use crossterm::event::KeyCode;
use std::path::PathBuf;
use tui::{
    layout::{Alignment, Rect},
    style::Style,
    terminal::Frame,
    text::{Span, Spans, Text},
    widgets::{Block, Paragraph},
};

use crate::config::{get_color, get_modifiers, WMSelectorConfig};

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
pub struct WindowManagerSelectorWidget {
    selector: WindowManagerSelector,
    config: WMSelectorConfig,
}

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
    pub fn new(window_managers: Vec<WindowManager>, config: WMSelectorConfig) -> Self {
        Self {
            selector: WindowManagerSelector::new(window_managers),
            config,
        }
    }

    fn do_show_neighbours(&self, area_width: usize) -> bool {
        self.config.show_neighbours
            && usize::from(self.config.max_display_length) * 3
                + self.config.left_mover.len()
                + self.config.right_mover.len()
                + usize::from(self.config.mover_margin) * 2
                + usize::from(self.config.neighbour_margin) * 2
                <= area_width
    }

    fn left(&mut self) {
        let Self {
            ref mut selector, ..
        } = self;
        selector.go_prev();
    }

    fn right(&mut self) {
        let Self {
            ref mut selector, ..
        } = self;
        selector.go_next();
    }

    fn cutoff_wm_title_with_padding<'a>(&self, title: &'a str) -> (String, &'a str, String) {
        if title.len() >= usize::from(self.config.max_display_length) {
            return (
                String::new(),
                &title[..usize::from(self.config.max_display_length)],
                String::new(),
            );
        };

        let length_difference = usize::from(self.config.max_display_length) - title.len();
        let padding = " ".repeat(length_difference / 2);
        if length_difference % 2 == 0 {
            (padding.clone(), title, padding)
        } else {
            let right_padding = " ".repeat(1 + length_difference / 2);
            (padding, title, right_padding)
        }
    }

    #[inline]
    fn empty_style(&self, is_focused: bool) -> Style {
        let mut style = Style::default().fg(if is_focused {
            get_color(&self.config.no_envs_color_focused)
        } else {
            get_color(&self.config.no_envs_color)
        });

        for modifier in get_modifiers(if is_focused {
            &self.config.no_envs_modifiers_focused
        } else {
            &self.config.no_envs_modifiers
        }) {
            style = style.add_modifier(modifier);
        }

        style
    }

    #[inline]
    fn arrow_style(&self, is_focused: bool) -> Style {
        let mut style = Style::default().fg(if is_focused {
            get_color(&self.config.mover_color_focused)
        } else {
            get_color(&self.config.mover_color)
        });

        for modifier in get_modifiers(if is_focused {
            &self.config.mover_modifiers_focused
        } else {
            &self.config.mover_modifiers
        }) {
            style = style.add_modifier(modifier);
        }

        style
    }

    #[inline]
    fn neighbour_wm_style(&self, is_focused: bool) -> Style {
        let mut style = Style::default().fg(if is_focused {
            get_color(&self.config.neighbour_color_focused)
        } else {
            get_color(&self.config.neighbour_color)
        });

        for modifier in get_modifiers(if is_focused {
            &self.config.neighbour_modifiers_focused
        } else {
            &self.config.neighbour_modifiers
        }) {
            style = style.add_modifier(modifier);
        }

        style
    }

    #[inline]
    fn current_wm_style(&self, is_focused: bool) -> Style {
        let mut style = Style::default().fg(get_color(if is_focused {
            &self.config.selected_color_focused
        } else {
            &self.config.selected_color
        }));

        for modifier in get_modifiers(if is_focused {
            &self.config.selected_modifiers_focused
        } else {
            &self.config.selected_modifiers
        }) {
            style = style.add_modifier(modifier);
        }

        style
    }

    #[inline]
    fn add_wm_title(
        &self,
        items: &mut Vec<Span>,
        window_manager: &WindowManager,
        is_focused: bool,
        is_current: bool,
    ) {
        // TODO: Maybe if the strings empty, there should be no span generated
        let (left_padding, title, right_padding) =
            self.cutoff_wm_title_with_padding(&window_manager.title);

        let style = if is_current {
            self.current_wm_style(is_focused)
        } else {
            self.neighbour_wm_style(is_focused)
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
        let Self { selector, .. } = &self;

        let mut spans = Vec::with_capacity(
            // Left + Right +
            // LeftPad + RightPad +
            // LeftWM(3) + RightWM(3) +
            // LeftWMPad + RightWMPad +
            // MiddleWM(3) = 15
            15,
        );
        if let Some(current) = selector.current() {
            let do_show_neighbours = self.do_show_neighbours(area.width.into());

            // Showing left item
            if let Some(prev) = selector.prev() {
                if self.config.show_movers {
                    spans.push(Span::styled(
                        &self.config.left_mover,
                        self.arrow_style(is_focused),
                    )); // Left Arrow
                    spans.push(Span::raw(" ".repeat(self.config.mover_margin.into())));
                    // LeftPad
                }

                if do_show_neighbours {
                    self.add_wm_title(&mut spans, prev, is_focused, false); // LeftWM
                    spans.push(Span::raw(" ".repeat(self.config.neighbour_margin.into())));
                    // LeftWMPad
                }
            } else {
                spans.push(Span::raw(" ".repeat(
                    if self.config.show_movers {
                        self.config.left_mover.len() + usize::from(self.config.mover_margin)
                    } else {
                        0
                    } + if do_show_neighbours {
                        usize::from(self.config.max_display_length + self.config.neighbour_margin)
                    } else {
                        0
                    },
                )));
            }

            self.add_wm_title(&mut spans, current, is_focused, true); // CurrentWM

            // Showing next item
            if let Some(next) = selector.next() {
                if do_show_neighbours {
                    spans.push(Span::raw(" ".repeat(self.config.neighbour_margin.into()))); // RightWMPad
                    self.add_wm_title(&mut spans, next, is_focused, false); // RightWM
                }

                if self.config.show_movers {
                    spans.push(Span::raw(" ".repeat(self.config.mover_margin.into()))); // RightPad
                    spans.push(Span::styled(
                        &self.config.right_mover,
                        self.arrow_style(is_focused),
                    )); // Right Arrow
                }
            } else {
                spans.push(Span::raw(" ".repeat(
                    if self.config.show_movers {
                        self.config.right_mover.len() + usize::from(self.config.mover_margin)
                    } else {
                        0
                    } + if do_show_neighbours {
                        usize::from(self.config.max_display_length + self.config.neighbour_margin)
                    } else {
                        0
                    },
                )));
            }
        } else {
            spans.push(Span::styled(
                &self.config.no_envs_text,
                self.empty_style(is_focused),
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
        let Self { selector, .. } = &self;
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
