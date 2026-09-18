//! Interactive image catalog panel.
//!
//! Defined in accordance with plan.md §Structure, FR-031, and FR-048.

use crate::models::research::ResearchImageArtifact;
use crate::ui::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub struct ImagePanelWidget;

impl ImagePanelWidget {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        artifacts: &[ResearchImageArtifact],
        theme: &Theme,
    ) {
        let mut lines = vec![Line::from(vec![Span::styled(
            "Registered Research Image Artifacts:",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )])];

        if artifacts.is_empty() {
            lines.push(Line::from("  (No image artifacts registered)"));
        } else {
            for art in artifacts {
                lines.push(Line::from(format!(
                    "  • {} [{:?}] ({})",
                    art.artifact_id, art.artifact_type, art.build_version_identity
                )));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Actions: [r] Register Image  [p] Prepare Loopback  [v] Verify Mount",
            Style::default().fg(theme.selected),
        )]));

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Image Catalog & Preparation ")
                    .border_style(Style::default().fg(theme.primary)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}
