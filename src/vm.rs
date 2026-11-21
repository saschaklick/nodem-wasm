//! The virtual machine: the program API (interrupt tables), compiling and removing programs,
//! and the `vm.*` commands of `process_commands`.

use bytes::BytesMut;
use nostd::{ format, string::String };
use nodem_rs::{ int_surface, surface::Surface };
use virtmach::{ VirtMach, interrupts::{ self, * } };
use wasm_bindgen::prelude::*;

use crate::GLOBALS;

/// The device API of user programs: interrupt "nodem", the first custom interrupt after virtmach's
/// built-in slots (number interrupts::BUILTIN_SLOTS, i.e. 10).
/// Implemented by the device firmware (nodem-ai-editor: firmware/nodem-vm/src/lib.rs) — keep numbers in sync.
pub static NODEM_FUNCTIONS: [interrupts::SoftInterruptFunction; 27] = [
    interrupts::SoftInterruptFunction { no: 0, name: "wait",        arguments: 1, returns: 0, help: "Pause the program for ms milliseconds, 0 = until the next frame (ms)->()" },
    interrupts::SoftInterruptFunction { no: 1, name: "button",      arguments: 0, returns: 1, help: "Next button press: 0 none, 1-4 short press of button n, 11-14 long press ()->(key)" },
    interrupts::SoftInterruptFunction { no: 2, name: "page",        arguments: 0, returns: 2, help: "Image slot shown now and number of slots ()->(active,count)" },
    interrupts::SoftInterruptFunction { no: 3, name: "set_page",    arguments: 1, returns: 0, help: "Show image slot n (n)->()" },
    interrupts::SoftInterruptFunction { no: 4, name: "led",         arguments: 2, returns: 0, help: "Switch LED n (1-4) on (1) or off (0) (n,on)->()" },
    interrupts::SoftInterruptFunction { no: 5, name: "set_text",    arguments: 2, returns: 0, help: "Show a number as the text of page node n (node,value)->()" },
    interrupts::SoftInterruptFunction { no: 6, name: "report",      arguments: 2, returns: 0, help: "Send a reading to the server as sensor vm<id> (id,value)->()" },
    interrupts::SoftInterruptFunction { no: 7, name: "uptime",      arguments: 0, returns: 1, help: "Seconds since the device started, wraps at 32767 ()->(s)" },
    interrupts::SoftInterruptFunction { no: 8, name: "gps",         arguments: 0, returns: 2, help: "GPS satellites tracked and whether there is a fix ()->(sats,fix)" },
    interrupts::SoftInterruptFunction { no: 9, name: "set_visible", arguments: 2, returns: 0, help: "Show (1) or hide (0) page node n (node,on)->()" },
    interrupts::SoftInterruptFunction { no: 10, name: "status",     arguments: 1, returns: 1, help: "Device status by id, 0 if the board does not know it (id)->(value)" },
    interrupts::SoftInterruptFunction { no: 11, name: "has",        arguments: 1, returns: 1, help: "Whether the board knows status id: 1 or 0 (id)->(known)" },
    interrupts::SoftInterruptFunction { no: 12, name: "value",      arguments: 1, returns: 1, help: "Number k the server set (k)->(value)" },
    interrupts::SoftInterruptFunction { no: 13, name: "str_clear",  arguments: 1, returns: 0, help: "Empty text slot s (s)->()" },
    interrupts::SoftInterruptFunction { no: 14, name: "str_add",    arguments: 2, returns: 0, help: "Append text slot t to text slot s (s,t)->()" },
    interrupts::SoftInterruptFunction { no: 15, name: "str_num",    arguments: 2, returns: 0, help: "Append a number to text slot s (s,value)->()" },
    interrupts::SoftInterruptFunction { no: 16, name: "str_char",   arguments: 2, returns: 0, help: "Append the character with code c to text slot s, 32 = space (s,c)->()" },
    interrupts::SoftInterruptFunction { no: 17, name: "str_len",    arguments: 1, returns: 1, help: "Characters in text slot s (s)->(len)" },
    interrupts::SoftInterruptFunction { no: 18, name: "str_to_num", arguments: 1, returns: 1, help: "The number text slot s starts with, 0 if none (s)->(value)" },
    interrupts::SoftInterruptFunction { no: 19, name: "str_eq",     arguments: 2, returns: 1, help: "Whether text slots a and b are equal: 1 or 0 (a,b)->(equal)" },
    interrupts::SoftInterruptFunction { no: 20, name: "show",       arguments: 2, returns: 0, help: "Show text slot s as the text of page node n (node,s)->()" },
    interrupts::SoftInterruptFunction { no: 21, name: "send",       arguments: 2, returns: 0, help: "Send text slot s to the server as text output id (id,s)->()" },
    interrupts::SoftInterruptFunction { no: 22, name: "str_new",    arguments: 1, returns: 1, help: "1 once after the server changed text slot s, else 0 (s)->(changed)" },
    interrupts::SoftInterruptFunction { no: 23, name: "layout",        arguments: 1, returns: 1, help: "Switch to text layout n (a #layout block): its slots start empty, the base stays; 1 if switched (n)->(ok)" },
    interrupts::SoftInterruptFunction { no: 24, name: "layout_active", arguments: 0, returns: 1, help: "The text layout in use, -1 = none ()->(n)" },
    interrupts::SoftInterruptFunction { no: 25, name: "text_used",     arguments: 0, returns: 1, help: "Bytes the current text slots reserve ()->(bytes)" },
    interrupts::SoftInterruptFunction { no: 26, name: "text_budget",   arguments: 0, returns: 1, help: "Bytes of text memory this board allows ()->(bytes)" },
];

