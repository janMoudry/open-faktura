import { readFile, writeFile } from "node:fs/promises";
const contract = JSON.parse(
  await readFile("contracts/catalog.openapi.json", "utf8"),
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
  contract.paths["/api/companies/{company}/catalog"].get.parameters
    .filter((p) => p.in === "query")
    .map((p) => [p.name, p.schema]),
);
const create = resolve(contract.components.schemas.CreateCatalog);
const update = resolve(contract.components.schemas.UpdateCatalog);
await writeFile(
  "contracts/catalog.tools.json",
  JSON.stringify(
    [
      make(
        "catalog_line",
        {
          company_id: string,
          id: string,
          ...Object.fromEntries(
            contract.paths[
              "/api/companies/{company}/catalog/{id}/line"
            ].get.parameters
              .filter((p) => p.in === "query")
              .map((p) => [p.name, p.schema]),
          ),
        },
        [
          "company_id",
          "id",
          ...contract.paths[
            "/api/companies/{company}/catalog/{id}/line"
          ].get.parameters
            .filter((p) => p.in === "query" && p.required)
            .map((p) => p.name),
        ],
      ),
      make("catalogs_list", { company_id: string, ...search }, ["company_id"]),
      make("catalog_get", { company_id: string, id: string }, [
        "company_id",
        "id",
      ]),
      make("catalog_create", { company_id: string, ...create.properties }, [
        "company_id",
        ...create.required,
      ]),
      make(
        "catalog_update",
        { company_id: string, id: string, ...update.properties },
        ["company_id", "id", ...update.required],
      ),
    ],
    null,
    2,
  ) + "\n",
);
