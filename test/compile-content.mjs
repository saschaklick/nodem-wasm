#!/usr/bin/env node
// Regression test for ticket 9b7dc7bf: nodem_template_compile must produce CONTENT-DEPENDENT packages.
//
// The bug was a bitmask mix-up in the compile path — generate_pkg was asked for source bit 0 (the
// resident sys.pkg fonts) instead of the imported page (source 2), so every compile returned the same
// bytes regardless of text/geometry. This test drives the built wasm the same way compile.ts does and
// asserts that different markup and different geometry yield different PKG0 packages.
//
// Run:  node test/compile-content.mjs   (build pkg/ first: `wasm-pack build --target web`)

import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const pkgDir = join(dirname(fileURLToPath(import.meta.url)), "..", "pkg");
const mod = await import(new URL("file:///" + join(pkgDir, "nodem_wasm.js").replace(/\\/g, "/")).href);
mod.initSync({ module: readFileSync(join(pkgDir, "nodem_wasm_bg.wasm")) });
await mod.main();

const PAGE_SOURCES = 0b11111110; // must match compile.ts — every imported source, not the system fonts
const md5 = (u8) => createHash("md5").update(Buffer.from(u8)).digest("hex");
const magic = (u8) => new TextDecoder().decode(Uint8Array.from(u8).slice(0, 4));
// Mirror layoutToMarkup: the target canvas is stamped onto the root body so the package is specific
// to both content and geometry.
const page = (t, w, h) => `<body vertical width=${w} height=${h}>${t.map((l) => `<div>${l}</div>`).join("")}</body>`;
function compile(lines, w = 250, h = 122) {
	mod.reset();
	mod.resize(w, h);
	mod.import_page(0, "home", page(lines, w, h));
	return new Uint8Array(mod.generate_pkg("home", PAGE_SOURCES));
}

const cases = {
	apfel: compile(["Apfel Elstar", "1,99"]),
	banane: compile(["Banane Bio", "1,29"]),
	birne: compile(["Birne Williams", "2,49"]),
	menu: compile(["Tageskarte", "Kuerbissuppe", "6,50"], 400, 300),
	apfel_big: compile(["Apfel Elstar", "1,99"], 400, 300), // same content, other geometry
};

const fails = [];
for (const [k, v] of Object.entries(cases)) {
	if (magic(v) !== "PKG0") fails.push(`${k}: not a PKG0 package (magic=${magic(v)})`);
}
const sums = Object.fromEntries(Object.entries(cases).map(([k, v]) => [k, md5(v)]));
const uniq = new Set(Object.values(sums));
// All five differ: three articles, a different template, and the same article in another geometry.
if (uniq.size !== Object.keys(cases).length) fails.push(`expected ${Object.keys(cases).length} distinct packages, got ${uniq.size}: ${JSON.stringify(sums)}`);
// Guard the exact regression: a package must not be independent of content.
if (sums.apfel === sums.banane) fails.push("apfel and banane produced identical packages (content-independent regression)");
if (sums.apfel === sums.apfel_big) fails.push("same content at 250x122 and 400x300 produced identical packages (geometry ignored)");

if (fails.length) {
	console.error("FAIL compile-content:\n - " + fails.join("\n - "));
	process.exit(1);
}
console.log("ok compile-content: 5 distinct PKG0 packages", JSON.stringify(sums, null, 0));
