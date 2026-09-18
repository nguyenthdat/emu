//! Interactive application lifecycle and container management panel.
//!
//! Defined in accordance with plan.md §Structure, FR-016, FR-024, and FR-048.

use crate::models::research::BackendType;
use crate::ui::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub struct AppPanelWidget;

impl AppPanelWidget {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        backend: Option<BackendType>,
        installed_apps: &[String],
        theme: &Theme,
    ) {
        let text = match backend {
            Some(BackendType::DarwinVm) => vec![
                Line::from(vec![Span::styled(
                    "Application frameworks unsupported on darwin-vm.",
                    Style::default().fg(Color::Yellow),
                )]),
                Line::from("Darwin VM provides headless kernel/Darwin execution only."),
            ],
            Some(BackendType::Inferno) => {
                let mut lines = vec![Line::from(vec![Span::styled(
                    "Installed iOS Applications (Inferno QEMU-SPTM):",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                )])];
                if installed_apps.is_empty() {
                    lines.push(Line::from("  (No applications installed)"));
                } else {
                    for app in installed_apps {
                        lines.push(Line::from(format!("  • {app}")));
                    }
                }
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "Actions: [i] Install IPA  [s] Start App  [x] Export Sandbox Container",
                    Style::default().fg(theme.selected),
                )]));
                lines
            }
            None => vec![Line::from(
                "Select a guest instance to view installed apps.",
            )],
        };

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" iOS Applications & Containers ")
                    .border_style(Style::default().fg(theme.primary)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}
