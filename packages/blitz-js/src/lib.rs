use blitz_dom::BaseDocument;
use blitz_traits::Viewport;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn create_empty_document() -> String {
    let viewport = Viewport::new(250, 250, 1.0, blitz_traits::ColorScheme::Dark);
    let doc = BaseDocument::new(viewport);
    format!(
        "nodes: {:#?}\nnodes_to_id: {:#?}\nchanged: {:#?}",
        doc.nodes, doc.nodes_to_id, doc.changed
    )
}
