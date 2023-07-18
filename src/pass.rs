use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Password {
    pub name: String,
    password: String,
}

impl Password {
    pub fn new() -> Password {
        Password { name: "New Password".to_string(), password: String::new() }
    }
}
