use orion::{aead::{self, SecretKey}, errors::UnknownCryptoError as CryptoError};
use std::{io::{self, Write}, fs, env, time};
use serde::{Serialize, Deserialize};
use crossterm::{queue, execute, cursor, style, terminal, event};

mod totp;
mod pass;
mod ui;

const THEME_COLOUR: style::Color = style::Color::Red;
const POLL_TIME: time::Duration = time::Duration::from_millis(100);

#[derive(Serialize, Deserialize, Debug)]
struct Passwords {
    pass: Vec<pass::Password>,
    totp: Vec<totp::TotpCode>,
}

#[derive(PartialEq)]
enum Tab {
    Password,
    Totp,
}


fn main() {
    let mut stdout = io::stdout();
    execute!(stdout,
             terminal::EnterAlternateScreen,
             terminal::DisableLineWrap);
    terminal::enable_raw_mode();

    'main: {
        let filename: String = {
            let mut filename: Option<String> = None;

            if let Some(dir_arg) = env::args().collect::<Vec<String>>().get(1) {
                filename = Some(dir_arg.to_string());

            } else if let Ok(dir_env) = env::var("PASSRS_DIR") {
                filename = Some(dir_env.to_string());

            } else if let Ok(dir_home) = env::var("HOME") {
                filename = Some(format!("{}/.passrs", dir_home));
            }

            filename.unwrap()
        };
        //dbg!(&filename);


        let master_pk: Option<SecretKey> = {
            if let Ok(_) = env::var("PASSRS_NOPASS") {
                None

            } else if let Ok(pass_env) = env::var("PASSRS_PASS") {
                let key = generate_orion_key(&pass_env).unwrap();
                Some(key)

            } else {
                /*
                let mut pass_stdin = String::new();
                if let Ok(_) = io::stdin().read_line(&mut pass_stdin) {
                */
                /*
                if let Ok(mut pass_stdin) = Password::new("Master password:").without_confirmation().prompt() {
                    Some(pass_stdin)
                }
                */
                //let pass = ui::input_password();

                let mut pass = String::new();

                let pass = loop {
                    let size = terminal::size().unwrap();
                    queue!(stdout,
                           terminal::Clear(terminal::ClearType::All),
                           cursor::MoveTo(ui::center_offset(size.0, -4), ui::center_offset(size.1, -1)),
                           style::Print(format!("Password:")),
                           cursor::MoveTo(ui::center_offset(size.0, -(pass.len() as i16) / 2), ui::center_offset(size.1, 0)));
                    for _ in 0..pass.len() {
                        queue!(stdout,
                               style::Print("*"));
                    }
                    stdout.flush();

                    let mut index = pass.len();
                    let ev = event::read().unwrap();
                    if ui::input_string(&mut pass, &mut index, &ui::input_key(&ev)) == ui::AfterAction::Enter {
                        break pass;
                    }
                };

                if !pass.is_empty() {
                    let key = generate_orion_key(&pass).unwrap();
                    Some(key)
                } else {
                    None
                }
            }
        };
        //dbg!(&master_pk);

        let mut password_set: Passwords = {
            if let Ok(bytes) = fs::read(&filename) {
                let json = {
                    if let Some(master_key) = master_pk {
                        if let Ok(json) = aead::open(&master_key, &bytes) {
                            json

                        } else {
                            eprintln!("Cannot decrypt data with provided password");
                            break 'main;
                        }

                    } else {
                        bytes
                    }
                };

                if let Ok(passwords) = serde_json::from_slice::<Passwords>(&json) {
                    passwords

                } else {
                    eprintln!("Cannot parse decrypted JSON:\n{}", std::str::from_utf8(&json).unwrap());
                    break 'main;
                }

            } else {
                eprintln!("Cannot read file, making new password set");
                Passwords { pass: Vec::new(), totp: Vec::new() }
            }
        };
        //dbg!(password_set);

        for x in 0..100 {
            password_set.totp.push(totp::TotpCode::new(format!("{}", x)));
        }

        main_ui(&mut password_set);
    }

    terminal::disable_raw_mode();
    execute!(stdout,
             terminal::EnableLineWrap,
             terminal::LeaveAlternateScreen);
}

