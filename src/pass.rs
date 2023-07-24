use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Password {
    pub name: String,
    pub password: String,
    #[serde(skip_serializing)]
    pub delete: bool,
}

impl Password {
    pub fn new() -> Password {
        Password {
            name: "New Password".to_string(), password: String::new(),
            delete: false,
        }
    }
}
