import { useState, type ReactNode } from "react";
import { Heading, Tabs, Badge, Icon } from "@open-faktura/ui";
import type { ContactData } from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";

/** Read-only counterpart of ContactFields, with the same section order. */
export function ContactOverview({ data }: { data: ContactData }) {
  const { t } = useAppTranslation();
  const [tab, setTab] = useState("identity");
  const facts = (rows: [AppKey, ReactNode][]) => (
    <dl className="contact-facts">
      {rows.map(([label, value]) => (
        <div key={label}>
          <dt>{t(label)}</dt>
          <dd>{value === "" || value == null ? "—" : value}</dd>
        </div>
      ))}
    </dl>
  );
  const address = (value: ContactData["billing_address"]) =>
    facts([
      ["street", value.street],
      ["city", value.city],
      ["postalCode", value.postal_code],
      ["country", value.country],
    ]);
  return (
    <Tabs
      label={t("contactSections")}
      value={tab}
      onChange={setTab}
      items={[
        {
          id: "identity",
          label: t("contactIdentity"),
          content: (
            <div className="contact-sheet">
              <section className="contact-section grid gap-group lg:grid-cols-2">
                <div className="space-y-inline">
                  <Heading level={2} size="card">
                    {t("contactIdentity")}
                  </Heading>
                  {facts([
                    ["contactName", data.name],
                    ["registrationNumber", data.registration_number],
                    ["vatNumber", data.vat_number],
                  ])}
                </div>
                <div className="space-y-inline">
                  <Heading level={2} size="card">
                    {t("contactBilling")}
                  </Heading>
                  {address(data.billing_address)}
                </div>
              </section>
              <section className="contact-section grid gap-group bg-sidebar lg:grid-cols-2">
                <div className="space-y-inline">
                  <Heading level={2} size="card">
                    {t("contactInformation")}
                  </Heading>
                  {facts([
                    ["email", data.email],
                    ["phone", data.phone],
                    ["contactRecipients", data.recipients.join("\n")],
                  ])}
                </div>
                <div className="space-y-inline">
                  <div className="contact-section-heading">
                    <Heading level={2} size="card">
                      {t("contactPeople")}
                    </Heading>
                  </div>
                  {data.persons.length ? (
                    data.persons.map((person, index) => (
                      <div
                        className="border-b border-border pb-field last:border-0"
                        key={index}
                      >
                        {facts([
                          ["accountName", person.name],
                          ["email", person.email],
                          ["phone", person.phone],
                        ])}
                      </div>
                    ))
                  ) : (
                    <p className="text-sm text-muted">{t("contactNoPeople")}</p>
                  )}
                </div>
              </section>
              <details className="contact-section">
                <summary className="contact-disclosure">
                  <Icon name="chevronRight" />
                  {t("contactMoreDetails")}
                </summary>
                <div className="mt-field grid gap-group lg:grid-cols-2">
                  {facts([
                    ["maturityDays", data.maturity_days ?? t("contactInherit")],
                    [
                      "paymentMethod",
                      t(
                        data.payment_method === "cash"
                          ? "cash"
                          : data.payment_method === "bank_transfer"
                            ? "bankTransfer"
                            : "contactInherit",
                      ),
                    ],
                    ["contactBankAccount", data.bank_account],
                    ["iban", data.iban],
                  ])}
                  {facts([["contactNote", data.note]])}
                </div>
              </details>
            </div>
          ),
        },
        {
          id: "delivery",
          label: t("contactDelivery"),
          content: (
            <div className="contact-sheet">
              <section className="contact-section space-y-group">
                <p className="text-sm text-muted">
                  {t("contactDefaultDelivery")}:{" "}
                  {data.delivery_addresses.find(
                    (a) => a.id === data.default_delivery_id,
                  )?.name || t("contactBilling")}
                </p>
                {data.delivery_addresses.length ? (
                  <div className="grid gap-group lg:grid-cols-2">
                    {data.delivery_addresses.map((item, index) => (
                      <div key={item.id} className="space-y-inline">
                        <div className="flex flex-wrap items-center gap-inline">
                          <Heading level={2} size="card">
                            {item.name ||
                              `${t("contactDelivery")} ${index + 1}`}
                          </Heading>
                          {item.id === data.default_delivery_id && (
                            <Badge>{t("contactDefaultDelivery")}</Badge>
                          )}
                        </div>
                        {address(item.address)}
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="text-sm">{t("contactNoDelivery")}</p>
                )}
              </section>
            </div>
          ),
        },
      ]}
    />
  );
}
