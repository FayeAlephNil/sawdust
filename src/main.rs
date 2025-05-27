use core::error;
use crossterm::{
    ExecutableCommand, QueueableCommand,
    cursor::{self, SetCursorStyle},
    event::{Event, KeyCode, KeyEvent, poll, read},
    style::{self, Print, Stylize},
    terminal::{self, enable_raw_mode},
};
use std::{
    env, fmt::format, fs, io::{self, Write}, path::{self, Path}, time::Duration
};

#[derive(PartialEq, Eq)]
enum Mode {
    INSERT,
    NORMAL,
    REPLACE,
    QUIT,
}

struct SawdustApp {
    mode: Mode,
    term: io::Stdout,
    // Stored as vector of lines
    // TODO: Rework to more efficient method
    text: Vec<String>,
    top_line: usize,
    num_lines: usize,
    current_line: usize,
    current_col: usize,

    left_margin: u16,

    file_path: Option<path::PathBuf>
}

impl SawdustApp {
    fn handle_key_input(&mut self, key_event: KeyEvent) -> io::Result<()> {
        match self.mode {
            Mode::INSERT => self.handle_key_input_ins(key_event),
            Mode::REPLACE => self.handle_key_input_rep(key_event),
            Mode::NORMAL => self.handle_key_input_norm(key_event)?,
            Mode::QUIT => (),
        }
        Ok(())
    }

    fn get_col_in_line(&self) -> usize {
        return match self.mode {
            Mode::INSERT => self.current_col.min(self.text[self.current_line].len()),
            _ => {
                if self.text[self.current_line].is_empty() {
                    0
                } else {
                    self.current_col.min(self.text[self.current_line].len() - 1)
                }
            }
        };
    }

    fn handle_key_input_rep(&mut self, key_event: KeyEvent) {
        let col_in_line = self.get_col_in_line();
        if key_event.is_press() {
            match key_event.code {
                KeyCode::Esc => self.mode = Mode::NORMAL,
                KeyCode::Char(c) => {
                    self.text[self.current_line].remove(col_in_line);
                    self.text[self.current_line].insert(col_in_line, c);
                }
                _ => (),
            }
        }
    }

    fn handle_key_input_ins(&mut self, key_event: KeyEvent) {
        let col_in_line = self.get_col_in_line();
        if key_event.is_press() {
            match key_event.code {
                KeyCode::Esc => {
                    self.mode = Mode::NORMAL;
                    self.term
                        .queue(SetCursorStyle::BlinkingBlock)
                        .expect("Cannot set Cursor Style to Blinking Block");
                }
                KeyCode::Char(c) => {
                    self.text[self.current_line].insert(col_in_line, c);
                    self.current_col += 1;
                }
                KeyCode::Backspace => {
                    if col_in_line > 0 {
                        self.text[self.current_line].remove(col_in_line - 1);
                        self.current_col -= 1;
                    } else if self.current_line > 0 {
                        let cur_line = self.text[self.current_line].clone();
                        let before_line = self.text[self.current_line - 1].clone();
                        let before_line_len = before_line.len();
                        if before_line_len > 0 {
                            self.current_col = before_line_len;
                        }
                        self.text.remove(self.current_line);

                        self.current_line -= 1;
                        self.text[self.current_line] = before_line + &cur_line;
                        // TODO: Line joines
                    }
                }
                KeyCode::Enter => {
                    self.text.insert(self.current_line + 1, "".to_string());
                    self.current_line += 1;
                }
                _ => (),
            }
        }
    }

