import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawn } from "node:child_process";
import { updaterConfig } from "./updater-config.mjs";
const config = updaterConfig();
const args = [
  resolve("node_modules/@tauri-apps/cli/tauri.js"),
  "build",
  ...process.argv.slice(2),
];
let temp;
try {
  if (config) {
    if (
      !process.env.TAURI_SIGNING_PRIVATE_KEY &&
      !process.env.TAURI_SIGNING_PRIVATE_KEY_PATH
    )
      throw new Error(
        "Configured updater builds require the operator signing key",
      );
    temp = await mkdtemp(join(tmpdir(), "open-faktura-updater-build-"));
    const path = join(temp, "updater.json");
    await writeFile(
      path,
      JSON.stringify({
        bundle: { createUpdaterArtifacts: true },
        plugins: {
          updater: {
            pubkey: config.publicKey,
            endpoints: [config.endpoint],
            requireSignedVersion: true,
            allowDowngrades: false,
            windows: { installMode: "basicUi" },
          },
        },
      }),
    );
    args.push("--config", path);
  }
  const child = spawn(process.execPath, args, {
    cwd: process.cwd(),
    stdio: "inherit",
    env: process.env,
  });
  const code = await new Promise((resolve, reject) => {
    child.on("error", reject);
    child.on("exit", resolve);
  });
  process.exitCode = code ?? 1;
} finally {
  if (temp) await rm(temp, { recursive: true, force: true });
}
