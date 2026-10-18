import { readFile, writeFile } from "node:fs/promises";
const contract = JSON.parse(
  await readFile("contracts/vat-registry.openapi.json", "utf8"),
);
await writeFile(
  "contracts/vat-registry.tools.json",
  JSON.stringify(
    [
      {
        name: "vat_registry_lookup",
        inputSchema: contract.components.schemas.Lookup,
      },
    ],
    null,
    2,
  ) + "\n",
);
