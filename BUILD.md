# nodem-wasm — build and wiring

A `wasm-bindgen` library (`crate-type = cdylib`) wrapping `nodem-rs` (the 1-bit renderer) and, via the
`vm` feature, `virtmach-rs`. It exposes what the editor and the device simulator call — `resize`,
`render_to_string`, `process_commands`, `load_pkg`/`generate_pkg`, `load_page_to_dom`, `import_*`,
`program_reference`, etc. (see `src/lib.rs`).

## Build

```
# one-time
curl https://sh.rustup.rs -sSf | sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack

# from this directory
wasm-pack build --target web --out-dir pkg -- --features dom,vm,compile
```

Add `int_nodem` to the features to include the device API interrupt `nodem` (off by default).

`nodem-rs` and `virtmach-rs` are git dependencies, so the build needs network access.

## Test

```
node test/compile-content.mjs
```

Checks that `generate_pkg` produces content- and geometry-dependent packages (needs `pkg/` built first).

## Wiring into nodem-ai-editor

The editor (`web/src/editor`) and the API worker load the `--target web` output. From the
nodem-ai-editor repo root, with this repo built:

```
cp <nodem-wasm>/pkg/nodem_wasm*.{js,ts,wasm} web/src/editor/wasm/
cp <nodem-wasm>/pkg/nodem_wasm_bg.wasm web/workers/api/nodem/
```

`generate_pkg(name, sources)` takes the package name as its first argument.

## VM interrupt numbers

Programs address interrupts by number. The interrupts are written once, in `src/vm.rs`, with virtmach's
`interrupt_set!`: `interrupts!(tables)` gives the tables programs are compiled and documented with
(`compile_tables`, `program_reference`, `vm.fnc`), `interrupts!(let list, ..)` the list the vm runs with.
Built-ins sit at the fixed `INDEX` of their module (math 0, proc 1, string 2, time 3, periph 8, surface 9),
slots without an interrupt run a dummy. The device API `nodem` (`src/nodem.rs`, feature `int_nodem`) is the first
custom one, number `BUILTIN_SLOTS` (10). A runtime must place each interrupt in that slot; here `nodem` is a stub, the
device firmware (`firmware/nodem-vm`) implements it for real. To add an interrupt, add it to that one list.

## Running a program (time)

The wasm side has no clock and does not sleep. JS owns the loop: it passes its clock in and waits as
long as `vm_run` says. `time.wait_for`, `time.wait_until` and the tick of `time.set_tick` (waited after
`HALT`) only store a deadline and pause the vm; `vm_run` does not run it before the deadline.

```js
// vm_run(now_ms, max_ops) -> ms until the next call: > 0 sleep, 0 call again now, -1 nothing left to run
class VmRunner {
  #timer = null;
  start() { this.kick(); }
  stop() { clearTimeout(this.#timer); }
  kick() {                        // also call it when something outside changed (button, server value)
    clearTimeout(this.#timer);
    const delay = wasm.vm_run(performance.now(), 1024);
    if (delay >= 0) this.#timer = setTimeout(() => this.kick(), delay);
    // delay < 0: ended or failed, wasm.vm_status() says which ("stp", "err:<error>", ...)
  }
}
```

Load the program first with `process_commands("vm.prg=<index>\n")`. `vm.run` / `vm.stp` of
`process_commands` are the debugger: they do not sleep. Timers fire late in background tabs (about 1 s),
so run the loop in a worker or pause on `visibilitychange` if that matters. `get_time` / `get_date`
return the local time of the browser.

```
node test/time.mjs
```
