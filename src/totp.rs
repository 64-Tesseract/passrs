use totp_rs::{TOTP, Secret, Algorithm};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TotpCode {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none", default = "Option::default")]
    raw_secret: Option<String>,
    pub data: TOTP,
    #[serde(skip, default = "Option::default")]
    pub cached_codes: Option<(String, String)>,
    #[serde(skip, default = "bool::default")]
    pub delete: bool,
}

impl TotpCode {
    pub fn new() -> TotpCode {
        TotpCode {
            name: "New code".to_string(), raw_secret: None,
            data: TOTP::new_unchecked(Algorithm::SHA1, 6, 0, 30, Vec::new()),
            cached_codes: None, delete: false,
        }
    }

    pub fn get_secret_string(&self) -> String {
        if let Some(raw) = &self.raw_secret {
            raw.to_string()
        } else {
            self.data.get_secret_base32()
        }
    }

    pub fn set_secret_string(&mut self, secret: String) {
        /*
        self.data.secret = Secret::Encoded(secret.clone()).to_bytes()
            .or(Secret::Raw(secret.as_bytes().to_vec()).to_bytes())
            .unwrap();
        */
        if let Ok(encoded) = Secret::Encoded(secret.clone()).to_bytes() {
            self.raw_secret = None;
            self.data.secret = encoded;
        } else {
            self.raw_secret = Some(secret.clone());
            self.data.secret = Secret::Raw(secret.as_bytes().to_vec()).to_bytes().unwrap();
        }
    }

    pub fn calculate_codes(&mut self) {
        let totp_now = self.data.generate_current().unwrap();
        let totp_next = self.data.generate(self.data.next_step_current().unwrap());

        self.cached_codes = Some((totp_now, totp_next));
    }

    pub fn get_code(&self, next: bool) -> &str {
        match &self.cached_codes {
            Some(c) => {
                if next {
                    &c.1
                } else {
                    &c.0
                }
            },
            None => &"------",
        }
    }
}
