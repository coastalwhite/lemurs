use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};
use Constraint::{Length, Min};

pub struct Chunks {
    pub key_menu: Rect,
    pub switcher: Rect,
    pub username_field: Rect,
    pub password_field: Rect,
    pub status_message: Rect,
}

impl Chunks {
    pub fn new<B: Backend>(frame: &mut Frame<B>) -> Self {
        let constraints = [
            Length(1),
            Length(1),
            Length(2),
            Length(1),
            Length(2),
            Length(3),
            Length(2),
            Length(3),
            Length(2),
            Length(1),
            Min(0),
        ];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .horizontal_margin(2)
            .vertical_margin(1)
            .constraints(constraints.as_ref())
            .split(frame.size());

        Self {
            key_menu: chunks[0],
            switcher: chunks[3],
            username_field: chunks[5],
            password_field: chunks[7],
            status_message: chunks[9],
        }
    }
}
