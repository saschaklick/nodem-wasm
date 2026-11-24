//! The virtual machine: the program API (interrupt tables), compiling and removing programs,
//! and the `vm.*` commands of `process_commands`.

use bytes::BytesMut;
use nostd::{ format, string::String };
use nodem_rs::{ int_surface, surface::Surface };
use virtmach::{ RuntimeError, Runtime, VMAtom, VirtMach, interrupt_set, interrupts::* };
use js_sys::Date;
use wasm_bindgen::prelude::*;

use crate::GLOBALS;

// The interrupts of the vm, written once for the tables a program is compiled and documented
// with (`interrupts!(tables)`) and for the list the vm runs with (`interrupts!(let list, <time>, <surface>)`,
// the references follow in the order of the `_` items). Each built-in interrupt is at its virtmach
// INDEX, slots without one run a dummy. With the `int_nodem` feature the custom "nodem" is the first
// number after the built-ins; only the device implements it, here it is a stub.
#[cfg(not(feature = "int_nodem"))]
interrupt_set!(interrupts = [math, proc, string, time => _, surface => _, periph]);

#[cfg(feature = "int_nodem")]
interrupt_set!(interrupts = [
    math, proc, string, time => _, surface => _, periph,
    ext("nodem", virtmach::interrupts::BUILTIN_SLOTS, &mut dummy::Interrupt {}, &crate::nodem::FUNCTIONS)
]);

/// The names in the signature at the end of a help text, `... (a,b)->(res)`: the arguments and the
/// returns, None if there is none.
fn signature(help: &str) -> Option<(&str, &str)> {
    let (head, rets) = help.trim_end().rsplit_once("->(")?;
    let rets = rets.strip_suffix(')')?;
    let head = head.strip_suffix(')')?;
    Some((&head[head.rfind('(')? + 1..], rets))
}

/// Appends a JSON array of `count` names: the comma separated ones of the signature, or
/// `<fallback>0`, `<fallback>1`... if there are not exactly that many.
fn push_names(out: &mut String, names: Option<&str>, count: usize, fallback: &str) {
    let names = names.filter(|n| n.split(',').filter(|n| !n.trim().is_empty()).count() == count);
    out.push('[');
    for i in 0..count {
        if i > 0 { out.push(','); }
        match names.and_then(|n| n.split(',').nth(i)) {
            Some(name) => out.push_str(&format!("\"{}\"", name.trim().replace('"', "'"))),
            None => out.push_str(&format!("\"{}{}\"", fallback, i))
        }
    }
    out.push(']');
}

