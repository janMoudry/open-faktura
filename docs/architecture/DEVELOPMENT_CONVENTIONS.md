# Vývojové konvence a přijatá rozhodnutí

Datum: 10. 10. 2026. Tento dokument má přednost před staršími návrhy ve STACK_RESEARCH.md.

## Přijatá rozhodnutí

- Backend a doménové jádro: Rust; databáze: PostgreSQL.
- Frontend: React a Tailwind CSS se sdílenými design tokeny.
- Atomic design a tradiční rozdělení `pages/`, `components/`, `hooks/`, `services/`, `utils/`.
- Jeden monorepo pro web, desktop, backend, kontrakty a provozní konfiguraci.
- Desktopový runtime: Tauri, sdílející React UI s webem.
- Desktopové instalátory ke stažení jako assets v GitHub Releases.
- Vývoj a úpravy budou provádět převážně agenti; pravidla mají být dohledatelná a kontrolovatelná.

Při založení kostry byly použity TypeScript, Vite, pnpm, Axum a SQLx; konkrétní verze jsou v manifestech a lockfiles. Mobilní PWA zůstává plánovaným směrem. Desktopový runtime Tauri je potvrzen uživatelem 10. 10. 2026. Tento dokument neurčuje finální vzhled ani hodnoty tokenů.

## Cílová struktura monorepa

Následuje cílový layout, nikoli tvrzení o existující aplikaci. Adresáře a manifesty zakládat spolu s první skutečnou implementací; žádné prázdné aplikace či nefunkční release workflow.

```text
apps/
  web/src/
    pages/                    obrazovky, např. invoices/InvoiceDetailPage.tsx
    components/
      atoms/                  lokální základní komponenty
      molecules/              menší kompozice, např. InvoiceStatusBadge
      organisms/              celky, např. invoices/InvoiceEditor
      templates/              rozložení obrazovek a sloty
    hooks/                    stav a orchestrace specifické pro aplikaci
    services/                 adaptéry API a definice dotazů
    utils/                    čisté pomocné funkce; bez skrytých I/O
    routes/                   tenké registrace cest odkazující na pages
    styles/                   vstup CSS importující sdílené tokeny
    types/                    pouze místní typy, ne kopie API kontraktu
  desktop/                    obal společného webového UI
packages/
  ui/src/components/          atoms, molecules, organisms, templates bez domény
  design-tokens/              autoritativní tokeny a mapování pro Tailwind
  api-client/                generované kontrakty a typovaný HTTP klient
crates/
  domain/                    účetní pravidla a hodnotové typy
  application/               příkazy, autorizace, porty a transakční hranice
  infrastructure/            PostgreSQL, soubory, e-mail a fronta
  server/                    kompozice a REST/MCP adaptéry
  worker/                    vykonávání úloh
contracts/                   OpenAPI a verzovaná schémata událostí
migrations/                  autoritativní databázové migrace
infra/                       Compose a konfigurace provozu
scripts/                     skutečně používaná automatizace
.github/workflows/           CI a později desktopové release buildy
```

Web a desktop sdílejí React UI; desktop nemá druhou kopii pages a components. Tauri obal v apps/desktop bude používat společný webový build; konkrétní konfiguraci zavést při scaffoldingu. Sdílenou komponentu přesunout do `packages/ui` jako jediný zdroj, nepřekopírovat ji zároveň do `apps/web`. Doménové komponenty zůstávají v aplikaci. Další členění podle oblasti lze dělat uvnitř pages, organisms, hooks a services; nezavádět souběžně top-level `features/`.

Směr závislostí: pages → lokální components a hooks/services → api-client. Sdílené UI neimportuje aplikace, router konkrétní aplikace ani její API. Atoms nepoužívají molecules/organisms, templates neimportují pages. Stejnou úroveň lze skládat bez cyklů. Není nutné vytvářet všech pět vrstev pro každý prvek.

Atoms jsou základní prvky, molecules jejich malé kompozice, organisms funkční celky a templates rozložení bez konkrétních dat. Pages propojují data s těmito celky. Síťové volání nepatří do sdíleného UI; organismům předávat data a callbacky, složitější orchestrace patří do hooks. `utils` není odkladiště účetních výpočtů ani API klientů.

V Rustu domain nezávisí na HTTP ani SQLx, application používá domain a definuje potřebné porty, infrastructure je implementuje. Server a worker sestavují závislosti. HTTP i MCP musí zachovat stejné aplikační příkazy. Porty musí umožnit atomické uložení dokladu, auditu a outboxu.

