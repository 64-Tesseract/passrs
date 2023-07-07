use totp_lite::{totp_custom, Sha512};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct TotpCode {
    pub name: String,
    pub digits: u8,
    pub codes: Option<(u64, String, String)>,
    secret: Vec<u8>,
}

impl TotpCode {
    pub fn new(name: String) -> TotpCode {
        TotpCode { name: name, digits: 6, codes: None, secret: vec![] }
    }

    pub fn calculate_codes(&mut self, totp_time: u64) {
        let totp_now = match &self.codes {
            Some(c) => {
                if c.0 != totp_time - 1 {
                    totp_custom::<Sha512>(1, self.digits as u32, &self.secret, totp_time)
                } else {
                    c.2.clone()
                }
            },
            None => totp_custom::<Sha512>(1, self.digits as u32, &self.secret, totp_time),
        };

        let totp_next = totp_custom::<Sha512>(1, self.digits as u32, &self.secret, totp_time + 1);

        self.codes = Some((totp_time, totp_now, totp_next));
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
