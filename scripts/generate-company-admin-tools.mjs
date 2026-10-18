import { readFile, writeFile } from "node:fs/promises";
const c = JSON.parse(
  await readFile("contracts/company-admin.openapi.json", "utf8"),
);
const company_id = { type: "string" };
const scope = {
  type: "object",
  required: ["company_id"],
  properties: { company_id },
  additionalProperties: false,
};
const tools = [
  { name: "company_administration", inputSchema: scope },
  {
    name: "company_trash",
    inputSchema: {
      type: "object",
      properties: {},
      additionalProperties: false,
    },
  },
  { name: "company_archive", inputSchema: scope },
  {
    name: "company_administration_execute",
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
  "contracts/company-admin.tools.json",
  JSON.stringify(tools, null, 2) + "\n",
);
