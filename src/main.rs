use orion::{aead::{self, SecretKey}, errors::UnknownCryptoError as CryptoError};
use std::{io::{self, Write}, fs, env, time, ops::Range};
use serde::{Serialize, Deserialize};
use clipboard::{ClipboardProvider, ClipboardContext};
use crossterm::{queue, execute, cursor, style, terminal, event};

mod totp;
mod pass;
mod ui;

const THEME_COLOUR: style::Color = style::Color::Red;
const POLL_TIME: time::Duration = time::Duration::from_millis(100);
const DEFAULT_TAB: Tab = Tab::Totp;

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

enum EditMenuValue<'a> {
    String(&'static str, &'a mut String),
    Int(&'static str, &'a mut usize, Range<usize>),
}

enum MasterPassResult {
    Password(String),
    NoPassword,
    Cancel,
}

#[macro_export]
macro_rules! safe_sub {
    ($t:ty; $a:expr, $b:expr) => {{
        if $b >= $a {
            <$t>::default()
        } else {
            $a as $t - $b as $t
        }
    }}
}

fn main() {
    let mut stdout = io::stdout();

    let mut filename: Option<String> = {
        if let Ok(dir_env) = env::var("PASSRS_FILE") {
            Some(dir_env.to_string())

        } else if let Ok(dir_home) = env::var("HOME") {
            Some(format!("{}/.local/share/passrs", dir_home))

        } else {
            None
        }
    };

    let mut script_print: Option<Tab> = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match &arg as &str {
            "--file" | "-f" => {
                filename = Some(args.next().expect("Expected a filename"));
            },
            "--totp" | "-t" => {
                script_print = Some(Tab::Totp);
            },
            "--pass" | "-p" => {
                script_print = Some(Tab::Password);
            },
            "--help" | "-h" => {
                println!("    passrs ~ Terminal password manager and authenticator");
                println!("");
                println!("passrs takes the following commandline arguments:");
                println!("--file, -f FILE     Specify a (possibly encrypted) file to read data from");
                println!("--totp, -t          Print all current TOTP codes and their names, useful for scripts");
                println!("--pass, -p          Print all passwords and their names, useful for scripts");
                println!("");
                println!("--help, -h          Print general help");
                println!("--help-gui, -H      Print help regarding GUI navigation");
                println!("");
                println!("passrs also reads the following environment variables:");
                println!("    HOME            The default data file is `$HOME/.local/share/passrs`");
                println!("    PASSRS_FILE     Set the file to read data from, overridden by `--file`, `-f`");
                println!("    PASSRS_PASS     Specify a password for passrs to use, bypassing the GUI password dialog");
                println!("    PASSRS_NOPASS   Specify that passrs shouldn't use encryption, bypassing the GUI password dialog");
                return;
            },
            "--help-gui" | "-H" => {
                println!("    passrs ~ GUI navigation help");
                println!("");
                println!("In the main view:");
                println!("    Tab             Switch between passwords and TOTP codes");
                println!("    Up/Down/j/k     Select the above/below item");
                println!("    Home/End/g/G    Select the first/last item");
                println!("    J/K             Move the selected item up/down");
                println!("    d               Mark the selected item for deletion upon exiting");
                println!("    v               Toggle viewing unselected items");
                println!("    n               Toggle viewing next TOTP code");
                println!("    e               Edit the selected item");
                println!("    o               Create a new item and edit it");
                println!("    p               Change encryption password for the current data file");
                println!("    Esc/q           Exit and save, excluding items marked for deletion");
                println!("");
                println!("In the edit item view:");
                println!("    Up/Down         Select the above/below field");
                println!("    Left/Right/Home/End    Move the cursor in a text field");
                println!("    Left/Right      Increment/Decrement a number field");
                println!("    Enter           Exit and save current item");
                println!("    Esc             Exit and cancel adding/editing item");
                println!("    *               Type in the selected text field");
                println!("");
                println!("In the password dialog:");
                println!("    Enter           Supply the current password, or if empty, disable encryption");
                println!("    Escape          Cancel entering password");
                println!("    *               Type in the password field");
                return;
            },
            a => {
                eprintln!("Unknown argument, `{}`, see `--help`, `-h`", a);
                return;
            }
        }
    }

    'main: {
        /*
        let filename: String = {
            if let Some(dir_arg) = env::args().collect::<Vec<String>>().get(1) {
                dir_arg.to_string()

            } else if let Ok(dir_env) = env::var("PASSRS_FILE") {
                dir_env.to_string()

            } else if let Ok(dir_home) = env::var("HOME") {
                format!("{}/.local/share/passrs", dir_home)

            } else {
                error = "Expected file directory as argument, environment variable, or a home directory environment variable".to_string();
                break 'main;
            }
        };
        */
        let filename = filename.unwrap();

        let mut master_pk: Option<SecretKey> = {
            if let Ok(_) = env::var("PASSRS_NOPASS") {
                None

            } else if let Ok(pass_env) = env::var("PASSRS_PASS") {
                let key = generate_orion_key(&pass_env).unwrap();
                Some(key)

            } else {
                if script_print == None {
                    match master_pass_ui() {
                        MasterPassResult::Password(pass) => Some(generate_orion_key(&pass).unwrap()),
                        MasterPassResult::NoPassword => None,
                        MasterPassResult::Cancel => { break 'main; },
                    }

                } else {
                    eprintln!("Print mode requires a password to be specified with PASSRS_PASS or PASSRS_NOPASS");
                    return;
                }
            }
        };

        let mut password_set: Passwords = {
            if let Ok(bytes) = fs::read(&filename) {
                let json = {
                    if let Some(ref master_key) = master_pk {
                        if let Ok(json) = aead::open(master_key, &bytes) {
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
                    eprintln!("Cannot parse raw JSON, you might require a password:\n{}", String::from_utf8_lossy(&json));
                    break 'main;
                }

            } else {
                eprintln!("Cannot read file, making new password set");
                Passwords { pass: Vec::new(), totp: Vec::new() }
            }
        };

        match script_print {
            Some(Tab::Password) => {
                for pass in &password_set.pass {
                    println!("{}\t{}", pass.name, pass.password);
                }
            },
            Some(Tab::Totp) => {
                for totp in &mut password_set.totp {
                    totp.calculate_codes();
                    println!("{}\t{}", totp.name, totp.get_code(false));
                }
            },
            None => {
                main_ui(&mut password_set, &mut master_pk);

                password_set.pass.retain(|p| !p.delete);
                password_set.totp.retain(|t| !t.delete);

                let bytes = {
                    let json = serde_json::to_string(&password_set).unwrap();

                    if let Some(ref master_key) = master_pk {
                        if let Ok(bytes) = aead::seal(master_key, &json.clone().into_bytes()) {
                            bytes
                        } else {
                            eprintln!("Could not encrypt JSON:\n{}", &json);
                            break 'main;
                        }
                    } else {
                        json.into_bytes()
                    }
                };

                if fs::write(&filename, bytes).is_err() {
                    eprintln!("Could not save file");
                    break 'main;
                }
            },
        }
    }
}

fn main_ui(password_set: &mut Passwords, master_pk: &mut Option<SecretKey>) {
    let mut stdout = io::stdout();

    enter_alt_screen(&mut stdout);

    use event::{Event, KeyCode};
    let mut clip: ClipboardContext = ClipboardProvider::new().unwrap();
    let mut tab = DEFAULT_TAB;
    let mut show_all = false;
    let mut totp_next = false;
    let mut totp_last_time: u64 = 0;
    let mut pass_scroll: usize = 0;
    let mut totp_scroll: usize = 0;

    'ui: loop {
        let size = terminal::size().unwrap();
        let list_scroll = match tab { Tab::Password => &mut pass_scroll, Tab::Totp => &mut totp_scroll };
        let list_length = match tab { Tab::Password => password_set.pass.len(), Tab::Totp => password_set.totp.len() };

        if size.0 > 1 && size.1 > 1 {
            let view = ui::visible_scrolled(safe_sub!(u16; size.1, 1), list_length, *list_scroll);

            match tab {
                Tab::Password => {
                    queue!(stdout,
                           terminal::Clear(terminal::ClearType::All),
                           cursor::MoveTo(ui::center_offset(size.0, 9), 0),
                           style::Print("Passwords"));

                    for (index, y_pos) in view.zip(1..size.1) {
                        let this_pass = &password_set.pass[index];

                        let pass_string =
                            if show_all || index == *list_scroll {
                                format!(" {name:width$} {pass} ",
                                        name = this_pass.name,
                                        pass = this_pass.password,
                                        width = safe_sub!(usize; safe_sub!(usize; size.0, 3), this_pass.password.char_indices().count()))
                            } else {
                                format!(" {name:width$}",
                                        name = this_pass.name,
                                        width = safe_sub!(usize; size.0, 1))
                            };

                        if this_pass.delete {
                            queue!(stdout, style::Print(style::Attribute::CrossedOut));
                        }

                        if index == *list_scroll {
                            queue!(stdout,
                                   cursor::MoveTo(0, y_pos),
                                   style::SetForegroundColor(THEME_COLOUR),
                                   style::Print(pass_string));

                        } else {
                            queue!(stdout,
                                   cursor::MoveTo(0, y_pos),
                                   style::Print(pass_string));
                        }

                        queue!(stdout,
                               style::ResetColor,
                               style::Print(style::Attribute::NotCrossedOut));
                    }
                },
                Tab::Totp => {
                    queue!(stdout,
                           terminal::Clear(terminal::ClearType::All),
                           cursor::MoveTo(ui::center_offset(size.0, 13), 0),
                           style::Print("Authenticator"));

                    let time = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();

                    {
                        let totp_now_time = time.as_secs() / 30;
                        if totp_last_time != totp_now_time {
                            totp_next = false;
                            for totp_code in &mut password_set.totp {
                                totp_code.calculate_codes();
                            }
                            totp_last_time = totp_now_time;
                        }
                    }

                    for (index, y_pos) in view.zip(1..size.1) {
                        let this_totp = &password_set.totp[index];

                        let totp_string =
                            if show_all || index == *list_scroll {
                                let this_totp_code = this_totp.get_code(totp_next);
                                format!(" {name:width$} {code1} {code2} ",
                                        name = this_totp.name,
                                        code1 = this_totp_code[..this_totp.data.digits / 2].to_string(),
                                        code2 = this_totp_code[this_totp.data.digits / 2..].to_string(),
                                        width = safe_sub!(usize; safe_sub!(usize; size.0, 4), this_totp.data.digits))
                            } else {
                                format!(" {name:width$}",
                                        name = this_totp.name,
                                        width = safe_sub!(usize; size.0, 1))
                            };

                        if this_totp.delete {
                            queue!(stdout, style::Print(style::Attribute::CrossedOut));
                        }

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
                                   style::Print(string_parts.1));

                        } else {
                            queue!(stdout,
                                   cursor::MoveTo(0, y_pos),
                                   style::Print(totp_string));
                        }

                        queue!(stdout,
                               style::ResetColor,
                               style::Print(style::Attribute::NotCrossedOut));
                    }
                },
            }
        }

        stdout.flush();

        if let Ok(true) = event::poll(POLL_TIME) {
            let ev = event::read().unwrap();
            let keyev = ui::input_key(&ev);

            match keyev {
                KeyCode::Esc | KeyCode::Char('q') => break 'ui,
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
                KeyCode::Home | KeyCode::Char('g') => {
                    *list_scroll = 0;
                },
                KeyCode::End | KeyCode::Char('G') => {
                    *list_scroll = list_length - 1;
                },
                KeyCode::Char('p') => {
                    match master_pass_ui() {
                        MasterPassResult::Password(pass) => { *master_pk = Some(generate_orion_key(&pass).unwrap()); },
                        MasterPassResult::NoPassword => { *master_pk = None; },
                        MasterPassResult::Cancel => {},
                    }
                },
                KeyCode::Char('v') => {
                    show_all = !show_all;
                },
                KeyCode::Char('n') => {
                    totp_next = !totp_next;
                },
                KeyCode::Char('y') => {
                    match tab {
                        Tab::Password => {
                            clip.set_contents(password_set.pass[pass_scroll].password.clone());
                        },
                        Tab::Totp => {
                            clip.set_contents(password_set.totp[totp_scroll].get_code(totp_next).to_string());
                        },
                    }
                },
                KeyCode::Char('d') => {
                    match tab {
                        Tab::Password => {
                            password_set.pass[pass_scroll].delete = !password_set.pass[pass_scroll].delete;
                        },
                        Tab::Totp => {
                            password_set.totp[totp_scroll].delete = !password_set.totp[totp_scroll].delete;
                        },
                    }
                },
                KeyCode::Char('e') => {
                    match tab {
                        Tab::Password => {
                            if password_set.pass.len() != 0 {
                                let this_pass: &mut pass::Password = &mut password_set.pass[pass_scroll];
                                let mut temp_pass: pass::Password = this_pass.clone();

                                if edit_values_ui("Edit Password", &mut [
                                    EditMenuValue::String("Name", &mut temp_pass.name),
                                    EditMenuValue::String("Password", &mut temp_pass.password),
                                ]) {
                                    *this_pass = temp_pass;
                                }
                            }
                        },
                        Tab::Totp => {
                            if password_set.totp.len() != 0 {
                                let this_totp: &mut totp::TotpCode = &mut password_set.totp[totp_scroll];
                                let mut temp_totp: totp::TotpCode = this_totp.clone();
                                let mut temp_secret = temp_totp.get_secret_string();

                                if edit_values_ui("Edit TOTP", &mut [
                                    EditMenuValue::String("Name", &mut temp_totp.name),
                                    EditMenuValue::Int("Digits", &mut temp_totp.data.digits, 4..8),
                                    EditMenuValue::String("Secret", &mut temp_secret),
                                ]) {
                                    temp_totp.set_secret_string(temp_secret);
                                    temp_totp.calculate_codes();
                                    *this_totp = temp_totp;
                                }
                            }
                        },
                    }
                },
                KeyCode::Char('o') => {
                    match tab {
                        Tab::Password => {
                            let mut temp_pass = pass::Password::new();

                            if edit_values_ui("Edit Password", &mut [
                                EditMenuValue::String("Name", &mut temp_pass.name),
                                EditMenuValue::String("Password", &mut temp_pass.password),
                            ]) {
                                if pass_scroll + 1 >= password_set.pass.len() {
                                    password_set.pass.push(temp_pass);
                                } else {
                                    password_set.pass.insert(pass_scroll + 1, temp_pass);
                                }

                                if password_set.pass.len() != 1 {
                                    pass_scroll += 1;
                                }
                            }
                        },
                        Tab::Totp => {
                            let mut temp_totp = totp::TotpCode::new();
                            let mut temp_secret = temp_totp.get_secret_string();

                            if edit_values_ui("Edit TOTP", &mut [
                                EditMenuValue::String("Name", &mut temp_totp.name),
                                EditMenuValue::Int("Digits", &mut temp_totp.data.digits, 4..8),
                                EditMenuValue::String("Secret", &mut temp_secret),
                            ]) {
                                temp_totp.set_secret_string(temp_secret);
                                temp_totp.calculate_codes();
                                if totp_scroll + 1 >= password_set.totp.len() {
                                    password_set.totp.push(temp_totp);
                                } else {
                                    password_set.totp.insert(totp_scroll + 1, temp_totp);
                                }

                                if password_set.totp.len() != 1 {
                                    totp_scroll += 1;
                                }
                            }
                        },
                    }
                },
                _ => {},
            }
        }
    }

    exit_alt_screen(&mut stdout);
}

