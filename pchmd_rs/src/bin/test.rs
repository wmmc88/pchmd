use std::io::{stdout, Write};
use std::thread;
use std::time::Duration;

use crossterm::{ExecutableCommand, QueueableCommand};

fn main() {
    // correct()
    broken()
}


fn correct() {
    let mut stdout = stdout();

    stdout.execute(crossterm::cursor::Hide).unwrap();

    for i in 0..20 {
        stdout
            .queue(crossterm::style::Print(format!("{i} \n")))
            .unwrap();
        stdout
            .queue(crossterm::style::Print(format!("{} \n", i * 2)))
            .unwrap();
        stdout.flush().unwrap();

        thread::sleep(Duration::from_millis(100));
        stdout.queue(crossterm::cursor::MoveUp(2)).unwrap(); // Required since: https://github.com/crossterm-rs/crossterm/issues/673
        stdout
            .queue(crossterm::terminal::Clear(
                crossterm::terminal::ClearType::FromCursorDown,
            ))
            .unwrap();
    }

    stdout.execute(crossterm::cursor::Show).unwrap();
}

fn broken() {
    let mut stdout = stdout();

    stdout.execute(crossterm::cursor::Hide).unwrap();
    crossterm::terminal::enable_raw_mode().expect("Failed to enable raw mode for terminal");

    for i in 0..20 {
        stdout.queue(crossterm::cursor::SavePosition).unwrap();
        stdout.queue(crossterm::style::Print(format!("{i} \n"))).unwrap();
        stdout
            .queue(crossterm::style::Print(format!("{} \n", i * 2)))
            .unwrap();
        stdout.flush().unwrap();

        thread::sleep(Duration::from_millis(100));
        stdout.queue(crossterm::cursor::RestorePosition).unwrap();
    }

    stdout.execute(crossterm::cursor::Show).unwrap();
    crossterm::terminal::disable_raw_mode().expect("Failed to disable raw mode for terminal");
}
