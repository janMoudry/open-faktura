// Generate the reviewed archive allowlist from a fresh disposable local database.
// Never inspects or modifies accounting records in DATABASE_URL's existing DB.
import { execFileSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import { readdir, writeFile } from "node:fs/promises";
const raw = process.env.DATABASE_URL;
if (!raw)
  throw new Error(
    "DATABASE_URL must point to isolated local PostgreSQL with CREATEDB",
  );
const url = new URL(raw);
if (!["localhost", "127.0.0.1", "[::1]"].includes(url.hostname))
  throw new Error(
    "Archive schema generation only supports local test PostgreSQL",
  );
const env = {
  ...process.env,
  PGHOST: url.hostname,
  PGPORT: url.port || "5432",
  PGUSER: decodeURIComponent(url.username),
  PGPASSWORD: decodeURIComponent(url.password),
  PGDATABASE: url.pathname.slice(1),
};
const database = `archive_schema_${randomBytes(12).toString("hex")}`;
const psql = (args, db = env.PGDATABASE) =>
  execFileSync("psql", ["-X", "-q", "-At", "-v", "ON_ERROR_STOP=1", ...args], {
    env: { ...env, PGDATABASE: db },
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
psql(["-c", `CREATE DATABASE ${database}`]);
try {
  for (const name of (await readdir("migrations"))
    .filter((v) => v.endsWith(".sql"))
    .sort())
    psql(["-f", `migrations/${name}`], database);
  const query =
    "SELECT jsonb_object_agg(table_name,cols) FROM (SELECT table_name,jsonb_agg(column_name ORDER BY ordinal_position) cols FROM information_schema.columns WHERE table_schema='public' AND table_name IN (SELECT table_name FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE') AND table_name IN (SELECT table_name FROM information_schema.columns WHERE table_schema='public' AND column_name='company_id' UNION SELECT 'company') AND column_name NOT IN ('secret','password','token_hash','url') GROUP BY table_name) x";
  const schema = JSON.parse(psql(["-c", query], database));
  const sorted = Object.fromEntries(
    Object.entries(schema).sort(([a], [b]) => a.localeCompare(b)),
  );
  await writeFile(
    "contracts/company-archive-schema.json",
    JSON.stringify(sorted, null, 2) + "\n",
  );
  process.stdout.write(
    `Generated ${Object.keys(sorted).length} table allowlists. Review every changed table/column before shipping; nested credentials require explicit exclusions.\n`,
  );
} finally {
  psql(["-c", `DROP DATABASE ${database}`]);
}