fn master_pass_ui() -> MasterPassResult {
    let mut stdout = io::stdout();

    enter_alt_screen(&mut stdout);

    let mut pass = String::new();

    let master_pass = 'ui: loop {
        let size = terminal::size().unwrap();
        queue!(stdout,
               terminal::Clear(terminal::ClearType::All),
               cursor::MoveTo(ui::center_offset(size.0, 9), ui::center_offset(size.1, 0) - 1),
               style::Print(format!("Password:")),
               cursor::MoveTo(ui::center_offset(size.0, pass.len() as u16), ui::center_offset(size.1, 0)));

        for _ in 0..pass.len() {
            queue!(stdout, style::Print("*"));
        }

        stdout.flush();

        let ev = event::read().unwrap();
        let ev_key = &ui::input_key(&ev);
        if pass.len() >= 32 {
            if let event::KeyCode::Char(_) = ev_key {
                continue;
            }
        }

        let mut index = pass.char_indices().count();
        match ui::input_string(&mut pass, &mut index, ev_key) {
            ui::AfterAction::Enter => {
                if pass.len() == 0 {
                    break 'ui MasterPassResult::NoPassword;
                } else {
                    break 'ui MasterPassResult::Password(pass);
                }
            },
            ui::AfterAction::Cancel => {
                break 'ui MasterPassResult::Cancel;
            },
            _ => {},
        }
    };

    exit_alt_screen(&mut stdout);

    return master_pass;
}

