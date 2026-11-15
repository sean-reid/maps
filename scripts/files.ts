import { readdirSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";

/** Every file under `dir` as a forward-slash path relative to it. */
export function listFiles(dir: string): string[] {
  const out: string[] = [];
  const walk = (at: string): void => {
    for (const name of readdirSync(at).sort()) {
      const path = join(at, name);
      if (statSync(path).isDirectory()) walk(path);
      else out.push(relative(dir, path).split(sep).join("/"));
    }
  };
  walk(dir);
  return out;
}

export function contentType(path: string): string {
  return path.endsWith(".json") ? "application/json" : "application/octet-stream";
}
