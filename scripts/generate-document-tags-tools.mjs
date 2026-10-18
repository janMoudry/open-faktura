import { readFile, writeFile } from "node:fs/promises";
const c = JSON.parse(
  await readFile("contracts/document-tags.openapi.json", "utf8"),
);
const company_id = { type: "string" };
const plain = {
  type: "object",
  properties: { company_id },
  required: ["company_id"],
  additionalProperties: false,
};
const target = {
  ...c.components.schemas.Target,
  properties: { company_id, ...c.components.schemas.Target.properties },
  required: ["company_id", ...c.components.schemas.Target.required],
};
const tools = [
  {
    name: "tags_list",
    inputSchema: { type: "object", oneOf: [plain, target] },
  },
  {
    name: "tags_execute",
    inputSchema: {
      type: "object",
      oneOf: c.components.schemas.Command.oneOf.map((s) => ({
        ...s,
        required: ["company_id", ...s.required],
        properties: { company_id, ...s.properties },
      })),
    },
  },
];
await writeFile(
  "contracts/document-tags.tools.json",
  JSON.stringify(tools, null, 2) + "\n",
);
