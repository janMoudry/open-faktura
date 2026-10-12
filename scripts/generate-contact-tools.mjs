import { readFile, writeFile } from "node:fs/promises";
const contract = JSON.parse(
  await readFile("contracts/contact.openapi.json", "utf8"),
);
function resolve(value) {
  if (Array.isArray(value)) return value.map(resolve);
  if (!value || typeof value !== "object") return value;
  if (value.$ref)
    return resolve(contract.components.schemas[value.$ref.split("/").at(-1)]);
  const result = Object.fromEntries(
    Object.entries(value)
      .filter(([key]) => key !== "nullable")
      .map(([key, item]) => [key, resolve(item)]),
  );
  if (value.nullable) result.type = [value.type, "null"];
  return result;
}
const string = { type: "string" };
const make = (name, properties, required) => ({
  name,
  inputSchema: {
    type: "object",
    properties,
    required,
    additionalProperties: false,
  },
});
const search = Object.fromEntries(
  contract.paths["/api/companies/{company}/contacts"].get.parameters
    .filter((p) => p.in === "query")
    .map((p) => [p.name, p.schema]),
);
const create = resolve(contract.components.schemas.CreateContact);
const update = resolve(contract.components.schemas.UpdateContact);
await writeFile(
  "contracts/contact.tools.json",
  JSON.stringify(
    [
      make(
        "contact_lookup_ares",
        {
          company_id: string,
          ico: contract.paths[
            "/api/companies/{company}/contacts/ares/{ico}"
          ].get.parameters.find((p) => p.name === "ico").schema,
        },
        ["company_id", "ico"],
      ),
      make("contacts_list", { company_id: string, ...search }, ["company_id"]),
      make("contact_get", { company_id: string, id: string }, [
        "company_id",
        "id",
      ]),
      make("contact_create", { company_id: string, ...create.properties }, [
        "company_id",
        ...create.required,
      ]),
      make(
        "contact_update",
        { company_id: string, id: string, ...update.properties },
        ["company_id", "id", ...update.required],
      ),
    ],
    null,
    2,
  ) + "\n",
);
