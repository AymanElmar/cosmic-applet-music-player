use serde::Deserialize;
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct LyricLine {
    pub time_us: u64,
    pub text: String,
}

#[derive(Deserialize)]
struct LrclibTrack {
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
}

pub async fn fetch_lyrics(
    artist: &str,
    title: &str,
    album: &str,
    duration_us: u64,
) -> Option<Vec<LyricLine>> {
    let duration_secs = duration_us / 1_000_000;
    let client = reqwest::Client::new();
    if let Some(lrc) = match_lrclib(&client, artist, title, album, duration_secs).await {
        return parse_lrc(&lrc);
    }
    if let Some(lrc) = search_lrclib(&client, artist, title).await {
        return parse_lrc(&lrc);
    }
    None
}

async fn match_lrclib(
    client: &reqwest::Client,
    artist: &str,
    title: &str,
    album: &str,
    duration_secs: u64,
) -> Option<String> {
    let resp = client
        .get("https://lrclib.net/api/get")
        .query(&[
            ("artist_name", artist),
            ("track_name", title),
            ("album_name", album),
            ("track_duration", &duration_secs.to_string()),
        ])
        .send()
        .await
        .ok()?;
    let track: LrclibTrack = resp.json().await.ok()?;
    track.synced_lyrics.filter(|s| !s.is_empty())
}

async fn search_lrclib(
    client: &reqwest::Client,
    artist: &str,
    title: &str,
) -> Option<String> {
    let query = format!("{} {}", artist, title);
    let resp = client
        .get("https://lrclib.net/api/search")
        .query(&[("q", &query)])
        .send()
        .await
        .ok()?;
    let tracks: Vec<LrclibTrack> = resp.json().await.ok()?;
    tracks
        .into_iter()
        .find_map(|t| t.synced_lyrics.filter(|s| !s.is_empty()))
}

fn parse_lrc(lrc: &str) -> Option<Vec<LyricLine>> {
    let mut lines: Vec<LyricLine> = lrc
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let close = line.find(']')?;
            let timestamp = &line[1..close];
            let text = line[close + 1..].trim().to_string();
            let time_us = parse_timestamp_us(timestamp)?;
            Some(LyricLine { time_us, text })
        })
        .collect();
    if lines.is_empty() {
        return None;
    }
    if lines[0].time_us != 0 {
        lines.insert(0, LyricLine { time_us: 0, text: String::new() });
    }
    Some(lines)
}

fn parse_timestamp_us(ts: &str) -> Option<u64> {
    let (min_str, sec_str) = ts.split_once(':')?;
    let minutes: f64 = min_str.parse().ok()?;
    let seconds: f64 = sec_str.parse().ok()?;
    Some(((minutes * 60.0 + seconds) * 1_000_000.0) as u64)
}

pub fn current_line_idx(lyrics: &[LyricLine], position_us: u64) -> usize {
    if lyrics.is_empty() {
        return 0;
    }
    let idx = lyrics.partition_point(|l| l.time_us <= position_us);
    idx.saturating_sub(1)
}

pub fn chunk_idx_for_position(
    line_start_us: u64,
    line_end_us: u64,
    position_us: u64,
    num_chunks: usize,
) -> usize {
    if num_chunks <= 1 {
        return 0;
    }
    let elapsed = position_us.saturating_sub(line_start_us);
    if line_end_us == u64::MAX {
        return ((elapsed / 2_000_000) as usize) % num_chunks;
    }
    let duration = line_end_us.saturating_sub(line_start_us);
    if duration == 0 {
        return 0;
    }
    let fraction = elapsed as f64 / duration as f64;
    ((fraction * num_chunks as f64) as usize).min(num_chunks - 1)
}

pub fn to_romaji(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let romaji = kakasi::convert(text).romaji;
    let collapsed: Cow<str> = if romaji.contains("  ") {
        Cow::Owned(romaji.split_whitespace().collect::<Vec<_>>().join(" "))
    } else {
        Cow::Borrowed(&romaji)
    };
    collapsed.trim().to_string()
}

pub const MAX_LYRIC_CHARS: usize = 45;

pub fn split_into_chunks(text: &str, max_chars: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    if text.chars().count() <= max_chars {
        return vec![text.to_string()];
    }
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let mut word_remaining = word;
        while !word_remaining.is_empty() {
            let fits: usize = word_remaining
                .char_indices()
                .take(max_chars)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(0);
            let (head, tail) = word_remaining.split_at(fits);
            if current.is_empty() {
                current.push_str(head);
            } else if current.chars().count() + 1 + head.chars().count() <= max_chars {
                current.push(' ');
                current.push_str(head);
            } else {
                chunks.push(current.clone());
                current = head.to_string();
            }
            word_remaining = tail;
            if !tail.is_empty() {
                chunks.push(current.clone());
                current = String::new();
            }
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    if chunks.is_empty() {
        chunks.push(String::new());
    }
    chunks
}
