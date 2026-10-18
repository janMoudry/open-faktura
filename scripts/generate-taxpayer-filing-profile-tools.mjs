import { readFile, writeFile } from "node:fs/promises";
const c = JSON.parse(
  await readFile("contracts/taxpayer-filing-profile.openapi.json", "utf8"),
);
function resolve(v) {
  if (Array.isArray(v)) return v.map(resolve);
  if (!v || typeof v !== "object") return v;
  if (v.$ref) return resolve(c.components.schemas[v.$ref.split("/").at(-1)]);
  const r = Object.fromEntries(
    Object.entries(v)
      .filter(([k]) => k !== "nullable")
      .map(([k, x]) => [k, resolve(x)]),
  );
  return v.nullable ? { anyOf: [r, { type: "null" }] } : r;
}
const out = [];
for (const methods of Object.values(c.paths))
  for (const op of Object.values(methods)) {
    const properties = { company_id: { type: "string" } },
      required = ["company_id"];
    for (const p of op.parameters) {
      if (p.name === "company") continue;
      properties[p.name] = p.schema;
      if (p.required) required.push(p.name);
    }
    if (op.requestBody) {
      const body = resolve(op.requestBody.content["application/json"].schema);
      Object.assign(properties, body.properties);
      required.push(...body.required);
    }
    out.push({
      name: op.operationId,
      inputSchema: {
        type: "object",
        additionalProperties: false,
        properties,
        required,
      },
    });
  }
await writeFile(
  "contracts/taxpayer-filing-profile.tools.json",
  JSON.stringify(out, null, 2) + "\n",
);
