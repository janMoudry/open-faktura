import { useEffect, useState, type ReactNode } from "react";
import {
  Button,
  Field,
  Tabs,
  Heading,
  Icon,
  Select,
  TextareaField,
} from "@open-faktura/ui";
import type { ContactData } from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../../locales";
export function ContactFields({
  value,
  onChange,
  disabled,
  fields,
  ares,
  footer,
}: {
  footer?: ReactNode;
  value: ContactData;
  onChange: (value: ContactData) => void;
  disabled: boolean;
  fields: string[];
  ares: { loading: boolean; message: AppKey | ""; lookup: () => void };
}) {
  const { t } = useAppTranslation();
  const [tab, setTab] = useState("identity");
  const [details, setDetails] = useState(false);
  useEffect(() => {
    if (!fields.length) return;
    setTab(
      fields.some((f) =>
        ["delivery_addresses", "default_delivery_id"].includes(f),
      )
        ? "delivery"
        : "identity",
    );
    if (
      fields.some((f) =>
        [
          "maturity_days",
          "payment_method",
          "bank_account",
          "iban",
          "note",
        ].includes(f),
      )
    )
      setDetails(true);
  }, [fields]);
  const patch = (data: Partial<ContactData>) => onChange({ ...value, ...data });
  const text = (
    key:
      | "name"
      | "registration_number"
      | "vat_number"
      | "email"
      | "phone"
      | "bank_account"
      | "iban",
    label: AppKey,
    type = "text",
  ) => (
    <Field
      horizontal
      key={key}
      label={t(label)}
      type={type}
      value={value[key]}
      readOnly={disabled}
      required={key === "name"}
      maxLength={key === "email" ? 254 : key === "name" ? 200 : 80}
      error={fields.includes(key) ? t("contactInvalidField") : undefined}
      onBlur={
        key === "registration_number" && !disabled ? ares.lookup : undefined
      }
      onChange={(e) => patch({ [key]: e.target.value })}
    />
  );
  const address = (
    data: ContactData["billing_address"],
    change: (address: ContactData["billing_address"]) => void,
  ) => (
    <div className="space-y-inline">
      {(
        [
          ["street", "street"],
          ["city", "city"],
          ["postal_code", "postalCode"],
          ["country", "country"],
        ] as const
      ).map(([key, label]) => (
        <Field
          horizontal
          key={key}
          label={t(label)}
          value={data[key]}
          readOnly={disabled}
          maxLength={key === "country" ? 2 : 200}
          required={key === "country"}
          onChange={(e) => change({ ...data, [key]: e.target.value })}
        />
      ))}
    </div>
  );
  return (
    <Tabs
      label={t("contactSections")}
      value={tab}
      onChange={setTab}
      items={[
        {
          id: "identity",
          label: t("contactIdentity"),
          content:
            tab === "identity" ? (
              <div className="contact-sheet">
                <section className="contact-section grid gap-group lg:grid-cols-2">
                  <div className="space-y-inline">
                    <Heading level={2} size="card">
                      {t("contactIdentity")}
                    </Heading>
                    {text("name", "contactName")}
                    <Field
                      horizontal
                      label={t("registrationNumber")}
                      value={value.registration_number}
                      readOnly={disabled}
                      maxLength={32}
                      help={t("aresHint")}
                      error={
                        fields.includes("registration_number")
                          ? t("contactInvalidField")
                          : undefined
                      }
                      onChange={(e) =>
                        patch({ registration_number: e.target.value })
                      }
                      onBlur={!disabled ? ares.lookup : undefined}
                      trailing={
                        !disabled && (
                          <Button
                            size="sm"
                            variant="secondary"
                            loading={ares.loading}
                            disabled={!value.registration_number.trim()}
                            onClick={ares.lookup}
                            aria-label={t("aresLookup")}
                            title={t("aresLookup")}
                          >
                            <Icon name="search" />
                          </Button>
                        )
                      }
                    />
                    {ares.message && (
                      <p
                        role="status"
                        className={`text-sm ${["aresLoading", "aresFilled"].includes(ares.message) ? "text-muted" : "text-danger"}`}
                      >
                        {t(ares.message)}
                      </p>
                    )}
                    {text("vat_number", "vatNumber")}
                  </div>
                  <div className="space-y-inline">
                    <Heading level={2} size="card">
                      {t("contactBilling")}
                    </Heading>
                    {address(value.billing_address, (billing_address) =>
                      patch({ billing_address }),
                    )}
                  </div>
                </section>
                <section className="contact-section grid gap-group bg-sidebar lg:grid-cols-2">
                  <div className="space-y-inline">
                    <Heading level={2} size="card">
                      {t("contactInformation")}
                    </Heading>
                    <div className="space-y-inline">
                      {text("email", "email", "email")}
                      {text("phone", "phone", "tel")}
                    </div>
                    <Field
                      horizontal
                      label={t("contactRecipients")}
                      help={t("contactRecipientsHint")}
                      readOnly={disabled}
                      value={value.recipients.join(",")}
                      onChange={(e) =>
                        patch({ recipients: e.target.value.split(/[,;\n]/) })
                      }
                    />
                  </div>
                  <div className="space-y-inline">
                    <div className="contact-section-heading">
                      {" "}
                      <Heading level={2} size="card">
                        {t("contactPeople")}
                      </Heading>{" "}
                      {!disabled && (
                        <Button
                          variant="ghost"
                          aria-label={t("contactAddPerson")}
                          title={t("contactAddPerson")}
                          size="sm"
                          disabled={value.persons.length >= 20}
                          onClick={() =>
                            patch({
                              persons: [
                                ...value.persons,
                                { name: "", email: "", phone: "" },
                              ],
                            })
                          }
                        >
                          <Icon name="plus" />
                        </Button>
                      )}
                    </div>
                    <>
                      {value.persons.map((person, index) => (
                        <fieldset
                          key={index}
                          className="space-y-inline border-b border-border pb-inline last:border-0"
                        >
                          <legend className="sr-only">
                            {t("contactPerson")} {index + 1}
                          </legend>
                          <div className="space-y-inline">
                            {(
                              [
                                ["name", "accountName"],
                                ["email", "email"],
                                ["phone", "phone"],
                              ] as const
                            ).map(([key, label]) => (
                              <Field
                                horizontal
                                key={key}
                                label={t(label)}
                                type={key === "email" ? "email" : "text"}
                                required={key === "name"}
                                readOnly={disabled}
                                value={person[key]}
                                onChange={(e) =>
                                  patch({
                                    persons: value.persons.map((p, i) =>
                                      i === index
                                        ? { ...p, [key]: e.target.value }
                                        : p,
                                    ),
                                  })
                                }
                              />
                            ))}
                          </div>
                          {!disabled && (
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() =>
                                patch({
                                  persons: value.persons.filter(
                                    (_, i) => i !== index,
                                  ),
                                })
                              }
                            >
                              {t("contactRemovePerson")}
                            </Button>
                          )}
                        </fieldset>
                      ))}
                    </>
                  </div>
                </section>
                <details
                  className="contact-section"
                  open={details}
                  onToggle={(event) => setDetails(event.currentTarget.open)}
                >
                  <summary className="contact-disclosure">
                    <Icon name="chevronRight" />
                    {t("contactMoreDetails")}
                  </summary>
                  <div className="mt-field space-y-field">
                    <>
                      <p className="text-sm text-muted">
                        {t("contactPreferencesHint")}
                      </p>
                      <div className="space-y-inline">
                        <Field
                          horizontal
                          label={t("maturityDays")}
                          type="number"
                          min={0}
                          max={365}
                          readOnly={disabled}
                          value={value.maturity_days ?? ""}
                          onChange={(e) =>
                            patch({
                              maturity_days:
                                e.target.value === ""
                                  ? null
                                  : Number(e.target.value),
                            })
                          }
                        />
                        <Select
                          label={t("paymentMethod")}
                          disabled={disabled}
                          value={value.payment_method || "inherit"}
                          options={[
                            { value: "inherit", label: t("contactInherit") },
                            {
                              value: "bank_transfer",
                              label: t("bankTransfer"),
                            },
                            { value: "cash", label: t("cash") },
                          ]}
                          onChange={(method) =>
                            patch({
                              payment_method:
                                method === "bank_transfer" || method === "cash"
                                  ? method
                                  : "",
                            })
                          }
                        />
                        {text("bank_account", "contactBankAccount")}
                        {text("iban", "iban")}
                      </div>
                    </>
                    <TextareaField
                      horizontal
                      label={t("contactNote")}
                      value={value.note}
                      readOnly={disabled}
                      maxLength={2000}
                      onChange={(e) => patch({ note: e.target.value })}
                    />
                  </div>
                </details>
                {footer}
              </div>
            ) : null,
        },
        {
          id: "delivery",
          label: t("contactDelivery"),
          content:
            tab === "delivery" ? (
              <div className="contact-sheet">
                <section className="contact-section space-y-field">
                  <>
                    <Select
                      label={t("contactDefaultDelivery")}
                      disabled={disabled}
                      value={value.default_delivery_id ?? "billing"}
                      options={[
                        { value: "billing", label: t("contactBilling") },
                        ...value.delivery_addresses.map((a, i) => ({
                          value: a.id,
                          label: a.name || `${t("contactDelivery")} ${i + 1}`,
                        })),
                      ]}
                      onChange={(id) =>
                        patch({
                          default_delivery_id: id === "billing" ? null : id,
                        })
                      }
                    />
                    {value.delivery_addresses.map((a, index) => (
                      <fieldset
                        key={a.id}
                        className="space-y-inline border-b border-border pb-inline last:border-0"
                      >
                        <legend className="sr-only">
                          {t("contactDelivery")} {index + 1}
                        </legend>
                        <Field
                          horizontal
                          label={t("contactAddressName")}
                          value={a.name}
                          readOnly={disabled}
                          maxLength={120}
                          onChange={(e) =>
                            patch({
                              delivery_addresses: value.delivery_addresses.map(
                                (x) =>
                                  x.id === a.id
                                    ? { ...x, name: e.target.value }
                                    : x,
                              ),
                            })
                          }
                        />
                        {address(a.address, (next) =>
                          patch({
                            delivery_addresses: value.delivery_addresses.map(
                              (x) =>
                                x.id === a.id ? { ...x, address: next } : x,
                            ),
                          }),
                        )}
                        {!disabled && (
                          <Button
                            variant="ghost"
                            onClick={() =>
                              patch({
                                delivery_addresses:
                                  value.delivery_addresses.filter(
                                    (x) => x.id !== a.id,
                                  ),
                                default_delivery_id:
                                  value.default_delivery_id === a.id
                                    ? null
                                    : value.default_delivery_id,
                              })
                            }
                          >
                            {t("contactRemoveAddress")}
                          </Button>
                        )}
                      </fieldset>
                    ))}
                    {!disabled && (
                      <Button
                        variant="secondary"
                        disabled={value.delivery_addresses.length >= 20}
                        onClick={() =>
                          patch({
                            delivery_addresses: [
                              ...value.delivery_addresses,
                              {
                                id: crypto.randomUUID(),
                                name: "",
                                address: {
                                  street: "",
                                  city: "",
                                  postal_code: "",
                                  country: "CZ",
                                },
                              },
                            ],
                          })
                        }
                      >
                        {t("contactAddAddress")}
                      </Button>
                    )}
                  </>
                </section>
                {footer}
              </div>
            ) : null,
        },
      ]}
    />
  );
}
