# Open Faktura

Kompletně otevřená fakturační aplikace pro české podnikatele a firmy, kterou si každý může nainstalovat a provozovat na vlastním serveru.

Cílem je plná funkční alternativa iDokladu: fakturace, nabídky, zálohy, přijaté doklady, sklad, banka, pokladna, automatizace, daňové podklady a integrace. API a MCP budou součástí produktu od začátku.

## Stav projektu

Projekt má funkční základ monorepa: React/Tailwind web, Tauri desktop, Rust API a PostgreSQL. Produkt má účty, fakturační profily, adresář, návrhy a vystavení faktur s číslováním a PDF, ruční úhrady, základní přehled, ceník, šablony a cenové nabídky s převodem na fakturu. Faktury a nabídky podporují odesílání e-mailem s historií a vlastními přílohami; SMTP lze nastavit v UI. Nabídky mají veřejné odkazy. Rozhraní je v češtině a angličtině, moduly mají REST/MCP kontrakty. Import CSV/XLSX má mapování a náhled; předvolby iDokladu a FakturyOnline zatím nejsou ověřené zdrojové adaptéry. Produkční vstup je oddělený od vývojového sandboxu; katalog komponent je při `pnpm dev` dostupný na `/sandbox.html#/design-system` a návrh faktur na `/sandbox.html#/invoices`.

Spuštění a kontroly: [Lokální vývoj](docs/DEVELOPMENT.md). Po instalaci závislostí a vytvoření `.env` spusť `node scripts/init-secrets.mjs`, `pnpm db:up` a `pnpm dev`; desktop používá `pnpm dev:desktop` se samostatně běžícím API.

- [Soupis funkcí iDokladu, zdroje a návrh otevřené alternativy](FEATURES.md)
- [Pravidla práce a lokální skills](AGENTS.md)
- [Přijatý stack, struktura monorepa a desktopová distribuce](docs/architecture/DEVELOPMENT_CONVENTIONS.md)
- [Implementační plán po dodávkách P00–P20](docs/IMPLEMENTATION_PLAN.md)
- [Spuštění vlastní instalace, přihlášení a obnova správce](docs/INSTALLATION.md)
- [Instalace, první správce a průvodce firmou](docs/specs/installation-onboarding.md)
- [Průzkum stacku, škálování a rozšiřitelnosti](docs/architecture/STACK_RESEARCH.md)
- [Konceptuální datový model](docs/DATA_MODEL.md)
- [Evidence reverse-engineeringu](docs/research/EVIDENCE.md)
- [Scénáře pro ověření reference](docs/research/SCENARIOS.md)
- [Uložené návrhy faktur — P06](docs/specs/invoice-drafts.md)
- [První specifikace vydaných faktur](docs/specs/issued-invoices.md)

Podklady rozlišují kontrakty přečtené z oficiálního SDK, dosud neprovedené experimenty a vlastní návrh Open Faktury. Schéma databáze je verzované v `migrations/`. Rozsah ověřené shody a odchylky uvádějí jednotlivé specifikace; úplná shoda s iDokladem není dokončená. Screenshoty z lokálního průzkumu a vizuálních kontrol nejsou součástí veřejného repozitáře.

## Principy

- Celý produkt bude otevřený: frontend, backend, API, MCP, databázové migrace a vlastní konektory.
- Všechny funkční moduly budou dostupné i při provozu na vlastním serveru.
- Uživatel bude mít kontrolu nad daty, přílohami, zálohami a exporty.
- Základní fakturace bude fungovat bez placené služby provozovatele projektu.
- Web, API a MCP budou používat stejná pravidla, výpočty a oprávnění.
- Případný placený hosting nabídne spravovaný provoz stejné otevřené aplikace.

## Plánovaný rozsah

- Vydané faktury, dobropisy, zálohy a jejich vyúčtování.
- Cenové nabídky, šablony a pravidelná fakturace.
- Přijaté faktury, účtenky a inbox s vytěžováním dokumentů.
- Adresář, ARES a ověřování DIČ.
- Ceník, sklad a skladové pohyby.
- Úhrady, banka, párování plateb, pokladna a prodejky.
- E-maily, upomínky a poděkování za úhradu.
- Přehledy, daňové podklady a účetní exporty.
- Více firem, uživatelé, oprávnění a audit změn.
- Mobilní použití, API, integrační události a MCP.
- Instalace přes Docker Compose, zálohování, obnova a aktualizace.

## Postup vývoje

1. Doplnit veřejný soupis o pozorování aplikace, dokumentované API a exporty.
2. Ke každé funkci popsat vstupy, výstupy, stavy, pravidla a hraniční případy.
3. Připravit scénáře a testy pro ověření funkční shody.
4. Implementovat vlastní aplikaci a vlastní uživatelské rozhraní.
5. Postupně pokrýt celý cílový rozsah a připravit samostatnou instalaci.

## Licence

Cílem je kompletně open-source produkt. Konkrétní licence zatím nebyla vybrána a tento repozitář zatím licenci neobsahuje; rozhodnutí bude doplněno před zveřejněním implementace.

## Referenční produkt

iDoklad slouží jako reference funkčního rozsahu. Open Faktura je samostatný projekt bez spojení se společností Seyfor nebo produktem iDoklad.
