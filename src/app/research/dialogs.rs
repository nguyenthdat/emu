//! Interactive Two-Step Safety Gate modal dialogs.
//!
//! Defined in accordance with plan.md §Structure, FR-008, and SC-015.

use crate::models::research::MutationProposal;
use crate::ui::Theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

pub struct SafetyGateModal;

impl SafetyGateModal {
    pub fn render(frame: &mut Frame, area: Rect, proposal: &MutationProposal, theme: &Theme) {
        // Center modal layout
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Length(12),
                Constraint::Percentage(25),
            ])
            .split(area);

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(vertical[1]);

        let modal_area = horizontal[1];
        frame.render_widget(Clear, modal_area);

        let lines = vec![
            Line::from(vec![Span::styled(
                "⚠️  TWO-STEP SAFETY GATE CONFIRMATION REQUIRED  ⚠️",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(format!("Operation: {:?}", proposal.operation_type)),
            Line::from(format!(
                "Target Guest: {}",
                proposal.target_instance_id.as_deref().unwrap_or("none")
            )),
            Line::from(format!(
                "Proposal Digest: {}",
                &proposal.proposal_digest.as_str()[..32]
            )),
            Line::from(format!("Expires At: {}", proposal.expires_at)),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Type confirmation string or press [Enter] to confirm, [Esc] to abort",
                Style::default().fg(theme.selected),
            )]),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Destructive Mutation Authorization ")
            .border_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, modal_area);
    }
}
