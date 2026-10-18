import { readFile, writeFile } from "node:fs/promises";
const contract = JSON.parse(
  await readFile("contracts/vies.openapi.json", "utf8"),
);
await writeFile(
  "contracts/vies.tools.json",
  JSON.stringify(
    [{ name: "vies_lookup", inputSchema: contract.components.schemas.Lookup }],
    null,
    2,
  ) + "\n",
);
