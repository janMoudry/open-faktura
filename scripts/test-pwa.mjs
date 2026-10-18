import { webcrypto } from "node:crypto";
import { readFile } from "node:fs/promises";
import { createContext, runInContext } from "node:vm";
import test from "node:test";
import assert from "node:assert/strict";
const source = await readFile("apps/web/public/pwa-sw.js", "utf8");
const assets = Object.fromEntries(
  await Promise.all(
    ["offline.html", "offline.css", "offline.js"].map(async (name) => [
      "/pwa/" + name,
      await readFile("apps/web/public/pwa/" + name, "utf8"),
    ]),
  ),
);
function worker(wrongAsset = false) {
  const handlers = {},
    stored = new Map(),
    puts = [],
    fetches = [],
    deleted = [];
  class Request {
    constructor(input, options = {}) {
      Object.assign(
        this,
        typeof input === "object" && !(input instanceof URL)
          ? input
          : { url: String(input), method: "GET", mode: "cors" },
        options,
      );
    }
  }
  let offline = false;
  const cache = {
    put: async (path, value) => {
      puts.push(path);
      stored.set(path, value.clone());
    },
    match: async (path) => stored.get(path)?.clone(),
  };
  const caches = {
    open: async () => cache,
    keys: async () => ["other-app", "open-faktura-offline-old"],
    delete: async (name) => {
      deleted.push(name);
    },
  };
  const fetch = async (request) => {
    fetches.push(request);
    if (offline) throw new Error("offline");
    const type = request.url.endsWith(".css")
      ? "text/css"
      : request.url.endsWith(".js")
        ? "text/javascript"
        : "text/html";
    return new Response(
      wrongAsset
        ? "unexpected app shell"
        : assets[new URL(request.url).pathname] || "online",
      { headers: { "Content-Type": type } },
    );
  };
  runInContext(
    source,
    createContext({
      self: {
        location: { origin: "https://app.example.test" },
        clients: { claim: async () => {} },
        addEventListener: (name, fn) => {
          handlers[name] = fn;
        },
      },
      crypto: webcrypto,
      Request,
      Response,
      URL,
      caches,
      fetch,
    }),
  );
  return {
    puts,
    fetches,
    deleted,
    stored,
    offline: () => {
      offline = true;
    },
    dispatch: async (name, request) => {
      let result;
      handlers[name]({
        request,
        waitUntil: (p) => {
          result = p;
        },
        respondWith: (p) => {
          result = p;
        },
      });
      return result;
    },
  };
}
test("PWA caches exactly public fallback assets without credentials", async () => {
  const w = worker();
  await w.dispatch("install");
  assert.deepEqual(w.puts, [
    "/pwa/offline.html",
    "/pwa/offline.css",
    "/pwa/offline.js",
  ]);
  assert.ok(
    w.fetches.every(
      (r) =>
        r.credentials === "omit" &&
        r.cache === "no-store" &&
        r.redirect === "error",
    ),
  );
  await w.dispatch("activate");
  assert.deepEqual(w.deleted, ["open-faktura-offline-old"]);
});
test("API, PDF, bearer links, queries and writes never enter worker cache or response handler", async () => {
  const w = worker();
  await w.dispatch("install");
  w.offline();
  for (const path of [
    "/api/companies/1/invoices",
    "/api/public/invoices/secret/pdf",
    "/shared/invoice/secret",
    "/document.pdf",
    "/?token=secret",
    "/assets/app.js",
  ]) {
    assert.equal(
      await w.dispatch("fetch", {
        method: "GET",
        mode: "navigate",
        url: "https://app.example.test" + path,
      }),
      undefined,
    );
  }
  assert.equal(
    await w.dispatch("fetch", {
      method: "POST",
      mode: "cors",
      url: "https://app.example.test/",
    }),
    undefined,
  );
  assert.equal(
    await w.dispatch("fetch", {
      method: "GET",
      mode: "navigate",
      url: "https://other.example.test/",
    }),
    undefined,
  );
  assert.equal(w.puts.length, 3);
});
test("offline root navigation returns only generic 503 fallback without caching navigation", async () => {
  const w = worker();
  await w.dispatch("install");
  w.offline();
  const response = await w.dispatch("fetch", {
    method: "GET",
    mode: "navigate",
    url: "https://app.example.test/",
  });
  assert.equal(response.status, 503);
  assert.equal(response.headers.get("cache-control"), "no-store");
  assert.equal(await response.text(), assets["/pwa/offline.html"]);
  assert.equal(w.puts.length, 3);
  const css = await w.dispatch("fetch", {
    method: "GET",
    mode: "cors",
    url: "https://app.example.test/pwa/offline.css",
  });
  assert.equal(css.headers.get("content-type"), "text/css");
});
test("manifest PNG declarations match actual source dimensions", async () => {
  const manifest = JSON.parse(
    await readFile("apps/web/public/manifest.webmanifest", "utf8"),
  );
  for (const icon of manifest.icons) {
    const png = await readFile("apps/web/public" + icon.src);
    assert.equal(png.subarray(1, 4).toString(), "PNG");
    assert.equal(icon.sizes, `${png.readUInt32BE(16)}x${png.readUInt32BE(20)}`);
  }
  assert.equal(manifest.display, "standalone");
  assert.equal(manifest.start_url, "/");
});

test("registration skips development, Tauri and insecure contexts", async () => {
  const { transpileModule, ModuleKind } = await import("typescript");
  const source = await readFile("apps/web/src/services/pwa.ts", "utf8");
  for (const config of [
    { prod: false },
    { prod: true, tauri: true },
    { prod: true, secure: false },
    { prod: true, protocol: "tauri:" },
    { prod: true },
  ]) {
    const calls = [];
    const exports = {};
    const context = createContext({
      exports,
      navigator: {
        serviceWorker: {
          register: (...args) => {
            calls.push(args);
            return Promise.resolve();
          },
        },
      },
      window: {
        isSecureContext: config.secure !== false,
        location: { protocol: config.protocol || "https:" },
        ...(config.tauri ? { __TAURI_INTERNALS__: {} } : {}),
      },
      document: { readyState: "complete" },
    });
    runInContext(
      transpileModule(
        source.replace("import.meta.env.PROD", String(config.prod)),
        { compilerOptions: { module: ModuleKind.CommonJS } },
      ).outputText,
      context,
    );
    exports.registerOnlinePwa();
    assert.equal(
      calls.length,
      config.prod &&
        !config.tauri &&
        config.secure !== false &&
        (!config.protocol || config.protocol === "https:")
        ? 1
        : 0,
    );
  }
});

test("misconfigured SPA fallback cannot enter the offline cache", async () => {
  const w = worker(true);
  await assert.rejects(w.dispatch("install"), /Offline asset mismatch/);
  assert.equal(w.puts.length, 0);
});
