import { readFile, writeFile } from "node:fs/promises";
const c = JSON.parse(
  await readFile("contracts/document-style.openapi.json", "utf8"),
);
const s = c.components.schemas.Command;
const company_id = { type: "string" };
await writeFile(
  "contracts/document-style.tools.json",
  JSON.stringify(
    [
      {
        name: "document_style_get",
        inputSchema: {
          type: "object",
          properties: { company_id },
          required: ["company_id"],
          additionalProperties: false,
        },
      },
      {
        name: "document_style_set",
        inputSchema: {
          ...s,
          properties: { company_id, ...s.properties },
          required: ["company_id", ...s.required],
        },
      },
    ],
    null,
    2,
  ) + "\n",
);
