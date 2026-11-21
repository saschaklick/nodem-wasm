//! Rendering and the framebuffer: sizing, rasterising the DOM, pixel conversion to RGBA,
//! text preview, palette and refresh rate.

use bitmap_writer::{ Bitmap, Frame, Style, Writer };
use js_sys::{ JsString, Uint8ClampedArray };
use nodem_rs::*;
use nostd::string::String;
use wasm_bindgen::prelude::*;

use crate::GLOBALS;

static mut LOOP_CNT: u32 = 0;

/// Bytes the framebuffer needs for width x height in the compiled `nodem_mode`
/// (1, 8, 12, 16 or 24 bits per pixel, rows are not padded).
fn fb_size(width: usize, height: usize) -> usize {
    (width * height * Color::BITS_PER_PIXEL + 7) / 8
}

/// Scale a `bits`-wide channel value to 0..=255.
#[inline]
#[allow(dead_code)]
fn expand(v: u32, bits: u32) -> u8 {
    (v * 255 / ((1 << bits) - 1)) as u8
}

/// RGBA of pixel `i` of a framebuffer in the compiled `nodem_mode`.
/// Layouts mirror nodem-rs `Surface::draw_pixel_unsafe` / `Color::from_rgb`.
/// `palette` (RGBA entries) is only used by the 1-bit index mode.
#[inline]
fn pixel_rgba(fb: &[u8], i: usize, palette: &[u8]) -> [u8; 4] {
    cfg_select! {
        nodem_mode = "rgb332" => {
            let _ = palette;
            let c = fb[i] as u32; // rrrgggbb
            [expand((c >> 5) & 0x7, 3), expand((c >> 2) & 0x7, 3), expand(c & 0x3, 2), 255]
        }
        nodem_mode = "rgb444" => {
            // 3 nibbles per pixel (r, g, b), packed high nibble first
            let _ = palette;
            let nib = |n: usize| -> u32 { ((fb[n / 2] >> (if n % 2 == 0 { 4 } else { 0 })) & 0xf) as u32 };
            let n = i * 3;
            [expand(nib(n), 4), expand(nib(n + 1), 4), expand(nib(n + 2), 4), 255]
        }
        nodem_mode = "rgb555" => {
            let _ = palette;
            let c = u16::from_le_bytes([fb[i * 2], fb[i * 2 + 1]]) as u32; // 0rrrrrgggggbbbbb
            [expand((c >> 10) & 0x1f, 5), expand((c >> 5) & 0x1f, 5), expand(c & 0x1f, 5), 255]
        }
        nodem_mode = "rgb565" => {
            let _ = palette;
            let c = u16::from_le_bytes([fb[i * 2], fb[i * 2 + 1]]) as u32; // rrrrrggggggbbbbb
            [expand((c >> 11) & 0x1f, 5), expand((c >> 5) & 0x3f, 6), expand(c & 0x1f, 5), 255]
        }
        nodem_mode = "rgb666" => {
            // one byte per channel, each holding a 6-bit value
            let _ = palette;
            [expand((fb[i * 3] & 0x3f) as u32, 6), expand((fb[i * 3 + 1] & 0x3f) as u32, 6), expand((fb[i * 3 + 2] & 0x3f) as u32, 6), 255]
        }
        nodem_mode = "rgb24" => {
            let _ = palette;
            [fb[i * 3], fb[i * 3 + 1], fb[i * 3 + 2], 255]
        }
        _ => {
            let bit = ((fb[i / 8] >> (7 - (i % 8))) & 0b1) as usize;
            [palette[bit * 4], palette[bit * 4 + 1], palette[bit * 4 + 2], palette[bit * 4 + 3]]
        }
    }
}

#[wasm_bindgen]
pub fn resize(width: usize, height: usize) {
    if unsafe { GLOBALS.is_none() } {
        log::error!("resize locked");
        return;
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };

    let framebuffer = &mut globals.scr_buffer;
    let surface = &mut globals.surface;

    let fb_req_size = fb_size(width, height);
    framebuffer.resize(fb_req_size, 0);

    surface.resize(framebuffer.as_mut(), width as SizeW, height as SizeH);

    unsafe { GLOBALS.replace(globals) };
}

