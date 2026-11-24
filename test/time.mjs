#!/usr/bin/env node
// Time interrupt: vm_run takes the clock of JS and returns how long JS should sleep before calling it
// again (> 0 ms, 0 = call again now, -1 = nothing left to run). Drives the built wasm with a fake clock.
//
// Run:  node test/time.mjs   (build pkg/ first: `wasm-pack build --target web --out-dir pkg -- --features dom,vm,compile`)

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const pkgDir = join(dirname(fileURLToPath(import.meta.url)), "..", "pkg");
const mod = await import(new URL("file:///" + join(pkgDir, "nodem_wasm.js").replace(/\\/g, "/")).href);
mod.initSync({ module: readFileSync(join(pkgDir, "nodem_wasm_bg.wasm")) });
await mod.main();

let failed = 0;
function expect(name, got, want) {
	const ok = got === want;
	if (!ok) failed++;
	console.log(`${ok ? "ok  " : "FAIL"} ${name}: ${got}${ok ? "" : ` (want ${want})`}`);
}

function load(source) {
	mod.reset();
	const err = mod.import_program(0, "t", "REM test\n" + source);
	if (err) throw new Error("compile: " + err);
	if (mod.process_commands("vm.prg=0\n") !== "vm.0\r\n") throw new Error("program not loaded");
}

// wait_for: the deadline is kept between calls, the vm does not run before it
load("REQ time\nWHILE 1\n  time.wait_for(0, 100)\nWEND");
expect("wait_for starts the wait", mod.vm_run(1000, 1024), 100);
expect("wait_for still waiting", mod.vm_run(1040, 1024), 60);
expect("wait_for passed, next wait", mod.vm_run(1100, 1024), 100);

// wait_until: keeps its period, a call that is late returns at once and counts on from now
load("REQ time\nWHILE 1\n  OK = time.wait_until(1, 0)\nWEND");
expect("wait_until, first interval counts from the start", mod.vm_run(0, 1024), 1000);
expect("wait_until, period kept after a late call by 20ms", mod.vm_run(1020, 1024), 980);
expect("wait_until, call 1.5 periods late: no catch-up, counts on from now", mod.vm_run(4500, 1024), 1000);

// tick: after HALT the host waits the interval of set_tick
load("REQ time\ntime.set_tick(0, 250)\nWHILE 1\n  HALT\nWEND");
expect("tick after HALT", mod.vm_run(0, 1024), 250);
expect("tick not over", mod.vm_run(100, 1024), 150);
expect("tick over, next HALT", mod.vm_run(250, 1024), 250);

// op budget used up: call again at once
load("WHILE 1\nWEND");
expect("budget used up", mod.vm_run(0, 16), 0);

// a program that ends
load("REQ time\ntime.wait_for(0, 10)\nEND");
expect("wait before END", mod.vm_run(0, 1024), 10);
expect("END", mod.vm_run(10, 1024), -1);
expect("status after END", mod.vm_status(), "stp");
expect("nothing left to run", mod.vm_run(20, 1024), -1);

// negative interval is an error
load("REQ time\ntime.wait_for(-1, 0)");
expect("negative interval", mod.vm_run(0, 1024), -1);
expect("status after error", mod.vm_status(), "err:InterruptError");

process.exit(failed ? 1 : 0);
