import { Heading } from "@open-faktura/ui";
import { useState } from "react";
import {
  Autocomplete,
  Card,
  Checkbox,
  DatePicker,
  Select,
} from "@open-faktura/ui";
const customers = [
  {
    value: "sever",
    label: "Studio Sever",
    description: "Ukázkový kontakt · Praha",
  },
  {
    value: "forma",
    label: "Ateliér Forma",
    description: "Ukázkový kontakt · Brno",
  },
  {
    value: "misto",
    label: "Dobré místo",
    description: "Ukázkový kontakt · Olomouc",
  },
];
export function SelectionGallery() {
  const [customer, setCustomer] = useState("");
  const [loading, setLoading] = useState(false);
  const [date, setDate] = useState("2026-10-10");
  return (
    <section aria-labelledby="selection-title" className="my-8">
      <Heading level={2} size="section" id="selection-title">
        Molekuly · výběr a zadávání
      </Heading>
      <p className="mt-2 mb-5 text-sm text-muted">
        Vlastní nabídky a kalendář. Stejné komponenty používají filtry faktur i
        editor.
      </p>
      <div className="grid items-start gap-group xl:grid-cols-3">
        <Card titleLevel={3} title="Select">
          <div className="space-y-group">
            <Select
              label="Způsob platby"
              placeholder="Vyberte způsob"
              options={[
                {
                  value: "bank",
                  label: "Bankovní převod",
                  description: "Platba na účet",
                },
                { value: "cash", label: "Hotově" },
                { value: "card", label: "Kartou", disabled: true },
              ]}
              hint="Šipky pro pohyb, Enter pro výběr, Escape pro zavření."
            />
            <Select
              label="Povinný výběr"
              required
              error="Vyberte jednu možnost."
              options={[
                { value: "cz", label: "Česká republika" },
                { value: "sk", label: "Slovensko" },
              ]}
              placeholder="Vyberte zemi"
            />
            <Select
              label="Zakázaný výběr"
              disabled
              defaultValue="czk"
              options={[{ value: "czk", label: "Česká koruna" }]}
            />
          </div>
        </Card>
        <Card titleLevel={3} title="DatePicker">
          <div className="space-y-group">
            <DatePicker
              label="Ukázkové datum"
              value={date}
              onChange={setDate}
              hint="Datum zadejte klávesnicí nebo vyberte v kalendáři."
            />
            <p className="text-xs text-muted">
              Vybráno: <output>{date || "Bez data"}</output>
            </p>
            <DatePicker
              label="Omezený rozsah"
              defaultValue="2026-10-15"
              min="2026-10-10"
              max="2026-10-20"
              hint="Povoleno 10.–20. října 2026."
            />
            <DatePicker
              label="Datum pouze pro čtení"
              value="2026-10-10"
              readOnly
            />
            <DatePicker
              label="Chybějící datum"
              value=""
              required
              error="Vyberte datum před pokračováním."
            />
            <DatePicker
              label="Zakázané datum"
              defaultValue="2026-10-10"
              disabled
            />
          </div>
        </Card>
        <Card titleLevel={3} title="Autocomplete">
          <div className="space-y-group">
            <Autocomplete
              label="Vyhledat kontakt"
              options={customers}
              value={customer}
              onChange={setCustomer}
              loading={loading}
              placeholder="Například atelier"
              hint="Hledání ignoruje diakritiku. Lze zadat i vlastní text."
            />
            <Checkbox
              label="Simulovat načítání návrhů"
              checked={loading}
              onChange={(e) => setLoading(e.target.checked)}
            />
            <Autocomplete
              label="Prázdné návrhy"
              options={[]}
              placeholder="Zadejte vlastní hodnotu"
            />
            <Autocomplete
              label="Chyba našeptávání"
              options={customers}
              error="Kontakt se nepodařilo ověřit. Zkuste jej vyhledat znovu."
            />
            <Autocomplete
              label="Zakázané našeptávání"
              options={customers}
              disabled
            />
          </div>
        </Card>
      </div>
    </section>
  );
}
