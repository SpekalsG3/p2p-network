use std::sync::Arc;
use protocol::core::state::ProtocolState;
use protocol::types::package::AppPackage;
use crate::utils::ui::UITerminal;

pub struct AppStateInner {
    // don't like that this fields are public
    pub protocol_state: ProtocolState, // this is accessed only to be passed to protocol functions
    pub ui: UITerminal, // this is accessed only by frontend
}

impl AppStateInner {
    pub fn new_package(&self, package: AppPackage) {
        match package {
            AppPackage::Message(message) => {
                let msg = String::from_utf8_lossy(&message.msg).to_string();
                self.ui.new_message(&format!("User: {}", message.from), &msg);
            }
            AppPackage::Alert(alert) => {
                // todo: write macro to wrap sending packages and ignore `level: DEBUG` in release mode
                self.ui.new_message(&format!("System: {}", alert.level), &alert.msg);
            }
        }
    }
}

pub type AppState = Arc<AppStateInner>;