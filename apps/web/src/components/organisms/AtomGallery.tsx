import { useState } from "react";
import { Heading } from "@open-faktura/ui";
import {
  Badge,
  Button,
  Card,
  Checkbox,
  Icon,
  IconButton,
  Input,
  Radio,
  Separator,
  Spinner,
  Switch,
  Textarea,
} from "@open-faktura/ui";

export function AtomGallery() {
  const [mixed, setMixed] = useState(true);
  return (
    <section aria-labelledby="atoms-title" className="my-8">
      <Heading level={2} size="section" id="atoms-title">
        Atomy · základní prvky
      </Heading>
      <p className="mt-2 mb-5 text-sm text-muted">
        Samostatné stavební prvky. Pole s popiskem, výběry a dialogy z nich
        skládáme níže.
      </p>
      <div className="grid items-start gap-group xl:grid-cols-3">
        <Card titleLevel={3} title="Button / IconButton">
          <div className="flex flex-wrap items-center gap-inline">
            <Button>Akce</Button>
            <Button variant="secondary">Další akce</Button>
            <IconButton icon="plus" label="Ukázková ikonová akce" />
          </div>
          <Separator />
          <div className="flex flex-wrap gap-inline">
            <Button disabled>Zakázaná akce</Button>
            <Button loading>Probíhá akce</Button>
            <IconButton
              label="Načítající se ikonová akce"
              icon="plus"
              loading
            />
          </div>
          <p className="mt-4 text-xs text-muted">
            Separator odděluje související skupiny.
          </p>
        </Card>
        <Card titleLevel={3} title="Input / Textarea">
          <div className="space-y-3">
            <Input
              aria-label="Samostatný input"
              placeholder="Základní textový vstup"
            />
            <Input
              aria-label="Chybný samostatný input"
              aria-invalid="true"
              aria-describedby="atom-input-error"
              defaultValue="Neplatná hodnota"
            />
            <p id="atom-input-error" className="text-xs text-danger">
              Ukázka chybového stavu vstupu.
            </p>
            <Input
              aria-label="Zakázaný samostatný input"
              disabled
              placeholder="Nedostupný vstup"
            />
            <Textarea
              aria-label="Samostatná textová oblast"
              placeholder="Víceřádkový vstup"
            />
          </div>
        </Card>
        <Card titleLevel={3} title="Checkbox / Radio / Switch">
          <Checkbox label="Označit položku" defaultChecked />
          <Checkbox label="Zakázaný checkbox" disabled />
          <Checkbox
            label="Částečně vybraná skupina"
            indeterminate={mixed}
            onChange={() => setMixed(false)}
          />
          <Separator />
          <fieldset>
            <legend className="mb-1 text-xs text-muted">
              Způsob doručení · jedna volba
            </legend>
            <Radio
              name="atom-delivery"
              value="email"
              label="E-mailem"
              defaultChecked
            />
            <Radio name="atom-delivery" value="download" label="Ke stažení" />
            <Radio
              name="atom-delivery"
              value="post"
              label="Poštou (nedostupné)"
              disabled
            />
          </fieldset>
          <Separator />
          <div className="flex flex-col gap-inline">
            <Switch label="Zapnout ukázkové oznámení" />
            <Switch label="Zakázaný přepínač" disabled />
            <Switch label="Zakázaný zapnutý přepínač" disabled defaultChecked />
          </div>
        </Card>
      </div>
      <div className="mt-6 rounded-panel border border-border bg-surface p-5">
        <Heading level={3} size="card" className="mb-4">
          Badge / Icon / Spinner
        </Heading>
        <div className="flex flex-wrap items-center gap-field">
          <Badge>Neutrální</Badge>
          <Badge tone="success">Úspěch</Badge>
          <Badge tone="warning">Upozornění</Badge>
          <Badge tone="danger">Chyba</Badge>
          <span className="flex items-center gap-inline text-sm">
            <Icon name="search" />
            Ikona s textem
          </span>
          <Spinner label="Načítání ukázky" />
        </div>
      </div>
    </section>
  );
}
