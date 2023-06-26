use ratatui::backend::Backend;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::auth::AuthenticationError;

#[derive(Clone)]
pub enum ErrorStatusMessage {
    AuthenticationError(AuthenticationError),
    NoGraphicalEnvironment,
    FailedGraphicalEnvironment,
    FailedDesktop,
    FailedShutdown,
    FailedReboot,
}

impl From<ErrorStatusMessage> for &'static str {
    fn from(err: ErrorStatusMessage) -> Self {
        use ErrorStatusMessage::*;

        match err {
            AuthenticationError(_) => "Authentication failed",
            NoGraphicalEnvironment => "No graphical environment specified",
            FailedGraphicalEnvironment => "Failed booting into the graphical environment",
            FailedDesktop => "Failed booting into desktop environment",
            FailedShutdown => "Failed to shutdown... Check the logs for more information",
            FailedReboot => "Failed to reboot... Check the logs for more information",
        }
    }
}

impl From<ErrorStatusMessage> for StatusMessage {
    fn from(err: ErrorStatusMessage) -> Self {
        Self::Error(err)
    }
}

#[derive(Clone, Copy)]
pub enum InfoStatusMessage {
    LoggingIn,
    Authenticating,
}

impl From<InfoStatusMessage> for &'static str {
    fn from(info: InfoStatusMessage) -> Self {
        use InfoStatusMessage::*;

        match info {
            LoggingIn => "Authentication successful. Logging in...",
            Authenticating => "Verifying credentials",
        }
    }
}

impl From<InfoStatusMessage> for StatusMessage {
    fn from(info: InfoStatusMessage) -> Self {
        Self::Info(info)
    }
}

#[derive(Clone)]
pub enum StatusMessage {
    Error(ErrorStatusMessage),
    Info(InfoStatusMessage),
}

impl From<StatusMessage> for &'static str {
    fn from(msg: StatusMessage) -> Self {
        use StatusMessage::*;

        match msg {
            Error(sm) => sm.into(),
            Info(sm) => sm.into(),
        }
    }
}

impl StatusMessage {
    /// Fetch whether status is an error
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    pub fn render<B: Backend>(status: Option<Self>, frame: &mut Frame<B>, area: Rect) {
        if let Some(status_message) = status {
            let widget = Paragraph::new(<&'static str>::from(status_message.clone())).style(
                Style::default().fg(if status_message.is_error() {
                    Color::Red
                } else {
                    Color::Yellow
                }),
            );

            frame.render_widget(widget, area);
        } else {
            // Clear the area

            let widget = Paragraph::new("");
            frame.render_widget(widget, area);
        }
    }
}
