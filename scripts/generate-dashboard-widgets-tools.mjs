import { readFile, writeFile } from "node:fs/promises";
const c = JSON.parse(
  await readFile("contracts/dashboard-widgets.openapi.json", "utf8"),
);
const schema = structuredClone(c.components.schemas.Layout);
schema.properties.widgets.items = c.components.schemas.Widget;
await writeFile(
  "contracts/dashboard-widgets.tools.json",
  JSON.stringify(
    [
      {
        name: "dashboard_widgets_get",
        inputSchema: {
          type: "object",
          properties: {},
          additionalProperties: false,
        },
      },
      { name: "dashboard_widgets_set", inputSchema: schema },
    ],
    null,
    2,
  ) + "\n",
);
