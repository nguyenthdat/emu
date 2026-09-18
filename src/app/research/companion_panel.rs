//! Interactive companion VM orchestration panel.
//!
//! Defined in accordance with plan.md §Structure, FR-037, and FR-048.

use crate::models::research::CompanionEnvironment;
use crate::ui::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub struct CompanionPanelWidget;

impl CompanionPanelWidget {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        companion: Option<&CompanionEnvironment>,
        theme: &Theme,
    ) {
        let mut lines = Vec::new();

        match companion {
            Some(comp) => {
                lines.push(Line::from(vec![
                    Span::raw("Companion VM: "),
                    Span::styled(
                        format!("{:?}", comp.lifecycle_state),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(format!(
                    "CPUs: {} | RAM: {} MB",
                    comp.cpu_limit, comp.memory_limit_mb
                )));
                lines.push(Line::from(format!("Socket: {}", comp.endpoint_socket_path)));
                lines.push(Line::from(format!(
                    "Active Dependents: {}",
                    comp.live_dependents_count
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "Actions: [s] Stop Companion (refused if dependents > 0)",
                    Style::default().fg(theme.selected),
                )]));
            }
            None => {
                lines.push(Line::from("Local companion Linux VM is not running."));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "Actions: [c] Start Helper Companion VM",
                    Style::default().fg(theme.selected),
                )]));
            }
        }

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Local Helper Companion VM ")
                    .border_style(Style::default().fg(theme.primary)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}
