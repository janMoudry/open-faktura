// Static fallback only: no app shell, credentials, company data, or document cache.
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
process.chdir(fileURLToPath(new URL("../", import.meta.url)));
import { readFile, writeFile } from "node:fs/promises";
import { format } from "prettier";
async function writeAsset(path, source) {
  const parser = path.endsWith(".html")
    ? "html"
    : path.endsWith(".css")
      ? "css"
      : path.endsWith(".webmanifest")
        ? "json"
        : "babel";
  await writeFile(path, await format(source, { parser }));
}
const cs = JSON.parse(
  await readFile("apps/web/src/locales/offline.cs.json", "utf8"),
);
const en = JSON.parse(
  await readFile("apps/web/src/locales/offline.en.json", "utf8"),
);
const tokens = (
  await readFile("packages/design-tokens/tokens.css", "utf8")
).replace("@theme static {", ":root {");
const escape = (s) =>
  s.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll('"', "&quot;");
await writeAsset(
  "apps/web/public/pwa/offline.css",
  tokens +
    `\nbody{margin:0;background:var(--color-canvas);color:var(--color-foreground);font:1rem/1.6 system-ui,sans-serif}main{max-width:38rem;margin:12vh auto;padding:var(--spacing-section)}h1{font-size:var(--text-heading-page);line-height:1.2}p{color:var(--color-muted)}a,button{color:var(--color-primary);font:inherit}a{display:inline-block;font-weight:600;margin-block:var(--spacing-field)}button{background:var(--color-surface);border:1px solid var(--color-border);border-radius:var(--radius-action);padding:var(--spacing-inline) var(--spacing-field);cursor:pointer}nav{display:flex;gap:var(--spacing-inline)}:focus-visible{outline:2px solid var(--color-focus);outline-offset:4px}\n`,
);
await writeAsset(
  "apps/web/public/pwa/offline.html",
  `<!doctype html>
<html lang="cs"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="robots" content="noindex,nofollow"><meta name="referrer" content="no-referrer"><title>${escape(cs.title)} | Open Faktura</title><link rel="stylesheet" href="/pwa/offline.css"><script src="/pwa/offline.js" defer></script></head><body><main><strong>Open Faktura</strong><h1 data-message="title">${escape(cs.title)}</h1><p data-message="intro">${escape(cs.intro)}</p><p data-message="detail">${escape(cs.detail)}</p><a href="/" data-message="retry" id="retry">${escape(cs.retry)}</a><nav aria-label="${escape(cs.language)}"><button type="button" data-language="cs" lang="cs">Čeština</button><button type="button" data-language="en" lang="en">English</button></nav></main></body></html>\n`,
);
await writeAsset(
  "apps/web/public/pwa/offline.js",
  `// Generated from offline.cs/en.json. No network or document storage.
const messages=${JSON.stringify({ cs, en })};
function show(language){const selected=language==="en"?"en":"cs";document.documentElement.lang=selected;document.title=messages[selected].title+" | Open Faktura";for(const el of document.querySelectorAll("[data-message]"))el.textContent=messages[selected][el.dataset.message];for(const el of document.querySelectorAll("[data-language]"))el.setAttribute("aria-pressed",String(el.dataset.language===selected));}
try{show(localStorage.getItem("open-faktura.language"))}catch{show("cs")}
for(const el of document.querySelectorAll("[data-language]"))el.addEventListener("click",()=>{show(el.dataset.language);try{localStorage.setItem("open-faktura.language",el.dataset.language)}catch{}});
document.getElementById("retry").addEventListener("click",event=>{event.preventDefault();location.reload()});\n`,
);

const hash = createHash("sha256");
const digests = {};
for (const name of ["offline.html", "offline.css", "offline.js"]) {
  const content = await readFile("apps/web/public/pwa/" + name);
  hash.update(content);
  digests["/pwa/" + name] = createHash("sha256").update(content).digest("hex");
}
await writeAsset(
  "apps/web/public/pwa-sw.js",
  (await readFile("scripts/pwa-sw.template.js", "utf8"))
    .replace("__ASSET_HASH__", hash.digest("hex").slice(0, 16))
    .replace("__ASSET_DIGESTS__", JSON.stringify(digests)),
);

const manifestPath = "apps/web/public/manifest.webmanifest";
const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
manifest.theme_color = tokens.match(/--color-brand:\s*([^;]+);/)[1].trim();
manifest.background_color = tokens
  .match(/--color-canvas:\s*([^;]+);/)[1]
  .trim();
await writeAsset(manifestPath, JSON.stringify(manifest, null, 2) + "\n");
