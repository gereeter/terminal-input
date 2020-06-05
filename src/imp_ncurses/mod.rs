use std::io::Write;
use std::ffi::CStr;
use const_cstr::ConstCStr;

use crate::Event::*;
use crate::{Event, Modifiers, KeyInput};

mod ext;

fn write_now(data: &[u8]) -> Result<(), std::io::Error> {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    lock.write_all(data)?;
    lock.flush()?;
    Ok(())
}

struct BracketedPaste {
    _priv: ()
}

impl Drop for BracketedPaste {
    fn drop(&mut self) {
        let _ = write_now(b"\x1b[?2004l");
    }
}

impl BracketedPaste {
    fn start() -> Option<BracketedPaste> {
        write_now(b"\x1b[?2004h").ok()?;
        Some(BracketedPaste { _priv: () })
    }
}

struct XTermModifyOtherKeys {
    _priv: ()
}

impl Drop for XTermModifyOtherKeys {
    fn drop(&mut self) {
        let _ = write_now(b"\x1b[>4n");
    }
}

impl XTermModifyOtherKeys {
    fn start() -> Option<XTermModifyOtherKeys> {
        write_now(b"\x1b[>4;2m").ok()?;
        Some(XTermModifyOtherKeys { _priv: () })
    }
}

enum XTermModifyKeyState {
    Off,
    ParsingMode(u32),
    ParsingChar(u32, u32)
}

struct KittyFullMode {
    _priv: ()
}

impl Drop for KittyFullMode {
    fn drop(&mut self) {
        let _ = write_now(b"\x1b[?2017l");
    }
}

impl KittyFullMode {
    fn start() -> Option<KittyFullMode> {
        write_now(b"\x1b[?2017h").ok()?;
        Some(KittyFullMode { _priv: () })
    }
}

#[derive(Copy, Clone, Debug)]
enum KeyType {
    Press,
    Release,
    Repeat
}

enum KittyFullModeState {
    Off,
    ParsingType,
    ParsingModifiers(KeyType),
    ParsingKey(KeyType, u8, u32)
}

pub struct InputStream {
    _bracketed_paste: Option<BracketedPaste>,
    _xterm_modify_keys: Option<XTermModifyOtherKeys>,
    _kitty_full_mode: Option<KittyFullMode>,

    extra_bound_keys: Vec<(i32, Event)>,
    
    in_progress_codepoint: u32,
    utf8_bytes_left: usize,
    xterm_modify_key_state: XTermModifyKeyState,
    kitty_full_mode_state: KittyFullModeState

}

unsafe fn define_if_necessary(def: &std::ffi::CStr, code: std::os::raw::c_int) -> Result<(), ()> {
    if ext::key_code_for(def) == Err(ext::KeyError::NotDefined) {
        ext::define_key_code(def, code)
    } else {
        Ok(())
    }
}

