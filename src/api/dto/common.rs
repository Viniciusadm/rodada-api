use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct PageResponse<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
}
