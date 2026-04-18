#![allow(non_snake_case)]
use crate::gui::EguiApp;
use aviutl2::{config::translate as tr, generic::*};
use aviutl2_eframe::EframeWindow;

mod gui;
mod settings;

const PLUGIN_NAME: &'static str = "中心ずらし_A";
const PLUGIN_AUTHOR: &'static str = "azurite";

pub static EDIT_HANDLE: aviutl2::generic::GlobalEditHandle =
    aviutl2::generic::GlobalEditHandle::new();

#[aviutl2::plugin(GenericPlugin)]
pub struct AdjustPivot {
    window: Option<EframeWindow>,
}

impl GenericPlugin for AdjustPivot {
    fn new(_info: aviutl2::AviUtl2Info) -> AnyResult<Self> {
        Ok(Self { window: None })
    }

    fn plugin_info(&self) -> GenericPluginTable {
        let name = tr(PLUGIN_NAME);
        let information = format!(
            "{} v{} {}",
            PLUGIN_NAME,
            env!("CARGO_PKG_VERSION"),
            PLUGIN_AUTHOR
        )
        .to_string();
        GenericPluginTable { name, information }
    }

    fn register(&mut self, host: &mut HostAppHandle) {
        let eframe_window = EframeWindow::new(tr(PLUGIN_NAME).as_str(), |cc, handle| {
            Ok(Box::new(EguiApp::new(cc, handle)))
        });

        if let Ok(w) = eframe_window {
            if let Ok(handle) = w.handle() {
                let _ = host.register_window_client(tr(PLUGIN_NAME).as_str(), &handle);
            }
            self.window = Some(w);
        }

        let edit_handle = host.create_edit_handle();
        EDIT_HANDLE.init(edit_handle);
    }
}

aviutl2::register_generic_plugin!(AdjustPivot);