## Lokalizace

Přijaté rozhodnutí: od začátku podporovat češtinu (`cs`, výchozí a fallback)
a angličtinu (`en`). Technický kód češtiny je `cs`, nikoli `cz`. Implementaci
základu a převod existujícího UI sleduje L10N-01 v implementačním plánu.
Aktuální katalogy, hooky a ověření popisuje [návod lokalizace](../LOCALIZATION.md).

- Nové produktové texty patří do překladových katalogů se stabilními klíči,
  včetně placeholderů, validací, tooltipů, prázdných stavů a přístupných názvů.
  Přidávat oba překlady současně; věty neskládat konkatenací, používat
  interpolaci a pluralizaci. CI má ověřovat shodu klíčů.
- Web a Tauri používají stejnou konfiguraci a katalogy. Sdílené UI nesmí
  importovat překlady z aplikace; obecné texty vlastní UI balíček,
  doménové popisky dostává přeložené přes props. Katalogy jsou v `src/locales` aplikace a UI balíčku.
- Čísla, měny a data formátovat centrálně podle locale (`cs-CZ`/`en-GB`).
  Zobrazení nesmí měnit přesnost účetních hodnot, měnu, časové pásmo
  ani kalendářní datum. Vstupy mají respektovat dohodnutý locale parser.
- API vrací stabilní chybové kódy a strukturované parametry; uživatelskou
  zprávu překládá klient. Neznámý kód má bezpečnou lokalizovanou náhradu.
- Volba jazyka je dostupná před přihlášením, přetrvá reload a nastavuje
  HTML `lang` i locale kalendářů. Přepnutí zachová rozpracovaná data.
- Jazyk UI je nezávislý na jazyku dokladu, měně a legislativě firmy.
  Uživatelský obsah se automaticky nepřekládá. Dokumentace může zůstat česky.
- Každou novou obrazovku ověřit v obou jazycích, včetně délky textů,
  mobilního rozložení, chybových stavů a klávesnice.

## Tailwind a tokeny

Jediný zdroj tokenů: `packages/design-tokens`. Stanovit škály pro barvy, spacing, typografii, radius, stíny a pohyb. Barevné role pojmenovat sémanticky (`surface`, `foreground`, `muted`, `primary`, `danger`, `border`, `focus`), nikoli podle konkrétní stránky. Light/dark a branding forku přepínají hodnoty stejných rolí.

