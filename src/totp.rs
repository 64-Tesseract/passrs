use totp_lite::{totp_custom, Sha512};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TotpCode {
    pub name: String,
    pub digits: usize,
    pub secret: String,
    #[serde(skip_serializing)]
    pub codes: Option<(u64, String, String)>,
    #[serde(skip_serializing)]
    pub delete: bool,
}

impl TotpCode {
    pub fn new() -> TotpCode {
        TotpCode {
            name: "New Code".to_string(), digits: 6, secret: String::new(),
            codes: None, delete: false,
        }
    }

    pub fn calculate_codes(&mut self, totp_time: u64) {
        let totp_now = match &self.codes {
            Some(c) => {
                if c.0 != totp_time - 1 {
                    totp_custom::<Sha512>(1, self.digits as u32, &self.secret.as_bytes(), totp_time)
                } else {
                    c.2.clone()
                }
            },
            None => totp_custom::<Sha512>(1, self.digits as u32, &self.secret.as_bytes(), totp_time),
        };

        let totp_next = totp_custom::<Sha512>(1, self.digits as u32, &self.secret.as_bytes(), totp_time + 1);

        self.codes = Some((totp_time, totp_now, totp_next));
    }

    pub fn recalculate_code(&mut self) {
        if let Some(codes) = &self.codes {
            self.codes = Some((
                codes.0,
                totp_custom::<Sha512>(1, self.digits as u32, &self.secret.as_bytes(), codes.0),
                totp_custom::<Sha512>(1, self.digits as u32, &self.secret.as_bytes(), codes.0 + 1)
                ));
        }
    }

    pub fn get_code(&self, next: bool) -> &str {
        match &self.codes {
            Some(c) => {
                if next {
                    &c.2
                } else {
                    &c.1
                }
            },
            None => &"------",
        }
    }
}
