import { Card, Heading } from "@open-faktura/ui";

const spacing = [
  {
    token: "inline",
    value: "8 px",
    usage: "Ikona a text, kompaktní prvky",
    width: "w-inline",
  },
  {
    token: "control",
    value: "12 px",
    usage: "Související ovládací prvky",
    width: "w-control",
  },
  {
    token: "field",
    value: "16 px",
    usage: "Pole formuláře, menší obsah",
    width: "w-field",
  },
  {
    token: "group",
    value: "24 px",
    usage: "Skupiny polí a karty v mřížce",
    width: "w-group",
  },
  {
    token: "section",
    value: "32 px",
    usage: "Oddělení sekcí obrazovky",
    width: "w-section",
  },
  {
    token: "page",
    value: "48 px",
    usage: "Výraznější členění stránky",
    width: "w-page",
  },
] as const;

export function FoundationGallery() {
  return (
    <section aria-labelledby="foundation-title" className="my-section">
      <Heading id="foundation-title">Základy · nadpisy a rozestupy</Heading>
      <p className="mt-inline mb-group text-sm text-muted">
        Jedna škála napříč aplikací. HTML úroveň nadpisu určuje strukturu;
        velikost určuje jeho vizuální roli.
      </p>
      <div className="grid gap-group xl:grid-cols-2">
        <Card titleLevel={3} title="Hierarchie nadpisů">
          <div className="space-y-group">
            <div>
              <Heading level={3} size="page">
                Nadpis stránky
              </Heading>
              <p className="mt-inline text-xs text-muted">
                page · 32 px, na mobilu 24 px · řádkování 1,2
              </p>
            </div>
            <div>
              <Heading level={3} size="section">
                Nadpis sekce
              </Heading>
              <p className="mt-inline text-xs text-muted">
                section · 24 px · řádkování 1,3
              </p>
            </div>
            <div>
              <Heading level={3} size="card">
                Nadpis karty
              </Heading>
              <p className="mt-inline text-xs text-muted">
                card · 16 px · řádkování 1,5
              </p>
            </div>
            <div>
              <Heading level={3} size="label">
                Menší podnadpis
              </Heading>
              <p className="mt-inline text-xs text-muted">
                label · 14 px · řádkování 1,5
              </p>
            </div>
          </div>
          <p className="mt-group border-t border-border pt-field text-xs leading-relaxed text-muted">
            Manrope, váha 600. Jeden H1 na stránku; H2 pro sekce, H3 pro jejich
            obsah. Velikosti nepřepisujeme lokálně.
          </p>
        </Card>
        <Card titleLevel={3} title="Škála rozestupů">
          <ul className="space-y-field">
            {spacing.map((item) => (
              <li
                key={item.token}
                className="grid grid-cols-2 items-center gap-field border-b border-border pb-field"
              >
                <div>
                  <p className="text-sm font-semibold">
                    {item.token} · {item.value}
                  </p>
                  <p className="mt-1 text-xs text-muted">{item.usage}</p>
                </div>
                <div className="flex items-center gap-inline">
                  <span
                    aria-hidden="true"
                    className={`h-3 shrink-0 rounded-sm bg-primary ${item.width}`}
                  />
                  <code className="text-xs text-muted">gap-{item.token}</code>
                </div>
              </li>
            ))}
          </ul>
        </Card>
      </div>
    </section>
  );
}
