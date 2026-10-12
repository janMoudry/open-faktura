import { readFile, writeFile } from "node:fs/promises";
const c = JSON.parse(await readFile("contracts/import.openapi.json", "utf8"));
function resolve(v) {
  if (Array.isArray(v)) return v.map(resolve);
  if (!v || typeof v !== "object") return v;
  if (v.$ref) return resolve(c.components.schemas[v.$ref.split("/").at(-1)]);
  const r = Object.fromEntries(
    Object.entries(v)
      .filter(([k]) => k !== "nullable")
      .map(([k, x]) => [k, resolve(x)]),
  );
  if (v.nullable) r.type = [v.type, "null"];
  return r;
}
const tools = [];
for (const [path, methods] of Object.entries(c.paths))
  for (const op of Object.values(methods)) {
    if (op.operationId === "import_table") continue;
    const props = {
      company_id: { type: "string" },
      ...(path.includes("{id}") ? { id: { type: "string" } } : {}),
      ...(op.requestBody
        ? resolve(op.requestBody.content["application/json"].schema).properties
        : {}),
    };
    tools.push({
      name: op.operationId,
      inputSchema: {
        type: "object",
        properties: props,
        required: Object.keys(props),
        additionalProperties: false,
      },
    });
  }
await writeFile(
  "contracts/import.tools.json",
  JSON.stringify(tools, null, 2) + "\n",
);
