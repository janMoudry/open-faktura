import { readFile, writeFile } from "node:fs/promises";
const {
  components: { schemas },
} = JSON.parse(await readFile("contracts/sales-trash.openapi.json", "utf8"));
const company_id = { type: "string" };
await writeFile(
  "contracts/sales-trash.tools.json",
  JSON.stringify(
    [
      {
        name: "sales_trash_list",
        inputSchema: {
          type: "object",
          properties: { company_id, page: { type: "integer", minimum: 1 } },
          required: ["company_id"],
          additionalProperties: false,
        },
      },
      {
        name: "sales_trash_execute",
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
