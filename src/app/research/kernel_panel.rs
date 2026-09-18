//! Interactive kernel debugger registers, memory, and stepping panel.
//!
//! Defined in accordance with plan.md §Structure, FR-027, FR-028, FR-029, and FR-048.

use crate::ui::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub struct KernelPanelWidget;

impl KernelPanelWidget {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        lease_active: bool,
        cpu_paused: bool,
        pc: Option<u64>,
        sp: Option<u64>,
        theme: &Theme,
    ) {
        let mut lines = Vec::new();

        let lease_status = if lease_active {
            Span::styled(
                "[ EXCLUSIVE LEASE HELD ]",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled("[ NO ACTIVE LEASE ]", Style::default().fg(Color::DarkGray))
        };

        let runstate = if cpu_paused {
            Span::styled(
                "CPU PAUSED",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled("CPU RUNNING", Style::default().fg(Color::Green))
        };

        lines.push(Line::from(vec![Span::raw("Lease Status: "), lease_status]));
        lines.push(Line::from(vec![Span::raw("Runstate: "), runstate]));
        lines.push(Line::from(format!(
            "Program Counter (PC): 0x{:016x}",
            pc.unwrap_or(0)
        )));
        lines.push(Line::from(format!(
            "Stack Pointer (SP):   0x{:016x}",
            sp.unwrap_or(0)
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Controls: [p] Pause  [r] Resume  [s] Single-Step  [q] Disconnect (Preserve Paused)",
            Style::default().fg(theme.selected),
        )]));

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Kernel GDB RSP Debugger (ARM64) ")
                    .border_style(Style::default().fg(theme.primary)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}
