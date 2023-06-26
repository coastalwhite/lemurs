use crossterm::event::KeyCode;
use log::warn;
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    terminal::Frame,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph},
};

use crate::config::{get_color, get_modifiers, SwitcherConfig, SwitcherVisibility};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct SwitcherItem<T> {
    pub title: String,
    pub content: T,
}

#[derive(Debug, Clone)]
struct Switcher<T> {
    selected: Option<usize>,
    items: Vec<SwitcherItem<T>>,
}

/// A widget used to select a specific window manager
#[derive(Clone)]
pub struct SwitcherWidget<T> {
    selector: Switcher<T>,
    config: SwitcherConfig,
    /// Indicates whether the widget has been hidden by the config or keybind
    hidden: bool,
}

impl<T> SwitcherItem<T> {
    pub fn new(title: impl ToString, content: T) -> Self {
        let title = title.to_string();
        Self { title, content }
    }
}

impl<T> Switcher<T> {
    fn new(items: Vec<SwitcherItem<T>>) -> Self {
        let selected = if items.is_empty() { None } else { Some(0) };
        Self { selected, items }
    }

    #[inline]
    fn len(&self) -> usize {
        self.items.len()
    }

    pub fn try_select(&mut self, title: &str) {
        // Only set the selected if we find a matching title
        if let Some(selected) = self
            .items
            .iter()
            .enumerate()
            .find(|(_, item)| item.title == title)
            .map(|(index, _)| index)
        {
            self.selected = Some(selected);
        } else {
            warn!("Failed to find selection with title: '{}'", title);
        }
    }

    fn next_index(&self, index: usize) -> Option<usize> {
        let next_index = index + 1;

        if next_index == self.len() {
            None
        } else {
            Some(next_index)
        }
    }

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

    fn next(&self) -> Option<&SwitcherItem<T>> {
        self.selected.and_then(|index| {
            debug_assert!(self.len() > 0);
            match self.next_index(index) {
                Some(next_index) => self.items.get(next_index),
                None => None,
            }
        })
    }

    fn prev(&self) -> Option<&SwitcherItem<T>> {
        self.selected.and_then(|index| {
            debug_assert!(self.len() > 0);
            match self.prev_index(index) {
                Some(prev_index) => self.items.get(prev_index),
                None => None,
            }
        })
    }

    pub fn current(&self) -> Option<&SwitcherItem<T>> {
        self.selected.and_then(|index| {
            debug_assert!(self.len() > 0);
            self.items.get(index)
        })
    }
}

impl<T> SwitcherWidget<T> {
    pub fn new(items: Vec<SwitcherItem<T>>, config: SwitcherConfig) -> Self {
        // Always hidden by default unless explicitly stated to be visible
        let hidden = config.switcher_visibility != SwitcherVisibility::Visible;
        Self {
            selector: Switcher::new(items),
            config,
            hidden,
        }
    }

    pub fn try_select(&mut self, title: &str) {
        self.selector.try_select(title)
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

    fn add_wm_title(
        &self,
        items: &mut Vec<Span>,
        item: &SwitcherItem<T>,
        is_focused: bool,
        is_current: bool,
    ) {
        // TODO: Maybe if the strings empty, there should be no span generated
        let (left_padding, title, right_padding) = self.cutoff_wm_title_with_padding(&item.title);

        let style = if is_current {
            self.current_wm_style(is_focused)
        } else {
            self.neighbour_wm_style(is_focused)
        };

        items.push(Span::raw(left_padding));
        items.push(Span::styled(title.to_string(), style));
        items.push(Span::raw(right_padding));
    }

    pub fn hidden(&self) -> bool {
        self.hidden
    }

    pub fn render(
        &self,
        frame: &mut Frame<impl ratatui::backend::Backend>,
        area: Rect,
        is_focused: bool,
    ) {
        let Self {
            selector,
            config,
            hidden,
        } = &self;

        if *hidden {
            let text = Text::default();
            let widget = Paragraph::new(text)
                .block(Block::default())
                .alignment(Alignment::Center);

            frame.render_widget(widget, area);
            return;
        }

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
                if config.show_movers {
                    spans.push(Span::styled(
                        &config.left_mover,
                        self.arrow_style(is_focused),
                    )); // Left Arrow
                    spans.push(Span::raw(" ".repeat(config.mover_margin.into())));
                    // LeftPad
                }

                if do_show_neighbours {
                    self.add_wm_title(&mut spans, prev, is_focused, false); // LeftWM
                    spans.push(Span::raw(" ".repeat(config.neighbour_margin.into())));
                    // LeftWMPad
                }
            } else {
                spans.push(Span::raw(" ".repeat(
                    if config.show_movers {
                        config.left_mover.len() + usize::from(self.config.mover_margin)
                    } else {
                        0
                    } + if do_show_neighbours {
                        usize::from(config.max_display_length + config.neighbour_margin)
                    } else {
                        0
                    },
                )));
            }

            self.add_wm_title(&mut spans, current, is_focused, true); // CurrentWM

            // Showing next item
            if let Some(next) = selector.next() {
                if do_show_neighbours {
                    spans.push(Span::raw(" ".repeat(config.neighbour_margin.into()))); // RightWMPad
                    self.add_wm_title(&mut spans, next, is_focused, false); // RightWM
                }

                if config.show_movers {
                    spans.push(Span::raw(" ".repeat(config.mover_margin.into()))); // RightPad
                    spans.push(Span::styled(
                        &self.config.right_mover,
                        self.arrow_style(is_focused),
                    )); // Right Arrow
                }
            } else {
                spans.push(Span::raw(" ".repeat(
                    if config.show_movers {
                        config.right_mover.len() + usize::from(config.mover_margin)
                    } else {
                        0
                    } + if do_show_neighbours {
                        usize::from(config.max_display_length + config.neighbour_margin)
                    } else {
                        0
                    },
                )));
            }
        } else {
            spans.push(Span::styled(
                &config.no_envs_text,
                self.empty_style(is_focused),
            ));
        }

