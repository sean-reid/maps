import { dataKey, isVersion } from "./keys.ts";

interface Env {
  DATA: R2Bucket;
}

const POINTER = "current.json";
const POINTER_TTL_MS = 60_000;
let pointer: { version: string; read: number } | undefined;

/** The data version the site serves; the upload writes this pointer last. */
async function currentVersion(env: Env): Promise<string | undefined> {
  if (pointer && Date.now() - pointer.read < POINTER_TTL_MS) return pointer.version;
  const object = await env.DATA.get(POINTER);
  if (!object) return undefined;
  let version: unknown;
  try {
    ({ version } = await object.json<{ version?: unknown }>());
  } catch {
    return undefined;
  }
  if (typeof version !== "string" || !isVersion(version)) return undefined;
  pointer = { version, read: Date.now() };
  return version;
}

function notFound(): Response {
  return Response.json({ error: "not found" }, { status: 404 });
}

/** Answers from the bucket: headers only for HEAD, 304 when the client's copy is current. */
async function serve(
  request: Request,
  env: Env,
  key: string,
  headers: Record<string, string>,
): Promise<Response | undefined> {
  const object =
    request.method === "HEAD"
      ? await env.DATA.head(key)
      : await env.DATA.get(key, { onlyIf: request.headers });
  if (!object) return undefined;
  const all = { ...headers, etag: object.httpEtag };
  const body = "body" in object ? (object as R2ObjectBody).body : null;
  const status = body || request.method === "HEAD" ? 200 : 304;
  return new Response(body, { status, headers: all });
}

async function index(request: Request, env: Env): Promise<Response> {
  const version = await currentVersion(env);
  const found =
    version &&
    (await serve(request, env, `${version}/index.json`, {
      "content-type": "application/json",
      "cache-control": "public, max-age=300",
      "x-data-version": version,
    }));
  if (found) return found;
  pointer = undefined;
  return Response.json({ error: "no data yet" }, { status: 503 });
}

async function data(request: Request, env: Env, path: string): Promise<Response> {
  const key = dataKey(path);
  const found =
    key &&
    (await serve(request, env, key, {
      "content-type": "application/octet-stream",
      "cache-control": "public, max-age=31536000, immutable",
    }));
  return found || notFound();
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const { pathname } = new URL(request.url);
    if (request.method !== "GET" && request.method !== "HEAD") {
      return Response.json({ error: "method not allowed" }, { status: 405 });
    }
    if (pathname === "/api/health") return Response.json({ ok: true });
    if (pathname === "/api/index") return index(request, env);
    if (pathname.startsWith("/api/data/"))
      return data(request, env, pathname.slice("/api/data/".length));
    return notFound();
  },
};
