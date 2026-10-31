use crate::display::DisplayConfig;
use cownter_core::scheduler::Scheduler;
use cownter_core::secrets::SecretStore;
use cownter_core::sources::Fetcher;
use cownter_core::store::Store;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub store: Arc<Store>,
    pub scheduler: Scheduler,
    pub secrets: Arc<dyn SecretStore>,
    pub fetcher: Arc<Fetcher>,
    pub display: Mutex<DisplayConfig>,
}

impl AppState {
    pub fn display(&self) -> DisplayConfig {
        self.display.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    pub fn set_display(&self, cfg: DisplayConfig) {
        *self.display.lock().unwrap_or_else(|p| p.into_inner()) = cfg;
    }
}
