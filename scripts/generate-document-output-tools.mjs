import { readFile, writeFile } from "node:fs/promises";
const c = JSON.parse(
  await readFile("contracts/document-output.openapi.json", "utf8"),
);
const s = c.components.schemas.Request;
await writeFile(
  "contracts/document-output.tools.json",
  JSON.stringify(
    [
      {
        name: "document_output",
        inputSchema: {
          ...s,
          properties: { company_id: { type: "string" }, ...s.properties },
          required: ["company_id", ...s.required],
        },
      },
    ],
    null,
    2,
  ) + "\n",
);
