import { AresRegistration } from "../components/molecules/AresRegistration";
import { useCompanyAres, companyRegistryData } from "../hooks/useCompanyAres";
import { Link, useParams } from "react-router";
import {
  formatCalendarDate,
  Alert,
  Button,
  DatePicker,
  Field,
  Dialog,
  Icon,
  Badge,
  FilterTabs,
  Heading,
  Select,
} from "@open-faktura/ui";
import type { CompanySettings } from "@open-faktura/api-client";
import { useAppTranslation, type AppKey } from "../locales";
import { useWorkspace } from "../hooks/useCompanyWorkspace";

const labels: Record<keyof CompanySettings, AppKey> = {
  name: "companyName",
  entity_type: "entityType",
  registration_number: "registrationNumber",
  vat_number: "vatNumber",
  street: "street",
  city: "city",
  postal_code: "postalCode",
  country: "country",
  email: "email",
  phone: "phone",
  tax_mode: "taxMode",
  tax_effective_from: "taxEffective",
  currency: "currency",
  timezone: "timezone",
  maturity_days: "maturityDays",
  payment_method: "paymentMethod",
  iban: "iban",
  number_prefix: "numberPrefix",
  number_year: "numberYear",
  next_number: "nextNumber",
};
export function CompanyPage() {
  const { companyId = "" } = useParams();
  const { t, language } = useAppTranslation();
  const {
    onExpired,
    companies,
    company,
    settings,
    setSettings,
    loading,
    deleteTarget,
    setDeleteTarget,
    deleteName,
    setDeleteName,
    busy,
    error,
    fields,
    editing,
    setEditing,
    panel,
    setPanel,
    saved,
    setSaved,
    setAttempt,
    start,
    removeProfile,
    confirmDelete,
    save,
  } = useWorkspace();
  const ares = useCompanyAres(companyId, setSettings, onExpired);
  const field = (key: keyof CompanySettings, type = "text") => (
    <Field
      horizontal
      key={key}
      label={t(labels[key])}
      value={settings[key]}
      type={type}
      disabled={busy}
      error={fields.includes(key) ? t("companyFieldInvalid") : undefined}
      onChange={(e) => {
        setSaved(false);
        setSettings({
          ...settings,
          [key]:
            type === "number"
              ? e.target.value === ""
                ? 0
                : Number(e.target.value)
              : e.target.value,
        });
      }}
    />
  );
  const select = (
    key: keyof CompanySettings,
    options: { value: string; label: AppKey }[],
  ) => (
    <Select
      label={t(labels[key])}
      value={String(settings[key])}
      options={options.map((o) => ({ ...o, label: t(o.label) }))}
      disabled={busy}
      error={fields.includes(key) ? t("companyFieldInvalid") : undefined}
      onChange={(value) => {
        setSaved(false);
        setSettings({ ...settings, [key]: value });
      }}
    />
  );
  if (loading) return <p role="status">{t("companiesLoading")}</p>;
  const isForm =
    !!company && (!company.completed || editing) && company.role === "owner";
  const sectionTabs = (
    <div className="panel-tabs">
      <FilterTabs
        label={t("profileSections")}
        value={panel}
        onChange={setPanel}
        options={
          [
            { value: "identity", label: t("profileIdentityTab") },
            { value: "billing", label: t("profileBillingTab") },
            { value: "advanced", label: t("profileAdvanced") },
            {
              value: "branding",
              label: t("profileBranding"),
              disabled: true,
            },
          ] as {
            value: "identity" | "billing" | "advanced" | "branding";
            label: string;
            disabled?: boolean;
          }[]
        }
      />
    </div>
  );
  return (
    <>
      <div className="space-y-group">
        <div className="flex flex-wrap items-end justify-between gap-field">
          <div>
            <Heading level={1} size="page">
              {company
                ? company.settings.name || t("untitledCompany")
                : t("companiesTitle")}
            </Heading>
            <p className="mt-control text-sm text-muted">
              {t(company ? "companySettingsIntro" : "companiesIntro")}
            </p>
          </div>
          <div className="flex gap-inline">
            {!company && (
              <Button onClick={start} loading={busy}>
                <Icon name="plus" />
                {t("newCompany")}
              </Button>
            )}
            {company?.role === "owner" && (
              <Button
                variant="ghost"
                disabled={busy}
                onClick={() => confirmDelete(company)}
              >
                <Icon name="trash" />
                {t("deleteProfile")}
              </Button>
            )}
          </div>
        </div>
        {error && (
          <Alert tone="danger" role="alert">
            <p>{t(error)}</p>
            {fields.length > 0 && (
              <ul className="mt-control list-disc pl-5">
                {fields.map((f) => (
                  <li key={f}>
                    {f in labels
                      ? t(labels[f as keyof CompanySettings])
                      : t("companyFieldInvalid")}
                  </li>
                ))}
              </ul>
            )}
            {error === "companyConflict" && (
              <Button
                className="mt-control"
                variant="secondary"
                onClick={() => setAttempt((v) => v + 1)}
              >
                {t("reloadCompany")}
              </Button>
            )}
          </Alert>
        )}
        {!company &&
          !error &&
          (companies.length === 0 ? (
            <Alert>{t("noCompanies")}</Alert>
          ) : (
            <ul className="divide-y divide-border overflow-hidden rounded-control border border-border bg-surface">
              {companies.map((c) => (
                <li
                  key={c.id}
                  className="flex flex-wrap items-center justify-between gap-field p-form-inset"
                >
                  <div className="flex min-w-0 items-center gap-field">
                    <span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-control bg-primary-soft">
                      <Icon name="invoice" />
                    </span>
                    <div className="min-w-0">
                      <Link
                        className="break-words font-semibold hover:underline"
                        to={`/companies/${c.id}`}
                      >
                        {c.settings.name || t("untitledCompany")}
                      </Link>
                      <p className="mt-inline text-sm text-muted">
                        {t(
                          c.settings.entity_type === "individual"
                            ? "individual"
                            : "business",
                        )}{" "}
                        ·{" "}
                        {t(
                          c.role === "owner"
                            ? "roleOwner"
                            : c.role === "editor"
                              ? "roleEditor"
                              : "roleViewer",
                        )}
                      </p>
                    </div>
                  </div>
                  <div className="flex items-center gap-control">
                    <Badge>
                      {t(c.completed ? "companyReady" : "companyDraft")}
                    </Badge>
                    {c.role === "owner" && (
                      <Button
                        variant="ghost"
                        disabled={busy}
                        aria-label={t("deleteNamedProfile", {
                          name: c.settings.name || t("untitledCompany"),
                        })}
                        onClick={() => confirmDelete(c)}
                      >
                        <Icon name="trash" />
                      </Button>
                    )}
                  </div>
                </li>
              ))}
            </ul>
          ))}
        {!company && error && (
          <Button onClick={() => setAttempt((v) => v + 1)}>{t("retry")}</Button>
        )}
        {company && (
          <>
            {company.role !== "owner" && (
              <p className="text-sm text-muted">
                {t("companyRole", {
                  role: t(
                    company.role === "editor" ? "roleEditor" : "roleViewer",
                  ),
                })}
              </p>
            )}
            {saved && (
              <Alert tone="success" role="status">
                {t("companySaved")}
              </Alert>
            )}
            {isForm ? (
              <>
                <form
                  className="profile-form"
                  noValidate
                  onSubmit={(e) => {
                    e.preventDefault();
                    if (!ares.loading) void save(true);
                  }}
                >
                  {sectionTabs}
                  <section
                    hidden={panel !== "identity"}
                    className="profile-section"
                  >
                    <div>
                      <Heading level={2} size="card">
                        {t("profileIdentity")}
                      </Heading>
                      <p className="mt-inline text-sm text-muted">
                        {t("profileIdentityHint")}
                      </p>
                    </div>
                    <div className="profile-identity-fields">
                      <AresRegistration
                        compact
                        value={settings.registration_number}
                        disabled={busy}
                        error={
                          fields.includes("registration_number")
                            ? t("companyFieldInvalid")
                            : undefined
                        }
                        onChange={(registration_number) => {
                          ares.cancel();
                          setSaved(false);
                          setSettings({ ...settings, registration_number });
                        }}
                        ares={{
                          ...ares,
                          lookup: () => {
                            if (!busy && company?.role === "owner") {
                              setSaved(false);
                              void ares.lookup(companyRegistryData(settings));
                            }
                          },
                        }}
                      />
                      <div className="space-y-inline">
                        {field("name")}
                        {field("street")}
                        {field("city")}
                        {field("postal_code")}
                        {select("entity_type", [
                          { value: "individual", label: "individual" },
                          { value: "business", label: "business" },
                        ])}
                      </div>
                    </div>
                    <details
                      className="border-t border-border pt-field"
                      open={
                        fields.some((f) => ["email", "phone"].includes(f)) ||
                        undefined
                      }
                    >
                      <summary className="cursor-pointer text-sm font-semibold">
                        {t("profileContact")}
                      </summary>
                      <div className="profile-grid mt-field">
                        {field("email", "email")}
                        {field("phone", "tel")}
                      </div>
                    </details>
                  </section>
                  <section
                    hidden={panel !== "billing"}
                    className="profile-section"
                  >
                    <div>
                      <Heading level={2} size="card">
                        {t("profileBilling")}
                      </Heading>
                      <p className="mt-inline text-sm text-muted">
                        {t("profileBillingHint")}
                      </p>
                    </div>
                    <div className="profile-grid">
                      {select("tax_mode", [
                        { value: "non_payer", label: "nonPayer" },
                        { value: "vat_payer", label: "vatPayer" },
                      ])}
                      <DatePicker
                        label={t("taxEffective")}
                        value={settings.tax_effective_from}
                        disabled={busy}
                        error={
                          fields.includes("tax_effective_from")
                            ? t("companyFieldInvalid")
                            : undefined
                        }
                        onChange={(value) => {
                          setSaved(false);
                          setSettings({
                            ...settings,
                            tax_effective_from: value,
                          });
                        }}
                      />
                      {(settings.tax_mode === "vat_payer" ||
                        settings.vat_number ||
                        fields.includes("vat_number")) &&
                        field("vat_number")}
                      {select("payment_method", [
                        { value: "cash", label: "cash" },
                        { value: "bank_transfer", label: "bankTransfer" },
                      ])}
                      {field("maturity_days", "number")}
                      {(settings.payment_method === "bank_transfer" ||
                        settings.iban ||
                        fields.includes("iban")) &&
                        field("iban")}
                    </div>
                  </section>
                  <section
                    hidden={panel !== "advanced"}
                    className="profile-section"
                  >
                    <Heading level={2} size="card">
                      {t("profileAdvanced")}
                    </Heading>
                    <p className="mt-control text-sm text-muted">
                      {t("profileAdvancedHint")}
                    </p>
                    <div className="profile-grid mt-field">
                      {field("number_prefix")}
                      {field("number_year", "number")}
                      {field("next_number", "number")}
                      {select("country", [
                        { value: "CZ", label: "czechRepublic" },
                      ])}
                      {select("currency", [{ value: "CZK", label: "czk" }])}
                      {field("timezone")}
                      <p className="text-sm text-muted sm:col-span-2">
                        {t("numberPreview", {
                          number: `${settings.number_prefix}${settings.number_year}${String(settings.next_number).padStart(6, "0")}`,
                        })}
                      </p>
                    </div>
                  </section>
                  <div className="profile-actions">
                    <Button
                      type="submit"
                      loading={busy}
                      disabled={ares.loading}
                    >
                      {t("finishCompany")}
                    </Button>
                    <Button
                      variant="ghost"
                      disabled={busy}
                      onClick={() => {
                        if (!ares.loading) void save();
                      }}
                    >
                      {t("saveDraft")}
                    </Button>
                  </div>
                </form>
              </>
            ) : (
              <>
                <Alert>
                  {t(
                    company.completed
                      ? "emptyWorkspace"
                      : "companyIncompleteReadOnly",
                  )}
                </Alert>
                <div className="profile-form">
                  {sectionTabs}
                  <section className="profile-section">
                    <Heading level={2} size="card">
                      {t(
                        panel === "identity"
                          ? "profileIdentity"
                          : panel === "billing"
                            ? "profileBilling"
                            : "profileAdvanced",
                      )}
                    </Heading>
                    <Summary settings={company.settings} panel={panel} />
                  </section>
                  {company.role === "owner" && (
                    <div className="profile-actions">
                      <Button
                        variant="secondary"
                        onClick={() => {
                          setEditing(true);
                          setSaved(false);
                        }}
                      >
                        <Icon name="edit" />
                        {t("editCompany")}
                      </Button>
                    </div>
                  )}
                </div>
                {company.tax_history.length > 0 && (
                  <details>
                    <summary className="cursor-pointer text-sm text-muted">
                      {t("taxHistory")}
                    </summary>
                    <ul className="mt-control space-y-inline text-sm">
                      {company.tax_history.map((p) => (
                        <li key={p.revision}>
                          {formatCalendarDate(p.effective_from, language)} ·{" "}
                          {t(p.mode === "vat_payer" ? "vatPayer" : "nonPayer")}
                        </li>
                      ))}
                    </ul>
                  </details>
                )}
              </>
            )}
          </>
        )}
      </div>
      <Dialog
        open={!!deleteTarget}
        onClose={() => {
          if (!busy) setDeleteTarget(null);
        }}
        title={t("deleteProfile")}
        description={t("deleteProfileHint")}
      >
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void removeProfile();
          }}
          className="space-y-field"
        >
          <p className="break-words font-semibold">
            {deleteTarget?.settings.name || t("untitledCompany")}
          </p>
          <Field
            label={t("deleteConfirmName")}
            value={deleteName}
            disabled={busy}
            onChange={(e) => setDeleteName(e.target.value)}
          />
          {error && (
            <Alert tone="danger" role="alert">
              {t(error)}
            </Alert>
          )}
          <div className="flex justify-end gap-control">
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => setDeleteTarget(null)}
            >
              {t("cancelAction")}
            </Button>
            <Button
              type="submit"
              variant="danger"
              loading={busy}
              disabled={
                deleteName !==
                (deleteTarget?.settings.name || t("untitledCompany"))
              }
            >
              {t("deleteProfile")}
            </Button>
          </div>
        </form>
      </Dialog>
    </>
  );
}
function Summary({
  settings,
  panel,
}: {
  settings: CompanySettings;
  panel: string;
}) {
  const { t, language } = useAppTranslation();
  const values: Partial<Record<keyof CompanySettings, AppKey>> = {
    entity_type:
      settings.entity_type === "business" ? "business" : "individual",
    tax_mode: settings.tax_mode === "vat_payer" ? "vatPayer" : "nonPayer",
    payment_method:
      settings.payment_method === "bank_transfer" ? "bankTransfer" : "cash",
    country: "czechRepublic",
  };
  return (
    <dl className="profile-facts">
      {(Object.keys(labels) as (keyof CompanySettings)[])
        .filter((key) => {
          const identity = [
            "name",
            "entity_type",
            "registration_number",
            "street",
            "city",
            "postal_code",
            "email",
            "phone",
          ];
          const billing = [
            "tax_mode",
            "tax_effective_from",
            "vat_number",
            "payment_method",
            "maturity_days",
            "iban",
          ];
          return panel === "identity"
            ? identity.includes(key)
            : panel === "billing"
              ? billing.includes(key)
              : !identity.includes(key) && !billing.includes(key);
        })
        .map((key) => (
          <div key={key} className="min-w-0">
            <dt className="text-sm text-muted">{t(labels[key])}</dt>
            <dd className="min-w-0 break-words text-sm">
              {values[key]
                ? t(values[key]!)
                : key === "tax_effective_from" && settings[key]
                  ? formatCalendarDate(settings[key], language)
                  : String(settings[key]) || t("notProvided")}
            </dd>
          </div>
        ))}
    </dl>
  );
}
