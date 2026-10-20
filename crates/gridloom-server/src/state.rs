use gridloom_core::workbook::Workbook;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub workbook: Arc<RwLock<Workbook>>,
}

impl AppState {
    pub fn new(workbook: Workbook) -> Self {
        Self {
            workbook: Arc::new(RwLock::new(workbook)),
        }
    }
}
