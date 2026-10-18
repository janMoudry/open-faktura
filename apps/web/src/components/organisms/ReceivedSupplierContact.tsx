import { useEffect, useRef, useState } from "react";
import { Alert, Button, Dialog, Heading } from "@open-faktura/ui";
import {
  ContactFailure,
  createContact,
  getContact,
  listContacts,
  type Contact,
  type ContactData,
  type ReceivedData,
} from "@open-faktura/api-client";
import { InvoiceCustomerPicker } from "./InvoiceCustomerPicker";
import { ContactFields } from "./ContactFields";
import { useAresLookup } from "../../hooks/useAresLookup";
import { contactParty } from "../../utils/invoice";
import { contactError, emptyContact } from "../../utils/contact";
import { useAppTranslation, type AppKey } from "../../locales";
import { apiUrl } from "../../services/status";
function sameParty(a: ReceivedData["supplier"], b: ReceivedData["supplier"]) {
  return (
    ["name", "registration_number", "vat_number", "email", "phone"].every(
      (k) => a[k as keyof typeof a] === b[k as keyof typeof b],
    ) &&
    ["street", "city", "postal_code", "country"].every(
      (k) =>
        a.address[k as keyof typeof a.address] ===
        b.address[k as keyof typeof b.address],
    )
  );
}
function hasParty(p: ReceivedData["supplier"]) {
  return !!(
    p.name ||
    p.registration_number ||
    p.vat_number ||
    p.address.street ||
    p.address.city ||
    p.email ||
    p.phone
  );
}
export function ReceivedSupplierContact({
  companyId,
  data,
  disabled,
  onChange,
  onExpired,
}: {
  companyId: string;
  data: ReceivedData;
  disabled: boolean;
  onChange: (v: ReceivedData) => void;
  onExpired: () => void;
}) {
  const { t } = useAppTranslation();
  const [matches, setMatches] = useState<Contact[]>([]),
    [pending, setPending] = useState<Contact | null>(null),
    [creating, setCreating] = useState(false),
    [candidate, setCandidate] = useState<ContactData>(emptyContact),
    [busy, setBusy] = useState(false),
    [error, setError] = useState<AppKey | "">(""),
    [fields, setFields] = useState<string[]>([]),
    [saved, setSaved] = useState(false);
  const replay = useRef({ json: "", key: crypto.randomUUID() });
  const ares = useAresLookup(companyId, setCandidate, onExpired);
  useEffect(() => {
    const abort = new AbortController();
    setMatches([]);
    const timer = setTimeout(() => {
      const fetch = data.supplier_contact_id
        ? getContact(
            apiUrl,
            companyId,
            data.supplier_contact_id,
            abort.signal,
          ).then((c) => [c])
        : data.supplier.registration_number.trim()
          ? listContacts(
              apiUrl,
              companyId,
              {
                query: data.supplier.registration_number.trim(),
                archived: false,
                page: 1,
                page_size: 100,
              },
              abort.signal,
            ).then((p) =>
              p.items.filter(
                (c) =>
                  c.data.registration_number.trim() ===
                  data.supplier.registration_number.trim(),
              ),
            )
          : Promise.resolve([]);
      fetch
        .then((c) => {
          if (!abort.signal.aborted) setMatches(c);
        })
        .catch((e) => {
          if (
            !abort.signal.aborted &&
            e instanceof ContactFailure &&
            e.status === 401
          )
            onExpired();
        });
    }, 200);
    return () => {
      clearTimeout(timer);
      abort.abort();
    };
  }, [
    companyId,
    data.supplier_contact_id,
    data.supplier.registration_number,
    onExpired,
  ]);
  function apply(c: Contact, replace: boolean) {
    onChange({
      ...data,
      supplier_contact_id: c.id,
      ...(replace
        ? {
            supplier: contactParty(c),
            bank_account:
              data.kind === "receipt" ? "" : c.data.iban || c.data.bank_account,
          }
        : {}),
    });
    setPending(null);
  }
  function differs(c: Contact) {
    return (
      !sameParty(data.supplier, contactParty(c)) ||
      (data.kind !== "receipt" &&
        !!data.bank_account &&
        data.bank_account !== (c.data.iban || c.data.bank_account))
    );
  }
  function select(c: Contact) {
    if (hasParty(data.supplier) && differs(c)) setPending(c);
    else apply(c, true);
  }
  function openCreate() {
    setCandidate({
      ...emptyContact(),
      name: data.supplier.name,
      registration_number: data.supplier.registration_number,
      vat_number: data.supplier.vat_number,
      email: data.supplier.email,
      phone: data.supplier.phone,
      billing_address: { ...data.supplier.address },
      iban: /^[A-Z]{2}\d{2}/.test(data.bank_account) ? data.bank_account : "",
      bank_account: /^[A-Z]{2}\d{2}/.test(data.bank_account)
        ? ""
        : data.bank_account,
    });
    setFields([]);
    setError("");
    setCreating(true);
    setSaved(false);
  }
  async function create() {
    if (busy || disabled || ares.loading) return;
    setBusy(true);
    setError("");
    setFields([]);
    const json = JSON.stringify(candidate);
    if (replay.current.json !== json)
      replay.current = { json, key: crypto.randomUUID() };
    try {
      const c = await createContact(apiUrl, companyId, {
        request_id: replay.current.key,
        data: candidate,
      });
      onChange({ ...data, supplier_contact_id: c.id });
      setMatches([c]);
      setCreating(false);
      setSaved(true);
    } catch (e) {
      if (e instanceof ContactFailure) {
        setFields(e.fields);
        if (e.status === 401) onExpired();
      }
      setError(contactError(e));
    } finally {
      setBusy(false);
    }
  }
  const differing = matches.filter(differs);
  return (
    <div className="space-y-field">
      <InvoiceCustomerPicker
        label={t("receivedFindSupplier")}
        company={companyId}
        disabled={disabled || busy}
        onExpired={onExpired}
        onSelect={select}
      />
      {!data.supplier_contact_id && (
        <Button
          type="button"
          variant="secondary"
          disabled={disabled || busy}
          onClick={openCreate}
        >
          {t("inboxCreateSupplier")}
        </Button>
      )}
      {saved && <p role="status">{t("inboxSupplierCreated")}</p>}
      {!!matches.length && !data.supplier_contact_id && (
        <Alert tone="warning">
          {t("inboxSupplierExists")}
          <div className="flex flex-wrap gap-control">
            {matches.map((c) => (
              <Button
                type="button"
                key={c.id}
                variant="secondary"
                disabled={disabled}
                onClick={() => select(c)}
              >
                {t("inboxSelectSupplier")} · {c.data.name}
              </Button>
            ))}
          </div>
        </Alert>
      )}
      {data.supplier_contact_id && differing.length > 0 && (
        <Alert tone="warning">
          {t("inboxSupplierDifferent")}
          <Button
            type="button"
            variant="secondary"
            disabled={disabled}
            onClick={() => setPending(differing[0])}
          >
            {t("inboxCompareSupplier")}
          </Button>
        </Alert>
      )}
      <Dialog
        open={!!pending}
        onClose={() => setPending(null)}
        title={t("inboxCompareSupplier")}
        description={t("inboxSupplierDifferent")}
      >
        {pending && (
          <div className="space-y-field">
            <div className="grid gap-field sm:grid-cols-2">
              {[
                [
                  t("inboxSupplierFromDocument"),
                  data.supplier,
                  data.bank_account,
                ],
                [
                  t("inboxSupplierFromContact"),
                  contactParty(pending),
                  pending.data.iban || pending.data.bank_account,
                ],
              ].map(([label, party, bank]) => {
                const p = party as ReceivedData["supplier"];
                return (
                  <section key={label as string} className="space-y-control">
                    <Heading level={3}>{label as string}</Heading>
                    <p className="break-words">{p.name}</p>
                    <p>
                      {p.registration_number} · {p.vat_number}
                    </p>
                    <p className="break-words">
                      {p.address.street}
                      <br />
                      {p.address.postal_code} {p.address.city},{" "}
                      {p.address.country}
                    </p>
                    <p className="break-words">
                      {p.email} {p.phone}
                    </p>
                    {data.kind !== "receipt" && (
                      <p className="break-words">
                        {t("receivedBank")}: {bank as string}
                      </p>
                    )}
                  </section>
                );
              })}
            </div>
            <div className="flex flex-wrap gap-control">
              <Button
                type="button"
                disabled={disabled}
                onClick={() => apply(pending, false)}
              >
                {t("inboxKeepSupplierSnapshot")}
              </Button>
              <Button
                type="button"
                variant="secondary"
                disabled={disabled}
                onClick={() => apply(pending, true)}
              >
                {t("inboxUseSupplierContact")}
              </Button>
            </div>
          </div>
        )}
      </Dialog>
      <Dialog
        open={creating}
        onClose={() => {
          if (!busy) setCreating(false);
        }}
        title={t("inboxCreateSupplier")}
        description={t("inboxCreateSupplierReview")}
      >
        <div className="space-y-field">
          {error && (
            <Alert tone="danger" role="alert">
              {t(error)}
            </Alert>
          )}
          <ContactFields
            value={candidate}
            onChange={setCandidate}
            disabled={busy || disabled}
            fields={fields}
            ares={{
              loading: ares.loading,
              message: ares.message,
              lookup: () => void ares.lookup(candidate),
            }}
            footer={
              <Button
                type="button"
                disabled={busy || disabled || ares.loading}
                onClick={create}
              >
                {t("inboxConfirmCreateSupplier")}
              </Button>
            }
          />
        </div>
      </Dialog>
    </div>
  );
}
