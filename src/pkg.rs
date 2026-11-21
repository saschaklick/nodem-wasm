//! Media assets and packages: load/generate pkgs, import and remove images, borders, fonts, pages.

use js_sys::{ JsString, Uint8ClampedArray };
use wasm_bindgen::prelude::*;

use crate::GLOBALS;

#[wasm_bindgen]
pub async fn print_media()  {
    if unsafe { GLOBALS.is_none() } {
        log::error!("debug locked");
        return;
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let surface = &mut globals.surface;

    surface.media.log();

    unsafe { GLOBALS.replace(globals) };
}

#[wasm_bindgen]
pub fn load_pkg(data: &[u8], source: Option<u8>) {
    if unsafe { GLOBALS.is_none() } {
        log::error!("load_media locked");
        return;
    }

    let src = source.unwrap_or(1);
    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let surface = &mut globals.surface;
    // Keep this pkg's bytes in the source's OWN buffer: media.load_pkg only stores a pointer into it,
    // so it must stay alive and untouched by the next load of a different source (ticket 98c3fc18).
    let media = &mut globals.int_buffers[(src as usize) & 7];

    media.resize(data.len(), 0);
    log::debug!("pkg buffer resized: {}b", media.len());

    media.copy_from_slice(data);

    surface.media.load_pkg(media.as_ptr() as *const u8, media.len(), src);

    unsafe { GLOBALS.replace(globals) };
}

#[wasm_bindgen]
pub fn generate_pkg(name: JsValue, sources: Option<u8>) -> Uint8ClampedArray {
    if unsafe { GLOBALS.is_none() } {
        log::error!("generate_pkg locked");
        return Uint8ClampedArray::new(&JsValue::null());
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let surface = &mut globals.surface;

    let jsname = JsString::from(name);
    let buf = surface.media.generate_pkg(sources.unwrap_or(0b11111110), jsname.as_string().unwrap().as_str(), true);

    unsafe { GLOBALS.replace(globals) };

    return Uint8ClampedArray::from(&buf[..]);
}

#[wasm_bindgen]
pub fn import_image(index: u8, name: &str, rgba: &[u8], width: u8, height: u8, monochrome: Option<bool>, alpha: Option<bool>, threshold_0: Option<u8>, threshold_1: Option<u8>) {
    if unsafe { GLOBALS.is_none() } {
        log::error!("import_image locked");
        return;
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let surface = &mut globals.surface;

    surface.media.import_image(index, name, rgba, width, height, monochrome.unwrap_or(false), alpha.unwrap_or(true), threshold_0.unwrap_or(128), threshold_1.unwrap_or(128));

    unsafe { GLOBALS.replace(globals) };
}

#[wasm_bindgen]
pub fn remove_image(index: u8) {
    if unsafe { GLOBALS.is_none() } {
        log::error!("remove_image locked");
        return;
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let surface = &mut globals.surface;

    surface.media.remove_image(index);

    unsafe { GLOBALS.replace(globals) };
}

#[wasm_bindgen]
pub fn import_border(index: u8, name: &str, rgba: &[u8], width: u8, height: u8) {
    if unsafe { GLOBALS.is_none() } {
        log::error!("import_border locked");
        return;
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let surface = &mut globals.surface;

    surface.media.import_border(index, name, rgba, width, height, true);

    unsafe { GLOBALS.replace(globals) };
}

#[wasm_bindgen]
pub fn remove_border(index: u8) {
    if unsafe { GLOBALS.is_none() } {
        log::error!("remove_border locked");
        return;
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let surface = &mut globals.surface;

    surface.media.remove_border(index);

    unsafe { GLOBALS.replace(globals) };
}

#[wasm_bindgen]
pub fn import_font(index: u8, name: &str, rgba: &[u8], width: u32, height: u32) {
    if unsafe { GLOBALS.is_none() } {
        log::error!("import_border locked");
        return;
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let surface = &mut globals.surface;

    surface.media.import_font(index, name, rgba, width, height);

    unsafe { GLOBALS.replace(globals) };
}

#[wasm_bindgen]
pub fn remove_font(index: u8) {
    if unsafe { GLOBALS.is_none() } {
        log::error!("remove_font locked");
        return;
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let surface = &mut globals.surface;

    surface.media.remove_font(index);

    unsafe { GLOBALS.replace(globals) };
}

#[wasm_bindgen]
pub fn import_page(index: u8, name: &str, content: &str) {
    if unsafe { GLOBALS.is_none() } {
        log::error!("load_media locked");
        return;
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let surface = &mut globals.surface;

    let page = crate::media::Media::xml_to_bin(content);

    surface.media.import_page(index, name, &page);

    unsafe { GLOBALS.replace(globals) };
}

#[wasm_bindgen]
pub fn remove_page(index: u8) {
    if unsafe { GLOBALS.is_none() } {
        log::error!("load_media locked");
        return;
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let surface = &mut globals.surface;

    surface.media.remove_page(index);

    unsafe { GLOBALS.replace(globals) };
}
