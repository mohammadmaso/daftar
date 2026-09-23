/// Build information of the linked core, shown in Settings › About.
pub struct CoreInfo {
    pub app_name: String,
    pub version: String,
    pub repo_schema_version: u32,
    pub target: String,
}

#[flutter_rust_bridge::frb(sync)]
pub fn core_info() -> CoreInfo {
    let i = daftar_core::core_info();
    CoreInfo {
        app_name: i.app_name,
        version: i.version,
        repo_schema_version: i.repo_schema_version,
        target: i.target,
    }
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
