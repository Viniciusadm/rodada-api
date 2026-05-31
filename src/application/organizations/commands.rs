#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateOrganizationCommand {
    pub name: String,
    pub slug: String,
    pub contact_email: Option<String>,
    pub owner_user_id: Option<String>,
}