/// Every function a program can call, as JSON, `_i` being the number of the interrupt or function:
///
/// ```json
/// { "math": { "_i": 0, "and": { "_i": 0, "args": ["num0", "num1"], "rets": ["res"], "help": "..." }, ... },
///   "proc": { ... } }
/// ```
#[wasm_bindgen]
pub fn program_reference() -> JsValue {
    let mut out = String::from("{");
    for (i, table) in interrupts!(tables).iter().enumerate() {
        if i > 0 { out.push(','); }
        out.push_str(&format!("\"{}\":{{\"_i\":{}", table.name, table.number));
        for (j, f) in table.functions.iter().enumerate() {
            // a name used twice is the last one for the compiler, so it is for the reference
            if table.functions[j + 1..].iter().any(|g| g.name == f.name) { continue; }
            let (args, rets) = signature(f.help).unzip();
            out.push_str(&format!(",\"{}\":{{\"_i\":{},\"args\":", f.name, f.no));
            push_names(&mut out, args, f.arguments, "arg");
            out.push_str(",\"rets\":");
            push_names(&mut out, rets, f.returns, "ret");
            out.push_str(&format!(",\"help\":\"{}\"}}", f.help.replace('"', "'")));
        }
        out.push('}');
    }
    out.push('}');
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

    let ret = match VirtMach::compile_tables(name, content, &interrupts!(tables)) {
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

/// Time of the vm. The wasm side has no clock to sleep on: JS passes the time in with every
/// `vm_run` and waits as long as `vm_run` returns. Waits only store a deadline here and pause
/// the vm, `vm_run` does not run it again before the deadline. All values are milliseconds on
/// the clock JS passes in (`performance.now()`).
#[derive(Default)]
pub struct TimeState {
    /// the clock at the start of the current `vm_run`
    now: f64,
    /// when the last wait_until returned, the first one counts from the start of the program
    last: f64,
    /// the vm must not run before this, set by wait_for, wait_until and the tick
    deadline: Option<f64>,
    /// what the program set with set_tick, waited after HLT
    tick: f64
}

impl TimeState {
    /// A new program starts now: no deadline, no tick, wait_until counts from here
    fn start(&mut self, now: f64) {
        *self = TimeState { now, last: now, deadline: None, tick: 0.0 };
    }

    /// ms until the deadline, clears it once it has passed
    fn remaining(&mut self) -> Option<f64> {
        match self.deadline {
            Some(deadline) if deadline > self.now => Some(deadline - self.now),
            _ => { self.deadline = None; None }
        }
    }
}

/// seconds * 1000 + milliseconds, None if negative
fn interval(seconds: VMAtom, milliseconds: VMAtom) -> Option<f64> {
    if seconds < 0 || milliseconds < 0 { return None; }
    Some(seconds as f64 * 1000.0 + milliseconds as f64)
}

/// The time interrupt (slot `time::INDEX`) on the clock `vm_run` passes in. get_time and get_date
/// read the local time of the browser, year needs at least 16 bit atoms.
pub struct Time<'a> {
    pub state: &'a mut TimeState
}

impl SoftInterrupt for Time<'_> {
    fn name(&self) -> &str {
        time::NAME
    }

    fn functions(&self) -> &'static [SoftInterruptFunction<'static>] where Self: Sized {
        &time::FUNCTIONS
    }

    fn call(&mut self, vm: &mut VirtMach) {
        let op = vm.stack_pop();
        let state = &mut *self.state;
        match op {
            // wait_for(seconds, milliseconds): the interval counts from now
            0 => match interval(vm.stack_pop(), vm.stack_pop()) {
                Some(ms) => { state.deadline = Some(state.now + ms); vm.pause(); }
                None => { vm.error = RuntimeError::InterruptError; }
            }
            // wait_until(seconds, milliseconds): the interval counts from the last wait_until,
            // if that is over already return at once and count the next one from now
            1 => match interval(vm.stack_pop(), vm.stack_pop()) {
                Some(ms) => {
                    let target = state.last + ms;
                    let waited = target > state.now;
                    state.last = if waited { target } else { state.now };
                    vm.stack_push(waited as VMAtom);
                    if waited { state.deadline = Some(target); vm.pause(); }
                }
                None => { vm.stack_pop(); vm.stack_push(0); vm.error = RuntimeError::InterruptError; }
            }
            // set_tick(seconds, milliseconds)
            3 => match interval(vm.stack_pop(), vm.stack_pop()) {
                Some(ms) => { state.tick = ms; }
                None => { vm.error = RuntimeError::InterruptError; }
            }
            // pushed as hours, minutes, seconds, ms
            10 => {
                let now = Date::new_0();
                for v in [now.get_hours(), now.get_minutes(), now.get_seconds(), now.get_milliseconds()] { vm.stack_push(v as VMAtom); }
            }
            // pushed as year, month (1-12), day
            11 => {
                let now = Date::new_0();
                for v in [now.get_full_year(), now.get_month() + 1, now.get_date()] { vm.stack_push(v as VMAtom); }
            }
            _ => { vm.error = RuntimeError::UnimplementedInterruptFunc; }
        }
    }
}