#[wasm_bindgen]
pub fn render(mem: &mut [u8], width: usize, height: usize, draw_dom: bool) -> Uint8ClampedArray {
    if unsafe { GLOBALS.is_none() } {
        log::error!("render locked");
        return Uint8ClampedArray::from(&mem[..]);
    }
    if mem.len() != width * height * 4 {
        log::error!("memsize and dimensions mismatch");
        return Uint8ClampedArray::from(&mem[..]);
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };

    let framebuffer = &mut globals.scr_buffer;
    let palette = &globals.palette;
    #[cfg(feature = "dom")]
    let dom = &mut globals.dom;
    let surface = &mut globals.surface;

    let fb_req_size = fb_size(width, height);
    if framebuffer.len() != fb_req_size {
        framebuffer.resize(fb_req_size, 0);
    }

    surface.resize(framebuffer.as_mut(), width as SizeW, height as SizeH);

    if draw_dom {
        surface.clear(0);
        #[cfg(feature = "dom")]
        surface.update(dom);
        #[cfg(not(feature = "dom"))]
        log::error!("dom feature not found");
    }

    // surface.draw_rect(Area { point: Point { x: 10, y: 5 }, size: Size { width: 10, height: 5 } }, 1);
    // surface.draw_image(10, Point { x: 35 + (((unsafe { LOOP_CNT } as f32) / 10.0).cos() * 40.0) as PosX, y: 3 + (((unsafe { LOOP_CNT } as f32) / 17.0).cos() * 5.0) as PosY }, None);

    for (i, out) in mem.chunks_exact_mut(4).enumerate().take(width * height) {
        out.copy_from_slice(&pixel_rgba(framebuffer, i, palette));
    }

    unsafe { GLOBALS.replace(globals) };

    unsafe { LOOP_CNT += 1; }

    log::info!("render");

    return Uint8ClampedArray::from(&mem[..]);
}

#[wasm_bindgen]
pub fn render_to_string(width: usize, height: usize, draw_dom: bool) -> JsValue {
    if unsafe { GLOBALS.is_none() } {
        log::error!("render locked");
        return JsValue::null()
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };

    let framebuffer = &mut globals.scr_buffer;
    #[cfg(feature = "dom")]
    let dom = &mut globals.dom;
    let surface = &mut globals.surface;

    let fb_req_size = fb_size(width, height);
    if framebuffer.len() != fb_req_size {
        framebuffer.resize(fb_req_size, 0);
    }

    surface.resize(framebuffer.as_mut(), width as SizeW, height as SizeH);

    if draw_dom {
        surface.clear(0);
        #[cfg(feature = "dom")]
        surface.update(dom);
        #[cfg(not(feature = "dom"))]
        log::error!("dom feature not found");
    }

    // surface.draw_rect(Area { point: Point { x: 10, y: 5 }, size: Size { width: 10, height: 5 } }, 1);
    // surface.draw_image(10, Point { x: 35 + (((unsafe { LOOP_CNT } as f32) / 10.0).cos() * 40.0) as PosX, y: 3 + (((unsafe { LOOP_CNT } as f32) / 17.0).cos() * 5.0) as PosY }, None);

    let image = Bitmap::new(surface.width.into(), surface.height.into(), &framebuffer);

    let mut buf = String::new();
    let mut p = Writer::new();
    p.style(Style::UnicodeBlock1x2)
    .frame(Frame::UnicodeDoubleUFrame)
    .ansi_position_restore(false)
    .write(&mut buf, &image);

    unsafe { GLOBALS.replace(globals) };

    unsafe { LOOP_CNT += 1; }

    log::info!("render_to_string");

    return JsString::from(str::from_utf8(buf.as_bytes()).unwrap_or("")).into();
}

#[wasm_bindgen]
pub fn get_refresh_rate() -> JsValue {
    return JsValue::from_f64(1000.0 / 30.0);
}

#[wasm_bindgen]
pub fn get_palette() -> JsValue {
    if unsafe { GLOBALS.is_none() } {
        log::error!("debug locked");
        return JsValue::null();
    }

    let globals = unsafe { GLOBALS.take().unwrap() };
    let palette = globals.palette.to_vec();

    unsafe { GLOBALS.replace(globals) };
    return JsValue::from(Uint8ClampedArray::from(palette.clone().to_vec().as_slice()));
}

#[wasm_bindgen]
pub fn set_palette(rgb: Uint8ClampedArray) {
    if unsafe { GLOBALS.is_none() } {
        log::error!("debug locked");
        return;
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let palette = &mut globals.palette;

    for i in 0..core::cmp::min(rgb.length(), palette.len() as u32) {
        palette[i as usize] = rgb.get_index(i);
    }

    unsafe { GLOBALS.replace(globals) };
}

#[wasm_bindgen]
pub fn color_to_rgba(color: ColorType) -> Uint8ClampedArray {
    if color == Color::TRANSPARENT {
        Uint8ClampedArray::from([0x00, 0x00, 0x00, 0x00].as_ref())
    } else {
        let buf = Uint8ClampedArray::from(Color::to_rgb(color).as_ref());
        buf.set_index(3, 0xff);
        buf
    }
}