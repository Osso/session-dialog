//! Iced session-lock dialog UI

use crate::DialogConfig;
use iced::border::Radius;
use iced::keyboard::{self, Key};
use iced::theme::Palette;
use iced::widget::{column, container, horizontal_rule, row, text};
use iced::window::Id;
use iced::Color;
use iced::{Element, Event, Subscription, Task, Theme};
use iced_sessionlock::build_pattern::application;
use iced_sessionlock::to_session_message;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::OnceLock;

static CONFIG: OnceLock<DialogConfig> = OnceLock::new();
static EXIT_CODE: AtomicI32 = AtomicI32::new(1); // Default: denied

/// Run the dialog UI and return exit code
///
/// Exit codes:
/// - 0: Confirmed
/// - 1: Denied
/// - 2: Timeout
/// - 3: Error
pub fn run(config: DialogConfig) -> i32 {
    let _ = CONFIG.set(config);

    let result = application(App::update, App::view)
        .theme(App::theme)
        .subscription(App::subscription)
        .run_with(App::new);

    match result {
        Ok(()) => EXIT_CODE.load(Ordering::SeqCst),
        Err(_) => 3, // Error
    }
}

struct App {
    start_time: std::time::Instant,
}

#[to_session_message]
#[derive(Debug, Clone)]
enum Message {
    Event(Event),
    Tick,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                start_time: std::time::Instant::now(),
            },
            Task::none(),
        )
    }

    fn theme(_: &Self) -> Theme {
        ayu_dark_theme()
    }

    fn subscription(_: &Self) -> Subscription<Message> {
        let events = iced::event::listen().map(Message::Event);

        // Check timeout if configured
        if let Some(config) = CONFIG.get() {
            if config.timeout_secs.is_some() {
                let tick = iced::time::every(std::time::Duration::from_secs(1))
                    .map(|_| Message::Tick);
                return Subscription::batch([events, tick]);
            }
        }

        events
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Event(Event::Keyboard(keyboard::Event::KeyPressed { key, .. })) => {
                match key {
                    Key::Named(keyboard::key::Named::Enter) => {
                        EXIT_CODE.store(0, Ordering::SeqCst); // Confirmed
                        Task::done(Message::UnLock)
                    }
                    Key::Named(keyboard::key::Named::Escape) => {
                        EXIT_CODE.store(1, Ordering::SeqCst); // Denied
                        Task::done(Message::UnLock)
                    }
                    _ => Task::none(),
                }
            }
            Message::Tick => {
                if let Some(config) = CONFIG.get() {
                    if let Some(timeout) = config.timeout_secs {
                        if self.start_time.elapsed().as_secs() >= timeout as u64 {
                            EXIT_CODE.store(2, Ordering::SeqCst); // Timeout
                            return Task::done(Message::UnLock);
                        }
                    }
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn view(&self, _id: Id) -> Element<'_, Message> {
        let config = CONFIG.get().expect("config not set");
        let theme = ayu_dark_theme();

        let title = text(config.title()).size(48);
        let subtitle = text(config.subtitle()).size(28);
        let detail = text(config.detail()).size(32);

        let actions = row![
            text("[Enter] Allow").size(32).color(theme.palette().success),
            text("[Esc] Deny").size(32).color(theme.palette().danger),
        ]
        .spacing(30);

        // Show timeout if configured
        let mut content_items: Vec<Element<'_, Message>> = vec![
            title.into(),
            horizontal_rule(1).into(),
            subtitle.into(),
            detail.into(),
        ];

        if let Some(timeout) = config.timeout_secs {
            let elapsed = self.start_time.elapsed().as_secs() as u32;
            let remaining = timeout.saturating_sub(elapsed);
            let timeout_text = text(format!("Auto-deny in {}s", remaining))
                .size(24)
                .color(theme.palette().danger);
            content_items.push(timeout_text.into());
        }

        content_items.push(actions.into());

        let content = column(content_items).spacing(16).padding(30);

        container(content)
            .center_x(iced::Length::Fill)
            .center_y(iced::Length::Fill)
            .style(|_theme| container::Style {
                background: Some(Color::from_rgba(0.05, 0.06, 0.08, 0.95).into()),
                border: iced::Border {
                    color: Color::from_rgb8(0x56, 0x5B, 0x66),
                    width: 2.0,
                    radius: Radius::from(16.0),
                },
                ..Default::default()
            })
            .into()
    }
}

fn ayu_dark_theme() -> Theme {
    Theme::custom(
        "Ayu Dark".to_string(),
        Palette {
            background: Color::from_rgb8(0x0B, 0x0E, 0x14),
            text: Color::from_rgb8(0xBF, 0xBD, 0xB6),
            primary: Color::from_rgb8(0xE6, 0xB4, 0x50),
            success: Color::from_rgb8(0xAA, 0xD9, 0x4C),
            danger: Color::from_rgb8(0xD9, 0x57, 0x57),
        },
    )
}
