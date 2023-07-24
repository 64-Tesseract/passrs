use std::io::{self, stdout, Write};
use std::{cmp, ops::Range};
use crossterm::{queue, cursor, event::{Event::{self, Key}, KeyCode::{self, *}, KeyModifiers}, style, terminal};


#[derive(PartialEq)]
pub enum AfterAction {
    Enter,
    Cancel,
    Continue,
}

pub fn input_string(string: &mut String, index: &mut usize, keyev: &KeyCode) -> AfterAction {
    match keyev {
        Char(chr) => {
            string.insert(string.len() - *index, *chr);
            //*index += 1;
        },
        Left => {
            if *index < string.len() {
                *index += 1;
            }
        },
        Right => {
            if *index > 0 {
                *index -= 1;
            }
        },
        Backspace => {
            if string.len() != 0 && *index < string.len() {
                string.remove(string.len() - *index - 1);
                //*index -= 1;
            }
        },
        Delete => {
            if string.len() != 0 && *index > 0 {
                string.remove(string.len() - *index);
                *index -= 1;
            }
        },
        Enter => {
            return AfterAction::Enter;
        },
        Esc => {
            return AfterAction::Cancel;
        },
        _ => {},
    }

    return AfterAction::Continue;
}

pub fn input_key(ev: &Event) -> KeyCode {
    if let Key(keyev) = ev {
        return keyev.code;
    } else {
        return KeyCode::Null;
    }
}

pub fn print_typing(x: u16, y: u16, string: &String, cursor: Option<usize>) {
    let mut stdout = stdout();
    queue!(stdout,
           cursor::MoveTo(x, y),
           style::Print(format!("{} ", string)));

    if let Some(pos) = cursor {
        queue!(stdout,
               cursor::MoveTo(x + (string.len() - pos) as u16, y),
               style::SetForegroundColor(style::Color::Black),
               style::SetBackgroundColor(style::Color::White),
               style::Print(if pos == 0 { ' ' } else { string.chars().nth(string.len() - pos).unwrap() }));
    }
}

pub fn center_offset(center: u16, width: u16) -> u16 {
    if width > 0 && (width / 2 as u16) > center {
        return 0;
    } else {
        return ((center as f32 - width as f32) / 2.0) as u16;
    }
}

pub fn visible_scrolled(size: u16, length: usize, selected: usize) -> Range<usize> {
    let size = size as usize;

    if length <= size {
        return 0..length;
    }
    
    let view = ((selected + 1) as f32 / length as f32 * (length - size) as f32) as usize;
    return view..(view + size);
}
