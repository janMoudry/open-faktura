import { readFile, writeFile } from "node:fs/promises";
const contract = JSON.parse(
  await readFile("contracts/invoice.openapi.json", "utf8"),
);
function resolve(value) {
  if (Array.isArray(value)) return value.map(resolve);
  if (!value || typeof value !== "object") return value;
  if (value.$ref)
    return resolve(contract.components.schemas[value.$ref.split("/").at(-1)]);
  return Object.fromEntries(
    Object.entries(value).map(([k, v]) => [k, resolve(v)]),
  );
}
const input = resolve(contract.components.schemas.InvoiceCalculation);
await writeFile(
  "contracts/invoice.tools.json",
  JSON.stringify(
    {
      type: "object",
      additionalProperties: false,
      required: ["company_id", ...input.required],
      properties: { company_id: { type: "string" }, ...input.properties },
    },
    null,
    2,
  ) + "\n",
);
