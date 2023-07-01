use orion::{aead::{self, SecretKey}, errors::UnknownCryptoError as CryptoError};
use std::{io::{self, Write}, fs, env};
use serde::{Serialize, Deserialize};
use crossterm::{queue, execute, cursor, style, terminal, event};

mod totp;
mod pass;
mod ui;

#[derive(Debug)]
enum MasterPassword {
    Disabled,
    Set(String, SecretKey),
}

#[derive(Serialize, Deserialize, Debug)]
struct Passwords {
    pass: Vec<pass::Password>,
    totp: Vec<totp::TotpCode>,
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


        let master_pk: MasterPassword = {
            if let Ok(_) = env::var("PASSRS_NOPASS") {
                MasterPassword::Disabled

            } else if let Ok(pass_env) = env::var("PASSRS_PASS") {
                let key = generate_orion_key(&pass_env).unwrap();
                MasterPassword::Set(pass_env, key)

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
                    if ui::input_string(&mut pass, &mut index, &ev) == ui::AfterAction::Enter {
                        break pass;
                    }
                };

                if !pass.is_empty() {
                    let key = generate_orion_key(&pass).unwrap();
                    MasterPassword::Set(pass, key)
                } else {
                    MasterPassword::Disabled
                }
            }
        };
        //dbg!(&master_pk);

        let mut password_set: Passwords = {
            if let Ok(bytes) = fs::read(&filename) {
                let json = {
                    if let MasterPassword::Set(_, master_key) = master_pk {
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

    execute!(stdout,
             terminal::LeaveAlternateScreen,
             terminal::EnableLineWrap);
    terminal::disable_raw_mode();
}

fn main_ui(password_set: &mut Passwords) {
    use event::{Event, KeyCode::*};

    #[derive(PartialEq)]
    enum Tab {
        Password,
        Totp,
    }

    let mut stdout = io::stdout();
    let mut tab = Tab::Password;
    let mut pass_scroll: usize = 0;
    let mut totp_scroll: usize = 0;

    loop {
        let size = terminal::size().unwrap();
        let list_scroll = match tab { Tab::Password => &mut pass_scroll, Tab::Totp => &mut totp_scroll };
        let list_length = match tab { Tab::Password => password_set.pass.len(), Tab::Totp => password_set.totp.len() };
        let view = ui::visible_scrolled(size.1 - 2, list_length, *list_scroll);

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

                for (index, y_pos) in view.zip(1..size.1 - 1) {
                    queue!(stdout,
                           cursor::MoveTo(0, y_pos),
                           //style::Print(format!("{} @ {}", index, y_pos)));
                           style::Print(&password_set.totp[index].name));
                    if index == *list_scroll {
                        queue!(stdout,
                               style::Print(" <".to_string()));
                    }
                }
            },
        }

        stdout.flush();

        let ev = event::read().unwrap();

        match ui::input_key(&ev) {
            Esc | Char('q') => return,
            Tab => {
                tab = match tab { Tab::Password => Tab::Totp, Tab::Totp => Tab::Password };
            },
            Up | Char('k') => {
                if *list_scroll != 0 {
                    *list_scroll -= 1;
                }
            },
            Down | Char('j') => {
                if list_length != 0 && *list_scroll < list_length - 1 {
                    *list_scroll += 1;
                }
            },
            _ => {},
        }
    }
}

#[inline]
fn generate_orion_key(key: &str) -> Result<SecretKey, CryptoError> {
    let bytes = key.as_bytes();
    return SecretKey::from_slice(&bytes);
}
