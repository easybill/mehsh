use tokio::fs;

pub struct MaintenanceMode;

impl MaintenanceMode {
    pub async fn is_active() -> bool {
        let file = "/tmp/mehsh_maintenance";
        let metadata = match fs::metadata(file).await {
            Ok(f) => f,
            Err(e) => {
                return false;
            }
        };

        metadata.is_file()
    }
}