fn edit_values_ui(title: &str, values: &mut [EditMenuValue]) -> bool {
    use event::{Event, KeyCode};

    let mut stdout = io::stdout();
    let mut selected: usize = 0;
    let mut string_index: usize = {
        if let EditMenuValue::String(_, string_val) = &values[selected] {
            string_val.char_indices().count()
        } else {
            0
        }
    }
;

    'ui: loop {
        let size = terminal::size().unwrap();

        queue!(stdout,
               terminal::Clear(terminal::ClearType::All),
               cursor::MoveTo(ui::center_offset(size.0, title.len() as u16), 0),
               style::Print(title));

        for value_index in 0..values.len() {
            if selected == value_index {
                queue!(stdout, style::SetForegroundColor(THEME_COLOUR));
            }

            queue!(stdout,
                   cursor::MoveTo(1, 2 + value_index as u16 * 3));
            match &values[value_index] {
                EditMenuValue::String(label, string_value) => {
                    queue!(stdout, style::Print(label));
                    ui::print_typing(5..size.0, 3 + value_index as u16 * 3, string_value,
                                     if selected == value_index { Some(string_index) } else { None });
                },
                EditMenuValue::Int(label, int_value, _) => {
                    queue!(stdout,
                           style::Print(label),
                           cursor::MoveTo(5, 3 + value_index as u16 * 3),
                           style::Print(int_value));
                },
            }

            queue!(stdout, style::ResetColor);
        }

        stdout.flush();

        if let Ok(true) = event::poll(POLL_TIME) {
            let ev = event::read().unwrap();
            let keyev = ui::input_key(&ev);

            match keyev {
                KeyCode::Esc => {
                    break 'ui false;
                },
                KeyCode::Enter => {
                    break 'ui true;
                },
                KeyCode::Up => {
                    if selected == 0 {
                        selected = values.len() - 1;
                    } else {
                        selected = selected - 1;
                    }
                    if let EditMenuValue::String(_, string_val) = &values[selected] {
                        string_index = string_val.char_indices().count();
                    }
                },
                KeyCode::Down => {
                    selected = (selected + 1) % values.len();
                    if let EditMenuValue::String(_, string_val) = &values[selected] {
                        string_index = string_val.char_indices().count();
                    }
                },
                _ => {
                    match &mut values[selected] {
                        EditMenuValue::String(_, string_val) => {
                            ui::input_string(*string_val, &mut string_index, &keyev);
                        },
                        EditMenuValue::Int(_, int_val, range) => {
                            match keyev {
                                KeyCode::Left => {
                                    if **int_val > range.start {
                                        **int_val -= 1;
                                    }
                                },
                                KeyCode::Right => {
                                    if **int_val < range.end {
                                        **int_val += 1;
                                    }
                                },
                                _ => {},
                            }
                        },
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
fn enter_alt_screen(stdout: &mut io::Stdout) {
    execute!(stdout,
             terminal::EnterAlternateScreen,
             terminal::DisableLineWrap,
             cursor::Hide);
    terminal::enable_raw_mode();
}

#[inline]
fn exit_alt_screen(stdout: &mut io::Stdout) {
    terminal::disable_raw_mode();
    execute!(stdout,
             terminal::LeaveAlternateScreen,
             terminal::EnableLineWrap,
             cursor::Show);
    stdout.flush();
}

#[inline]
fn generate_orion_key(key: &str) -> Result<SecretKey, CryptoError> {
    let padded = format!("{:32}", key);
    let bytes = padded.as_bytes();
    return SecretKey::from_slice(&bytes);
}
