extern crate ncurses;
extern crate terminal_input;

use std::io::Write as _;
use std::os::raw::c_char;

use terminal_input::{InputStream, Event, Modifiers, KeyInput};

struct Screen(ncurses::WINDOW);

impl Drop for Screen {
    fn drop(&mut self) {
        ncurses::endwin();
    }
}

fn main() {
    unsafe { ncurses::ll::setlocale(ncurses::LC_ALL, b"\0".as_ptr() as *const c_char); }
    let screen = Screen(ncurses::initscr());
    ncurses::scrollok(screen.0, true);
    let stdin = std::io::stdin();
    let mut input_stream = unsafe { InputStream::init_with_ncurses(stdin.lock(), screen.0) };

    let mut out_file = None;
    if let Some(arg) = std::env::args_os().nth(1) {
        out_file = Some(std::fs::File::create(arg).unwrap());
    }

    loop {
        let event = input_stream.next_event();
        if let Some(ref mut file) = out_file {
            write!(file, "{:?}\n", event).unwrap();
        }
        ncurses::wprintw(screen.0, &format!("{:?}\n", event));
        ncurses::wrefresh(screen.0);

        if let Ok(Event::KeyPress { modifiers: Modifiers::CTRL, key: KeyInput::Codepoint('c'), .. })
             | Ok(Event::KeyPress { modifiers: Modifiers::CTRL, key: KeyInput::Codepoint('q'), .. }) = event {
            return;         
        }
    }
}
