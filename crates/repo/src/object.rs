use crate::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Object(String);

impl Object {
    pub fn parse(value: &str) -> Result<Self, Error> {
        if !matches!(value.len(), 40 | 64) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(Error::Invalid(
                "object must be one full hexadecimal Git ID".into(),
            ));
        }
        Ok(Self(value.to_ascii_lowercase()))
    }

    pub fn absent(&self) -> bool {
        self.0.bytes().all(|byte| byte == b'0')
    }

    pub fn hex(&self) -> &str {
        &self.0
    }
}