Tailwind mapuje theme variables do utilit; aktuální dokumentace používá `@theme`. Přesnou syntaxi i konfiguraci skenování sdílených balíčků přizpůsobit připnuté verzi. Tailwind sám nezakazuje libovolné hodnoty: při zavedení CI přidat lint/kontrolu barevných literálů a obcházení škál, s vysvětlenými výjimkami pro skutečně dynamické rozměry. [Tailwind theme variables](https://tailwindcss.com/docs/theme).

V běžném UI používat tokeny a deklarované varianty. Nový významný token přidat centrálně, nikoli řešit lokální hex barvou. Hodnoty tvořící výsledek měření (např. šířka sloupce po resize) nemusí být token. Skládat celé názvy Tailwind tříd, ne dynamické fragmenty, které build nedohledá. Při změně tokenu ověřit dotčené stavy a kontrast.

## Typografie a ikony

Aktuální rozhodnutí 10. 10. 2026: **monochromatický základ se sémantickými stavovými barvami** a **Manrope Variable** pro celé rozhraní včetně nadpisů. `font-display` je zachovaný sémantický token pro budoucí brand, nyní ukazuje na Manrope. Částky používají `tabular-nums`. Source Serif 4 již není součástí aktuálního UI; dřívější zeleno-krémový návrh je historická iterace.

Primary zůstává neutrální. Success je zelený, warning jantarový a danger červený; mají vlastní tokeny nezávislé na brandové primární barvě. Stav navíc rozlišuje text a ikona (check / clock / warning), nikoli samotný odstín. Destruktivní akce musí mít jednoznačný popisek. Focus je kontrastní černý obrys; žádné lokální barevné výjimky.

Font se distribuuje lokálně přes připnutý Fontsource balíček s rozšířenou latinkou, `font-display: swap` a unicode rozsahy. Bez externího fontového CDN. [Manrope, OFL-1.1](https://fontsource.org/fonts/manrope/about).

Ikony: **Lucide React**, explicitní importy v `packages/ui/src/components/atoms/Icon.tsx`. Nové významy přidávat do společné mapy; nekreslit vlastní SVG a nemíchat další knihovny. Základní velikost 16 px, tah 1.75, barva `currentColor`. Ikony doprovázející text jsou dekorativní; tlačítko bez textu musí mít vlastní přístupný název. [Dokumentace](https://lucide.dev/guide/react/), [licence ISC a MIT pro zděděné Feather ikony](https://lucide.dev/license).

Úplné licence z nainstalovaných balíčků jsou v `apps/web/public/licenses/`; Vite je kopíruje do výsledného webového i desktopového frontend balíku. Tyto licence zůstávají samostatné; vlastní kód Open Faktury používá [GNU AGPL v3](../../LICENSE) (`AGPL-3.0-only`).

## Desktop a GitHub Releases

GitHub Releases umožňují připojit distribuční soubory k vydání založenému na tagu. Pro Tauri připravit matici buildů a instalátorů pro podporované OS. [GitHub Releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases).

Navržené cíle: Windows x64 instalátor, macOS Apple Silicon a Intel disk image, Linux x64 AppImage a případně deb. Skutečně podporované kombinace a formáty potvrdit podle připnuté verze Tauri a dostupných runnerů; neoznačovat netestované buildy jako podporované. [Tauri distribuce](https://v2.tauri.app/distribute/).

Release pipeline z tagu má ověřit verzi, spustit kontroly, sestavit aplikaci, podepsat podle platformy, otestovat instalaci a přiložit assets s verzí/OS/architekturou, kontrolními součty a poznámkami o kompatibilitě serveru. macOS podpis/notarizace a Windows podpis vyžadují vlastní konfiguraci a přihlašovací materiál. Chybějící podpis nesmí být vykázán jako dokončený podepsaný release.

Build z pull requestu nesmí mít přístup k release tajemstvím. Práva pro publikaci omezit na release job; aplikované GitHub Actions připnout na ověřené commity. Nejprve sestavit kompletní draft release; publikace se řídí konkrétním pověřením k vydání. Požadavek na distribuční kanál sám neznamená publikaci nedokončené aplikace nyní.

Instalátor obsahuje desktopového klienta, nikoli automaticky PostgreSQL a server. Při spuštění bude potřebovat adresu backendu. Kompletní self-hosted provoz nadále řeší Compose. Automatické aktualizace jsou samostatná navazující funkce; GitHub asset sám update mechanismus neimplementuje.

## Ověření při založení aplikace

Doplnit skutečné příkazy pro formátování, lint, typovou kontrolu, testy a build do AGENTS.md. CI ověří hranice importů, kontrakty, tokeny a relevantní testy. Konkrétní příkazy nelze vydávat za funkční, dokud nejsou manifesty a implementace. Spustitelná kostra a kontroly jsou popsané v [lokálním vývoji](../DEVELOPMENT.md); podepsané veřejné instalátory zatím nejsou připravené.

## Sdílený design systém v0.1

Autoritativní sada a její kontrakty: [packages/ui/README.md](../../packages/ui/README.md). Živý katalog v aplikaci: `/sandbox.html#/design-system` (pouze vývojový server) (navigace Komponenty). Nová UI práce nejprve použije existující exporty; primární tlačítka, inputy, selecty, štítky a chybové stavy nevytvářet znovu lokálně. Doménové celky zůstávají v aplikaci a skládají sdílené prvky. Nový významný stav přidat do katalogu. Testy katalogu a regresní testy faktur jsou součástí `pnpm test:e2e`.

## Redesign podle brandu

Použij repo skill [.agents/skills/brand-redesign/SKILL.md](../../.agents/skills/brand-redesign/SKILL.md) při změně vizuální identity. Monochromatický základ je výchozí stav, nikoli zákaz budoucího barevného brandu. Zadání konkrétního redesignu má přednost.

## Grafy a datové přehledy

Recharts 3.10.1, React/SVG, MIT; rozhodnutí 10. 10. 2026.
Používat sdílený `DataChart` z `@open-faktura/ui/charts`, kontrakty a omezení viz
[UI README](../../packages/ui/README.md#grafy). Importovat grafy jen v modulech,
které je používají, a načítat je lazy. Barvy, tooltipy, font a datová alternativa
patří do sdíleného UI. Účetní agregace zůstávají v doméně/backendu.
Zdroje: [repozitář](https://github.com/recharts/recharts),
[instalace](https://recharts.github.io/en-US/guide/installation/).

## Výběrová pole a atomy

Atomy a jejich stavy mají vlastní oddíl v katalogu. Sdílené Select, DatePicker a
Autocomplete jsou molekuly nad React Aria Components, stylované našimi tokeny.
Nové obrazovky používají tyto exporty místo viditelného nativního select,
input[type=date] a datalist. Kontrakty a zdroje jsou v
[UI README](../../packages/ui/README.md#atomy-a-výběrové-komponenty).

## Hierarchie a rytmus UI

Nadpisy skládat komponentou Heading, s HTML úrovní podle struktury a velikostí
page/section/card/label podle role. Pro rozložení používat pojmenované spacing
tokeny inline/control/field/group/section/page (8/12/16/24/32/48 px).
Hodnoty a ukázky mají jediný zdroj v design-tokens a katalogu; nevytvářet lokální
škály nadpisů nebo mezer. Viz [UI kontrakty](../../packages/ui/README.md#nadpisy-rozestupy-a-taby).

## Produktový shell — 11. 10. 2026

Reference iDokladu E-012 se promítá do kompaktního appbaru a hierarchické navigace.
Původní dlouhý plochý sidebar a samostatné karty nastavení jsou historická iterace.
Rozměry shellu vlastní design-tokens (workspace-sidebar, workspace-content);
kompozice ProductWorkspace je oddělená od sandboxového WorkspaceLayout.
Profilový ovladač předává trvalý ProductRouteLayout přímo do AppLayout jako prop;
bez DOM portálu. API a stav profilu vlastní layoutový hook useCompanyWorkspace,
CompanyPage vykresluje pouze obsah. Značka, font a sémantické barvy se nemění.


## Routing a životnost layoutů — 11. 10. 2026

Produkt používá React Router 7.18.4 a createHashRouter kvůli stávajícím URL
`#/…` a společnému web/Tauri klientu. Referenční princip:
[vnořené routy a Outlet](https://reactrouter.com/start/declarative/routing).
Registrace rout patří do routes/ProductRoutes.tsx; sandbox zůstává oddělený.

- AccessLayout ověřuje relaci při vstupu, po explicitním přihlášení a při
  návratu focusu. Změna cesty sama nespouští setup/session request ani stav
  loading celého shellu. Selhání revalidace při výpadku sítě nezahazuje editaci.
- ProductRouteLayout zůstává připojený přes změny dětských rout. Vlastní appbar,
  profilový přepínač, účetní menu a ProductWorkspace se sidebarovou navigací.
  Stav se resetuje při odhlášení nebo změně identity, nikoli podle pathname.
- Suspense obaluje pouze Outlet uvnitř obsahové oblasti. CompanyPage je lazy
  a obsahuje jen stránku; ani načítání modulu, ani dat nesmí odpojit sidebar.
  ProductSidebar je memoizovaný: změna polí formuláře jej zbytečně nerenderuje.
- useCompanyWorkspace obsahuje profilový kontext a současné příkazy editoru;
  při dalším dělení stránek oddělovat jejich lokální formuláře od globálního
  seznamu a aktivního profilu. Kontext poskytuje jediný trvalý layout.
- Navigace používá Link/NavLink/useNavigate, nikoli vlastní hashchange listener
  nebo location.hash. useBlocker chrání rozpracovaný profil při přechodech i
  historii; beforeunload chrání opuštění dokumentu. Stav sidebarových sekcí
  a mobilní navigace přežije přechod.
- API stále kontroluje autentizaci a oprávnění; router není bezpečnostní hranice.
  Nepřipravená fakturace má vlastní placeholder, ne alias na nastavení profilu.

Regrese: routing.spec.ts zdržuje načtení modulu a dat, kontroluje identitu DOM
uzlů appbaru/sidebaru, otevřené sekce, Zpět, chybu API a absenci auth requestů
na každou navigaci. Company/auth/product/localization scénáře ověřují funkční
chování s touto strukturou.

### Hustota produktových stránek

Produktový workspace využívá dostupnou šířku vedle sidebaru; nevkládat další
centrovaný `max-width` kolem celé stránky. Boční odsazení řídí token
`workspace-gutter` (16 px), vnitřek formulářových sekcí `form-inset` (16 px).
Společné třídy `workspace-panel`, `panel-toolbar`, `panel-tabs`, `panel-footer`,
`profile-section` a `profile-actions` sjednocují seznamy a formuláře. Tabulky
skládají společné `Table`, `TableHeader`, `TableRow`, `TableHead` a `TableCell`
z `@open-faktura/ui`; na mobilu scrolluje
obal tabulky, nikoli celá stránka. Široké identity lze dělit do sloupců, na mobilu
se skládají. Nepřidávat další obal s bočním paddingem okolo téhož formuláře.
