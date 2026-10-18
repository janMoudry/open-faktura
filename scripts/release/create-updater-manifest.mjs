import { readdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { updaterConfig, verifyUpdaterArtifact } from "./updater-config.mjs";
const [dir, base] = process.argv.slice(2);
const config = updaterConfig();
if (config) {
  const url = new URL(base);
  if (
    url.protocol !== "https:" ||
    url.username ||
    url.password ||
    url.hash ||
    url.search
  )
    throw new Error("Public HTTPS asset base required");
  const platforms = {};
  let version;
  const manifests = (await readdir(dir)).filter((v) =>
    /^manifest-.*\.json$/.test(v),
  );
  if (manifests.length !== 4)
    throw new Error("All four desktop targets are required");
  for (const name of manifests) {
    const m = JSON.parse(await readFile(join(dir, name), "utf8"));
    if (!m.updater || !m.updates?.length || (version && version !== m.version))
      throw new Error("Incomplete or mixed updater release");
    version = m.version;
    for (const u of m.updates) {
      if (platforms[u.platform] || u.name !== u.name.split(/[\\/]/).at(-1))
        throw new Error("Duplicate or invalid updater platform");
      verifyUpdaterArtifact(
        await readFile(join(dir, u.name)),
        u.signature,
        config.publicKey,
        version,
      );
      platforms[u.platform] = {
        url: base.replace(/\/$/, "") + "/" + encodeURIComponent(u.name),
        signature: u.signature,
      };
    }
  }
  for (const required of [
    "darwin-aarch64-app",
    "darwin-x86_64-app",
    "windows-x86_64-nsis",
    "linux-x86_64-appimage",
  ]) {
    if (!platforms[required])
      throw new Error("Missing required signed updater target");
  }
  await writeFile(
    join(dir, "latest.json"),
    JSON.stringify(
      {
        version,
        notes: `Open Faktura ${version}. Use a compatible server from the same release.`,
        pub_date: new Date().toISOString(),
        platforms,
      },
      null,
      2,
    ) + "\n",
  );
}
