use super::*;
use crate::config::StatusIndicatorStyle;
use std::time::{Duration, Instant};

fn animated_state(status: AgentStatus) -> ClientShellState {
    let mut config = Config::default();
    config.ui.status_indicators = StatusIndicatorStyle::Animated;
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&config));
    let mut projected = snapshot();
    projected.workspaces[0].agent_status = status;
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state
}

fn frame_text(state: &mut ClientShellState) -> String {
    let frame = state.compose(106, 24).expect("shell frame");
    frame
        .cells
        .chunks(frame.width as usize)
        .map(|row| {
            row.iter()
                .map(|cell| cell.symbol.as_str())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn animated_working_glyph_cycles_through_braille_frames() {
    for (phase, expected) in SPINNER_FRAMES.iter().enumerate() {
        let indicators = StatusIndicators {
            style: StatusIndicatorStyle::Animated,
            spinner_phase: phase as u8,
        };
        assert_eq!(status_icon(AgentStatus::Working, indicators), *expected);
        // Every non-working glyph matches the static symbol set regardless of phase.
        assert_eq!(status_icon(AgentStatus::Blocked, indicators), "×");
        assert_eq!(status_icon(AgentStatus::Done, indicators), "✓");
        assert_eq!(status_icon(AgentStatus::Idle, indicators), "○");
        assert_eq!(status_icon(AgentStatus::Unknown, indicators), "·");
    }
    let wrapped = StatusIndicators {
        style: StatusIndicatorStyle::Animated,
        spinner_phase: SPINNER_FRAMES.len() as u8,
    };
    assert_eq!(
        status_icon(AgentStatus::Working, wrapped),
        SPINNER_FRAMES[0]
    );
}

#[test]
fn static_styles_ignore_the_spinner_phase() {
    for style in [StatusIndicatorStyle::Dots, StatusIndicatorStyle::Symbols] {
        let still = StatusIndicators::new(style);
        let moved = StatusIndicators {
            style,
            spinner_phase: 3,
        };
        assert_eq!(
            status_icon(AgentStatus::Working, still),
            status_icon(AgentStatus::Working, moved)
        );
    }
}

#[test]
fn spinner_only_arms_while_an_agent_is_working() {
    let now = Instant::now();
    let mut state = animated_state(AgentStatus::Idle);
    assert!(!state.tick_spinner(now));
    assert_eq!(state.spinner_deadline, None);
    assert_eq!(state.timer_delay(now), Duration::from_millis(100));

    let mut state = animated_state(AgentStatus::Working);
    // First tick arms the timer without repainting.
    assert!(!state.tick_spinner(now));
    assert_eq!(state.spinner_deadline, Some(now + SPINNER_INTERVAL));
    // The loop wakes for the frame deadline once it falls inside the idle poll.
    assert_eq!(
        state.timer_delay(now + Duration::from_millis(50)),
        SPINNER_INTERVAL - Duration::from_millis(50)
    );
    assert_eq!(state.config.status_indicators.spinner_phase, 0);
    // Early wake-ups leave the frame alone.
    assert!(!state.tick_spinner(now + SPINNER_INTERVAL / 2));
    assert_eq!(state.config.status_indicators.spinner_phase, 0);
    // The due tick advances the frame and asks for a repaint.
    assert!(state.tick_spinner(now + SPINNER_INTERVAL));
    assert_eq!(state.config.status_indicators.spinner_phase, 1);
    assert_eq!(
        state.spinner_deadline,
        Some(now + SPINNER_INTERVAL + SPINNER_INTERVAL)
    );

    // Leaving the animated style disarms the timer.
    state.config.status_indicators.style = StatusIndicatorStyle::Symbols;
    assert!(!state.tick_spinner(now + SPINNER_INTERVAL * 2));
    assert_eq!(state.spinner_deadline, None);
    assert_eq!(state.timer_delay(now), Duration::from_millis(100));
}

#[test]
fn spinner_phase_wraps_after_the_last_frame() {
    let now = Instant::now();
    let mut state = animated_state(AgentStatus::Working);
    assert!(!state.tick_spinner(now));
    for tick in 1..=SPINNER_FRAMES.len() {
        assert!(state.tick_spinner(now + SPINNER_INTERVAL * tick as u32));
    }
    assert_eq!(state.config.status_indicators.spinner_phase, 0);
}

#[test]
fn composed_shell_paints_the_current_spinner_frame() {
    let now = Instant::now();
    let mut state = animated_state(AgentStatus::Working);
    let text = frame_text(&mut state);
    assert!(text.contains(SPINNER_FRAMES[0]), "frame 0 missing:\n{text}");
    assert!(!text.contains(SPINNER_FRAMES[1]));

    assert!(!state.tick_spinner(now));
    assert!(state.tick_spinner(now + SPINNER_INTERVAL));
    let text = frame_text(&mut state);
    assert!(text.contains(SPINNER_FRAMES[1]), "frame 1 missing:\n{text}");
    assert!(!text.contains(SPINNER_FRAMES[0]));
}