// TODO: instead parse any keys with numbers after them according to this pattern
const KNOWN_EXTRA_TERM_CAPABILITIES: &'static [(ConstCStr, Event)] = &[
    (const_cstr!("kDC3"), KeyPress { modifiers: Modifiers(3 - 1), key: KeyInput::Special(ncurses::KEY_DC), is_repeat: false }),
    (const_cstr!("kDC4"), KeyPress { modifiers: Modifiers(4 - 1), key: KeyInput::Special(ncurses::KEY_DC), is_repeat: false }),
    (const_cstr!("kDC5"), KeyPress { modifiers: Modifiers(5 - 1), key: KeyInput::Special(ncurses::KEY_DC), is_repeat: false }),
    (const_cstr!("kDC6"), KeyPress { modifiers: Modifiers(6 - 1), key: KeyInput::Special(ncurses::KEY_DC), is_repeat: false }),
    (const_cstr!("kDC7"), KeyPress { modifiers: Modifiers(7 - 1), key: KeyInput::Special(ncurses::KEY_DC), is_repeat: false }),
    (const_cstr!("kDC8"), KeyPress { modifiers: Modifiers(8 - 1), key: KeyInput::Special(ncurses::KEY_DC), is_repeat: false }),

    (const_cstr!("kLFT3"), KeyPress { modifiers: Modifiers(3 - 1), key: KeyInput::Special(ncurses::KEY_LEFT), is_repeat: false }),
    (const_cstr!("kLFT4"), KeyPress { modifiers: Modifiers(4 - 1), key: KeyInput::Special(ncurses::KEY_LEFT), is_repeat: false }),
    (const_cstr!("kLFT5"), KeyPress { modifiers: Modifiers(5 - 1), key: KeyInput::Special(ncurses::KEY_LEFT), is_repeat: false }),
    (const_cstr!("kLFT6"), KeyPress { modifiers: Modifiers(6 - 1), key: KeyInput::Special(ncurses::KEY_LEFT), is_repeat: false }),
    (const_cstr!("kLFT7"), KeyPress { modifiers: Modifiers(7 - 1), key: KeyInput::Special(ncurses::KEY_LEFT), is_repeat: false }),
    (const_cstr!("kLFT8"), KeyPress { modifiers: Modifiers(8 - 1), key: KeyInput::Special(ncurses::KEY_LEFT), is_repeat: false }),

    (const_cstr!("kRIT3"), KeyPress { modifiers: Modifiers(3 - 1), key: KeyInput::Special(ncurses::KEY_RIGHT), is_repeat: false }),
    (const_cstr!("kRIT4"), KeyPress { modifiers: Modifiers(4 - 1), key: KeyInput::Special(ncurses::KEY_RIGHT), is_repeat: false }),
    (const_cstr!("kRIT5"), KeyPress { modifiers: Modifiers(5 - 1), key: KeyInput::Special(ncurses::KEY_RIGHT), is_repeat: false }),
    (const_cstr!("kRIT6"), KeyPress { modifiers: Modifiers(6 - 1), key: KeyInput::Special(ncurses::KEY_RIGHT), is_repeat: false }),
    (const_cstr!("kRIT7"), KeyPress { modifiers: Modifiers(7 - 1), key: KeyInput::Special(ncurses::KEY_RIGHT), is_repeat: false }),
    (const_cstr!("kRIT8"), KeyPress { modifiers: Modifiers(8 - 1), key: KeyInput::Special(ncurses::KEY_RIGHT), is_repeat: false }),

    (const_cstr!("kUP3"), KeyPress { modifiers: Modifiers(3 - 1), key: KeyInput::Special(ncurses::KEY_UP), is_repeat: false }),
    (const_cstr!("kUP4"), KeyPress { modifiers: Modifiers(4 - 1), key: KeyInput::Special(ncurses::KEY_UP), is_repeat: false }),
    (const_cstr!("kUP5"), KeyPress { modifiers: Modifiers(5 - 1), key: KeyInput::Special(ncurses::KEY_UP), is_repeat: false }),
    (const_cstr!("kUP6"), KeyPress { modifiers: Modifiers(6 - 1), key: KeyInput::Special(ncurses::KEY_UP), is_repeat: false }),
    (const_cstr!("kUP7"), KeyPress { modifiers: Modifiers(7 - 1), key: KeyInput::Special(ncurses::KEY_UP), is_repeat: false }),
    (const_cstr!("kUP8"), KeyPress { modifiers: Modifiers(8 - 1), key: KeyInput::Special(ncurses::KEY_UP), is_repeat: false }),

    (const_cstr!("kDN3"), KeyPress { modifiers: Modifiers(3 - 1), key: KeyInput::Special(ncurses::KEY_DOWN), is_repeat: false }),
    (const_cstr!("kDN4"), KeyPress { modifiers: Modifiers(4 - 1), key: KeyInput::Special(ncurses::KEY_DOWN), is_repeat: false }),
    (const_cstr!("kDN5"), KeyPress { modifiers: Modifiers(5 - 1), key: KeyInput::Special(ncurses::KEY_DOWN), is_repeat: false }),
    (const_cstr!("kDN6"), KeyPress { modifiers: Modifiers(6 - 1), key: KeyInput::Special(ncurses::KEY_DOWN), is_repeat: false }),
    (const_cstr!("kDN7"), KeyPress { modifiers: Modifiers(7 - 1), key: KeyInput::Special(ncurses::KEY_DOWN), is_repeat: false }),
    (const_cstr!("kDN8"), KeyPress { modifiers: Modifiers(8 - 1), key: KeyInput::Special(ncurses::KEY_DOWN), is_repeat: false }),

    (const_cstr!("kHOM3"), KeyPress { modifiers: Modifiers(3 - 1), key: KeyInput::Special(ncurses::KEY_HOME), is_repeat: false }),
    (const_cstr!("kHOM4"), KeyPress { modifiers: Modifiers(4 - 1), key: KeyInput::Special(ncurses::KEY_HOME), is_repeat: false }),
    (const_cstr!("kHOM5"), KeyPress { modifiers: Modifiers(5 - 1), key: KeyInput::Special(ncurses::KEY_HOME), is_repeat: false }),
    (const_cstr!("kHOM6"), KeyPress { modifiers: Modifiers(6 - 1), key: KeyInput::Special(ncurses::KEY_HOME), is_repeat: false }),
    (const_cstr!("kHOM7"), KeyPress { modifiers: Modifiers(7 - 1), key: KeyInput::Special(ncurses::KEY_HOME), is_repeat: false }),
    (const_cstr!("kHOM8"), KeyPress { modifiers: Modifiers(8 - 1), key: KeyInput::Special(ncurses::KEY_HOME), is_repeat: false }),

    (const_cstr!("kEND3"), KeyPress { modifiers: Modifiers(3 - 1), key: KeyInput::Special(ncurses::KEY_END), is_repeat: false }),
    (const_cstr!("kEND4"), KeyPress { modifiers: Modifiers(4 - 1), key: KeyInput::Special(ncurses::KEY_END), is_repeat: false }),
    (const_cstr!("kEND5"), KeyPress { modifiers: Modifiers(5 - 1), key: KeyInput::Special(ncurses::KEY_END), is_repeat: false }),
    (const_cstr!("kEND6"), KeyPress { modifiers: Modifiers(6 - 1), key: KeyInput::Special(ncurses::KEY_END), is_repeat: false }),
    (const_cstr!("kEND7"), KeyPress { modifiers: Modifiers(7 - 1), key: KeyInput::Special(ncurses::KEY_END), is_repeat: false }),
    (const_cstr!("kEND8"), KeyPress { modifiers: Modifiers(8 - 1), key: KeyInput::Special(ncurses::KEY_END), is_repeat: false }),

    (const_cstr!("kPRV3"), KeyPress { modifiers: Modifiers(3 - 1), key: KeyInput::Special(ncurses::KEY_PPAGE), is_repeat: false }),
    (const_cstr!("kPRV4"), KeyPress { modifiers: Modifiers(4 - 1), key: KeyInput::Special(ncurses::KEY_PPAGE), is_repeat: false }),
    (const_cstr!("kPRV5"), KeyPress { modifiers: Modifiers(5 - 1), key: KeyInput::Special(ncurses::KEY_PPAGE), is_repeat: false }),
    (const_cstr!("kPRV6"), KeyPress { modifiers: Modifiers(6 - 1), key: KeyInput::Special(ncurses::KEY_PPAGE), is_repeat: false }),
    (const_cstr!("kPRV7"), KeyPress { modifiers: Modifiers(7 - 1), key: KeyInput::Special(ncurses::KEY_PPAGE), is_repeat: false }),
    (const_cstr!("kPRV8"), KeyPress { modifiers: Modifiers(8 - 1), key: KeyInput::Special(ncurses::KEY_PPAGE), is_repeat: false }),

    (const_cstr!("kNXT3"), KeyPress { modifiers: Modifiers(3 - 1), key: KeyInput::Special(ncurses::KEY_NPAGE), is_repeat: false }),
    (const_cstr!("kNXT4"), KeyPress { modifiers: Modifiers(4 - 1), key: KeyInput::Special(ncurses::KEY_NPAGE), is_repeat: false }),
    (const_cstr!("kNXT5"), KeyPress { modifiers: Modifiers(5 - 1), key: KeyInput::Special(ncurses::KEY_NPAGE), is_repeat: false }),
    (const_cstr!("kNXT6"), KeyPress { modifiers: Modifiers(6 - 1), key: KeyInput::Special(ncurses::KEY_NPAGE), is_repeat: false }),
    (const_cstr!("kNXT7"), KeyPress { modifiers: Modifiers(7 - 1), key: KeyInput::Special(ncurses::KEY_NPAGE), is_repeat: false }),
    (const_cstr!("kNXT8"), KeyPress { modifiers: Modifiers(8 - 1), key: KeyInput::Special(ncurses::KEY_NPAGE), is_repeat: false }),
];

