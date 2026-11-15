import { dataKey } from "./keys.ts";

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
  const { version } = await object.json<{ version?: unknown }>();
  if (typeof version !== "string" || !dataKey(`${version}/index.json`)) return undefined;
  pointer = { version, read: Date.now() };
  return version;
}

function notFound(): Response {
  return Response.json({ error: "not found" }, { status: 404 });
}

async function index(env: Env): Promise<Response> {
  const version = await currentVersion(env);
  const object = version ? await env.DATA.get(`${version}/index.json`) : null;
  if (!version || !object) return Response.json({ error: "no data yet" }, { status: 503 });
  return new Response(object.body, {
    headers: {
      "content-type": "application/json",
      "cache-control": "public, max-age=300",
      "x-data-version": version,
      etag: object.httpEtag,
    },
  });
}

async function data(env: Env, path: string): Promise<Response> {
  const key = dataKey(path);
  const object = key ? await env.DATA.get(key) : null;
  if (!object) return notFound();
  return new Response(object.body, {
    headers: {
      "content-type": "application/octet-stream",
      "cache-control": "public, max-age=31536000, immutable",
      etag: object.httpEtag,
    },
  });
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const { pathname } = new URL(request.url);
    if (request.method !== "GET" && request.method !== "HEAD") {
      return Response.json({ error: "method not allowed" }, { status: 405 });
    }
    if (pathname === "/api/health") return Response.json({ ok: true });
    if (pathname === "/api/index") return index(env);
    if (pathname.startsWith("/api/data/")) return data(env, pathname.slice("/api/data/".length));
    return notFound();
  },
};
