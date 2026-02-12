/// Typed access to NEBO_APP_* environment variables set by Nebo's sandbox.
#[derive(Debug, Clone)]
pub struct AppEnv {
    pub dir: String,
    pub sock_path: String,
    pub id: String,
    pub name: String,
    pub version: String,
    pub data_dir: String,
}

impl AppEnv {
    pub fn load() -> Self {
        Self {
            dir: std::env::var("NEBO_APP_DIR").unwrap_or_default(),
            sock_path: std::env::var("NEBO_APP_SOCK").unwrap_or_default(),
            id: std::env::var("NEBO_APP_ID").unwrap_or_default(),
            name: std::env::var("NEBO_APP_NAME").unwrap_or_default(),
            version: std::env::var("NEBO_APP_VERSION").unwrap_or_default(),
            data_dir: std::env::var("NEBO_APP_DATA").unwrap_or_default(),
        }
    }
}
