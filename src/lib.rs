#![no_std]
#![allow(static_mut_refs)]

extern crate wee_alloc;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use js_sys::Uint8ClampedArray;
use wasm_bindgen::prelude::*;
use bytes::{ BytesMut };
use core::{ slice };

use nodem_rs::*;
use nodem_rs::surface::*;
use nodem_rs::control::*;
#[cfg(feature = "dom")]
use nodem_rs::dom::*;
#[cfg(feature = "vm")]
use virtmach::VirtMach;

mod pkg;
mod surface;
#[cfg(feature = "dom")]
mod dom;
#[cfg(feature = "vm")]
mod vm;

/// Without the `dom` feature the DOM exports only report that it is missing.
#[cfg(not(feature = "dom"))]
mod dom {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub fn load_xml(_data: &str) { log::error!("dom feature not found"); }

    #[wasm_bindgen]
    pub fn get_xml() -> JsValue { JsValue::null() }

    #[wasm_bindgen]
    pub fn load_page_to_dom(_index: u8) { log::error!("dom feature not found"); }
}

struct Globals <'a> {
    palette: BytesMut,
    surface: Surface,
    control: Control,
    #[cfg(feature = "dom")]
    dom: DOM,
    #[cfg(feature = "vm")]
    vm: VirtMach<'a>,
    scr_buffer: BytesMut,
    dom_buffer: BytesMut,
    // One persistent buffer PER source. media.load_pkg stores the lib data as a POINTER into the
    // buffer it is handed (no copy), so each source needs its own backing store — a single shared
    // buffer meant a second load_pkg overwrote the first source's font/page data (ticket 98c3fc18).
    int_buffers: [BytesMut; 8]
}

static mut GLOBALS: Option<Globals> = None;

pub static SYS_PKG: &'static [u8] = include_bytes!("sys.pkg");
//pub static INT_PKG: &'static [u8] = include_bytes!("../../nodem-pkg/pkg/int.pkg");

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
    wasm_logger::init(wasm_logger::Config::default());
    log::set_max_level(log::LevelFilter::Error);

    let mut surface = Surface::new(&mut [0], 1, 1);

    surface.media.load_pkg(unsafe { slice::from_raw_parts(SYS_PKG.as_ptr(), SYS_PKG.len()).as_ptr() as *const u8 }, SYS_PKG.len(), 0);
    //surface.media.load_pkg(unsafe { slice::from_raw_parts(INT_PKG.as_ptr(), INT_PKG.len()).as_ptr() as *const u8 }, INT_PKG.len(), 2);

    let palette: &[u8] = &[
        48, 48, 48, 255,
        220, 220, 255, 255
    ];

    unsafe { GLOBALS.replace( Globals {
        palette: BytesMut::from(palette),
        surface: surface,
        control: Control::default(),
        #[cfg(feature = "dom")]
        dom: DOM::default(),
        #[cfg(feature = "vm")]
        vm: VirtMach::new(),
        scr_buffer: BytesMut::zeroed(1),
        dom_buffer: BytesMut::zeroed(1),
        int_buffers: core::array::from_fn(|_| BytesMut::zeroed(1))
    }); }

    log::info!("setup");

    Ok(())
}

#[wasm_bindgen]
pub fn reset() {
     if unsafe { GLOBALS.is_none() } {
        log::error!("clear_int locked");
        return;
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    globals.surface.media.clear();
    #[cfg(feature = "dom")]
    globals.dom.clear();
    globals.scr_buffer = BytesMut::zeroed(1);
    globals.dom_buffer = BytesMut::zeroed(1);
    for b in globals.int_buffers.iter_mut() { *b = BytesMut::zeroed(1); }

    unsafe { GLOBALS.replace(globals) };
}

#[wasm_bindgen]
pub fn process_commands(cmds: JsValue) -> JsValue {
    if cmds.as_string().is_some() || cmds.is_instance_of::<Uint8ClampedArray>() {

        if unsafe { GLOBALS.is_none() } {
            log::error!("debug locked");
            return JsValue::null();
        }

        let cmd_s;
        let cmd_a;
        let cmd_v;
        let cmd_b: &[u8];
        if cmds.as_string().is_some() {
            cmd_s = cmds.as_string().unwrap();
            cmd_b = cmd_s.as_bytes();
        }else{
            cmd_a = <wasm_bindgen::JsValue as Into<Uint8ClampedArray>>::into(cmds);
            cmd_v = cmd_a.to_vec();
            cmd_b = cmd_v.as_slice();
        }

        let mut globals = unsafe { GLOBALS.take().unwrap() };
        let surface = &mut globals.surface;
        #[cfg(feature = "dom")]
        let dom = &mut globals.dom;
        #[cfg(feature = "vm")]
        let vm = &mut globals.vm;
        let control = &mut globals.control;

        let mut res = BytesMut::with_capacity(1024);

        let mut listener: [Option<&mut dyn IControl>; 4] = [None, None, None, None];
        #[cfg(feature = "dom")]
        { listener[0] = Some(dom); }
        #[cfg(feature = "vm")]
        { listener[1] = Some(vm); }

        let cmd_str = str::from_utf8(cmd_b).unwrap_or("").split("\n").next().unwrap_or("");
        #[cfg(feature = "vm")]
        let is_vm_cmd = vm::is_command(cmd_str);
        #[cfg(not(feature = "vm"))]
        let is_vm_cmd = false;

        let ret = if is_vm_cmd {
            #[cfg(feature = "vm")]
            vm::command(cmd_str, vm, surface, &mut res);
            Ok(())
        } else {
            control.process(cmd_b, surface, &mut listener, &mut res).1
        };

        unsafe { GLOBALS.replace(globals) };

        if ret.is_ok() {
            return JsValue::from_str(str::from_utf8(&res).unwrap_or(""));
        }else{
            return JsValue::null();
        }
    }
    JsValue::null()
}

#[wasm_bindgen]
pub fn command_mode() -> JsValue {
    if unsafe { GLOBALS.is_none() } {
        log::error!("debug locked");
        return JsValue::null();
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let control = &mut globals.control;

    let mode = match control.mode {
        ControlMode::PKGMode => 1,
        _ => 0
    };

    unsafe { GLOBALS.replace(globals) };

    JsValue::from_f64(mode as f64)
}
