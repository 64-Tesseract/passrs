use std::io::stdout;
use std::{ops::Range};
use crossterm::{queue, cursor, event::{Event::{self, Key}, KeyCode::{self, *}}, style};
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

pub fn print_typing(mut x: (u16, u16), y: u16, string: &String, cursor: Option<usize>) {
    let mut stdout = stdout();
    //let spacing: u16 = string.char_indices().map(|(_, c)| c.len_utf8()).max().or(Some(1)).unwrap() as u16;
    let (spacing, mut characters) = spaced_chars(string);

    let scroll = visible_scrolled(((x.1 - x.0) / spacing - 1) as usize, string.char_indices().count(), cursor.or(Some(0)).unwrap());

    queue!(stdout, style::Print(style::Attribute::Underlined));
    //for (index, (char_x, character)) in (0..).zip(characters.map(|(p, c)| (p - scroll.start as u16 * spacing, c)).filter(|(p, _)| scroll.contains(&(*p as usize)))) {
    for (index, (char_x, character)) in scroll.clone().zip(characters[scroll.clone()].iter().map(|(p, c)| (p - scroll.start as u16 * spacing, c))) {
        queue!(stdout,
               cursor::MoveTo(x.0 + char_x, y),
               style::Print(format!("{c:width$}", c = character, width = spacing as usize)));

        if let Some(pos) = cursor {
            if pos == index && *character as usize > 0x7f {
                queue!(stdout,
                       cursor::MoveTo(x.0 + char_x, y + 1),
                       style::Print(style::Attribute::NoUnderline),
                       style::Print(format!("\\u{:x}", *character as usize)),
                       style::Print(style::Attribute::Underlined));
            }
        }
    }
    queue!(stdout, style::Print(style::Attribute::NoUnderline));

    if let Some(pos) = cursor {
        queue!(stdout,
               cursor::MoveTo(x.0 + safe_sub!(pos, scroll.start) as u16 * spacing, y),
               style::SetForegroundColor(style::Color::Black),
               style::SetBackgroundColor(style::Color::White),
               style::Print({
                   if let Some((_, c)) = string.char_indices().nth(pos) {
                       format!("{c:width$}", c = c, width = spacing as usize)
                   } else {
                       format!("{c:width$}", c = "", width = spacing as usize)
                   }
               }));
    }
}

pub fn center_offset(center: u16, width: u16) -> u16 {
    return ((center as f32 - width as f32).max(0.0) / 2.0) as u16;
}

pub fn spaced_chars(string: &String) -> (u16, Vec<(u16, char)>) {
    let spacing: u16 = string.char_indices().map(|(_, c)| c.len_utf8()).max().unwrap_or(1) as u16;
    (spacing, (0..).zip(string.char_indices()).map(move |(i, (_, c))| (i as u16 * spacing, c)).collect())
}

pub fn visible_scrolled(size: usize, length: usize, selected: usize) -> Range<usize> {
    if length <= size {
        return 0..length;
    }
    
    let view = ((selected + 1) as f32 / length as f32 * (length - size) as f32) as usize;
    return view..(view + size);
}
