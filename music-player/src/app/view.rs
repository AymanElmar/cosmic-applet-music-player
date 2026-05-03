use crate::app::{CosmicAppletMusic, Message};
use crate::lyrics::MAX_LYRIC_CHARS;
use cosmic::widget::Id;
use cosmic::Element;
use mpris::PlaybackStatus;
use std::sync::LazyLock;

pub mod view_window;

static AUTOSIZE_MAIN_ID: LazyLock<Id> = LazyLock::new(|| Id::new("autosize-main"));

const BUTTON_WIDTH: f32 = 400.0;

pub fn view(app: &CosmicAppletMusic) -> Element<'_, Message> {
    let show_all_players = app
        .config_manager
        .as_ref()
        .map(|c| c.get_show_all_players())
        .unwrap_or(false);

    let show_lyrics = app
        .config_manager
        .as_ref()
        .map(|c| c.get_show_lyrics())
        .unwrap_or(false);

    let content = if show_lyrics && !app.current_lyric.is_empty() {
        pad_to_fixed(&app.current_lyric)
    } else if show_all_players {
        let playing = app
            .all_players_info
            .iter()
            .find(|p| p.status == PlaybackStatus::Playing);
        if let Some(t) = playing {
            format_track("♫", &t.artist, &t.title)
        } else if let Some(t) = app.all_players_info.first() {
            format_track("‖", &t.artist, &t.title)
        } else {
            "♫".to_string()
        }
    } else {
        match app.player_info.status {
            PlaybackStatus::Playing => {
                format_track("♫", &app.player_info.artist, &app.player_info.title)
            }
            PlaybackStatus::Paused => {
                format_track("‖", &app.player_info.artist, &app.player_info.title)
            }
            PlaybackStatus::Stopped => "♫".to_string(),
        }
    };

    let suggested_padding = app.core.applet.suggested_padding(true);

    use cosmic::iced::{mouse, Length};

    let label = cosmic::widget::row()
        .width(Length::Fill)
        .align_y(cosmic::iced::Alignment::Center)
        .push(cosmic::widget::horizontal_space())
        .push(app.core.applet.text(content))
        .push(cosmic::widget::horizontal_space());

    cosmic::widget::autosize::autosize(
        cosmic::widget::mouse_area(
            cosmic::widget::button::custom(label)
                .width(Length::Fixed(BUTTON_WIDTH))
                .padding([0, suggested_padding])
                .class(cosmic::theme::Button::AppletIcon)
                .on_press(Message::TogglePopup),
        )
        .on_scroll(|delta| match delta {
            mouse::ScrollDelta::Lines { y, .. } => {
                if y > 0.0 { Message::ScrollUp } else { Message::ScrollDown }
            }
            mouse::ScrollDelta::Pixels { y, .. } => {
                if y > 0.0 { Message::ScrollUp } else { Message::ScrollDown }
            }
        })
        .on_middle_press(Message::MiddleClick),
        AUTOSIZE_MAIN_ID.clone(),
    )
    .into()
}

const TRACK_MAX_CHARS: usize = 40;

fn format_track(symbol: &str, artist: &str, title: &str) -> String {
    let text = if artist.is_empty() || artist == "Unknown Artist" {
        format!("{} {}", symbol, title)
    } else {
        format!("{} {} - {}", symbol, artist, title)
    };
    truncate(&text, TRACK_MAX_CHARS)
}

fn truncate(s: &str, max: usize) -> String {
    let mut chars = s.chars();
    let truncated: String = chars.by_ref().take(max).collect();
    if chars.next().is_some() {
        format!("{}…", truncated)
    } else {
        truncated
    }
}

// Pad lyric to a fixed char width so the panel button doesn't reflow on every line change.
fn pad_to_fixed(text: &str) -> String {
    let n = text.chars().count();
    if n < MAX_LYRIC_CHARS {
        format!("{}{}", text, " ".repeat(MAX_LYRIC_CHARS - n))
    } else {
        text.to_string()
    }
}
