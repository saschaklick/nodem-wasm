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

Programs address interrupts by number. virtmach's built-ins have fixed numbers (their `INDEX`:
math 0, proc 1, string 2, random 3, time 4, trig 5, surface 6, gpio 7, uart 8, i2c 9, pwm 10); custom
interrupts follow in the order given to the compiler. The device API `nodem` (`NODEM_FUNCTIONS`) is the
first custom one, number 11. A runtime must place each interrupt in that slot — `process_commands` does
this here, with `nodem` as a stub; the device firmware (`firmware/nodem-vm`) implements it for real.
