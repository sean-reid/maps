const VERSION = /^v\d{4}-\d{2}-\d{2}(-\d+){0,2}$/;
const SLUG = /^[a-z0-9]+(-[a-z0-9]+)*$/;

export function isVersion(version: string): boolean {
  return VERSION.test(version);
}

/** The bucket key for a path under /api/data/, or undefined when the path is not one the pipeline writes. */
export function dataKey(path: string): string | undefined {
  const parts = path.split("/");
  const [version = "", kind, file = ""] = parts;
  if (!isVersion(version)) return undefined;
  if (parts.length === 2 && kind === "index.json") return path;
  if (parts.length !== 3 || kind !== "series" || !file.endsWith(".bin")) return undefined;
  return SLUG.test(file.slice(0, -4)) ? path : undefined;
}
