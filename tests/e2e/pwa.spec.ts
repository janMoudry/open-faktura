import { test, expect } from "@playwright/test";
const origin = process.env.PWA_BASE_URL;
test.use({ serviceWorkers: "allow" });
for (const language of ["cs", "en"] as const) {
  test(`production PWA installability and private offline boundary (${language})`, async ({
    page,
    context,
  }, info) => {
    test.skip(
      !origin,
      "Requires a separately coordinated production preview via PWA_BASE_URL; Vite dev is intentionally not a PWA.",
    );
    test.setTimeout(60000);
    const base = new URL(origin!);
    expect(["127.0.0.1", "localhost"]).toContain(base.hostname);
    await page.addInitScript((lang) => {
      localStorage.setItem("open-faktura.language", lang);
    }, language);
    // All online data is synthetic. No database or accounting endpoint is mutated.
    await page.route("**/api/**", (route) =>
      route.fulfill({
        status: 200,
        json: { synthetic_pwa_probe: true },
        headers: { "Cache-Control": "no-store" },
      }),
    );
    try {
      await page.goto(`${base.origin}/#/shared/invoice/synthetic-pwa-probe`);
      await page.evaluate(async () => {
        await navigator.serviceWorker.ready;
      });
      await expect
        .poll(() =>
          page.evaluate(
            () => navigator.serviceWorker.controller?.scriptURL || "",
          ),
        )
        .toBe(`${base.origin}/pwa-sw.js`);
      if (info.project.name === "desktop") {
        const cdp = await context.newCDPSession(page);
        await cdp.send("Page.enable");
        const manifest = await cdp.send("Page.getAppManifest");
        expect(manifest.url).toBe(`${base.origin}/manifest.webmanifest`);
        expect(manifest.errors.filter((error) => error.critical)).toEqual([]);
        const parsed = JSON.parse(manifest.data!);
        expect(parsed.display).toBe("standalone");
        expect(parsed.icons.map((i: { sizes: string }) => i.sizes)).toEqual([
          "192x192",
          "512x512",
        ]);
        await expect
          .poll(
            async () =>
              (await cdp.send("Page.getInstallabilityErrors"))
                .installabilityErrors,
          )
          .toEqual([]);
        await cdp.detach();
      }
      const probes = [
        "/api/pwa-synthetic-accounting-probe",
        "/api/pwa-synthetic.pdf",
        "/api/public/invoices/synthetic-pwa-bearer/pdf",
      ];
      for (const path of probes)
        expect(
          await page.evaluate(async (url) => (await fetch(url)).ok, path),
        ).toBe(true);
      const entries = await page.evaluate(async () => {
        const keys = await caches.keys();
        return Promise.all(
          keys.map(async (key) => ({
            key,
            urls: (await (await caches.open(key)).keys())
              .map((r) => new URL(r.url).pathname)
              .sort(),
          })),
        );
      });
      expect(entries).toHaveLength(1);
      expect(entries[0].key).toMatch(/^open-faktura-offline-[a-f0-9]{16}$/);
      expect(entries[0].urls).toEqual([
        "/pwa/offline.css",
        "/pwa/offline.html",
        "/pwa/offline.js",
      ]);
      // Route fulfillment can bypass browser offline mode, so remove every mock first.
      await page.unrouteAll({ behavior: "wait" });
      await context.setOffline(true);
      const response = await page.reload();
      expect(response?.status()).toBe(503);
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        language === "cs"
          ? "Připojení není dostupné"
          : "Connection unavailable",
      );
      await expect(page.locator("html")).toHaveAttribute("lang", language);
      await page.screenshot({
        path: info.outputPath("offline.png"),
        fullPage: true,
      });
      for (const path of probes)
        expect(
          await page.evaluate(async (url) => {
            try {
              await fetch(url);
              return "responded";
            } catch {
              return "unavailable";
            }
          }, path),
        ).toBe("unavailable");
      await page
        .getByRole("button", {
          name: language === "cs" ? "English" : "Čeština",
          exact: true,
        })
        .click();
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(
        language === "cs"
          ? "Connection unavailable"
          : "Připojení není dostupné",
      );
      expect(
        await page.evaluate(async () => {
          const keys = await caches.keys();
          const urls = [];
          for (const key of keys)
            for (const r of await (await caches.open(key)).keys())
              urls.push(new URL(r.url).pathname);
          return urls.sort();
        }),
      ).toEqual(["/pwa/offline.css", "/pwa/offline.html", "/pwa/offline.js"]);
    } finally {
      await context.setOffline(false);
      await page
        .evaluate(async () => {
          for (const reg of await navigator.serviceWorker.getRegistrations())
            await reg.unregister();
          for (const name of await caches.keys())
            if (name.startsWith("open-faktura-offline-"))
              await caches.delete(name);
        })
        .catch(() => {});
    }
  });
}
