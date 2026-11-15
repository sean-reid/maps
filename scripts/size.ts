import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { gzipSync } from "node:zlib";

const BUDGET = 60 * 1024;
const dist = resolve(import.meta.dirname, "../dist");
const html = readFileSync(resolve(dist, "index.html"), "utf8");
const scripts = [...html.matchAll(/<script[^>]+src="([^"]+\.js)"/g)].map((m) => m[1] ?? "");
let total = 0;
for (const src of scripts) total += gzipSync(readFileSync(resolve(dist, `.${src}`))).length;
console.log(`first-load JS: ${total} bytes gzipped, budget ${BUDGET}`);
if (scripts.length === 0 || total > BUDGET) process.exit(1);
