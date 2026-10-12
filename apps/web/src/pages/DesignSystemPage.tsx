import { FoundationGallery } from "../components/organisms/FoundationGallery";
import { Heading } from "@open-faktura/ui";
import { AtomGallery } from "../components/organisms/AtomGallery";
import { SelectionGallery } from "../components/organisms/SelectionGallery";
import { lazy, Suspense, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Checkbox,
  Dialog,
  EmptyState,
  Field,
  FilterTabs,
  Icon,
  IconButton,
  SelectField,
  DatePicker,
  TextareaField,
  Table,
  TableHeader,
  TableRow,
  TableHead,
  TableCell,
} from "@open-faktura/ui";
import { WorkspaceLayout } from "../components/templates/WorkspaceLayout";

const ChartGallery = lazy(() => import("../components/organisms/ChartGallery"));

export function DesignSystemPage() {
  const [loading, setLoading] = useState(false);
  const [open, setOpen] = useState(false);
  const [tab, setTab] = useState("all");
  const [name, setName] = useState("");
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  return (
    <WorkspaceLayout active="design-system">
      <div className="mb-8 flex flex-wrap items-end justify-between gap-field">
        <div>
          <p className="mb-2 text-xs uppercase tracking-widest text-muted">
            Základy / Komponenty
          </p>
          <Heading level={1} size="page">
            Jeden systém. Jeden jazyk.
          </Heading>
          <p className="mt-3 max-w-2xl text-sm leading-relaxed text-muted">
            Monochromatický základ se stavovými barvami. Stejné prvky používá
            seznam faktur i editor.
          </p>
        </div>
        <Badge tone="success">Design system · v0.1</Badge>
      </div>
      <div className="mb-6 grid gap-group xl:grid-cols-3">
        <Card title="Typografie" className="xl:col-span-2">
          <p className="font-display text-2xl font-semibold">
            Dobře odvedená práce si zaslouží dobrou fakturu.
          </p>
          <p className="mt-4 text-sm leading-relaxed">
            Manrope pro nadpisy, ovládání i data. Jednotná typografie napříč
            aplikací.
            <br />
            Příliš žluťoučký kůň úpěl ďábelské ódy.
          </p>
          <p className="mt-5 text-2xl font-semibold tabular-nums text-primary">
            123 456,78 Kč
          </p>
          <p className="mt-2 text-xs text-muted">
            Částky mají tabulární číslice a zarovnání doprava.
          </p>
        </Card>
        <Card title="Barvy a význam">
          <div className="space-y-3">
            {[
              { label: "Primární akce", color: "bg-primary" },
              { label: "Text", color: "bg-foreground" },
              { label: "Pomocný text", color: "bg-muted" },
              { label: "Úspěch / uhrazeno", color: "bg-success" },
              { label: "Upozornění", color: "bg-warning" },
              { label: "Chyba", color: "bg-danger" },
            ].map((c) => (
              <div
                key={c.label}
                className="flex items-center gap-control text-sm"
              >
                <span className={`h-6 w-6 rounded-md ${c.color}`} />
                {c.label}
              </div>
            ))}
          </div>
        </Card>
      </div>
      <FoundationGallery />
      <AtomGallery />
      <SelectionGallery />
      <Heading level={2} size="section" className="mb-5">
        Složené komponenty · stavy a interakce
      </Heading>
      <div className="grid grid-cols-1 items-start gap-group xl:grid-cols-2">
        <div className="min-w-0 space-y-group">
          <Card title="Akce a tlačítka">
            <div className="flex flex-wrap gap-control">
              <Button onClick={() => setNotice("Primární akce byla spuštěna.")}>
                <Icon name="plus" />
                Primární
              </Button>
              <Button variant="secondary" onClick={() => setOpen(true)}>
                Sekundární
              </Button>
              <Button
                variant="ghost"
                onClick={() => setNotice("Doplňková akce byla spuštěna.")}
              >
                Doplňková
              </Button>
              <Button variant="danger" onClick={() => setOpen(true)}>
                <Icon name="trash" />
                Destruktivní
              </Button>
            </div>
            <div className="mt-4 flex flex-wrap items-center gap-control">
              <Button
                size="sm"
                variant="secondary"
                onClick={() => setNotice("Kompaktní akce byla spuštěna.")}
              >
                Kompaktní
              </Button>
              <IconButton
                label="Otevřít ukázkový náhled"
                icon="eye"
                variant="secondary"
                onClick={() => setOpen(true)}
              />
              <Button disabled>Nedostupné</Button>
              <Button
                loading={loading}
                onClick={() => setNotice("Ukázková akce je připravená.")}
              >
                {loading ? "Ukládání…" : "Připraveno"}
              </Button>
            </div>
            <div className="mt-4 border-t border-border pt-3">
              <Checkbox
                label="Ukázat stav načítání"
                checked={loading}
                onChange={(e) => setLoading(e.target.checked)}
              />
            </div>
          </Card>
          <Card title="Pole a validace">
            <form
              noValidate
              onSubmit={(e) => {
                e.preventDefault();
                if (!name.trim()) {
                  setError("Vyplňte název zákazníka.");
                  e.currentTarget.querySelector("input")?.focus();
                  return;
                }
                setError("");
                setNotice(
                  `Ukázkový formulář: ${name}. Údaje se nikam neodesílají.`,
                );
              }}
              className="min-w-0 space-y-group"
            >
              <Field
                label="Název zákazníka"
                help="Název se zobrazí na dokladu. Nápovědu zobrazíte najetím na ikonu nebo jejím zaostřením klávesnicí; Escape ji zavře."
                required
                placeholder="Například Studio Sever"
                value={name}
                onChange={(e) => {
                  setName(e.target.value);
                  if (error) setError("");
                }}
                hint="Název firmy nebo jméno podnikatele."
                error={error}
              />
              <div className="grid gap-field sm:grid-cols-2">
                <SelectField
                  label="Měna"
                  defaultValue="CZK"
                  options={[
                    { value: "CZK", label: "CZK · Česká koruna" },
                    { value: "EUR", label: "EUR · Euro" },
                    {
                      value: "USD",
                      label: "USD · Americký dolar",
                      disabled: true,
                    },
                  ]}
                />
                <DatePicker label="Datum vystavení" defaultValue="2026-10-10" />
              </div>
              <div className="grid gap-field sm:grid-cols-2">
                <Field label="Pouze pro čtení" value="2026-042" readOnly />
                <Field
                  label="Nedostupné pole"
                  value="Přebírá se z nastavení"
                  disabled
                />
              </div>
              <TextareaField
                label="Poznámka"
                placeholder="Prostor pro delší text…"
                hint="Výšku pole lze upravit."
              />
              <Checkbox label="Ukázková volba" defaultChecked />
              <Checkbox label="Nedostupná volba" disabled />
              <Button type="submit">Ověřit formulář</Button>
            </form>
          </Card>
        </div>
        <div className="min-w-0 space-y-group">
          <Card title="Stavy a zpětná vazba">
            <div className="mb-5 flex flex-wrap gap-inline">
              <Badge>Koncept</Badge>
              <Badge tone="success">Uhrazeno</Badge>
              <Badge tone="warning">Částečně uhrazeno</Badge>
              <Badge tone="danger">Po splatnosti</Badge>
            </div>
            <div className="space-y-3">
              <Alert>Informace zůstávají srozumitelné i bez barev.</Alert>
              <Alert tone="success">Koncept je uložený.</Alert>
              <Alert tone="warning">Zkontrolujte datum splatnosti.</Alert>
              <Alert tone="danger">
                Doklad se nepodařilo uložit. Zadaná data zůstala zachovaná.
              </Alert>
            </div>
          </Card>
          <Card title="Filtry a prázdný stav">
            <FilterTabs
              label="Ukázkové filtry"
              value={tab}
              onChange={setTab}
              options={[
                { value: "all", label: "Všechny", count: 0 },
                { value: "draft", label: "Koncepty", count: 0 },
                {
                  value: "archived",
                  label: "Archiv (nedostupný)",
                  disabled: true,
                },
              ]}
            />
            <EmptyState
              title={
                tab === "all"
                  ? "Zatím tu nejsou žádné doklady"
                  : "Žádné rozpracované koncepty"
              }
              description="Prázdná stránka vysvětlí, co chybí, a nabídne další krok."
              action={
                <Button variant="secondary" onClick={() => setOpen(true)}>
                  <Icon name="plus" />
                  Vytvořit ukázku
                </Button>
              }
            />
          </Card>
          <Card title="Tabulky">
            <div
              className="overflow-x-auto"
              role="region"
              aria-label="Ukázková tabulka"
              tabIndex={0}
            >
              <Table aria-label="Ukázková tabulka">
                <TableHeader>
                  <TableRow>
                    <TableHead>Doklad</TableHead>
                    <TableHead>Stav</TableHead>
                    <TableHead className="text-right">Částka</TableHead>
                  </TableRow>
                </TableHeader>
                <tbody>
                  <TableRow>
                    <TableCell>2026-042</TableCell>
                    <TableCell>
                      <Badge tone="success">Uhrazeno</Badge>
                    </TableCell>
                    <TableCell className="whitespace-nowrap text-right tabular-nums">
                      12 500,00 Kč
                    </TableCell>
                  </TableRow>
                  <TableRow>
                    <TableCell>2026-043</TableCell>
                    <TableCell>
                      <Badge>Koncept</Badge>
                    </TableCell>
                    <TableCell className="whitespace-nowrap text-right tabular-nums">
                      8 250,00 Kč
                    </TableCell>
                  </TableRow>
                </tbody>
              </Table>
            </div>
            <p className="mt-4 text-xs text-muted">
              Na úzkých obrazovkách mají rozsáhlé tabulky vlastní posuv. Seznam
              i na mobilu zachovává zarovnání sloupců.
            </p>
          </Card>
          <Card title="Ikony Lucide">
            <div className="grid grid-cols-4 gap-control">
              {(
                [
                  "invoice",
                  "search",
                  "plus",
                  "check",
                  "settings",
                  "eye",
                  "trash",
                  "back",
                ] as const
              ).map((icon) => (
                <div
                  key={icon}
                  className="flex min-h-20 flex-col items-center justify-center gap-inline rounded-lg bg-canvas text-primary"
                >
                  <Icon name={icon} className="h-5 w-5" />
                  <span className="text-xs text-muted">{icon}</span>
                </div>
              ))}
            </div>
            <p className="mt-4 text-xs leading-relaxed text-muted">
              16 px v ovládání, 20–24 px pro větší kontext. Ikonové tlačítko
              vždy dostává přístupný název.
            </p>
          </Card>
        </div>
      </div>
      <Suspense
        fallback={<p className="mt-8 text-sm text-muted">Načítání grafů…</p>}
      >
        <ChartGallery />
      </Suspense>
      {notice && (
        <div className="mt-6">
          <Alert tone="success" role="status">
            {notice}
          </Alert>
        </div>
      )}
      <Dialog
        open={open}
        onClose={() => setOpen(false)}
        title="Ukázkový dialog"
        description="Dialog drží focus uvnitř, podporuje Escape a po zavření vrací focus na původní tlačítko."
      >
        <div className="flex flex-wrap justify-end gap-control">
          <Button variant="secondary" onClick={() => setOpen(false)}>
            Zrušit
          </Button>
          <Button
            onClick={() => {
              setOpen(false);
              setNotice("Ukázka byla potvrzena. Žádný doklad se nezměnil.");
            }}
          >
            Potvrdit ukázku
          </Button>
        </div>
      </Dialog>
    </WorkspaceLayout>
  );
}
