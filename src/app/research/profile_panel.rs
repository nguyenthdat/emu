//! Interactive research profiles and baseline recovery panel.
//!
//! Defined in accordance with plan.md §Structure, FR-040, FR-043, and FR-048.

use crate::models::research::{RecoveryBaseline, ResearchExperimentProfile};
use crate::ui::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub struct ProfilePanelWidget;

impl ProfilePanelWidget {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        profiles: &[ResearchExperimentProfile],
        baselines: &[RecoveryBaseline],
        theme: &Theme,
    ) {
        let mut lines = vec![Line::from(vec![Span::styled(
            "Research Profiles & Recovery Baselines:",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )])];

        lines.push(Line::from(format!("Profiles Loaded: {}", profiles.len())));
        for p in profiles.iter().take(3) {
            lines.push(Line::from(format!("  • {} ({})", p.profile_id, p.name)));
        }

        lines.push(Line::from(format!(
            "Baselines Recorded: {}",
            baselines.len()
        )));
        for b in baselines.iter().take(3) {
            lines.push(Line::from(format!(
                "  • {} (Deadline: {}ms)",
                b.baseline_id, b.declared_recovery_deadline_ms
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Actions: [a] Apply Profile  [b] Create Baseline  [r] Restore Baseline",
            Style::default().fg(theme.selected),
        )]));

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Profiles & Recovery Baselines ")
                    .border_style(Style::default().fg(theme.primary)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}
