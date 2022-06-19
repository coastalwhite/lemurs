use tui::backend::Backend;
use tui::layout::Rect;
use tui::style::Color;
use tui::widgets::Paragraph;
use tui::Frame;

use crate::auth::AuthenticationError;

#[derive(Clone, Copy)]
pub enum ErrorStatusMessage {
    AuthenticationError(AuthenticationError),
    NoGraphicalEnvironment,
    FailedGraphicalEnvironment,
    FailedDesktop,
    FailedShutdown,
    FailedReboot,
}

impl Into<&'static str> for ErrorStatusMessage {
    fn into(self) -> &'static str {
        use ErrorStatusMessage::*;

        match self {
            AuthenticationError(_) => "Authentication failed",
            NoGraphicalEnvironment => "No graphical environment specified",
            FailedGraphicalEnvironment => "Failed booting into the graphical environment",
            FailedDesktop => "Failed booting into desktop environment",
            FailedShutdown => "Failed to shutdown... Check the logs for more information",
            FailedReboot => "Failed to reboot... Check the logs for more information",
        }
    }
}

impl Into<StatusMessage> for ErrorStatusMessage {
    fn into(self) -> StatusMessage {
        StatusMessage::Error(self)
    }
}

#[derive(Clone, Copy)]
pub enum InfoStatusMessage {
    LoggingIn,
    Authenticating,
}

impl Into<&'static str> for InfoStatusMessage {
    fn into(self) -> &'static str {
        use InfoStatusMessage::*;

        match self {
            LoggingIn => "Authentication successful. Logging in...",
            Authenticating => "Verifying credentials",
        }
    }
}

impl Into<StatusMessage> for InfoStatusMessage {
    fn into(self) -> StatusMessage {
        StatusMessage::Info(self)
    }
}

#[derive(Clone, Copy)]
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
        match self {
            Self::Error(_) => true,
            _ => false,
        }
    }

    pub fn render<'a, B: Backend>(status: Option<Self>, frame: &mut Frame<'a, B>, area: Rect) {
        if let Some(status_message) = status {
            let widget = Paragraph::new(<&'static str>::from(status_message)).style(
                tui::style::Style::default().fg(if status_message.is_error() {
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
