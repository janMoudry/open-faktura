# Open Faktura

Fakturace pro české živnostníky a firmy s provozem na vlastním serveru. Doklady, kontakty i přílohy zůstávají ve vaší správě; k aplikaci přistupujete přes web, REST API nebo MCP pro práci s AI asistenty.

**Projekt je v aktivním vývoji.** Základní fakturace funguje, další účetní moduly a veřejná distribuce se připravují. Dostupný rozsah a omezení popisuje [přehled funkcí](FEATURES.md).

## Co už můžete používat

- **Faktury:** návrhy, vystavení s číslováním, PDF a evidence ručních úhrad.
- **Kontakty a ceník:** adresář obchodních partnerů a položky pro opakované použití.
- **Nabídky a šablony:** příprava nabídky, PDF, veřejný odkaz a převod do návrhu faktury.
- **Odesílání:** faktury a nabídky e-mailem přes vlastní SMTP, historie odeslání a přílohy.
- **Vlastní agenda:** více fakturačních profilů, základní přehled a import CSV/XLSX s náhledem a mapováním sloupců.
- **Integrace a jazyky:** REST API, lokální MCP a společné rozhraní v češtině a angličtině.

Zálohové a přijaté faktury, bankovní párování, sklad, pravidelná fakturace a daňové výstupy jsou součástí [plánu vývoje](docs/IMPLEMENTATION_PLAN.md).

## Jak začít

| Chci… | Kde pokračovat |
| --- | --- |
| Provozovat aplikaci na vlastním serveru | [Instalace, první správce a zálohy](docs/INSTALLATION.md) |
| Spustit projekt pro vývoj | [Lokální vývoj](docs/DEVELOPMENT.md) |
| Zjistit dostupné funkce a omezení | [Přehled funkcí](FEATURES.md) |
| Porozumět API, architektuře nebo přispět | [Rozcestník dokumentace](docs/README.md) |

Serverová instalace používá Docker Compose a PostgreSQL. Desktopový klient v Tauri sdílí webové rozhraní a vyžaduje samostatně běžící backend. Samostatná instalace na počítač včetně databáze i podepsané veřejné instalátory se teprve připravují.

## Pro vývojáře

Frontend používá React a Tailwind, backend Rust a databázi PostgreSQL. Web, REST API a MCP sdílejí aplikační pravidla, výpočty a kontrolu oprávnění.

Po instalaci [vývojových závislostí](docs/DEVELOPMENT.md#požadavky) spusťte z kořene repozitáře:

```sh
pnpm install --frozen-lockfile
# Pouze pokud ještě nemáte vlastní .env:
cp .env.example .env
node scripts/init-secrets.mjs
pnpm db:up
pnpm dev
```

Web je dostupný na <http://127.0.0.1:5173>. První účet založíte podle [instalačního návodu](docs/INSTALLATION.md#první-přihlášení-a-firma). Vývojové nastavení není určené pro veřejný server.

Před úpravami si přečtěte [vývojové konvence](docs/architecture/DEVELOPMENT_CONVENTIONS.md) a [pravidla přispívání a práce agentů](AGENTS.md). Příkazy pro kontrolu změn najdete v [návodu pro vývoj](docs/DEVELOPMENT.md#kontroly).

## Licence

Open Faktura je svobodný a open-source software pod **GNU Affero General Public License, verze 3** (`AGPL-3.0-only`). Můžete ji používat, upravovat, sdílet i prodávat při dodržení licence. Při distribuci platí povinnosti zpřístupnění odpovídajícího zdrojového kódu; u upravené verze provozované přes síť musí její uživatelé dostat nabídku tohoto kódu.

Copyright (C) 2026 přispěvatelé Open Faktury. Software je poskytován bez záruky v rozsahu dovoleném právem. Úplné znění je v [LICENSE](LICENSE), praktické vysvětlení v [licencování](docs/LICENSING.md).

Licence použitých fontů a knihoven zůstávají zachované u příslušných součástí. Referenční zdroje a původ požadavků jsou dohledatelné ve [výzkumné dokumentaci](docs/README.md#výzkumné-podklady).
