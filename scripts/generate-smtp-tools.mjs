import { readFile, writeFile } from "node:fs/promises";
const api = JSON.parse(await readFile("contracts/smtp.openapi.json", "utf8"));
const definitions = [
  ["smtp_get", null],
  ["smtp_save", "SmtpSave"],
  ["smtp_test", "SmtpTest"],
];
await writeFile(
  "contracts/smtp.tools.json",
  JSON.stringify(
    definitions.map(([name, schema]) => ({
      name,
      inputSchema: schema
        ? api.components.schemas[schema]
        : { type: "object", properties: {}, additionalProperties: false },
    })),
    null,
    2,
  ) + "\n",
);
