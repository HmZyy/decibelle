use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl NotificationLevel {
    pub fn prefix(&self) -> &'static str {
        match self {
            NotificationLevel::Debug => "󰃤 DEBUG",
            NotificationLevel::Info => "󰋼 INFO",
            NotificationLevel::Warning => "󰀪 WARN",
            NotificationLevel::Error => "󰅚 ERROR",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub level: NotificationLevel,
    pub text: String,
    pub created_at: Instant,
    pub duration: Duration,
}

impl Notification {
    pub fn new(level: NotificationLevel, text: impl Into<String>, duration: Duration) -> Self {
        Self {
            level,
            text: text.into(),
            created_at: Instant::now(),
            duration,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }
}

#[derive(Debug, Default)]
pub struct NotificationManager {
    notifications: Vec<Notification>,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
        }
    }

    pub fn notify(
        &mut self,
        level: NotificationLevel,
        text: impl Into<String>,
        duration: Duration,
    ) {
        self.notifications
            .push(Notification::new(level, text, duration));
    }

    pub fn debug(&mut self, text: impl Into<String>) {
        self.notify(NotificationLevel::Debug, text, Duration::from_secs(2));
    }

    pub fn info(&mut self, text: impl Into<String>) {
        self.notify(NotificationLevel::Info, text, Duration::from_secs(2));
    }

    pub fn warning(&mut self, text: impl Into<String>) {
        self.notify(NotificationLevel::Warning, text, Duration::from_secs(3));
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.notify(NotificationLevel::Error, text, Duration::from_secs(4));
    }

    pub fn tick(&mut self) {
        self.notifications.retain(|n| !n.is_expired());
    }

    pub fn active_notifications(&self) -> &[Notification] {
        &self.notifications
    }
}