impl InputStream {
    pub unsafe fn init(window: ncurses::WINDOW) -> Self {
        // TODO: error handling?
        ncurses::ll::keypad(window, true as ncurses::ll::c_bool);
        ncurses::ll::raw();
        ncurses::ll::noecho();

        // TODO: Make this configurable to allow for mouse movements and for clicks
        ncurses::ll::mousemask(ncurses::ALL_MOUSE_EVENTS as ncurses::mmask_t, core::ptr::null_mut());
        ncurses::ll::mouseinterval(0); // We care about up/down, not clicks

        // Start bracketed paste mode, but only if we can successfully handle the brackets
        // TODO: Should we query support first?
        let bracketed_paste_guard = if ext::define_key_code(const_cstr!("\x1b[200~").as_cstr(), 2000).is_ok() &&
                                       ext::define_key_code(const_cstr!("\x1b[201~").as_cstr(), 2001).is_ok() {
            BracketedPaste::start()
        } else {
            None
        };

        let xterm_modify_other_keys_guard = if ext::define_key_code(const_cstr!("\x1b[27;").as_cstr(), 2100).is_ok() {
            XTermModifyOtherKeys::start()
        } else {
            None
        };

        // TODO: Should we query support first?
        let kitty_full_mode_guard = if ext::define_key_code(const_cstr!("\x1b_K").as_cstr(), 2200).is_ok() &&
                                       ext::define_key_code(const_cstr!("\x1b\\").as_cstr(), 2201).is_ok() {
            KittyFullMode::start()
        } else {
            None
        };

        // We use Esc heavily and modern computers are quite fast, so unless the user has overridden it directly,
        // set ESCDELAY to a small 25ms. The normal default of 1 second is too high.
        // TODO: If one of the other protocols causes the Esc key to be sent unambiguously, increase this value significantly
        if std::env::var_os("ESCDELAY").is_none() {
            ncurses::ll::set_escdelay(25);
        }

        let mut extra_bound_keys = Vec::new();
        for &(name, inp) in KNOWN_EXTRA_TERM_CAPABILITIES {
            if let Some(description) = ext::get_terminfo_string(name.as_cstr()) {
                if let Ok(code) = ext::key_code_for(description) {
                    extra_bound_keys.push((code, inp));
                }
            }
        }

        // Hackily detect if our terminal is using rxvt-style codes and add the rest if necessary. Note that this
        // should never override an existing binding, so it shouldn't cause problems even if it happens to be enabled
        // on a terminal that uses different bindings.
        if ext::key_code_for(const_cstr!("\x1b[A").as_cstr()) == Ok(ncurses::KEY_UP) &&
           ext::key_code_for(const_cstr!("\x1b[B").as_cstr()) == Ok(ncurses::KEY_DOWN) &&
           ext::key_code_for(const_cstr!("\x1b[C").as_cstr()) == Ok(ncurses::KEY_RIGHT) &&
           ext::key_code_for(const_cstr!("\x1b[D").as_cstr()) == Ok(ncurses::KEY_LEFT) &&
           ext::key_code_for(const_cstr!("\x1b[c").as_cstr()) == Ok(ncurses::KEY_SRIGHT) &&
           ext::key_code_for(const_cstr!("\x1b[d").as_cstr()) == Ok(ncurses::KEY_SLEFT) {

            let _ = define_if_necessary(const_cstr!("\x1bOa").as_cstr(), 2340);
            let _ = define_if_necessary(const_cstr!("\x1bOb").as_cstr(), 2341);
            let _ = define_if_necessary(const_cstr!("\x1bOc").as_cstr(), 2342);
            let _ = define_if_necessary(const_cstr!("\x1bOd").as_cstr(), 2343);
            // And AltSendsEscape versions as well (TODO: fold into a general AltSendsEscape mechanism)
            let _ = define_if_necessary(const_cstr!("\x1b\x1bOa").as_cstr(), 2360);
            let _ = define_if_necessary(const_cstr!("\x1b\x1bOb").as_cstr(), 2361);
            let _ = define_if_necessary(const_cstr!("\x1b\x1bOc").as_cstr(), 2362);
            let _ = define_if_necessary(const_cstr!("\x1b\x1bOd").as_cstr(), 2363);

            let _ = define_if_necessary(const_cstr!("\x1b\x1b[A").as_cstr(), 2320);
            let _ = define_if_necessary(const_cstr!("\x1b\x1b[B").as_cstr(), 2321);
            let _ = define_if_necessary(const_cstr!("\x1b\x1b[C").as_cstr(), 2322);
            let _ = define_if_necessary(const_cstr!("\x1b\x1b[D").as_cstr(), 2323);

            let _ = define_if_necessary(const_cstr!("\x1b\x1b[a").as_cstr(), 2330);
            let _ = define_if_necessary(const_cstr!("\x1b\x1b[b").as_cstr(), 2331);
            let _ = define_if_necessary(const_cstr!("\x1b\x1b[c").as_cstr(), 2332);
            let _ = define_if_necessary(const_cstr!("\x1b\x1b[d").as_cstr(), 2333);

            if ext::key_code_for(const_cstr!("\x1b[3~").as_cstr()) == Ok(ncurses::KEY_DC) {
                let _ = define_if_necessary(const_cstr!("\x1b[3^").as_cstr(), 2348);
                let _ = define_if_necessary(const_cstr!("\x1b\x1b[3^").as_cstr(), 2368);
            }
        }

        // Hackily detect if our terminal is using XTerm-style codes and add the rest if necessary
        if ext::key_code_for(const_cstr!("\x1bOA").as_cstr()) == Ok(ncurses::KEY_UP) &&
           ext::key_code_for(const_cstr!("\x1bOB").as_cstr()) == Ok(ncurses::KEY_DOWN) &&
           ext::key_code_for(const_cstr!("\x1bOC").as_cstr()) == Ok(ncurses::KEY_RIGHT) &&
           ext::key_code_for(const_cstr!("\x1bOD").as_cstr()) == Ok(ncurses::KEY_LEFT) &&
           ext::key_code_for(const_cstr!("\x1b[1;2C").as_cstr()) == Ok(ncurses::KEY_SRIGHT) &&
           ext::key_code_for(const_cstr!("\x1b[1;2D").as_cstr()) == Ok(ncurses::KEY_SLEFT) {

            for mode in 2..=7 {
                for &(indicator, key) in &[(b'A', 0), (b'B', 1), (b'C', 2), (b'D', 3), (b'H', 4), (b'F', 5)] {
                    let _ = define_if_necessary(
                        CStr::from_bytes_with_nul(&[0x1b, b'[', b'1', b';', b'1' + mode as u8, indicator, 0]).unwrap(),
                        2300 + mode * 10 + key
                    );
                }
            }

            if ext::key_code_for(const_cstr!("\x1b[3~").as_cstr()) == Ok(ncurses::KEY_DC) &&
               ext::key_code_for(const_cstr!("\x1b[5~").as_cstr()) == Ok(ncurses::KEY_PPAGE) &&
               ext::key_code_for(const_cstr!("\x1b[6~").as_cstr()) == Ok(ncurses::KEY_NPAGE) {

                for mode in 2..=7 {
                    for &(indicator, key) in &[(b'3', 8), (b'5', 6), (b'6', 7)] {
                        let _ = define_if_necessary(
                            CStr::from_bytes_with_nul(&[0x1b, b'[', indicator, b';', b'1' + mode as u8, b'~', 0]).unwrap(),
                            2300 + mode * 10 + key
                        );
                    }
                }
            }
        }

        // TODO: What about in front of, e.g., arrow keys? Generalize this.
        // Brute-force handle the most common cases for AltSendsEscape
        // TODO: Because we are passing in a CStr, we can't detect sequences with null bytes
        for byte in (1..=26).chain(48..=57).chain(65..=90).chain(97..=122) {
            let _ = define_if_necessary(CStr::from_bytes_with_nul(&[0x1b, byte as u8, 0]).unwrap(), 3000 + byte);
        }

        ncurses::ll::ungetch(ncurses::KEY_RESIZE);

        InputStream {
            _bracketed_paste: bracketed_paste_guard,
            _xterm_modify_keys: xterm_modify_other_keys_guard,
            _kitty_full_mode: kitty_full_mode_guard,

            extra_bound_keys: extra_bound_keys,

            in_progress_codepoint: 0,
            utf8_bytes_left: 0,
            xterm_modify_key_state: XTermModifyKeyState::Off,
            kitty_full_mode_state: KittyFullModeState::Off
        }
    }

