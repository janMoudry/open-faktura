import { readFile, writeFile } from "node:fs/promises";
const c = JSON.parse(
  await readFile("contracts/document-exports.openapi.json", "utf8"),
);
const company_id = { type: "string" };
const scope = (s) => ({
  ...s,
  properties: { company_id, ...s.properties },
  required: ["company_id", ...s.required],
});
await writeFile(
  "contracts/document-exports.tools.json",
  JSON.stringify(
    [
      {
        name: "document_export",
        inputSchema: scope(c.components.schemas.Request),
      },
      {
        name: "document_export_unlock",
        inputSchema: scope(c.components.schemas.Unlock),
      },
      {
        name: "document_export_history",
        inputSchema: scope(c.components.schemas.Target),
      },
      {
        name: "document_export_file",
        inputSchema: scope({
          type: "object",
          properties: { id: { type: "string", format: "uuid" } },
          required: ["id"],
          additionalProperties: false,
        }),
      },
    ],
    null,
    2,
  ) + "\n",
);
