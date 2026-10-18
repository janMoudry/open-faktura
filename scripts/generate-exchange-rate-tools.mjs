import { readFile, writeFile } from "node:fs/promises";
const contract = JSON.parse(
  await readFile("contracts/exchange-rate.openapi.json", "utf8"),
);
await writeFile(
  "contracts/exchange-rate.tools.json",
  JSON.stringify(contract.components.schemas.RateLookup, null, 2) + "\n",
);