    pub fn next_event(&mut self, window: ncurses::WINDOW) -> Result<Event, ()> {
        use crate::KeyInput::*;
        const NONE: Modifiers = crate::Modifiers::NONE;
        const CTRL: Modifiers = crate::Modifiers::CTRL;
        const ALT: Modifiers = crate::Modifiers::ALT;
        const SHIFT: Modifiers = crate::Modifiers::SHIFT;

        loop {
            let curses_input = unsafe { ncurses::ll::wgetch(window) };
            if curses_input == ncurses::ERR {
                return Err(());
            }

            let input;
            // We need to parse utf8.
            if curses_input < 256 {
                let byte = curses_input as u8;
                if self.utf8_bytes_left == 0 {
                    // New character
                    if byte >> 7 == 0b0 {
                        self.utf8_bytes_left = 0;
                        self.in_progress_codepoint = (byte & 0x7f) as u32;
                    } else if byte >> 5 == 0b110 {
                        self.utf8_bytes_left = 1;
                        self.in_progress_codepoint = (byte & 0x1f) as u32;
                    } else if byte >> 4 == 0b1110 {
                        self.utf8_bytes_left = 2;
                        self.in_progress_codepoint = (byte & 0x0f) as u32;
                    } else if byte >> 3 == 0b11110 {
                        self.utf8_bytes_left = 3;
                        self.in_progress_codepoint = (byte & 0x07) as u32;
                    } else {
                        return Ok(KeyPress { modifiers: NONE, key: Byte(byte), is_repeat: false });
                    }
                } else if byte >> 6 == 0b10 {
                    // Continuation bytes
                    self.utf8_bytes_left -= 1;
                    self.in_progress_codepoint = (self.in_progress_codepoint << 6) | ((byte & 0x3f) as u32);
                } else {
                    return Ok(KeyPress { modifiers: NONE, key: Byte(byte), is_repeat: false });
                }
                if self.utf8_bytes_left == 0 {
                    // FIXME: This should not crash
                    input = Codepoint(std::char::from_u32(self.in_progress_codepoint).expect("BUG: Bad char cast"));
                } else {
                    continue;
                }
            } else {
                input = Special(curses_input);
            }

            // Translate keys bound to non-standard terminfo entries
            if let Special(code) = input {
                for &(possible_code, possible_inp) in &self.extra_bound_keys {
                    if possible_code == code {
                        return Ok(possible_inp);
                    }
                }
            }

            // Translate various known special keys to a decomposed form
            match input {
                // Non-key inputs
                Special(ncurses::KEY_RESIZE) => {
                    let mut height = 0;
                    let mut width = 0;
                    ncurses::getmaxyx(window, &mut height, &mut width);
                    return Ok(Resize {
                        width: width as u32,
                        height: height as u32
                    });
                },
                Special(ncurses::KEY_MOUSE) => {
                    let mut event = ncurses::ll::MEVENT {
                        id: 0,
                        x: 0,
                        y: 0,
                        z: 0,
                        bstate: 0
                    };
                    if ncurses::getmouse(&mut event) != ncurses::OK {
                        return Err(());
                    }
                    return Ok(Mouse {
                        device_id: event.id as u16,
                        x: event.x as u32,
                        y: event.y as u32,
                        buttons: event.bstate & !((ncurses::BUTTON_CTRL | ncurses::BUTTON_ALT | ncurses::BUTTON_SHIFT) as u32),
                        modifiers: if event.bstate & (ncurses::BUTTON_CTRL as u32)  != 0 { CTRL  } else { NONE }
                                 | if event.bstate & (ncurses::BUTTON_ALT as u32)   != 0 { ALT   } else { NONE }
                                 | if event.bstate & (ncurses::BUTTON_SHIFT as u32) != 0 { SHIFT } else { NONE }
                    });
                }
                Special(2000) => return Ok(PasteBegin),
                Special(2001) => return Ok(PasteEnd),
                // Shifted standard keys
                Special(ncurses::KEY_SLEFT)  => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_LEFT), is_repeat: false }),
                Special(ncurses::KEY_SRIGHT) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_RIGHT), is_repeat: false }),
                Special(ncurses::KEY_SR)     => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_UP), is_repeat: false }),
                Special(ncurses::KEY_SF)     => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_DOWN), is_repeat: false }),
                Special(ncurses::KEY_SHOME)  => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_HOME), is_repeat: false }),
                Special(ncurses::KEY_SEND)   => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_END), is_repeat: false }),
                Special(ncurses::KEY_SDC)    => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_DC), is_repeat: false }),
                Special(ncurses::KEY_BTAB)   => return Ok(KeyPress { modifiers: SHIFT, key: Codepoint('\t'), is_repeat: false }),
                // Shifted uncommon keys
                Special(ncurses::KEY_SBEG) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_BEG), is_repeat: false }),
                Special(ncurses::KEY_SCANCEL) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_CANCEL), is_repeat: false }),
                Special(ncurses::KEY_SCOMMAND) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_COMMAND), is_repeat: false }),
                Special(ncurses::KEY_SCOPY) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_COPY), is_repeat: false }),
                Special(ncurses::KEY_SCREATE) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_CREATE), is_repeat: false }),
                Special(ncurses::KEY_SDL) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_DL), is_repeat: false }),
                Special(ncurses::KEY_SEOL) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_EOL), is_repeat: false }),
                Special(ncurses::KEY_SEXIT) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_EXIT), is_repeat: false }),
                Special(ncurses::KEY_SFIND) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_FIND), is_repeat: false }),
                Special(ncurses::KEY_SHELP) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_HELP), is_repeat: false }),
                Special(ncurses::KEY_SIC) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_IC), is_repeat: false }),
                Special(ncurses::KEY_SMESSAGE) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_MESSAGE), is_repeat: false }),
                Special(ncurses::KEY_SMOVE) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_MOVE), is_repeat: false }),
                Special(ncurses::KEY_SNEXT) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_NEXT), is_repeat: false }),
                Special(ncurses::KEY_SOPTIONS) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_OPTIONS), is_repeat: false }),
                Special(ncurses::KEY_SPREVIOUS) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_PREVIOUS), is_repeat: false }),
                Special(ncurses::KEY_SPRINT) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_PRINT), is_repeat: false }),
                Special(ncurses::KEY_SREDO) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_REDO), is_repeat: false }),
                Special(ncurses::KEY_SREPLACE) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_REPLACE), is_repeat: false }),
                Special(ncurses::KEY_SRSUME) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_RESUME), is_repeat: false }),
                Special(ncurses::KEY_SSAVE) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_SAVE), is_repeat: false }),
                Special(ncurses::KEY_SUNDO) => return Ok(KeyPress { modifiers: SHIFT, key: Special(ncurses::KEY_UNDO), is_repeat: false }),
                // Ctrl+Z triggers a suspend
                Special(ncurses::KEY_SUSPEND) => return Ok(KeyPress { modifiers: CTRL, key: Codepoint('z'), is_repeat: false }),
                Special(ncurses::KEY_SSUSPEND) => return Ok(KeyPress { modifiers: CTRL | SHIFT, key: Codepoint('z'), is_repeat: false }),
                // The DEL and BACKSPACE have different meanings, but since they are inconsistently assigned, we unify them into one code
                Codepoint('\u{7f}') => return Ok(KeyPress { modifiers: NONE, key: Special(ncurses::KEY_BACKSPACE), is_repeat: false }),
                // Both Ctrl+` and Ctrl+Space generate a null bytem but Ctrl+Space seems much more common
                Codepoint('\0') => return Ok(KeyPress { modifiers: CTRL, key: Codepoint(' '), is_repeat: false }),
                // Assume that control characters aren't from actual typing and are instead generated by Ctrl + a printable character
                Codepoint(chr) if (chr as u32) > 0 && (chr as u32) < 27 && chr != '\t' && chr != '\n' && chr != '\u{8}'
                    => return Ok(KeyPress { modifiers: CTRL, key: Codepoint(std::char::from_u32(chr as u32 + 96).unwrap()), is_repeat: false }),
                Codepoint(chr) if (chr as u32) > 128 && (chr as u32) < 155 // TODO: Consider whitelist? Cancel is sometimes used for Backspace
                    => return Ok(KeyPress { modifiers: CTRL | ALT, key: Codepoint(std::char::from_u32(chr as u32 - 32).unwrap()), is_repeat: false }),
                // AltSendsEscape + either a control character (assumed to be from Ctrl) or a printable character
                Special(code @ 3001..=3255) => if code < 3027 && code != 3008 && code != 3009 && code != 3013 {
                    // Note that we actually treat \n as a control code originating from Ctrl+j, unlike above; this is because the actual Enter
                    // key will be sent as a carriage return.
                    return Ok(KeyPress { modifiers: CTRL | ALT, key: Codepoint(std::char::from_u32(code as u32 - 3000 + 96).unwrap()), is_repeat: false });
                } else if code == 3013 {
                    // The Enter key at a terminal actually sends \r, not \n. Normally, either the ICRNL termios flag translates it or
                    // ncurses translates it, but we are handling it manually, so we need to translate ourselves.
                    return Ok(KeyPress { modifiers: ALT, key: Codepoint('\n'), is_repeat: false });
                } else {
                    return Ok(KeyPress { modifiers: ALT, key: Codepoint(std::char::from_u32(code as u32 - 3000).unwrap()), is_repeat: false });
                },
                // XTerm-style modified keys that weren't in the Terminfo
                Special(code @ 2300..=2399) => {
                    let base_code = code - 2300;
                    let modifiers = Modifiers((base_code / 10) as u8);
                    match base_code % 10 {
                        0 => return Ok(KeyPress { modifiers: modifiers, key: Special(ncurses::KEY_UP), is_repeat: false }),
                        1 => return Ok(KeyPress { modifiers: modifiers, key: Special(ncurses::KEY_DOWN), is_repeat: false }),
                        2 => return Ok(KeyPress { modifiers: modifiers, key: Special(ncurses::KEY_RIGHT), is_repeat: false }),
                        3 => return Ok(KeyPress { modifiers: modifiers, key: Special(ncurses::KEY_LEFT), is_repeat: false }),
                        4 => return Ok(KeyPress { modifiers: modifiers, key: Special(ncurses::KEY_HOME), is_repeat: false }),
                        5 => return Ok(KeyPress { modifiers: modifiers, key: Special(ncurses::KEY_END), is_repeat: false }),
                        6 => return Ok(KeyPress { modifiers: modifiers, key: Special(ncurses::KEY_PPAGE), is_repeat: false }),
                        7 => return Ok(KeyPress { modifiers: modifiers, key: Special(ncurses::KEY_NPAGE), is_repeat: false }),
                        8 => return Ok(KeyPress { modifiers: modifiers, key: Special(ncurses::KEY_DC), is_repeat: false }),
                        _ => { }
                    }
                },
                _ => { }
            }

            // Handle XTerm's modifyOtherKeys extension, parsing manually
            if let Special(2100) = input {
                self.xterm_modify_key_state = XTermModifyKeyState::ParsingMode(0);
                continue;
            }
            match self.xterm_modify_key_state {
                XTermModifyKeyState::Off => { },
                XTermModifyKeyState::ParsingMode(mode_so_far) => {
                    if let Codepoint(chr) = input {
                        if let Some(digit) = chr.to_digit(10) {
                            self.xterm_modify_key_state = XTermModifyKeyState::ParsingMode(mode_so_far * 10 + digit);
                            continue;
                        } else if chr == ';' {
                            self.xterm_modify_key_state = XTermModifyKeyState::ParsingChar(mode_so_far, 0);
                            continue;
                        }
                    }
                },
                XTermModifyKeyState::ParsingChar(mode, char_so_far) => {
                    if let Codepoint(chr) = input {
                        if let Some(digit) = chr.to_digit(10) {
                            self.xterm_modify_key_state = XTermModifyKeyState::ParsingChar(mode, char_so_far * 10 + digit);
                            continue;
                        } else if chr == '~' {
                            self.xterm_modify_key_state = XTermModifyKeyState::Off;
                            if 1 <= mode {
                                // FIXME: This should not crash
                                return Ok(KeyPress { modifiers: Modifiers((mode as u8) - 1), key: Codepoint(std::char::from_u32(char_so_far).unwrap()), is_repeat: false });
                            } else {
                                eprintln!("0 mode?");
                                continue;
                            }
                        }
                    }
                }
            }

            // Handle Kitty's full mode extension, parsing manually
            if let Special(2200) = input {
                self.kitty_full_mode_state = KittyFullModeState::ParsingType;
                continue;
            }
            match self.kitty_full_mode_state {
                KittyFullModeState::Off => { },
                KittyFullModeState::ParsingType => match input {
                    Codepoint('p') => {
                        self.kitty_full_mode_state = KittyFullModeState::ParsingModifiers(KeyType::Press);
                        continue;
                    },
                    Codepoint('r') => {
                        self.kitty_full_mode_state = KittyFullModeState::ParsingModifiers(KeyType::Release);
                        continue;
                    },
                    Codepoint('t') => {
                        self.kitty_full_mode_state = KittyFullModeState::ParsingModifiers(KeyType::Repeat);
                        continue;
                    },
                    _ => { }
                },
                KittyFullModeState::ParsingModifiers(key_type) => {
                    if let Codepoint(chr) = input {
                        // Decode base 64
                        let decoded = if 'A' <= chr && chr <= 'Z' {
                            Some(chr as u32 - 'A' as u32)
                        } else if 'a' <= chr && chr <= 'z' {
                            Some(chr as u32 - 'a' as u32 + 26)
                        } else if '0' <= chr && chr <= '9' {
                            Some(chr as u32 - '0' as u32 + 52)
                        } else if chr == '+' {
                            Some(62)
                        } else if chr == '/' {
                            Some(63)
                        } else {
                            None
                        };
                        if let Some(mode) = decoded {
                            self.kitty_full_mode_state = KittyFullModeState::ParsingKey(key_type, mode as u8, 0);
                            continue;
                        }
                    }
                },
                KittyFullModeState::ParsingKey(key_type, mode, key_so_far) => {
                    if let Codepoint(chr) = input {
                        let decoded = if 'A' <= chr && chr <= 'Z' {
                            Some(chr as u32 - 'A' as u32)
                        } else if 'a' <= chr && chr <= 'z' {
                            Some(chr as u32 - 'a' as u32 + 26)
                        } else if '0' <= chr && chr <= '9' {
                            Some(chr as u32 - '0' as u32 + 52)
                        } else {
                            ".-:+=^!/*?&<>()[]{}@%$#".chars().position(|c| c == chr).map(|i| i as u32 + 62)
                        };
                        if let Some(value) = decoded {
                            self.kitty_full_mode_state = KittyFullModeState::ParsingKey(key_type, mode, key_so_far * 85 + value);
                            continue;
                        }
                    } else if let Special(2201) = input {
                        self.kitty_full_mode_state = KittyFullModeState::Off;
                        let modifiers = Modifiers(mode);
                        // FIXME: Kitty does not provide an indication of the correct capital version of a shifted
                        // key; decide on a strategy for dealing with that since keyboard layouts aren't always consistent
                        // Note that without Ctrl or Alt, this protocol is not used, so the capital variants are available
                        let translated = match key_so_far {
                            0 => Codepoint(' '),
                            1 if modifiers & SHIFT == NONE => Codepoint('\''),
                            2 if modifiers & SHIFT == NONE => Codepoint(','),
                            3 if modifiers & SHIFT == NONE => Codepoint('-'),
                            4 if modifiers & SHIFT == NONE => Codepoint('.'),
                            5 if modifiers & SHIFT == NONE => Codepoint('/'),
                            6..=15 if modifiers & SHIFT == NONE => {
                                Codepoint(std::char::from_u32('0' as u32 + key_so_far as u32 - 6).unwrap())
                            },
                            16 if modifiers & SHIFT == NONE => Codepoint(';'),
                            17 if modifiers & SHIFT == NONE => Codepoint('='),
                            18..=43 => if modifiers & SHIFT == NONE { // If shift, capitalize the letter
                                Codepoint(std::char::from_u32('a' as u32 + key_so_far as u32 - 18).unwrap())
                            } else {
                                Codepoint(std::char::from_u32('A' as u32 + key_so_far as u32 - 18).unwrap())
                            },
                            44 if modifiers & SHIFT == NONE => Codepoint('['),
                            45 if modifiers & SHIFT == NONE => Codepoint('\\'),
                            46 if modifiers & SHIFT == NONE => Codepoint(']'),
                            47 if modifiers & SHIFT == NONE => Codepoint('`'),
                            50 => Codepoint('\u{1b}'), // Escape
                            51 => Codepoint('\n'),
                            52 => Codepoint('\t'),
                            53 => Special(ncurses::KEY_BACKSPACE),
                            54 => Special(ncurses::KEY_IC),
                            55 => Special(ncurses::KEY_DC),
                            56 => Special(ncurses::KEY_RIGHT),
                            57 => Special(ncurses::KEY_LEFT),
                            58 => Special(ncurses::KEY_DOWN),
                            59 => Special(ncurses::KEY_UP),
                            60 => Special(ncurses::KEY_PPAGE),
                            61 => Special(ncurses::KEY_NPAGE),
                            62 => Special(ncurses::KEY_HOME),
                            63 => Special(ncurses::KEY_END),
                            69 => Special(ncurses::KEY_F1),
                            70 => Special(ncurses::KEY_F2),
                            71 => Special(ncurses::KEY_F3),
                            72 => Special(ncurses::KEY_F4),
                            73 => Special(ncurses::KEY_F5),
                            74 => Special(ncurses::KEY_F6),
                            75 => Special(ncurses::KEY_F7),
                            76 => Special(ncurses::KEY_F8),
                            77 => Special(ncurses::KEY_F9),
                            78 => Special(ncurses::KEY_F10),
                            79 => Special(ncurses::KEY_F11),
                            80 => Special(ncurses::KEY_F12),
                            150..=181 => if modifiers & SHIFT == NONE { // Cyrillic characters
                                Codepoint(std::char::from_u32('а' as u32 + key_so_far as u32 - 150).unwrap())
                            } else {
                                Codepoint(std::char::from_u32('А' as u32 + key_so_far as u32 - 150).unwrap())
                            },
                            // Ie with grave (ѐ) is skipped
                            182 => if modifiers & SHIFT == NONE {
                                Codepoint('ё')
                            } else {
                                Codepoint('Ё')
                            },
                            _ => Special(key_so_far as i32 + 600)
                        };
                        return Ok(match key_type {
                            KeyType::Press   => KeyPress { modifiers: modifiers, key: translated, is_repeat: false },
                            KeyType::Repeat  => KeyPress { modifiers: modifiers, key: translated, is_repeat: true },
                            KeyType::Release => KeyRelease { modifiers: modifiers, key: translated },
                        });
                    }
                }
            }

            return Ok(KeyPress { modifiers: NONE, key: input, is_repeat: false })
        }
    }
}
