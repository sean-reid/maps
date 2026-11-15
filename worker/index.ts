export default {
  fetch(request: Request): Response {
    const { pathname } = new URL(request.url);
    if (pathname === "/api/health") return Response.json({ ok: true });
    return Response.json({ error: "not found" }, { status: 404 });
  },
};
