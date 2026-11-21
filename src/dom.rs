//! DOM and XML: load/export page markup and build the DOM from a page in media.

use wasm_bindgen::prelude::*;
use bytes::BytesMut;
use core::slice;

use crate::GLOBALS;

#[wasm_bindgen]
pub fn load_xml(data: &str) {
    if unsafe { GLOBALS.is_none() } {
        log::error!("load_xml locked");
        return;
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let dom = &mut globals.dom;
    let xml = &mut globals.dom_buffer;

    xml.resize(data.len(), 0);
    log::debug!("dom buffer resized: {}b", xml.len());

    xml.copy_from_slice(data.as_bytes());

    dom.from_xml(str::from_utf8(unsafe { slice::from_raw_parts(xml.as_ptr(), xml.len()) } ).unwrap());

    unsafe { GLOBALS.replace(globals) };
}

#[wasm_bindgen]
pub fn get_xml() -> JsValue {
    if unsafe { GLOBALS.is_none() } {
        log::error!("load_xml locked");
        return JsValue::null();
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let dom = &mut globals.dom;

    let mut xml = BytesMut::with_capacity(1024);

    let res = dom.to_xml(&mut xml, Some("  "), false);
    if res.is_err() {
        log::error!("get_xml failed: {:?}", res.err().unwrap());
        return JsValue::null();
    }

    unsafe { GLOBALS.replace(globals) };

    return JsValue::from_str(str::from_utf8(&xml).unwrap_or(""));
}

/// Build the DOM from a page already loaded into media (e.g. via `load_pkg`), so `render(draw_dom)`
/// rasterises the ACTUALLY received package — not a re-parse of the source markup. This mirrors what
/// the device runtime does after a pkg reload (`dom.from(surface.media.get_page(0).content)`,
/// nodem-rs runtime.rs), and is the bridge the device simulator's display view needs (ticket
/// 6e3096a8): receive pkg over the wire → load_pkg → load_page_to_dom → render.
#[wasm_bindgen]
pub fn load_page_to_dom(index: u8) {
    if unsafe { GLOBALS.is_none() } {
        log::error!("load_page_to_dom locked");
        return;
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let dom = &mut globals.dom;
    let surface = &globals.surface;

    dom.from(surface.media.get_page(index).content);

    unsafe { GLOBALS.replace(globals) };
}
