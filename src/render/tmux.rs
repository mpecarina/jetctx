use serde::Serialize;

use crate::config::Config;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TmuxTarget {
    Right,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TmuxContext {
    pub battery_percent: Option<u8>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    pub time_label: Option<String>,
}

pub fn render(
    target: TmuxTarget,
    context: &TmuxContext,
    _app_config: &Config,
    theme: &Theme,
) -> String {
    match target {
        TmuxTarget::Right => render_right(context, _app_config, theme),
    }
}

fn render_right(context: &TmuxContext, config: &Config, theme: &Theme) -> String {
    let mut segments = Vec::new();
    let tmux = &theme.tmux;

    if let Some(battery) = render_battery_label(context, &config.tmux.battery_symbol) {
        let (bg, fg) = battery_style(context.battery_percent, tmux);
        segments.push(style_segment(&battery, bg, fg));
    }

    if config.tmux.show_memory {
        if let Some(memory) = render_memory_label(context, &config.tmux.memory_symbol) {
            segments.push(style_segment(
                &memory,
                tmux.segment_info_bg.as_deref(),
                tmux.segment_info_fg.as_deref(),
            ));
        }
    }

    if let Some(time) = non_empty(context.time_label.as_deref()) {
        segments.push(style_segment(
            &format!("{} {}", config.tmux.time_symbol, time),
            tmux.segment_time_bg.as_deref(),
            tmux.segment_time_fg.as_deref(),
        ));
    }

    join_segments(segments)
}

fn render_battery_label(context: &TmuxContext, symbol: &str) -> Option<String> {
    let percent = context.battery_percent?;
    Some(format!("{} {percent}%", symbol))
}

fn render_memory_label(context: &TmuxContext, symbol: &str) -> Option<String> {
    let used = context.memory_used_bytes?;
    let total = context.memory_total_bytes?;

    if total == 0 {
        return None;
    }

    Some(format!(
        "{} {}/{}",
        symbol,
        human_bytes(used),
        human_bytes(total)
    ))
}

fn battery_style(
    percent: Option<u8>,
    tmux: &crate::theme::TmuxPalette,
) -> (Option<&str>, Option<&str>) {
    match percent {
        Some(p) if p <= 20 => (
            tmux.segment_error_bg.as_deref(),
            tmux.segment_error_fg.as_deref(),
        ),
        Some(p) if p <= 60 => (
            tmux.segment_warn_bg.as_deref(),
            tmux.segment_warn_fg.as_deref(),
        ),
        _ => (
            tmux.segment_info_bg.as_deref(),
            tmux.segment_info_fg.as_deref(),
        ),
    }
}

fn join_segments(segments: Vec<String>) -> String {
    segments
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn style_segment(text: &str, bg: Option<&str>, fg: Option<&str>) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut parts = Vec::new();

    if let Some(bg) = non_empty(bg) {
        parts.push(format!("bg={bg}"));
    }

    if let Some(fg) = non_empty(fg) {
        parts.push(format!("fg={fg}"));
    }

    if parts.is_empty() {
        text.to_string()
    } else {
        format!("#[{}] {} #[default]", parts.join(","), text)
    }
}

fn human_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;

    let b = bytes as f64;

    if b >= GIB {
        format!("{:.1}G", b / GIB)
    } else if b >= MIB {
        format!("{:.1}M", b / MIB)
    } else if b >= KIB {
        format!("{:.1}K", b / KIB)
    } else {
        format!("{bytes}B")
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::theme::{Theme, TmuxPalette};

    fn test_theme() -> Theme {
        Theme {
            name: "test".to_string(),
            base: Default::default(),
            semantic: Default::default(),
            prompt: Default::default(),
            tmux: TmuxPalette {
                segment_info_bg: Some("#222222".to_string()),
                segment_info_fg: Some("#ffffff".to_string()),
                segment_warn_bg: Some("#444400".to_string()),
                segment_warn_fg: Some("#ffffff".to_string()),
                segment_error_bg: Some("#660000".to_string()),
                segment_error_fg: Some("#ffffff".to_string()),
                segment_time_bg: Some("#333333".to_string()),
                segment_time_fg: Some("#ffffff".to_string()),
            },
        }
    }

    fn test_config() -> Config {
        Config::default()
    }

    #[test]
    fn renders_battery_memory_and_time() {
        let context = TmuxContext {
            battery_percent: Some(82),
            memory_used_bytes: Some(8 * 1024 * 1024 * 1024),
            memory_total_bytes: Some(16 * 1024 * 1024 * 1024),
            time_label: Some("10:42".to_string()),
        };

        let rendered = render(TmuxTarget::Right, &context, &test_config(), &test_theme());

        assert_eq!(
            rendered,
            "#[bg=#222222,fg=#ffffff] BAT 82% #[default] \
             #[bg=#222222,fg=#ffffff] MEM 8.0G/16.0G #[default] \
             #[bg=#333333,fg=#ffffff] ◷ 10:42 #[default]"
        );
    }

    #[test]
    fn preserves_battery_threshold_styles() {
        let mut context = TmuxContext {
            battery_percent: Some(20),
            time_label: None,
            ..Default::default()
        };

        assert_eq!(
            render(TmuxTarget::Right, &context, &test_config(), &test_theme()),
            "#[bg=#660000,fg=#ffffff] BAT 20% #[default]"
        );

        context.battery_percent = Some(60);
        assert_eq!(
            render(TmuxTarget::Right, &context, &test_config(), &test_theme()),
            "#[bg=#444400,fg=#ffffff] BAT 60% #[default]"
        );
    }

    #[test]
    fn flat_theme_keeps_tmux_output_unstyled() {
        let context = TmuxContext {
            battery_percent: Some(82),
            memory_used_bytes: None,
            memory_total_bytes: None,
            time_label: Some("10:42".to_string()),
        };
        let theme = Theme {
            name: "flat".to_string(),
            base: Default::default(),
            semantic: Default::default(),
            prompt: Default::default(),
            tmux: Default::default(),
        };

        assert_eq!(
            render(TmuxTarget::Right, &context, &test_config(), &theme),
            "BAT 82% ◷ 10:42"
        );
    }
}
