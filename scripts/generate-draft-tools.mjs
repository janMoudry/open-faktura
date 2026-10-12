import { readFile, writeFile } from "node:fs/promises";
const contract = JSON.parse(
  await readFile("contracts/invoice.openapi.json", "utf8"),
);
function resolve(v) {
  if (Array.isArray(v)) return v.map(resolve);
  if (!v || typeof v !== "object") return v;
  if (v.$ref)
    return resolve(contract.components.schemas[v.$ref.split("/").at(-1)]);
  if (v.nullable) {
    const base = { ...v };
    delete base.nullable;
    return { anyOf: [resolve(base), { type: "null" }] };
  }
  return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, resolve(x)]));
}
const string = { type: "string" };
const make = (name, properties, required) => ({
  name,
  inputSchema: {
    type: "object",
    additionalProperties: false,
    properties,
    required,
  },
});
const create = resolve(contract.components.schemas.CreateDraft),
  update = resolve(contract.components.schemas.UpdateDraft);
const query = Object.fromEntries(
  contract.paths["/api/companies/{company}/invoices"].get.parameters
    .filter((p) => p.in === "query")
    .map((p) => [p.name, p.schema]),
);
await writeFile(
  "contracts/draft.tools.json",
  JSON.stringify(
    [
      make("company_overview", { company_id: string }, ["company_id"]),
      make("invoice_payments", { company_id: string, id: string }, [
        "company_id",
        "id",
      ]),
      make(
        "invoice_register_payment",
        {
          company_id: string,
          id: string,
          ...resolve(contract.components.schemas.RegisterPayment).properties,
        },
        [
          "company_id",
          "id",
          ...contract.components.schemas.RegisterPayment.required,
        ],
      ),
      make(
        "invoice_reverse_payment",
        {
          company_id: string,
          id: string,
          payment_id: string,
          ...resolve(contract.components.schemas.ReversePayment).properties,
        },
        [
          "company_id",
          "id",
          "payment_id",
          ...contract.components.schemas.ReversePayment.required,
        ],
      ),
      make("invoices_list", { company_id: string, ...query }, ["company_id"]),
      make(
        "invoice_issue",
        {
          company_id: string,
          id: string,
          ...resolve(contract.components.schemas.IssueInvoice).properties,
        },
        ["company_id", "id", "request_id", "revision"],
      ),
      make("invoice_pdf", { company_id: string, id: string }, [
        "company_id",
        "id",
      ]),
      make("invoice_get", { company_id: string, id: string }, [
        "company_id",
        "id",
      ]),
      make(
        "invoice_create_draft",
        { company_id: string, ...create.properties },
        ["company_id", ...create.required],
      ),
      make(
        "invoice_update_draft",
        { company_id: string, id: string, ...update.properties },
        ["company_id", "id", ...update.required],
      ),
    ],
    null,
    2,
  ) + "\n",
);
