use blitz_dom::BaseDocument;
use blitz_renderer_vello::BlitzVelloRenderer;
use blitz_traits::{ColorScheme, Devtools, DocumentRenderer, Viewport};
use log::{info, Level};
use raw_window_handle::{
    DisplayHandle, HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
    WindowHandle, XlibDisplayHandle, XlibWindowHandle,
};
use std::{panic, ptr::NonNull, sync::Arc};
use wasm_bindgen::prelude::wasm_bindgen;

struct WasmWindowHandle {
    display_handle: u32,
    screen_handle: i32,
    window_handle: u32,
}
impl WasmWindowHandle {
    fn from_window(display_handle: u32, screen_handle: i32, window_handle: u32) -> Self {
        WasmWindowHandle {
            display_handle,
            screen_handle,
            window_handle,
        }
    }
}

impl HasWindowHandle for WasmWindowHandle {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let raw = RawWindowHandle::Xlib(XlibWindowHandle::new(self.window_handle.into()));
        let handle = unsafe { WindowHandle::borrow_raw(raw) };
        Ok(handle)
    }
}
impl HasDisplayHandle for WasmWindowHandle {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        let pointer = unsafe { NonNull::new_unchecked(self.display_handle as *mut _) };
        let raw = RawDisplayHandle::Xlib(XlibDisplayHandle::new(Some(pointer), self.screen_handle));
        let handle = unsafe { DisplayHandle::borrow_raw(raw) };
        Ok(handle)
    }
}

#[wasm_bindgen]
pub fn create_empty_document(
    display_handle: u64,
    screen_handle: u64,
    window_handle: u64,
) -> String {
    let _ = console_log::init_with_level(Level::Debug);
    panic::set_hook(Box::new(console_error_panic_hook::hook));

    let display_handle = display_handle as u32;
    let screen_handle = screen_handle as i32;
    let window_handle = window_handle as u32;

    info!("init");

    let viewport = Viewport::new(250, 250, 1.0, ColorScheme::Dark);
    let doc = BaseDocument::new(viewport.clone());

    let handle = WasmWindowHandle::from_window(display_handle, screen_handle, window_handle);
    let window = Arc::new(handle);

    let mut renderer = BlitzVelloRenderer::new(window);
    
    info!("render");
    renderer.render(&doc, 1.0, 250, 250, Devtools::default());
    info!("resume");
    renderer.resume(&viewport);

    format!(
        "nodes: {:#?}\nnodes_to_id: {:#?}\nchanged: {:#?}",
        doc.nodes, doc.nodes_to_id, doc.changed
    )
}
