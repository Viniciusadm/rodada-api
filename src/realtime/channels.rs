use crate::domain::DomainId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RealtimeChannel {
    User(DomainId),
    Match(DomainId),
    Room(DomainId),
}

impl RealtimeChannel {
    pub fn key(&self) -> String {
        match self {
            Self::User(id) => format!("user:{}", id.as_str()),
            Self::Match(id) => format!("match:{}", id.as_str()),
            Self::Room(id) => format!("room:{}", id.as_str()),
        }
    }

    pub fn parse(value: &str) -> Result<Self, crate::errors::AppError> {
        let (kind, id) = value.split_once(':').ok_or_else(|| {
            crate::errors::AppError::Validation("invalid realtime channel".to_owned())
        })?;
        let id = DomainId::new(id.to_owned())?;
        match kind {
            "user" => Ok(Self::User(id)),
            "match" => Ok(Self::Match(id)),
            "room" => Ok(Self::Room(id)),
            _ => Err(crate::errors::AppError::Validation(
                "invalid realtime channel".to_owned(),
            )),
        }
    }
}
