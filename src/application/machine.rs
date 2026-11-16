use ratatui::{
    style::{Color, Style}, 
    text::{Line, Span},
};



pub enum MachineData {
    FromMachine(String),
    ToMachine(String),
}


impl<'a> Into<Line<'a>> for &MachineData {
    fn into(self) -> Line<'a> {
        match self {
            MachineData::FromMachine(data) => {
                let lines: Vec<Span> = data.clone() .lines()
                    .map(|line| Span::styled(format!("{}\n", line), Style::default().fg(Color::DarkGray)))
                    .collect();

                Line::from(lines)
            },
            MachineData::ToMachine(data) => {
                Line::from(vec![
                    Span::styled("> ", Style::default().fg(Color::White)),
                    Span::styled(data.clone(), Style::default().fg(Color::White)),
                ])
            }
        }
    }
}