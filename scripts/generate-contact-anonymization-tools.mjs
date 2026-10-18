import { readFile, writeFile } from "node:fs/promises";
const contract = JSON.parse(
  await readFile("contracts/contact-anonymization.openapi.json", "utf8"),
);
const command = contract.components.schemas.Command;
await writeFile(
  "contracts/contact-anonymization.tools.json",
  JSON.stringify(
    [
      {
        name: "contact_anonymize",
        inputSchema: {
          ...command,
          properties: {
            company_id: { type: "string" },
            contact_id: { type: "string" },
            ...command.properties,
          },
          required: ["company_id", "contact_id", ...command.required],
        },
      },
    ],
    null,
    2,
  ) + "\n",
);
