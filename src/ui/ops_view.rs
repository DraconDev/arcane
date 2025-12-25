use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw_ops_tab<B: Backend>(f: &mut Frame<B>, _area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.size());

    let block = Block::default()
        .title(" Arcane Ops (Phase 1) ")
        .borders(Borders::ALL);

    let text = Paragraph::new(
        "Placeholder for Ops Dashboard.\nComing Soon: Sockets, SSH, and Containers.",
    )
    .block(block);

    f.render_widget(text, chunks[0]);
}
