//! Interactive root proof status panel and console controller.
//!
//! Defined in accordance with plan.md §Structure, FR-010, FR-011, FR-014, and FR-048.

use crate::models::research::{ResearchGuestInstance, RootVerificationState};
use crate::ui::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub struct RootStatusWidget;

impl RootStatusWidget {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        guest: Option<&ResearchGuestInstance>,
        theme: &Theme,
    ) {
        let text = match guest {
            Some(g) => {
                let (badge, badge_color) = match g.observed_privilege {
                    RootVerificationState::Verified => ("[ VERIFIED ROOT ]", Color::Green),
                    RootVerificationState::Verifying => ("[ VERIFYING... ]", Color::Yellow),
                    RootVerificationState::Invalidated => ("[ INVALIDATED ]", Color::Red),
                    RootVerificationState::Unverified => ("[ UNVERIFIED ]", Color::DarkGray),
                };

                vec![
                    Line::from(vec![
                        Span::raw("Privilege Status: "),
                        Span::styled(
                            badge,
                            Style::default()
                                .fg(badge_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(format!(
                        "Guest OS: {} ({})",
                        g.guest_os_version, g.build_identity
                    )),
                    Line::from(format!("Architecture: {:?}", g.guest_arch)),
                    Line::from(format!("Base Image Digest: {}", g.base_image_ref.as_str())),
                    Line::from(format!("Config Revision: {}", g.config_revision.as_str())),
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "Keybindings: [v] Verify Root Proof  [c] In-Guest Console",
                        Style::default().fg(theme.selected),
                    )]),
                ]
            }
            None => vec![Line::from("No research guest instance selected.")],
        };

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Root Verification & Console ")
                    .border_style(Style::default().fg(theme.primary)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}
