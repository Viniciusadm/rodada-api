use crate::domain::DomainId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RealtimeChannel {
    Organization(DomainId),
    Match(DomainId),
    Room(DomainId),
    Ranking(DomainId),
}

impl RealtimeChannel {
    pub fn key(&self) -> String {
        match self {
            Self::Organization(id) => format!("organization:{}", id.as_str()),
            Self::Match(id) => format!("match:{}", id.as_str()),
            Self::Room(id) => format!("room:{}", id.as_str()),
            Self::Ranking(id) => format!("ranking:{}", id.as_str()),
        }
    }

    pub fn parse(value: &str) -> Result<Self, crate::errors::AppError> {
        let (kind, id) = value.split_once(':').ok_or_else(|| {
            crate::errors::AppError::Validation("invalid realtime channel".to_owned())
        })?;
        let id = DomainId::new(id.to_owned())?;
        match kind {
            "organization" => Ok(Self::Organization(id)),
            "match" => Ok(Self::Match(id)),
            "room" => Ok(Self::Room(id)),
            "ranking" => Ok(Self::Ranking(id)),
            _ => Err(crate::errors::AppError::Validation(
                "invalid realtime channel".to_owned(),
            )),
        }
    }
}
