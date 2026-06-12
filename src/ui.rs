//! Iced session-lock dialog UI

use crate::DialogConfig;
use iced::border::Radius;
use iced::font::Weight;
use iced::keyboard::{self, Key};
use iced::widget::{column, container, row, text, Space};
use iced::window::Id;
use iced::{Alignment, Color, Element, Event, Font, Length, Subscription, Task};
use iced_sessionlock::actions::UnLockAction;
use iced_sessionlock::application;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;

static CONFIG: Mutex<Option<DialogConfig>> = Mutex::new(None);
static EXIT_CODE: AtomicI32 = AtomicI32::new(1); // Default: denied

// Ayu Dark palette
const SCRIM: Color = Color::from_rgba8(0x05, 0x08, 0x0D, 0.94);
const CARD_BG: Color = Color::from_rgb8(0x0F, 0x13, 0x1A);
const CARD_BORDER: Color = Color::from_rgb8(0x1C, 0x22, 0x2C);
const INSET_BG: Color = Color::from_rgb8(0x0A, 0x0D, 0x12);
const KEYCAP_BG: Color = Color::from_rgb8(0x1A, 0x20, 0x2A);
const KEYCAP_BORDER: Color = Color::from_rgb8(0x2B, 0x33, 0x40);
const TEXT_PRIMARY: Color = Color::from_rgb8(0xE6, 0xE1, 0xCF);
const TEXT_BODY: Color = Color::from_rgb8(0xBF, 0xBD, 0xB6);
const TEXT_MUTED: Color = Color::from_rgb8(0x8A, 0x91, 0x99);
const ACCENT: Color = Color::from_rgb8(0xE6, 0xB4, 0x50);
const SUCCESS: Color = Color::from_rgb8(0xAA, 0xD9, 0x4C);
const DANGER: Color = Color::from_rgb8(0xD9, 0x57, 0x57);

const CARD_WIDTH: f32 = 600.0;

/// Run the dialog UI and return exit code
///
/// Exit codes:
/// - 0: Confirmed
/// - 1: Denied
/// - 2: Timeout
/// - 3: Error
pub fn run(config: DialogConfig) -> i32 {
    *CONFIG.lock().unwrap() = Some(config);

    let result = application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .run();

    match result {
        Ok(()) => EXIT_CODE.load(Ordering::SeqCst),
        Err(_) => 3, // Error
    }
}

struct App {
    start_time: std::time::Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Event(Event),
    Tick,
    UnLock,
}

impl TryInto<UnLockAction> for Message {
    type Error = Self;
    fn try_into(self) -> Result<UnLockAction, Self::Error> {
        if let Self::UnLock = self {
            return Ok(UnLockAction);
        }
        Err(self)
    }
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

    fn subscription(&self) -> Subscription<Message> {
        let events = iced::event::listen().map(Message::Event);

        // Check timeout if configured
        let has_timeout = CONFIG
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|c| c.timeout_secs)
            .is_some();

        if has_timeout {
            let tick = iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick);
            return Subscription::batch([events, tick]);
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
                let timeout = CONFIG.lock().unwrap().as_ref().and_then(|c| c.timeout_secs);
                if let Some(timeout) = timeout {
                    if self.start_time.elapsed().as_secs() >= timeout as u64 {
                        EXIT_CODE.store(2, Ordering::SeqCst); // Timeout
                        return Task::done(Message::UnLock);
                    }
                }
                Task::none()
            }
            Message::UnLock => Task::done(Message::UnLock),
            _ => Task::none(),
        }
    }

    fn view(&self, _id: Id) -> Element<'_, Message> {
        let (title, subtitle, detail, timeout_secs) = {
            let guard = CONFIG.lock().unwrap();
            let config = guard.as_ref().expect("config not set");
            (
                config.title().to_string(),
                config.subtitle().to_string(),
                config.detail(),
                config.timeout_secs,
            )
        };

        let elapsed_secs = self.start_time.elapsed().as_secs() as u32;

        let card = container(
            column![
                header(&title, &subtitle),
                command_block(&detail),
                divider(),
                footer(timeout_secs, elapsed_secs),
            ]
            .spacing(20),
        )
        .width(Length::Fixed(CARD_WIDTH))
        .padding(28)
        .style(card_style);

        container(card)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(SCRIM.into()),
                ..Default::default()
            })
            .into()
    }
}

fn header<'a>(title: &str, subtitle: &str) -> Element<'a, Message> {
    column![
        text(title.to_string())
            .size(26)
            .font(Font {
                weight: Weight::Bold,
                ..Font::DEFAULT
            })
            .color(TEXT_PRIMARY),
        text(subtitle.to_string()).size(16).color(TEXT_BODY),
    ]
    .spacing(8)
    .into()
}

fn command_block<'a>(detail: &str) -> Element<'a, Message> {
    container(
        text(detail.to_string())
            .size(17)
            .font(Font::MONOSPACE)
            .color(ACCENT)
            .wrapping(text::Wrapping::WordOrGlyph),
    )
    .width(Length::Fill)
    .padding([14, 16])
    .style(|_theme| container::Style {
        background: Some(INSET_BG.into()),
        border: iced::Border {
            color: CARD_BORDER,
            width: 1.0,
            radius: Radius::from(8.0),
        },
        ..Default::default()
    })
    .into()
}

fn divider<'a>() -> Element<'a, Message> {
    container(Space::new().width(Length::Fill).height(1))
        .style(|_theme| container::Style {
            background: Some(CARD_BORDER.into()),
            ..Default::default()
        })
        .into()
}

fn footer<'a>(timeout_secs: Option<u32>, elapsed_secs: u32) -> Element<'a, Message> {
    let mut footer = row![
        keycap("Enter"),
        text("Allow").size(15).color(SUCCESS),
        Space::new().width(16),
        keycap("Esc"),
        text("Deny").size(15).color(DANGER),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    if let Some(timeout) = timeout_secs {
        let remaining = timeout.saturating_sub(elapsed_secs);
        let color = if remaining <= 5 { DANGER } else { TEXT_MUTED };
        footer = footer.push(Space::new().width(Length::Fill)).push(
            text(format!("Auto-deny in {}s", remaining))
                .size(14)
                .font(Font::MONOSPACE)
                .color(color),
        );
    }

    footer.into()
}

fn keycap<'a>(label: &'static str) -> Element<'a, Message> {
    container(text(label).size(13).font(Font::MONOSPACE).color(TEXT_BODY))
        .padding([3, 9])
        .style(|_theme| container::Style {
            background: Some(KEYCAP_BG.into()),
            border: iced::Border {
                color: KEYCAP_BORDER,
                width: 1.0,
                radius: Radius::from(5.0),
            },
            ..Default::default()
        })
        .into()
}

fn card_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(CARD_BG.into()),
        border: iced::Border {
            color: CARD_BORDER,
            width: 1.0,
            radius: Radius::from(12.0),
        },
        shadow: iced::Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.45),
            offset: iced::Vector::new(0.0, 12.0),
            blur_radius: 48.0,
        },
        ..Default::default()
    }
}