fn main_ui(password_set: &mut Passwords) {
    use event::{Event, KeyCode};

    let mut stdout = io::stdout();
    let mut tab = Tab::Password;
    let mut show_all = false;
    let mut totp_next = false;
    let mut totp_last_time: u64 = 0;
    let mut pass_scroll: usize = 0;
    let mut totp_scroll: usize = 0;

    loop {
        let size = terminal::size().unwrap();
        let list_scroll = match tab { Tab::Password => &mut pass_scroll, Tab::Totp => &mut totp_scroll };
        let list_length = match tab { Tab::Password => password_set.pass.len(), Tab::Totp => password_set.totp.len() };

        if size.0 > 1 && size.1 > 1 {
            let view = ui::visible_scrolled(size.1 - 1, list_length, *list_scroll);

            match tab {
                Tab::Password => {
                    queue!(stdout,
                           terminal::Clear(terminal::ClearType::All),
                           cursor::MoveTo(ui::center_offset(size.0, -5), 0),
                           style::Print("Passwords"));
                },
                Tab::Totp => {
                    queue!(stdout,
                           terminal::Clear(terminal::ClearType::All),
                           cursor::MoveTo(ui::center_offset(size.0, -7), 0),
                           style::Print("Authenticator"));

                    let time = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();

                    {
                        let totp_now_time = time.as_secs() / 30;
                        if totp_last_time != totp_now_time {
                            totp_next = false;
                            for totp_code in &mut password_set.totp {
                                totp_code.calculate_codes(totp_now_time);
                            }
                            totp_last_time = totp_now_time;
                        }
                    }

                    let main_width = size.0 as usize - 10;
                    for (index, y_pos) in view.zip(1..size.1) {

                        let this_totp = &password_set.totp[index];
                        let totp_string =
                            if show_all || index == *list_scroll {
                                let this_totp_code = this_totp.get_code(totp_next);
                                format!(" {name:width$} {code1} {code2} ",
                                        name = this_totp.name,
                                        code1 = this_totp_code[..3].to_string(),
                                        code2 = this_totp_code[3..].to_string(),
                                        width = main_width)
                            } else {
                                format!(" {}", this_totp.name)
                            };

                        if index == *list_scroll {
                            let string_split = (totp_string.len() as f32 * ((time.as_millis() % 30000) as f32 / 30000.0)) as usize + if totp_next { 0 } else { 1 };
                            let string_parts = (&totp_string[..string_split].to_string(), &totp_string[string_split..].to_string());
                            let colours = if totp_next { (style::Color::Black, THEME_COLOUR) } else { (THEME_COLOUR, style::Color::Black) };
                            queue!(stdout,
                                   cursor::MoveTo(0, y_pos),
                                   style::SetForegroundColor(colours.1),
                                   style::SetBackgroundColor(colours.0),
                                   style::Print(string_parts.0),
                                   style::SetForegroundColor(colours.0),
                                   style::SetBackgroundColor(colours.1),
                                   style::Print(string_parts.1),
                                   style::ResetColor);

                        } else {
                            queue!(stdout,
                                   cursor::MoveTo(0, y_pos),
                                   style::Print(totp_string));
                        }
                    }
                },
            }
        }

        stdout.flush();

        if let Ok(true) = event::poll(POLL_TIME) {
            let ev = event::read().unwrap();
            let keyev = ui::input_key(&ev);

            match keyev {
                KeyCode::Esc | KeyCode::Char('q') => return,
                KeyCode::Tab => {
                    tab = match tab { Tab::Password => Tab::Totp, Tab::Totp => Tab::Password };
                },
                KeyCode::Up | KeyCode::Char('k') => {
                    if *list_scroll != 0 {
                        *list_scroll -= 1;
                    }
                },
                KeyCode::Char('K') => {
                    match tab {
                        Tab::Password => shift_item::<pass::Password>(&mut password_set.pass, &mut pass_scroll, true),
                        Tab::Totp => shift_item::<totp::TotpCode>(&mut password_set.totp, &mut totp_scroll, true),
                    }
                },
                KeyCode::Down | KeyCode::Char('j') => {
                    if list_length != 0 && *list_scroll < list_length - 1 {
                        *list_scroll += 1;
                    }
                },
                KeyCode::Char('J') => {
                    match tab {
                        Tab::Password => shift_item::<pass::Password>(&mut password_set.pass, &mut pass_scroll, false),
                        Tab::Totp => shift_item::<totp::TotpCode>(&mut password_set.totp, &mut totp_scroll, false),
                    }
                },
                KeyCode::Char('v') => {
                    show_all = !show_all;
                },
                KeyCode::Char('n') => {
                    totp_next = !totp_next;
                },
                KeyCode::Char('e') => {
                    match tab {
                        Tab::Password => {
                            if let Some(p) = pass_edit_ui(Some(&password_set.pass[pass_scroll])) {
                                password_set.pass[pass_scroll] = p;
                            }
                        },
                        _ => {},
                    }
                },
                _ => {},
            }
        }
    }
}

