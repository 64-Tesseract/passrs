use std::io::{self, stdout, Write};
use std::{cmp, ops::Range};
use crossterm::{queue, cursor, event::{Event::{self, Key}, KeyCode::{self, *}}, style, terminal};


#[derive(PartialEq)]
pub enum AfterAction {
    Up,
    Down,
    Enter,
    Cancel,
    Quit,
    Continue,
}

pub fn input_string(string: &mut String, index: &mut usize, ev: &Event) -> AfterAction {
    match input_key(ev) {
        Char(chr) => {
            string.insert(*index, chr);
            *index += 1;
        },
        Left => {
            if *index > 0 {
                *index -= 1;
            }
        },
        Right => {
            if *index < string.len() {
                *index += 1;
            }
        },
        Backspace => {
            if *index > 0 {
                string.remove(*index - 1);
                *index -= 1;
            }
        },
        Delete => {
            if string.len() != 0 && *index < string.len() {
                string.remove(*index);
            }
        },
        Up => {
            return AfterAction::Up;
        },
        Down => {
            return AfterAction::Down;
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

pub fn print_at(x: u16, y: u16, string: &String, cursor: Option<usize>) {
    let mut stdout = stdout();
    queue!(stdout,
           cursor::MoveTo(x, y),
           style::Print(format!("{} ", string)));

    if let Some(pos) = cursor {
        queue!(stdout,
               cursor::MoveTo(x + pos as u16, y));
    }

    stdout.flush();
}

pub fn center_offset(center: u16, offset: i16) -> u16 {
    if offset < 0 && (offset.abs() as u16) > center {
        return 0;
    } else {
        return (center as f32 / 2.0 + offset as f32) as u16;
    }
}

pub fn visible_scrolled(size: u16, length: usize, selected: usize) -> Range<usize> {
    let size = size as usize;

    if length <= size {
        return 0..length;
    }

    /*
    let percent_selected = selected as f32 / length as f32;
    let percent_view = percent_selected * (length - size) as f32;
    let view = (percent_view * size as f32) as usize;
    return view..(view + size);
    */
    
    let view = (selected as f32 / length as f32 * (length - size + 1) as f32) as usize;
    return view..(view + size);
}
