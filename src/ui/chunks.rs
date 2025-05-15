use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};
use Constraint::{Length, Min};
use crate::config::{LayoutConfig};

pub struct Chunks {
    pub key_menu: Rect,
    pub switcher: Rect,
    pub username_field: Rect,
    pub password_field: Rect,
    pub status_message: Rect,
}

impl Chunks {
    pub fn new<B: Backend>(frame: &Frame<B>, config: &LayoutConfig) -> Self {
        let constraints = [
	    Length(config.pre_power_gap),
            Length(1), // key menu
            Length(config.power_switcher_gap),
            Length(1), // switcher
            Length(config.switcher_username_gap),
            Length(3), // username
            Length(config.username_password_gap),
            Length(3), // password
            Length(config.password_status_gap),
            Length(1), // status
            Min(0),
        ];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .horizontal_margin(2)
            .vertical_margin(1)
            .constraints(constraints.as_ref())
            .split(frame.size());

        Self {
            key_menu: chunks[1],
            switcher: chunks[3],
            username_field: chunks[5],
            password_field: chunks[7],
            status_message: chunks[9],
        }
    }
}
