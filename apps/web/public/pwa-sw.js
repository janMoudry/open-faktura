/* Online-only PWA. The only stored responses are these three public static files.
 * No app shell, API, accounting data, PDF, bearer link or request URL is cached.
 * Hash routes are invisible to a service worker; the same generic screen contains
 * no route-specific data and is the only fallback for failed document navigation.
 */
const CACHE = "open-faktura-offline-cb5b57e18d5da675";
const ASSET_DIGESTS = {
  "/pwa/offline.html":
    "88c21cbfba5b3eb184abd4f8ff0f42c856b5d929f326b30e7b01f4aca32971c6",
  "/pwa/offline.css":
    "c8666bb7eb61d249720cae81f70eb52f088085d09ff4b68730ac1cbc3dafc5d8",
  "/pwa/offline.js":
    "731d6035aed5fc21aecdc3a010a6eff7121e9e1fc28877ad9cec5fcc281b0707",
};
const PREFIX = "open-faktura-offline-";
const FALLBACK = "/pwa/offline.html";
const FILES = [FALLBACK, "/pwa/offline.css", "/pwa/offline.js"];
self.addEventListener("install", (event) => {
  event.waitUntil(
    (async () => {
      const responses = await Promise.all(
        FILES.map(async (path) => {
          const response = await fetch(
            new Request(new URL(path, self.location.origin), {
              credentials: "omit",
              cache: "no-store",
              redirect: "error",
            }),
          );
          const type = response.headers.get("content-type") || "";
          const expected = path.endsWith(".html")
            ? "text/html"
            : path.endsWith(".css")
              ? "text/css"
              : "javascript";
          if (
            !response.ok ||
            !type.includes(expected) ||
            response.type === "opaque"
          )
            throw new Error("Offline assets unavailable");
          const bytes = await response.clone().arrayBuffer();
          const digest = Array.from(
            new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)),
            (b) => b.toString(16).padStart(2, "0"),
          ).join("");
          if (digest !== ASSET_DIGESTS[path])
            throw new Error("Offline asset mismatch");
          return response;
        }),
      );
      const cache = await caches.open(CACHE);
      await Promise.all(
        FILES.map((path, index) => cache.put(path, responses[index])),
      );
      // No skipWaiting: an update must not replace a running invoice editor.
    })(),
  );
});
self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      await Promise.all(
        (await caches.keys())
          .filter((name) => name.startsWith(PREFIX) && name !== CACHE)
          .map((name) => caches.delete(name)),
      );
      await self.clients.claim();
    })(),
  );
});
self.addEventListener("fetch", (event) => {
  const request = event.request;
  const url = new URL(request.url);
  if (
    request.method !== "GET" ||
    url.origin !== self.location.origin ||
    url.search
  )
    return;
  if (FILES.includes(url.pathname)) {
    event.respondWith(
      (async () =>
        (await (await caches.open(CACHE)).match(url.pathname)) ||
        fetch(request))(),
    );
    return;
  }
  // Hash-router entry points only; do not intercept API, PDFs or direct bearer paths.
  if (
    request.mode !== "navigate" ||
    !["/", "/index.html"].includes(url.pathname)
  )
    return;
  event.respondWith(
    (async () => {
      try {
        return await fetch(new Request(request, { cache: "no-store" }));
      } catch {
        const fallback = await (await caches.open(CACHE)).match(FALLBACK);
        if (!fallback) throw new Error("Connection unavailable");
        return new Response(fallback.body, {
          status: 503,
          statusText: "Service Unavailable",
          headers: {
            "Content-Type": "text/html; charset=utf-8",
            "Cache-Control": "no-store",
            "Referrer-Policy": "no-referrer",
          },
        });
      }
    })(),
  );
});
