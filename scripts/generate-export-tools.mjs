import { readFile, writeFile } from "node:fs/promises";
const api = JSON.parse(await readFile("contracts/export.openapi.json", "utf8"));
function resolve(v) {
  if (Array.isArray(v)) return v.map(resolve);
  if (v && typeof v === "object") {
    if (v.$ref)
      return resolve(api.components.schemas[v.$ref.split("/").at(-1)]);
    return Object.fromEntries(
      Object.entries(v).map(([k, x]) => [k, resolve(x)]),
    );
  }
  return v;
}
const schema = resolve(api.components.schemas.ExportRequest);
schema.properties.company_id = { type: "string" };
schema.required.push("company_id");
await writeFile(
  "contracts/export.tools.json",
  JSON.stringify([{ name: "register_export", inputSchema: schema }], null, 2) +
    "\n",
);
