use rand::{Rng, thread_rng, distributions::Standard};
use std::iter;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Password {
    pub name: String,
    pub password: String,
    #[serde(skip)]
    pub delete: bool,
}

impl Password {
    pub fn new() -> Password {
        Password {
            name: "New Password".to_string(),
            password: String::from_iter(thread_rng().sample_iter::<char, &Standard>(&Standard).take(32)),
            delete: false,
        }
    }
}