/// Runs the vm for JS, see `process_commands` for the debugger commands. `now_ms` is the clock
/// of JS (`performance.now()`), the vm does not run before a pending wait has reached its
/// deadline. Returns when JS should call again, in ms:
///
/// * `> 0`: sleep that long, a wait_for/wait_until deadline, or the tick after HLT
/// * `0`: call again right away, the op budget `max_ops` (0 = unlimited) was used up
/// * `-1`: nothing to run, the program ended, failed (see `vm_status`) or none is loaded
///
/// The clock is read once per call, so a wait is measured from the start of the call it was made
/// in. A wait or tick pauses the vm like HLT, so BRK cannot be told apart from HLT here.
#[wasm_bindgen]
pub fn vm_run(now_ms: f64, max_ops: u32) -> f64 {
    if unsafe { GLOBALS.is_none() } {
        log::error!("vm_run locked");
        return -1.0;
    }

    let mut globals = unsafe { GLOBALS.take().unwrap() };
    let vm = &mut globals.vm;
    let time = &mut globals.time;
    let surface = &mut globals.surface;

    // a program starts at cycle 0, whatever the previous one was waiting for is over
    if vm.cycle_cnt == 0 { time.start(now_ms); }
    time.now = now_ms;
    let delay = if matches!(vm.state, Runtime::Stp | Runtime::Err | Runtime::Ini) {
        -1.0
    } else if let Some(remaining) = time.remaining() {
        remaining
    } else {
        let (mut time_int, mut surface_int) = (Time { state: time }, int_surface::IntSurface { surface });
        interrupts!(let list, &mut time_int, &mut surface_int);
        vm.run(max_ops as usize, list);
        match vm.state {
            Runtime::Run => 0.0,
            Runtime::Hlt => match time.remaining() {
                // paused by a wait_for/wait_until
                Some(remaining) => remaining,
                // HLT: the program sleeps the tick before its next round
                None if time.tick > 0.0 => { time.deadline = Some(now_ms + time.tick); time.tick }
                None => 0.0
            }
            _ => -1.0
        }
    };

    unsafe { GLOBALS.replace(globals) };

    delay
}

/// State of the vm as `ini`, `run`, `hlt`, `stp` or `err:<error>`, to tell why `vm_run` returned -1.
#[wasm_bindgen]
pub fn vm_status() -> String {
    if unsafe { GLOBALS.is_none() } {
        return String::from("locked");
    }

    let globals = unsafe { GLOBALS.take().unwrap() };
    let vm = &globals.vm;
    let status = match vm.state {
        Runtime::Ini => String::from("ini"),
        Runtime::Run => String::from("run"),
        Runtime::Hlt => String::from("hlt"),
        Runtime::Stp => String::from("stp"),
        Runtime::Err => format!("err:{:?}", vm.error)
    };

    unsafe { GLOBALS.replace(globals) };

    status
}

/// Whether `cmd` (first line of a `process_commands` input) is a `vm.*` command handled here.
pub(crate) fn is_command(cmd: &str) -> bool {
    matches!(cmd, "vm.stp" | "vm.run" | "vm.fnc" | "vm.dis")
}

/// Runs a `vm.*` command, see `is_command`; output goes to `res`. `vm.stp` and `vm.run` are the
/// debugger: they do not sleep, a wait just pauses the vm and the next command goes on.
pub(crate) fn command(cmd: &str, vm: &mut VirtMach<'_>, surface: &mut Surface, time: &mut TimeState, res: &mut BytesMut) {
    match cmd {
        "vm.fnc" => {
            let _ = vm.functions(interrupts!(tables).map(|t| (t.name, t.functions)).to_vec(), res);
        }
        "vm.stp" | "vm.run" => {
            time.deadline = None;
            let (mut time_int, mut surface_int) = (Time { state: time }, int_surface::IntSurface { surface });
            interrupts!(let list, &mut time_int, &mut surface_int);
            vm.run(match cmd { "vm.stp" => 1, _ => 1024 }, list);
        }
        "vm.dis" => {
            let _ = vm.disassemble(res);
        }
        _ => {}
    }
}
