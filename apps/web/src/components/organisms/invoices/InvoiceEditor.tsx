import { Heading } from "@open-faktura/ui";
import { useState, type ReactNode } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Field,
  Autocomplete,
  DatePicker,
  Icon,
  IconButton,
  TextareaField,
} from "@open-faktura/ui";
import type { DemoDraft, DemoLine } from "../../../types/invoice-demo";
import {
  displayDate,
  draftTotal,
  lineTotal,
  money,
} from "../../../services/invoice-demo";

function Section({
  number,
  title,
  children,
}: {
  number: string;
  title: string;
  children: ReactNode;
}) {
  return (
    <Card>
      <Heading
        level={2}
        size="card"
        className="mb-5 flex items-center gap-control"
      >
        <span className="flex h-6 w-6 items-center justify-center rounded-full bg-primary-soft text-xs font-normal text-primary">
          {number}
        </span>
        {title}
      </Heading>
      {children}
    </Card>
  );
}
export function InvoiceEditor({
  draft,
  onChange,
  onSave,
}: {
  draft: DemoDraft;
  onChange: (draft: DemoDraft) => void;
  onSave: () => void;
}) {
  const [preview, setPreview] = useState(false);
  const [error, setError] = useState("");
  const total = draftTotal(draft);
  const update = (key: keyof DemoDraft, value: string) =>
    onChange({ ...draft, [key]: value });
  const changeLine = (id: string, key: keyof DemoLine, value: string) =>
    onChange({
      ...draft,
      lines: draft.lines.map((l) => (l.id === id ? { ...l, [key]: value } : l)),
    });
  function save() {
    if (
      !draft.customer.trim() ||
      !draft.address.trim() ||
      !draft.issuedOn ||
      !draft.dueOn ||
      !draft.lines.every((l) => l.title.trim()) ||
      total === null
    ) {
      setError(
        "Vyplňte zákazníka, adresu, data a názvy položek. Množství musí být kladné a ceny nezáporné; nejvýše 2 desetinná místa.",
      );
      return;
    }
    if (draft.dueOn < draft.issuedOn) {
      setError("V tomto návrhu zvolte splatnost nejdříve v den vystavení.");
      return;
    }
    setError("");
    onSave();
  }
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        save();
      }}
    >
      <div className="grid items-start gap-group xl:grid-cols-3">
        <div className="space-y-group xl:col-span-2">
          <Section number="1" title="Komu fakturujete?">
            <div className="grid gap-field sm:grid-cols-2">
              <Autocomplete
                label="Zákazník"
                required
                value={draft.customer}
                onChange={(value) => update("customer", value)}
                placeholder="Název firmy nebo jméno"
                options={["Studio Sever", "Ateliér Forma", "Dobré místo"].map(
                  (label) => ({ value: label, label }),
                )}
              />
              <Field
                label="Fakturační adresa"
                required
                value={draft.address}
                onChange={(e) => update("address", e.target.value)}
                placeholder="Ulice, město a PSČ"
              />
            </div>
            <p className="mt-4 text-xs text-muted">
              Dodavatel:{" "}
              <span className="font-medium text-foreground">
                Studio Nový den
              </span>{" "}
              · Neplátce DPH
            </p>
          </Section>
          <Section number="2" title="Údaje dokladu">
            <div className="grid grid-cols-1 gap-field sm:grid-cols-2">
              <DatePicker
                label="Datum vystavení"
                required
                value={draft.issuedOn}
                onChange={(value) => update("issuedOn", value)}
              />
              <DatePicker
                label="Datum splatnosti"
                required
                value={draft.dueOn}
                onChange={(value) => update("dueOn", value)}
              />
            </div>
            <div className="mt-5 flex flex-wrap gap-x-6 gap-y-2 border-t border-border pt-4 text-xs text-muted">
              <span>
                Měna <b className="ml-2 font-medium text-foreground">CZK</b>
              </span>
              <span>
                Úhrada{" "}
                <b className="ml-2 font-medium text-foreground">
                  Bankovní převod
                </b>
              </span>
              <span>
                Číslo{" "}
                <b className="ml-2 font-medium text-foreground">
                  Až při vystavení
                </b>
              </span>
            </div>
          </Section>
          <Section number="3" title="Za co fakturujete?">
            <div className="space-y-group">
              {draft.lines.map((line, index) => (
                <div key={line.id} className="border-b border-border pb-5">
                  <div className="mb-3 flex items-end gap-inline">
                    <div className="flex-1">
                      <Field
                        label={`Položka ${index + 1}`}
                        value={line.title}
                        required
                        onChange={(e) =>
                          changeLine(line.id, "title", e.target.value)
                        }
                        placeholder="Název služby nebo produktu"
                      />
                    </div>
                    <IconButton
                      icon="trash"
                      variant="ghost"
                      label={`Odebrat položku ${index + 1}`}
                      disabled={draft.lines.length === 1}
                      onClick={() =>
                        onChange({
                          ...draft,
                          lines: draft.lines.filter((l) => l.id !== line.id),
                        })
                      }
                    />
                  </div>
                  <div className="grid grid-cols-2 items-end gap-control sm:grid-cols-4">
                    <Field
                      label={`Množství ${index + 1}`}
                      inputMode="decimal"
                      value={line.quantity}
                      onChange={(e) =>
                        changeLine(line.id, "quantity", e.target.value)
                      }
                      aria-invalid={lineTotal(line) === null}
                      aria-describedby={
                        lineTotal(line) === null
                          ? `line-error-${line.id}`
                          : undefined
                      }
                    />
                    <Field
                      label={`Jednotka ${index + 1}`}
                      value={line.unit}
                      onChange={(e) =>
                        changeLine(line.id, "unit", e.target.value)
                      }
                    />
                    <Field
                      label={`Cena za jednotku ${index + 1}`}
                      inputMode="decimal"
                      value={line.price}
                      onChange={(e) =>
                        changeLine(line.id, "price", e.target.value)
                      }
                      aria-invalid={lineTotal(line) === null}
                      aria-describedby={
                        lineTotal(line) === null
                          ? `line-error-${line.id}`
                          : undefined
                      }
                    />
                    <div className="pb-2 text-right">
                      <p className="mb-3 text-xs text-muted">Celkem</p>
                      <p className="text-sm font-semibold tabular-nums">
                        {lineTotal(line) === null
                          ? "—"
                          : money(lineTotal(line)!)}
                      </p>
                    </div>
                  </div>
                  {lineTotal(line) === null && (
                    <p
                      id={`line-error-${line.id}`}
                      className="mt-3 text-xs text-danger"
                    >
                      Zadejte kladné množství a nezápornou cenu, nejvýše 2
                      desetinná místa.
                    </p>
                  )}
                </div>
              ))}
            </div>
            <Button
              type="button"
              variant="ghost"
              className="mt-3"
              onClick={() =>
                onChange({
                  ...draft,
                  lines: [
                    ...draft.lines,
                    {
                      id: crypto.randomUUID(),
                      title: "",
                      quantity: "1",
                      unit: "ks",
                      price: "0",
                    },
                  ],
                })
              }
            >
              <Icon name="plus" />
              Přidat položku
            </Button>
          </Section>
          <Card>
            <TextareaField
              label="Poznámka pro zákazníka"
              rows={2}
              value={draft.note}
              onChange={(e) => update("note", e.target.value)}
            />
          </Card>
        </div>
        <aside className="space-y-4 xl:sticky xl:top-6">
          <div className="overflow-hidden rounded-xl border border-border bg-surface">
            <div className="border-b border-border p-5">
              <div className="flex items-center justify-between">
                <Heading level={2} size="card">
                  Souhrn faktury
                </Heading>
                <Badge>Koncept</Badge>
              </div>
              <p className="mt-2 text-xs text-muted">Faktura bez DPH · CZK</p>
            </div>
            <div className="space-y-4 p-5">
              <div className="flex justify-between text-sm">
                <span className="text-muted">Položky</span>
                <span>{draft.lines.length}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted">Splatnost</span>
                <span>{displayDate(draft.dueOn)}</span>
              </div>
              <div className="border-t border-border pt-5">
                <p className="text-xs text-muted">Celkem k úhradě</p>
                <p
                  aria-live="polite"
                  data-testid="draft-total"
                  className="mt-2 text-3xl font-medium tracking-tight text-primary tabular-nums"
                >
                  {total === null ? "Zkontrolujte položky" : money(total)}
                </p>
                <p className="mt-2 text-xs text-muted">
                  Dodavatel není plátcem DPH.
                </p>
              </div>
              <Button
                type="button"
                variant="secondary"
                className="w-full"
                onClick={() => setPreview(!preview)}
                aria-expanded={preview}
              >
                <Icon name="eye" />
                {preview ? "Skrýt náhled" : "Náhled dokladu"}
              </Button>
              <Button type="submit" className="w-full">
                <Icon name="check" />
                Uložit ukázkový koncept
              </Button>
            </div>
          </div>
          <p className="px-2 text-xs leading-relaxed text-muted">
            Koncept se uloží jen do této otevřené aplikace. Obnovením stránky se
            smaže. Vystavení, odeslání a PDF zatím nejsou připojené.
          </p>
          {error && (
            <Alert tone="danger" role="alert">
              {error}
            </Alert>
          )}
          {preview && (
            <article
              aria-label="Náhled dokladu"
              className="rounded-lg border border-border bg-surface p-6 shadow-paper"
            >
              <p className="text-xs uppercase tracking-widest text-muted">
                Ukázka · není účetní doklad
              </p>
              <Heading level={2} size="card" className="mt-5">
                Faktura
              </Heading>
              <p className="mt-1 text-xs text-muted">Koncept / bez čísla</p>
              <div className="my-5 border-y border-border py-4 text-xs leading-relaxed">
                <p className="font-semibold">
                  Studio Nový den → {draft.customer || "Váš zákazník"}
                </p>
                <p className="mt-2 text-muted">
                  {draft.address || "Fakturační adresa"}
                </p>
              </div>
              {draft.lines.map((l) => (
                <div
                  key={l.id}
                  className="mb-3 flex justify-between gap-field text-xs"
                >
                  <span>
                    {l.title || "Nová položka"}
                    <span className="mt-1 block text-muted">
                      {l.quantity} {l.unit}
                    </span>
                  </span>
                  <span className="whitespace-nowrap">
                    {lineTotal(l) === null ? "—" : money(lineTotal(l)!)}
                  </span>
                </div>
              ))}
              <p className="mt-5 border-t border-primary pt-4 text-right font-semibold text-primary">
                {total === null ? "—" : money(total)}
              </p>
              <p className="mt-5 text-xs text-muted">{draft.note}</p>
            </article>
          )}
        </aside>
      </div>
    </form>
  );
}