fn pass_edit_ui(pass: Option<&pass::Password>) -> Option<pass::Password> {
    use event::{Event, KeyCode};

    let mut stdout = io::stdout();
    let mut selected: u8 = 0;
    let mut string_index: usize = 0;
    let mut new_pass: pass::Password = {
        if let Some(p) = pass {
            p.clone()
        } else {
            pass::Password::new()
        }
    };

    loop {
        let size = terminal::size().unwrap();

        queue!(stdout,
               terminal::Clear(terminal::ClearType::All),
               cursor::MoveTo(ui::center_offset(size.0, -7), 0),
               style::Print("Edit Password"),
               cursor::MoveTo(1, 2),
               style::Print("Name:"),
               cursor::MoveTo(1, 3),
               style::Print(&new_pass.name));

        if let Ok(true) = event::poll(POLL_TIME) {
            let ev = event::read().unwrap();
            let keyev = ui::input_key(&ev);

            match keyev {
                KeyCode::Esc => {
                    return None;
                },
                KeyCode::Enter => {
                    return Some(new_pass);
                },
                KeyCode::Up => {
                     if selected == 0 {
                         selected = 2;
                     } else {
                         selected -= 1;
                     }
                },
                KeyCode::Up => {
                    selected = (selected + 1) % 3;
                },
                _ => {
                    match selected {
                        0 => {
                            ui::input_string(&mut new_pass.name, &mut string_index, &keyev);
                        },
                        _ => {},
                    }
                }
            }
        }
    }
}

fn totp_edit_ui(pass: Option<&totp::TotpCode>) -> Option<totp::TotpCode> {
    use event::{Event, KeyCode};

    let mut stdout = io::stdout();
    let mut selected: u8 = 0;
    let mut string_index: usize = 0;
    let mut new_totp: totp::TotpCode = {
        if let Some(t) = totp {
            t.clone()
        } else {
            totp::TotpCode::new()
        }
    };

    loop {
        let size = terminal::size().unwrap();

        queue!(stdout,
               terminal::Clear(terminal::ClearType::All),
               cursor::MoveTo(ui::center_offset(size.0, -7), 0),
               style::Print("Edit TOTP Code"),
               cursor::MoveTo(1, 2),
               style::Print("Name:"),
               cursor::MoveTo(1, 3),
               style::Print(&new_totp.name));

        if let Ok(true) = event::poll(POLL_TIME) {
            let ev = event::read().unwrap();
            let keyev = ui::input_key(&ev);

            match keyev {
                KeyCode::Esc => {
                    return None;
                },
                KeyCode::Enter => {
                    return Some(new_pass);
                },
                KeyCode::Up => {
                     if selected == 0 {
                         selected = 2;
                     } else {
                         selected -= 1;
                     }
                },
                KeyCode::Up => {
                    selected = (selected + 1) % 2;
                },
                _ => {
                    match selected {
                        0 => {
                            ui::input_string(&mut new_pass.name, &mut string_index, &keyev);
                        },
                        _ => {},
                    }
                }
            }
        }
    }
}

fn shift_item<T>(vec: &mut Vec<T>, selected: &mut usize, up: bool) {
    if vec.len() < 2 { return; }

    if up {
        if *selected != 0 {
            vec.swap(*selected, *selected - 1);
            *selected -= 1;
        }
    } else {
        if *selected != vec.len() - 1 {
            vec.swap(*selected, *selected + 1);
            *selected += 1;
        }
    }
}

#[inline]
fn generate_orion_key(key: &str) -> Result<SecretKey, CryptoError> {
    let bytes = key.as_bytes();
    return SecretKey::from_slice(&bytes);
}
