use std::io::{self, stdout, Write};
use std::{cmp, ops::Range};
use crossterm::{queue, cursor, event::{Event::{self, Key}, KeyCode::{self, *}, KeyModifiers}, style, terminal};
use super::safe_sub;


#[derive(PartialEq)]
pub enum AfterAction {
    Enter,
    Cancel,
    Continue,
}

pub fn input_string(string: &mut String, index: &mut usize, keyev: &KeyCode) -> AfterAction {
    macro_rules! byte {
        ($s:expr, $i:expr) => {{
            let mut indices = $s.char_indices();
            if let Some(c) = indices.nth($i) {
                c.0 as usize
            } else {
                $s.len() as usize
            }
        }}
    }

    match keyev {
        Char(chr) => {
            string.insert(byte!(string, *index), *chr);
            *index += 1;
        },
        Left => {
            if *index > 0 {
                *index -= 1;
            }
        },
        Right => {
            if *index < string.char_indices().count() {
                *index += 1;
            }
        },
        Home => {
            *index = 0;
        },
        End => {
            *index = string.char_indices().count();
        }
        Backspace => {
            if string.char_indices().count() != 0 && *index > 0 {
                string.remove(byte!(string, *index - 1));
                *index -= 1;
            }
        },
        Delete => {
            if string.char_indices().count() != 0 && *index < string.char_indices().count() {
                string.remove(byte!(string, *index));
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

pub fn print_typing(mut x: Range<u16>, y: u16, string: &String, cursor: Option<usize>) {
    let mut stdout = stdout();
    let spacing: u16 = string.char_indices().map(|(_, c)| c.len_utf8()).max().or(Some(1)).unwrap() as u16;
    x.end = x.end / spacing + spacing;

    let scroll_range = visible_scrolled(x.len() as u16, string.char_indices().count(), cursor.or(Some(0)).unwrap());
    let scroll_start = scroll_range.start;
    let characters: Vec<(usize, char)> = string.char_indices().collect();

    queue!(stdout, style::Print(style::Attribute::Underlined));
    for (x_offset, char_index) in (0..).zip(scroll_range) {
        let this_char = characters[char_index];

        queue!(stdout,
               cursor::MoveTo(x.start + x_offset * spacing, y),
               style::Print(format!("{}", this_char.1)));

        if let Some(pos) = cursor {
            if pos == char_index as usize && this_char.1 as usize > 0x7f {
                queue!(stdout,
                       cursor::MoveTo(x.start + x_offset * spacing, y + 1),
                       style::Print(style::Attribute::NoUnderline),
                       style::Print(format!("\\u{:x}", this_char.1 as usize)),
                       style::Print(style::Attribute::Underlined));
            }
        }
    }
    queue!(stdout, style::Print(style::Attribute::NoUnderline));

    if let Some(pos) = cursor {
        queue!(stdout,
               cursor::MoveTo(x.start + safe_sub!(usize; pos, scroll_start) as u16 * spacing, y),
               style::SetForegroundColor(style::Color::Black),
               style::SetBackgroundColor(style::Color::White),
               style::Print({
                   if let Some(c) = string.char_indices().nth(pos) {
                       c.1
                   } else {
                       ' '
                   }
               }));
    }
}

pub fn center_offset(center: u16, width: u16) -> u16 {
    return (safe_sub!(f32; center as f32, width as f32) / 2.0) as u16;
}

pub fn visible_scrolled(size: u16, length: usize, selected: usize) -> Range<usize> {
    let size = size as usize;

    if length <= size {
        return 0..length;
    }
    
    let view = ((selected + 1) as f32 / length as f32 * (length - size) as f32) as usize;
    return view..(view + size);
}
