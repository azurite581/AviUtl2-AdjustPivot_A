#![allow(non_snake_case)]

mod gui;
mod settings;
mod utils;

use aviutl2::{config::translate as tr, generic::*};

pub static EDIT_HANDLE: aviutl2::generic::GlobalEditHandle =
    aviutl2::generic::GlobalEditHandle::new();

#[aviutl2::plugin(GenericPlugin)]
pub struct AdjustPivot {
    window: aviutl2_eframe::EframeWindow,
}

impl GenericPlugin for AdjustPivot {
    fn new(_info: aviutl2::AviUtl2Info) -> AnyResult<Self> {
        let window =
            aviutl2_eframe::EframeWindow::new(tr(gui::PLUGIN_NAME).as_str(), move |cc, handle| {
                Ok(Box::new(gui::AdjustPivotApp::new(cc, handle)))
            })?;
        Ok(Self { window })
    }

    fn plugin_info(&self) -> GenericPluginTable {
        let name = tr(gui::PLUGIN_NAME);
        let information = format!(
            "{} v{} {}",
            gui::PLUGIN_NAME,
            env!("CARGO_PKG_VERSION"),
            gui::PLUGIN_AUTHOR
        )
        .to_string();
        GenericPluginTable { name, information }
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        if let Ok(handle) = self.window.handle() {
            registry
                .register_window_client(tr(gui::PLUGIN_NAME).as_str(), &handle)
                .unwrap();
        }
        EDIT_HANDLE.init(registry.create_edit_handle());
    }
}

aviutl2::register_generic_plugin!(AdjustPivot);
