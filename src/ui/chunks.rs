use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};
use Constraint::{Length, Min};

use crate::config::PanelPosition;

pub struct Chunks {
    pub key_menu: Rect,
    pub panel_root: Rect,
    pub switcher: Rect,
    pub username_field: Rect,
    pub password_field: Rect,
    pub status_message: Rect,
}

impl Chunks {
    pub fn new(frame: &Frame, position: PanelPosition) -> Self {
        // Main Vertical Layout: KeyMenu at top, Status at bottom, Content in middle
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Length(1), // Key Menu
                Min(0),    // Middle Content
                Length(1), // Status Message
            ])
            .horizontal_margin(2)
            .vertical_margin(1)
            .split(frame.area());

        let key_menu = main_chunks[0];
        let middle_content = main_chunks[1];
        let status_message = main_chunks[2];

        // Panel dimensions
        let panel_width = 50;
        let panel_height = 13;

        // Alignment Logic
        let (v_constraints, h_constraints) = match position {
            PanelPosition::Center => (
                [Min(0), Length(panel_height), Min(0)],
                [Min(0), Length(panel_width), Min(0)],
            ),
            PanelPosition::TopLeft => (
                [Length(panel_height), Min(0), Length(0)],
                [Length(panel_width), Min(0), Length(0)],
            ),
            PanelPosition::TopCenter => (
                [Length(panel_height), Min(0), Length(0)],
                [Min(0), Length(panel_width), Min(0)],
            ),
            PanelPosition::TopRight => (
                [Length(panel_height), Min(0), Length(0)],
                [Length(0), Min(0), Length(panel_width)],
            ),
            PanelPosition::CenterLeft => (
                [Min(0), Length(panel_height), Min(0)],
                [Length(panel_width), Min(0), Length(0)],
            ),
            PanelPosition::CenterRight => (
                [Min(0), Length(panel_height), Min(0)],
                [Length(0), Min(0), Length(panel_width)],
            ),
            PanelPosition::BottomLeft => (
                [Length(0), Min(0), Length(panel_height)],
                [Length(panel_width), Min(0), Length(0)],
            ),
            PanelPosition::BottomCenter => (
                [Length(0), Min(0), Length(panel_height)],
                [Min(0), Length(panel_width), Min(0)],
            ),
            PanelPosition::BottomRight => (
                [Length(0), Min(0), Length(panel_height)],
                [Length(0), Min(0), Length(panel_width)],
            ),
        };

        // Vertical positioning
        let vertical_chunks = Layout::default()
             .direction(Direction::Vertical)
             .constraints(v_constraints)
             .split(middle_content);

        // Indices for vertical layout depending on position
        let v_idx = match position {
             PanelPosition::Center | PanelPosition::CenterLeft | PanelPosition::CenterRight => 1,
             PanelPosition::TopLeft | PanelPosition::TopCenter | PanelPosition::TopRight => 0,
             PanelPosition::BottomLeft | PanelPosition::BottomCenter | PanelPosition::BottomRight => 2,
        };

        let row_for_panel = vertical_chunks[v_idx];

        // Horizontal positioning
        let horizontal_chunks = Layout::default()
             .direction(Direction::Horizontal)
             .constraints(h_constraints)
             .split(row_for_panel);

        // Indices for horizontal layout depending on position
        let h_idx = match position {
             PanelPosition::Center | PanelPosition::TopCenter | PanelPosition::BottomCenter => 1,
             PanelPosition::TopLeft | PanelPosition::CenterLeft | PanelPosition::BottomLeft => 0,
             PanelPosition::TopRight | PanelPosition::CenterRight | PanelPosition::BottomRight => 2,
        };

        let panel_root = horizontal_chunks[h_idx];

        // Panel Layout: Inside the box
        let panel_chunks = Layout::default()
            .direction(Direction::Vertical)
            .horizontal_margin(1) // Padding inside the box
            .vertical_margin(1)
            .constraints([
                Length(3), // Switcher
                Length(1), // Spacer
                Length(3), // Username
                Length(1), // Spacer
                Length(3), // Password
                Min(0),
            ])
            .split(panel_root);

        Self {
            key_menu,
            status_message,
            panel_root,
            switcher: panel_chunks[0],
            username_field: panel_chunks[2],
            password_field: panel_chunks[4],
        }
    }
}