        let text = Text::from(Line::from(spans));
        let widget = Paragraph::new(text)
            .block(Block::default())
            .alignment(Alignment::Center);

        frame.render_widget(widget, area);
    }

    pub(crate) fn key_press(&mut self, key_code: KeyCode) -> Option<super::ErrorStatusMessage> {
        match key_code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.left();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.right();
            }
            kc if self.config.switcher_visibility == SwitcherVisibility::Keybind(kc) => {
                self.hidden ^= true;
            }
            _ => {}
        }

        None
    }

    pub fn selected(&self) -> Option<&SwitcherItem<T>> {
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

            let mut selector: Switcher<()> = Switcher::new(vec![]);
            assert_eq!(selector.current(), None);
            selector.go_next();
            assert_eq!(selector.current(), None);
            selector.go_prev();
            assert_eq!(selector.current(), None);

            let mut selector: Switcher<()> = Switcher::new(vec![]);
            assert_eq!(selector.current(), None);
            selector.go_prev();
            assert_eq!(selector.current(), None);
            selector.go_next();
            assert_eq!(selector.current(), None);
        }

        #[test]
        fn single_creation() {
            let wm: SwitcherItem<String> = SwitcherItem::new("abc", "/abc".into());

            let mut selector = Switcher::new(vec![wm.clone()]);
            assert_eq!(selector.current(), Some(&wm));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm));
            selector.go_prev();
            assert_eq!(selector.current(), Some(&wm));

            let mut selector = Switcher::new(vec![wm.clone()]);
            assert_eq!(selector.current(), Some(&wm));
            selector.go_prev();
            assert_eq!(selector.current(), Some(&wm));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm));

            let mut selector = Switcher::new(vec![wm.clone()]);
            assert_eq!(selector.current(), Some(&wm));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm));

            let mut selector = Switcher::new(vec![wm.clone()]);
            assert_eq!(selector.current(), Some(&wm));
            selector.go_prev();
            assert_eq!(selector.current(), Some(&wm));
            selector.go_prev();
            assert_eq!(selector.current(), Some(&wm));
        }

        #[test]
        fn multiple_creation() {
            let wm1: SwitcherItem<String> = SwitcherItem::new("abc", "/abc".into());
            let wm2 = SwitcherItem::new("def", "/def".into());

            let mut selector = Switcher::new(vec![wm1.clone(), wm2.clone()]);
            assert_eq!(selector.current(), Some(&wm1));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm2));
            selector.go_prev();
            assert_eq!(selector.current(), Some(&wm1));

            let mut selector = Switcher::new(vec![wm1.clone(), wm2.clone()]);
            assert_eq!(selector.current(), Some(&wm1));
            selector.go_prev();
            assert_eq!(selector.current(), Some(&wm1));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm2));

            let mut selector = Switcher::new(vec![wm1.clone(), wm2.clone()]);
            assert_eq!(selector.current(), Some(&wm1));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm2));
            selector.go_next();
            assert_eq!(selector.current(), Some(&wm2));

            let mut selector = Switcher::new(vec![wm1.clone(), wm2.clone()]);
            assert_eq!(selector.current(), Some(&wm1));
            selector.go_prev();
            assert_eq!(selector.current(), Some(&wm1));

            let wm3 = SwitcherItem::new("ghi", "/ghi".into());
            let wm4 = SwitcherItem::new("jkl", "/jkl".into());

            let mut selector =
                Switcher::new(vec![wm1.clone(), wm2.clone(), wm3.clone(), wm4.clone()]);
            assert_eq!(selector.current(), Some(&wm1));
            selector.go_prev();
            assert_eq!(selector.current(), Some(&wm1));

            let mut selector =
                Switcher::new(vec![wm1.clone(), wm2.clone(), wm3.clone(), wm4.clone()]);
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
