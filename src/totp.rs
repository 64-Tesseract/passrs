use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct TotpCode {
    pub name: String,
    secret: String,
}

impl TotpCode {
    pub fn new(name: String) -> TotpCode {
        TotpCode { name: name, secret: "".to_string() }
    }
}
