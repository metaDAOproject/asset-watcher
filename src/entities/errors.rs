use std::fmt;

#[derive(Debug)]
pub struct GeneralError {
    details: String,
}

impl GeneralError {
    pub fn new(msg: &str) -> GeneralError {
        GeneralError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for GeneralError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl std::error::Error for GeneralError {
    fn description(&self) -> &str {
        &self.details
    }
}