    fn handle_key_input_norm(&mut self, key_event: KeyEvent) -> io::Result<()> {
        let this_line = &self.text[self.current_line];
        let this_line_len = this_line.len();

        if key_event.is_press() {
            match key_event.code {
                KeyCode::Char('i') => {
                    self.current_col = self.get_col_in_line();
                    self.mode = Mode::INSERT;
                    self.term
                        .queue(cursor::SetCursorStyle::BlinkingBar)
                        .expect("Cannot set to Blinking Bar");
                }
                KeyCode::Char('A') => {
                    self.mode = Mode::INSERT;
                    self.term
                        .queue(cursor::SetCursorStyle::BlinkingBar)
                        .expect("Cannot set to Blinking Bar");

                    self.current_col = this_line_len;
                }
                KeyCode::Char('I') => {
                    self.mode = Mode::INSERT;
                    self.term
                        .queue(cursor::SetCursorStyle::BlinkingBar)
                        .expect("Cannot set to Blinking Bar");

                    self.current_col = 0;
                }
                KeyCode::Char('r') => self.mode = Mode::REPLACE,
                KeyCode::Char('q') => self.mode = Mode::QUIT,
                KeyCode::Char('j') => {
                    if self.current_line < self.text.len() - 1 {
                        self.current_line += 1;
                    }
                }
                KeyCode::Char('k') => {
                    if self.current_line > 0 {
                        self.current_line -= 1;
                    }
                }
                KeyCode::Char('l') => {
                    if this_line_len > 0 && self.current_col < this_line_len - 1 {
                        self.current_col += 1;
                    }
                }
                KeyCode::Char('h') => {
                    if this_line_len > 0 {
                        self.current_col = self.current_col.min(this_line_len - 1);
                    }

                    if self.current_col > 0 {
                        self.current_col -= 1;
                    }
                }
                KeyCode::Char('u') => {
                    todo!("Undo not implemented")
                }
                KeyCode::Char('w') => {
                    self.write_to_file()?;
                }

                KeyCode::Char(':') => {
                    todo!("Command Mode not Implemented")
                }
                _ => (),
            }
        }
        Ok(())
    }

    fn write_to_file(&self) -> io::Result<()> {
        match &self.file_path {
            Some(path) => {
                let contents = self.text.join("\n");
                fs::write(path, contents)
            }
            None => Ok(())
        }
    }

    fn draw_line(&mut self, line: &String, height: u16, line_number: usize) -> io::Result<()> {
        if self.current_line == line_number {
            self.term
                .queue(cursor::MoveTo(0, height))?
                .queue(style::Print(format!("{:}", line_number)))?;
        } else {
            self.term
                .queue(cursor::MoveTo(self.left_margin - 2, height))?
                .queue(style::Print(format!("{:}", line_number)))?;
        }

        for (idx, c) in line.chars().take(80).enumerate() {
            self.term
                .queue(cursor::MoveTo(idx as u16 + self.left_margin, height))?
                .queue(style::Print(c))?;
        }
        Ok(())
    }

    fn draw_lines(&mut self) -> io::Result<()> {
        for idx in 0..self.num_lines {
            let str = match self.text.get(self.top_line + idx) {
                Some(x) => &x.clone(),
                None => &"".to_string(),
            };
            self.draw_line(str, idx as u16, self.top_line + idx)?
        }
        Ok(())
    }

    fn move_cursor(&mut self) -> io::Result<()> {
        let to_move_x = self.get_col_in_line() as u16 + self.left_margin;
        let to_move_y = self.current_line as u16;
        self.term.queue(cursor::MoveTo(to_move_x, to_move_y))?;
        Ok(())
    }
}

fn clean_up(term: &mut io::Stdout) -> io::Result<()> {
    term.queue(terminal::Clear(terminal::ClearType::All))?;
    term.queue(cursor::MoveTo(0, 0))?;
    term.flush()?;
    Ok(())
}

fn read_file(file_name: &std::path::Path) -> Result<Vec<String>, io::Error> {
    Ok(fs::read_to_string(file_name)?
        .split("\n")
        .map(|x| x.to_string())
        .collect())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let mut app = SawdustApp {
        mode: Mode::NORMAL,
        term: io::stdout(),
        text: vec!["".to_string()],
        top_line: 0,
        current_line: 0,
        num_lines: 20,
        current_col: 0,
        left_margin: 4,
        file_path: None,
    };
    enable_raw_mode()?;
    if !args[1].is_empty() {
        let file_path = path::Path::new(&args[1]);
        app.file_path = Some(file_path.to_path_buf());
        match read_file(file_path) {
            Ok(txt) => app.text = txt,
            Err(e) => println!("{:?}", e),
        }
    }

    while app.mode != Mode::QUIT {
        if poll(Duration::from_millis(500))? {
            match read()? {
                Event::Key(e) => app.handle_key_input(e)?,
                _ => (),
            }
        } else {
        }
        app.term.queue(terminal::Clear(terminal::ClearType::All))?;
        app.draw_lines()?;
        app.move_cursor()?;
        app.term.flush()?;
    }
    clean_up(&mut app.term)?;
    Ok(())
}
