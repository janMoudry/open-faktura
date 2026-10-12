import { readFile, writeFile } from "node:fs/promises";
const api = JSON.parse(
  await readFile("contracts/attachment.openapi.json", "utf8"),
);
const tools = [];
for (const [path, methods] of Object.entries(api.paths))
  for (const [method, op] of Object.entries(methods)) {
    const properties = Object.fromEntries(
      op.parameters.map((p) => [p.name, p.schema]),
    );
    if (method === "post")
      Object.assign(properties, api.components.schemas.Upload.properties);
    tools.push({
      name: op.operationId.replace(/^attachment/, "attachment_").toLowerCase(),
      inputSchema: {
        type: "object",
        additionalProperties: false,
        properties,
        required: Object.keys(properties),
      },
    });
  }
await writeFile(
  "contracts/attachment.tools.json",
  JSON.stringify(tools, null, 2) + "\n",
);
