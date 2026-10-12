import { randomBytes } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
await mkdir(".secrets", { recursive: true, mode: 0o700 });
// Exclusive creation makes repeated execution safe; existing secrets never rotate.
const password = randomBytes(32).toString("hex");
for (const [name, value] of [
  ["bootstrap", randomBytes(32).toString("hex")],
  ["smtp-key", randomBytes(32).toString("hex")],
  ["db_password", password],
]) {
  await writeFile(`.secrets/${name}`, value + "\n", {
    mode: 0o600,
    flag: "wx",
  }).catch((error) => {
    if (error.code !== "EEXIST") throw error;
  });
}
const { readFile } = await import("node:fs/promises");
const saved = (await readFile(".secrets/db_password", "utf8")).trim();
await writeFile(
  ".secrets/database_url",
  `postgres://faktura:${saved}@postgres:5432/faktura\n`,
  { mode: 0o600, flag: "wx" },
).catch((error) => {
  if (error.code !== "EEXIST") throw error;
});
console.log(
  "Private secrets are ready in .secrets/. Existing values were preserved.",
);
