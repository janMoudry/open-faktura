import { readFile, writeFile } from "node:fs/promises";
const c = JSON.parse(await readFile("contracts/retail.openapi.json", "utf8"));
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
for (const methods of Object.values(c.paths))
  for (const op of Object.values(methods)) {
    const p = { company_id: { type: "string" } },
      required = ["company_id"];
    for (const param of op.parameters) {
      if (param.name === "company") continue;
      p[param.name] = param.schema;
      if (param.required) required.push(param.name);
    }
    let inputSchema;
    if (op.requestBody) {
      const b = resolve(op.requestBody.content["application/json"].schema);
      if (b.oneOf) {
        const variants = b.oneOf.map((v) => ({
          ...v,
          properties: { ...v.properties, company_id: p.company_id },
          required: [...v.required, "company_id"],
        }));
        inputSchema = {
          type: "object",
          properties: Object.fromEntries(
            [
              ...new Set(variants.flatMap((v) => Object.keys(v.properties))),
            ].map((k) => [k, {}]),
          ),
          oneOf: variants,
        };
      } else {
        Object.assign(p, b.properties);
        required.push(...b.required);
      }
    }
    tools.push({
      name: op.operationId,
      inputSchema: inputSchema ?? {
        type: "object",
        properties: p,
        required,
        additionalProperties: false,
      },
    });
  }
await writeFile(
  "contracts/retail.tools.json",
  JSON.stringify(tools, null, 2) + "\n",
);
