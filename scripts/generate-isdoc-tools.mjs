import { readFile, writeFile } from "node:fs/promises";
const contract = JSON.parse(
  await readFile("contracts/isdoc.openapi.json", "utf8"),
);
await writeFile(
  "contracts/isdoc.tools.json",
  JSON.stringify(
    [
      {
        name: "invoice_isdoc",
        inputSchema: contract.components.schemas.IsdocExport,
      },
    ],
    null,
    2,
  ) + "\n",
);