/// Modules a program can `#req`. Built-ins get their fixed virtmach INDEX (see
/// interrupts::interrupt_numbers), custom ones (nodem) follow.
fn program_modules() -> [(&'static str, &'static [interrupts::SoftInterruptFunction<'static>]); 7] {
    [
        (math::NAME, math::FUNCTIONS.as_slice()),
        (proc::NAME, proc::FUNCTIONS.as_slice()),
        (string::NAME, string::FUNCTIONS.as_slice()),
        (time::NAME, time::FUNCTIONS.as_slice()),
        (interrupts::surface::NAME, interrupts::surface::FUNCTIONS.as_slice()),
        (periph::NAME, periph::FUNCTIONS.as_slice()),
        ("nodem", NODEM_FUNCTIONS.as_slice()),
    ]
}

/// Every function a program can call, as JSON: [{module, name, no, arguments, returns, help}].
#[wasm_bindgen]
pub fn program_reference() -> JsValue {
    let mut out = String::from("[");
    for (module, functions) in program_modules() {
        for f in functions {
            if out.len() > 1 { out.push(','); }
            out.push_str(&format!(
                "{{\"module\":\"{}\",\"name\":\"{}\",\"no\":{},\"arguments\":{},\"returns\":{},\"help\":\"{}\"}}",
                module, f.name, f.no, f.arguments, f.returns, f.help.replace('"', "'")
            ));
        }
    }
    out.push(']');
    JsValue::from_str(&out)
}

#[wasm_bindgen]
pub fn import_program(index: u8, name: &str, content: &str) -> JsValue {
    if unsafe { GLOBALS.is_none() } {
        log::error!("import_program locked");
        return JsValue::from_str("locked");
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let surface = &mut globals.surface;

    let ret = match VirtMach::compile(name, content, program_modules().to_vec()) {
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

/// Whether `cmd` (first line of a `process_commands` input) is a `vm.*` command handled here.
pub(crate) fn is_command(cmd: &str) -> bool {
    matches!(cmd, "vm.stp" | "vm.run" | "vm.fnc" | "vm.dis")
}

/// Runs a `vm.*` command, see `is_command`; output goes to `res`.
pub(crate) fn command(cmd: &str, vm: &mut VirtMach<'_>, surface: &mut Surface, res: &mut BytesMut) {
    match cmd {
        "vm.stp" | "vm.run" | "vm.fnc" => {
            // Slot = interrupt number: built-ins at their virtmach INDEX (unused ones dummy), then
            // the custom "nodem" (BUILTIN_SLOTS), which only the device implements — here it is a stub.
            let mut interrupts: [&mut dyn SoftInterrupt; (interrupts::BUILTIN_SLOTS + 1) as usize] = [
                &mut math::Interrupt {},
                &mut proc::Interrupt {},
                &mut string::Interrupt {},
                &mut time::Interrupt {},
                &mut dummy::Interrupt {},
                &mut dummy::Interrupt {},
                &mut dummy::Interrupt {},
                &mut dummy::Interrupt {},
                &mut periph::Interrupt {},
                &mut int_surface::IntSurface { surface : surface },
                &mut dummy::Interrupt {}
            ];

            match cmd {
                #[cfg(feature = "compile")]
                "vm.fnc" => {
                    let _ = vm.functions(interrupts.map(|a| (a.name(), a.functions()) ).to_vec(), res);
                }
                "vm.stp" | "vm.run" => {
                    vm.run(match cmd { "vm.stp" => 1, _ => 1024 }, &mut interrupts);
                }
                _ => {}
            }
        }
        "vm.dis" => {
            let _ = vm.disassemble(res);
        }
        _ => {}
    }
}
