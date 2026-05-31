#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRoleKind {
    Owner,
    Admin,
    Host,
    Member,
}
