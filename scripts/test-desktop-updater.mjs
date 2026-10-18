// Synthetic fixture only. Never installs a package or publishes a release.
import assert from "node:assert/strict";
import {
  mkdtemp,
  writeFile,
  readFile,
  mkdir,
  rm,
  chmod,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { execFileSync } from "node:child_process";
import {
  updaterConfig,
  verifyUpdaterArtifact,
} from "./release/updater-config.mjs";
const temp = await mkdtemp(join(tmpdir(), "open-faktura-update-test-"));
await chmod(temp, 0o700);
const env = {
  ...process.env,
  TAURI_SIGNING_PRIVATE_KEY: "",
  TAURI_SIGNING_PRIVATE_KEY_PATH: "",
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD: "",
  OPEN_FAKTURA_UPDATER_ENDPOINT: "",
  OPEN_FAKTURA_UPDATER_PUBLIC_KEY: "",
};
delete env.TAURI_SIGNING_PRIVATE_KEY;
delete env.TAURI_SIGNING_PRIVATE_KEY_PATH;
const cli = resolve("apps/desktop/node_modules/@tauri-apps/cli/tauri.js");
function run(file, args, extra = {}) {
  try {
    return execFileSync(process.execPath, [file, ...args], {
      env: { ...env, ...extra },
      stdio: "pipe",
    });
  } catch {
    throw new Error("Synthetic updater child operation failed");
  }
}
try {
  assert.equal(updaterConfig({}), null);
  assert.throws(() =>
    updaterConfig({
      OPEN_FAKTURA_UPDATER_ENDPOINT: "https://updates.example.test/latest.json",
    }),
  );
  const key = join(temp, "throwaway.key");
  run(cli, [
    "signer",
    "generate",
    "--ci",
    "--password",
    "",
    "--write-keys",
    key,
  ]);
  const publicKey = (await readFile(key + ".pub", "utf8")).trim();
  const config = {
    OPEN_FAKTURA_UPDATER_ENDPOINT: "https://updates.example.test/latest.json",
    OPEN_FAKTURA_UPDATER_PUBLIC_KEY: publicKey,
  };
  assert.ok(updaterConfig(config));
  assert.throws(() =>
    updaterConfig({
      ...config,
      OPEN_FAKTURA_UPDATER_ENDPOINT:
        "https://user:password@example.test/latest.json",
    }),
  );
  const bytes = Buffer.from("synthetic release fixture; never an installer"),
    file = join(temp, "synthetic.bin");
  await writeFile(file, bytes);
  run(cli, [
    "signer",
    "sign",
    "--password",
    "",
    "--app-version",
    "0.1.0",
    "--private-key-path",
    key,
    file,
  ]);
  const signature = (await readFile(file + ".sig", "utf8")).trim();
  assert.ok(verifyUpdaterArtifact(bytes, signature, publicKey, "0.1.0"));
  assert.throws(() =>
    verifyUpdaterArtifact(
      Buffer.from("tampered"),
      signature,
      publicKey,
      "0.1.0",
    ),
  );
  assert.throws(() =>
    verifyUpdaterArtifact(bytes, signature, publicKey, "0.1.1"),
  );
  assert.throws(() =>
    verifyUpdaterArtifact(bytes, "bad-signature", publicKey, "0.1.0"),
  );
  run(cli, [
    "signer",
    "sign",
    "--password",
    "",
    "--private-key-path",
    key,
    file,
  ]);
  const legacy = (await readFile(file + ".sig", "utf8")).trim();
  assert.throws(() => verifyUpdaterArtifact(bytes, legacy, publicKey, "0.1.0"));
  const output = join(temp, "assets");
  const fixtures = [
    ["aarch64-apple-darwin", "macos", "App.app.tar.gz"],
    ["x86_64-apple-darwin", "macos", "App.app.tar.gz"],
    ["x86_64-pc-windows-msvc", "nsis", "App.exe"],
    ["x86_64-unknown-linux-gnu", "appimage", "App.AppImage"],
  ];
  for (const [target, type, name] of fixtures) {
    const bundle = join(temp, target),
      folder = join(bundle, type);
    await mkdir(folder, { recursive: true });
    await writeFile(join(folder, name), bytes);
    await writeFile(join(folder, name + ".sig"), signature);
    if (type === "macos") {
      await mkdir(join(bundle, "dmg"));
      await writeFile(join(bundle, "dmg", "App.dmg"), bytes);
    }
    run(
      "scripts/release/collect-desktop.mjs",
      [target, bundle, output],
      config,
    );
  }
  run(
    "scripts/release/create-updater-manifest.mjs",
    [output, "https://releases.example.test/v0.1.0"],
    config,
  );
  const latest = JSON.parse(
    await readFile(join(output, "latest.json"), "utf8"),
  );
  assert.equal(latest.version, "0.1.0");
  assert.equal(Object.keys(latest.platforms).length, 4);
  assert.ok(latest.platforms["linux-x86_64-appimage"].signature);
  // Default development collection must not claim updater support or signatures.
  const unsigned = join(temp, "unsigned");
  run("scripts/release/collect-desktop.mjs", [
    fixtures[2][0],
    join(temp, fixtures[2][0]),
    unsigned,
  ]);
  const manifest = JSON.parse(
    await readFile(join(unsigned, `manifest-${fixtures[2][0]}.json`), "utf8"),
  );
  assert.equal(manifest.updater, false);
  assert.deepEqual(manifest.updates, []);
  await writeFile(join(temp, fixtures[2][0], "nsis", "App.exe.sig"), "invalid");
  assert.throws(() =>
    run(
      "scripts/release/collect-desktop.mjs",
      [fixtures[2][0], join(temp, fixtures[2][0]), join(temp, "bad")],
      config,
    ),
  );
  console.log(
    "Desktop updater signed-fixture and collector checks passed (no installation)",
  );
} finally {
  await rm(temp, { recursive: true, force: true });
}
