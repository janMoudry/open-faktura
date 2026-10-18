import { updaterConfig, verifyUpdaterArtifact } from "./updater-config.mjs";
import {
  readdir,
  readFile,
  mkdir,
  copyFile,
  writeFile,
} from "node:fs/promises";
import { createHash } from "node:crypto";
import { join, basename } from "node:path";
const [target, bundles, destination] = process.argv.slice(2);
if (
  !/^(aarch64-apple-darwin|x86_64-apple-darwin|x86_64-pc-windows-msvc|x86_64-unknown-linux-gnu)$/.test(
    target || "",
  ) ||
  !bundles ||
  !destination
)
  throw new Error("Expected target, bundle directory and output directory");
const config = JSON.parse(
  await readFile("apps/desktop/src-tauri/tauri.conf.json", "utf8"),
);
const version = config.version;
if (
  process.env.GITHUB_REF_TYPE === "tag" &&
  process.env.GITHUB_REF_NAME !== `v${version}`
)
  throw new Error("Release tag and desktop version differ");
const updater = updaterConfig();
const artifacts = [];
async function walk(path) {
  for (const entry of await readdir(path, { withFileTypes: true })) {
    const file = join(path, entry.name);
    if (entry.isDirectory() && !entry.name.endsWith(".app")) await walk(file);
    else if (
      entry.isFile() &&
      (/\.(dmg|exe|msi|deb|AppImage)$/.test(entry.name) ||
        (updater && entry.name.endsWith(".app.tar.gz")))
    )
      artifacts.push(file);
  }
}
const bundleTypes = target.endsWith("apple-darwin")
  ? ["dmg", ...(updater ? ["macos"] : [])]
  : target.endsWith("windows-msvc")
    ? ["nsis", "msi"]
    : ["deb", "appimage"];
for (const type of bundleTypes) {
  try {
    await walk(join(bundles, type));
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
}
if (!artifacts.length) throw new Error("No installers produced");
await mkdir(destination, { recursive: true });
const files = [];
const updates = [];
for (const file of artifacts.sort()) {
  const name = `open-faktura-${version}-${target}-${basename(file).replace(/[^a-zA-Z0-9._-]/g, "_")}`;
  if (files.some((v) => v.name === name))
    throw new Error("Duplicate artifact name");
  const bytes = await readFile(file);
  files.push({
    name,
    bytes: bytes.length,
    sha256: createHash("sha256").update(bytes).digest("hex"),
  });
  await copyFile(file, join(destination, name));
  const eligible =
    updater &&
    (target.endsWith("apple-darwin")
      ? file.endsWith(".app.tar.gz")
      : target.endsWith("windows-msvc")
        ? file.endsWith(".exe")
        : file.endsWith(".AppImage") || file.endsWith(".deb"));
  if (eligible) {
    let signature;
    try {
      signature = (await readFile(file + ".sig", "utf8")).trim();
    } catch (error) {
      if (file.endsWith(".deb") && error.code === "ENOENT") continue;
      throw new Error("Signed updater artifact is missing its signature");
    }
    verifyUpdaterArtifact(bytes, signature, updater.publicKey, version);
    const platform = target.endsWith("apple-darwin")
      ? target.startsWith("aarch64")
        ? "darwin-aarch64-app"
        : "darwin-x86_64-app"
      : target.endsWith("windows-msvc")
        ? "windows-x86_64-nsis"
        : file.endsWith(".deb")
          ? "linux-x86_64-deb"
          : "linux-x86_64-appimage";
    updates.push({ platform, name, signature });
    const sigbytes = Buffer.from(signature + "\n");
    files.push({
      name: name + ".sig",
      bytes: sigbytes.length,
      sha256: createHash("sha256").update(sigbytes).digest("hex"),
    });
    await writeFile(join(destination, name + ".sig"), sigbytes);
  }
}
if (updater && !updates.length)
  throw new Error("No verified updater artifact produced");
const manifest = {
  version,
  target,
  commit: process.env.GITHUB_SHA || null,
  signing: "unsigned-development",
  updater: !!updater,
  updates,
  backendBundled: false,
  serverCompatibility:
    "Use the server from the same release; back up PostgreSQL before server upgrades.",
  files,
};
await writeFile(
  join(destination, `manifest-${target}.json`),
  JSON.stringify(manifest, null, 2) + "\n",
);
await writeFile(
  join(destination, `SHA256SUMS-${target}.txt`),
  files.map((v) => `${v.sha256}  ${v.name}\n`).join(""),
);
console.log(`Collected ${files.length} installers for ${target}`);
