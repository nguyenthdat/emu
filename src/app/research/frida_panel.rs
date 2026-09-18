//! Interactive Frida dynamic instrumentation trace monitor panel.
//!
//! Defined in accordance with plan.md §Structure, FR-017, FR-018, FR-020, and FR-048.

use crate::ui::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub struct FridaPanelWidget;

impl FridaPanelWidget {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        session_info: Option<(&str, &str)>, // (bundle_id, session_id)
        recent_telemetry: &[String],
        theme: &Theme,
    ) {
        let mut lines = Vec::new();

        match session_info {
            Some((bundle, sess_id)) => {
                lines.push(Line::from(vec![
                    Span::raw("Active Frida Session: "),
                    Span::styled(
                        sess_id,
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(format!("Target App: {bundle}")));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "Recent Hook Telemetry:",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                )]));
                if recent_telemetry.is_empty() {
                    lines.push(Line::from("  (No telemetry events received)"));
                } else {
                    for event in recent_telemetry.iter().rev().take(6) {
                        lines.push(Line::from(format!("  • {event}")));
                    }
                }
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "Actions: [a] Attach Script  [d] Clean Detach",
                    Style::default().fg(theme.selected),
                )]));
            }
            None => {
                lines.push(Line::from(
                    "No active Frida dynamic instrumentation session.",
                ));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "Actions: [a] Attach to Target App",
                    Style::default().fg(theme.selected),
                )]));
            }
        }

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Frida 17.18.0 Dynamic Instrumentation ")
                    .border_style(Style::default().fg(theme.primary)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}
