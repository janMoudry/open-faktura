import { readFile, writeFile } from "node:fs/promises";
const {
  components: { schemas },
} = JSON.parse(await readFile("contracts/draft-trash.openapi.json", "utf8"));
const company_id = { type: "string" };
await writeFile(
  "contracts/draft-trash.tools.json",
  JSON.stringify(
    [
      {
        name: "invoice_trash_list",
        inputSchema: {
          type: "object",
          properties: { company_id, page: { type: "integer", minimum: 1 } },
          required: ["company_id"],
          additionalProperties: false,
        },
      },
      {
        name: "invoice_trash_execute",
        inputSchema: {
          ...schemas.Command,
          properties: { company_id, ...schemas.Command.properties },
          required: ["company_id", ...schemas.Command.required],
        },
      },
    ],
    null,
    2,
  ) + "\n",
);
