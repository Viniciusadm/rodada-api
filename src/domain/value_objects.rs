#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DomainId(String);

impl DomainId {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainValidationError::Empty("id"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    pub fn new(
        value: impl Into<String>,
        field: &'static str,
    ) -> Result<Self, DomainValidationError> {
        let value = value.into().trim().to_owned();
        if value.is_empty() {
            return Err(DomainValidationError::Empty(field));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slug(String);

impl Slug {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into().trim().to_ascii_lowercase();
        let valid = !value.is_empty()
            && value
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !valid {
            return Err(DomainValidationError::Invalid("slug"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);

impl Email {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into().trim().to_ascii_lowercase();
        if !value.contains('@') || value.starts_with('@') || value.ends_with('@') {
            return Err(DomainValidationError::Invalid("email"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtcDateTime(String);

impl UtcDateTime {
    pub fn new_iso8601(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainValidationError::Empty("utc_datetime"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Points(i32);

impl Points {
    pub fn new(value: i32) -> Self {
        Self(value)
    }

    pub fn value(self) -> i32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositiveLimit(u32);

impl PositiveLimit {
    pub fn new(value: u32, field: &'static str) -> Result<Self, DomainValidationError> {
        if value == 0 {
            return Err(DomainValidationError::Invalid(field));
        }
        Ok(Self(value))
    }

    pub fn value(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinPin(String);

impl JoinPin {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into();
        let valid = value.len() == 6 && value.chars().all(|c| c.is_ascii_digit());
        if !valid {
            return Err(DomainValidationError::Invalid("join_pin"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainValidationError {
    Empty(&'static str),
    Invalid(&'static str),
}
