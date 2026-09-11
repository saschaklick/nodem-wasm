#![no_std]
#![allow(static_mut_refs)]

extern crate wee_alloc;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use js_sys::JsString;
use js_sys::Uint8ClampedArray;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures;
use bytes::{ BytesMut };
use core::{ slice };
use nostd::{ format, string::String };
use bitmap_writer::{ Bitmap, Writer, Style, Frame };

use nodem_rs::*;
use nodem_rs::surface::*;
use nodem_rs::control::*;
#[cfg(feature = "dom")]
use nodem_rs::dom::*;
#[cfg(feature = "vm")]
#[warn(unused_imports)]
use virtmach::{ VirtMach, interrupts::{ self, * } };

static mut LOOP_CNT: u32 = 0;

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
    int_buffer: BytesMut
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
        int_buffer: BytesMut::zeroed(1)        
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
    globals.int_buffer = BytesMut::zeroed(1);
    

    unsafe { GLOBALS.replace(globals) }; 
}

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

#[cfg(feature = "dom")]
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

#[cfg(not(feature = "dom"))]
#[wasm_bindgen]
pub fn load_xml(_data: &str) {
    log::error!("dom feature not found");
}

#[cfg(feature = "dom")]
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

#[cfg(not(feature = "dom"))]
#[wasm_bindgen]
pub fn get_xml() -> JsValue {
    return JsValue::null()
}

#[wasm_bindgen]
pub fn load_pkg(data: &[u8], source: Option<u8>) {
    if unsafe { GLOBALS.is_none() } {
        log::error!("load_media locked");
        return;
    }    
    
    let mut globals = unsafe { GLOBALS.take().unwrap() };    
    let surface = &mut globals.surface;
    let media = &mut globals.int_buffer;

    media.resize(data.len(), 0);
    log::debug!("pkg buffer resized: {}b", media.len());

    media.copy_from_slice(data);        

    surface.media.load_pkg(media.as_ptr() as *const u8, media.len(), source.unwrap_or(1));    
    
    unsafe { GLOBALS.replace(globals) }; 
}

#[wasm_bindgen]
pub fn generate_pkg(sources: Option<u8>) -> Uint8ClampedArray {
    if unsafe { GLOBALS.is_none() } {
        log::error!("generate_pkg locked");
        return Uint8ClampedArray::new(&JsValue::null());
    }
    
    let mut globals = unsafe { GLOBALS.take().unwrap() };        
    let surface = &mut globals.surface;    

    let buf = surface.media.generate_pkg(sources.unwrap_or(0b11111110), true);    
    
    unsafe { GLOBALS.replace(globals) }; 

    return Uint8ClampedArray::from(&buf[..]);
}

#[wasm_bindgen]
pub fn import_image(index: u8, name: &str, rgba: &[u8], width: u8, height: u8, alpha: Option<bool>, threshold_0: Option<u8>, threshold_1: Option<u8>) {    
    if unsafe { GLOBALS.is_none() } {
        log::error!("import_image locked");
        return;
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };    
    let surface = &mut globals.surface;
                
    surface.media.import_image(index, name, rgba, width, height, alpha.unwrap_or(true), threshold_0.unwrap_or(128), threshold_1.unwrap_or(128));        

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
    
    surface.media.import_border(index, name, rgba, width, height);

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

#[wasm_bindgen]
pub fn import_program(index: u8, name: &str, content: &str) -> JsValue {    
    if unsafe { GLOBALS.is_none() } {
        log::error!("import_program locked");
        return JsValue::from_str("locked");
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };    
    let surface = &mut globals.surface;
    
    let ret = match VirtMach::compile(name, content, [
        (proc::NAME, proc::FUNCTIONS.as_slice()),
        (math::NAME, math::FUNCTIONS.as_slice()),
        (random::NAME, random::FUNCTIONS.as_slice()),
        (interrupts::surface::NAME, interrupts::surface::FUNCTIONS.as_slice())
    ].as_ref().to_vec()) {
        Ok(res) => {            
            surface.media.import_program(index, name, &res.0.data);
            JsValue::null()
        }
        Err(err) => {            
            JsValue::from_str(format!("{:?}", err).as_str())            
        }
    };    

    unsafe { GLOBALS.replace(globals) };

    return ret;
}

#[wasm_bindgen]
pub fn remove_program(index: u8) {    
    if unsafe { GLOBALS.is_none() } {
        log::error!("remove_program locked");
        return;
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };    
    let surface = &mut globals.surface;
    
    surface.media.remove_program(index);

    unsafe { GLOBALS.replace(globals) };
}

#[wasm_bindgen]
pub fn resize(width: usize, height: usize) {
    if unsafe { GLOBALS.is_none() } {
        log::error!("render locked");        
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };    

    let framebuffer = &mut globals.scr_buffer;    
    let surface = &mut globals.surface;
    
    let fb_req_size = (width * height + 7) / 8;        
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
    
    let fb_req_size = (width * height + 7) / 8;
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
    
    for i in 0 .. width * height {
        if framebuffer.len() < i / 8 {
            log::error!("framebuffer too small {} < {}", framebuffer.len(), i / 8);
            break;
        }
        if mem.len() < i * 4 + 4 {
            log::error!("mem too small {} < {}", mem.len(), i * 4 + 4);
            break;
        }
        let pixel = (framebuffer[i / 8] >> (7 - (i % 8)) & 0b1) as usize;
        mem[i * 4 .. i * 4 + 4].copy_from_slice(&palette[pixel * 4..pixel * 4 + 4]);
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
    
    let fb_req_size = (width * height + 7) / 8;
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
        let ret = match cmd_str {
            "vm.stp" | "vm.run" | "vm.fnc" => {
                let mut interrupts: [&mut dyn SoftInterrupt;4] = [
                    &mut proc::Interrupt {},
                    &mut math::Interrupt {},
                    &mut random::Interrupt {},
                    &mut int_surface::IntSurface { surface : surface }
                ];

                match cmd_str {
                    #[cfg(feature = "compile")]
                    "vm.fnc" => {                        
                        let _ = vm.functions(interrupts.map(|a| (a.name(), a.functions()) ).to_vec(), &mut res);                        
                    }
                    "vm.stp" | "vm.run" => {
                        vm.run(match cmd_str { "vm.stp" => 1, _ => 1024 }, &mut interrupts);                
                    }                    
                    _ => {}
                }
                Ok(())
            }            
            "vm.dis" => {
                let _ = vm.disassemble(&mut res);
                Ok(())
            }
            _ => control.process(cmd_b, surface, &mut listener, &mut res).1
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