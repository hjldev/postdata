use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pair {
    pub id: String,
    pub key: String,
    pub value: String,
    pub enabled: bool,
    #[serde(default)]
    pub kind: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Auth {
    pub kind: String,
    pub token: String,
    pub username: String,
    pub password: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequestBody {
    pub kind: String,
    pub text: String,
    pub fields: Vec<Pair>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestSpec {
    pub id: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub params: Vec<Pair>,
    pub headers: Vec<Pair>,
    pub auth: Auth,
    pub body: RequestBody,
    pub timeout_ms: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseData {
    pub status: u16,
    pub final_url: String,
    pub headers: Vec<(String, String)>,
    pub elapsed_ms: u64,
    pub size: usize,
    pub text: Option<String>,
    pub binary: bool,
    pub truncated: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub id: String,
    pub at: u64,
    pub request: RequestSpec,
    pub status: Option<u16>,
    pub elapsed_ms: u64,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedData {
    pub version: u32,
    pub favorites: Vec<RequestSpec>,
    pub history: Vec<History>,
}
impl Default for SavedData {
    fn default() -> Self {
        Self {
            version: 1,
            favorites: vec![],
            history: vec![],
        }
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendOutcome {
    pub response: Option<ResponseData>,
    pub error: Option<String>,
    pub storage_error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieInfo {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub expires_at: Option<i64>,
}
#[derive(Debug, Serialize)]
pub struct CookieList {
    pub cookies: Vec<CookieInfo>,
    pub warning: Option<String>,
}
