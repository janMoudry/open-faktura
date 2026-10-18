import { readFile, writeFile } from "node:fs/promises";
const c = JSON.parse(
  await readFile("contracts/communication.openapi.json", "utf8"),
);
function resolve(v) {
  if (Array.isArray(v)) return v.map(resolve);
  if (!v || typeof v !== "object") return v;
  if (v.$ref) return resolve(c.components.schemas[v.$ref.split("/").at(-1)]);
  const result = Object.fromEntries(
    Object.entries(v)
      .filter(([k]) => k !== "nullable")
      .map(([k, x]) => [k, resolve(x)]),
  );
  return v.nullable ? { anyOf: [result, { type: "null" }] } : result;
}
const tools = [];
for (const [path, methods] of Object.entries(c.paths))
  for (const [method, op] of Object.entries(methods)) {
    const p = { company_id: { type: "string" } };
    const required = ["company_id"];
    for (const param of op.parameters) {
      if (param.name === "company") continue;
      p[param.name] = param.schema;
      if (param.required) required.push(param.name);
    }
    let alternatives;
    if (op.requestBody) {
      const b = resolve(op.requestBody.content["application/json"].schema);
      Object.assign(p, b.properties);
      required.push(...b.required);
      alternatives = b.oneOf;
    }
    tools.push({
      name: op.operationId,
      inputSchema: {
        type: "object",
        properties: p,
        required,
        additionalProperties: false,
        ...(alternatives ? { oneOf: alternatives } : {}),
      },
    });
  }
await writeFile(
  "contracts/communication.tools.json",
  JSON.stringify(tools, null, 2) + "\n",
);
