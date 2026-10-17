import { readFile, writeFile } from "node:fs/promises";
const c = JSON.parse(
  await readFile("contracts/preferences.openapi.json", "utf8"),
);
await writeFile(
  "contracts/preferences.tools.json",
  JSON.stringify(
    [
      {
        name: "preferences_get",
        inputSchema: {
          type: "object",
          properties: {},
          additionalProperties: false,
        },
      },
      {
        name: "preferences_set",
        inputSchema: c.components.schemas.Preferences,
      },
    ],
    null,
    2,
  ) + "\n",
);
