//! Interactive research guest instance table widget and action controllers.
//!
//! Defined in accordance with plan.md §Structure, FR-006, and FR-048.

use crate::models::research::{BackendType, InstanceLifecycleState, ResearchGuestInstance};
use crate::ui::Theme;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
};

pub struct GuestTableWidget;

impl GuestTableWidget {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        guests: &[ResearchGuestInstance],
        selected_idx: Option<usize>,
        theme: &Theme,
    ) {
        let header_cells = [
            "ID",
            "Name",
            "Backend",
            "Status",
            "Privilege",
            "Architecture",
        ]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )
        });
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let rows = guests.iter().enumerate().map(|(i, g)| {
            let is_selected = selected_idx == Some(i);
            let backend_badge = match g.backend {
                BackendType::DarwinVm => "darwin-vm",
                BackendType::Inferno => "inferno",
            };

            let status_style = match g.lifecycle_state {
                InstanceLifecycleState::Running => Style::default().fg(Color::Green),
                InstanceLifecycleState::Stopped => Style::default().fg(Color::DarkGray),
                _ => Style::default().fg(Color::Yellow),
            };

            let row_style = if is_selected {
                Style::default()
                    .bg(theme.selected)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };

            Row::new(vec![
                Cell::from(g.id.simple().to_string()),
                Cell::from(g.display_name.clone()),
                Cell::from(backend_badge),
                Cell::from(format!("{:?}", g.lifecycle_state)).style(status_style),
                Cell::from(format!("{:?}", g.observed_privilege)),
                Cell::from(format!("{:?}", g.guest_arch)),
            ])
            .style(row_style)
            .height(1)
        });

        let widths = [
            Constraint::Length(10),
            Constraint::Length(20),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(14),
            Constraint::Length(12),
        ];

        let table = Table::new(rows, widths).header(header).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Research Guests (darwin-vm / Inferno) ")
                .border_style(Style::default().fg(theme.primary)),
        );

        frame.render_widget(table, area);
    }
}
