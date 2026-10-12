# Funkce Open Faktury

Přehled dostupných funkcí a plánovaného rozšíření. Aktualizováno 12. 10. 2026.
Podrobný stav jednotlivých dodávek, testů a otevřených pravidel sleduje [implementační plán](docs/IMPLEMENTATION_PLAN.md).

## Dostupné funkce

„Dostupné“ znamená implementované v popsaném rozsahu, nikoli dokončený účetní systém nebo veřejné stabilní vydání.

| Oblast | Co aplikace umí | Podrobnosti |
| --- | --- | --- |
| Fakturační profily | Založení firmy, její údaje, účetní nastavení, přepínání profilů a kontrola členství | [Firmy a nastavení](docs/COMPANIES.md) |
| Kontakty | Evidence, hledání a úpravy obchodních partnerů | [Adresář](docs/specs/contacts.md) |
| Faktury | Uložené návrhy, výpočty, vystavení, přidělení čísla a PDF | [Návrhy](docs/specs/invoice-drafts.md), [vystavení](docs/specs/invoice-issue.md) |
| Úhrady | Ruční úhrady, jejich opravy a zůstatek faktury | [Úhrady](docs/specs/invoice-payments.md) |
| Ceník | Položky a jejich výběr do dokladu, archivace a obnova | [Ceník](docs/specs/catalog.md) |
| Nabídky a šablony | Opakované použití šablony, nabídka s PDF a převod do návrhu faktury | [Prodejní dokumenty](docs/specs/sales-documents.md) |
| Odesílání a sdílení | E-maily s historií přes vlastní SMTP; veřejné odkazy pro nabídky | [Odesílání faktur](docs/specs/invoice-delivery.md), [nabídky](docs/specs/sales-documents.md) |
| Přílohy | Vlastní soubory u faktur a nabídek, výběr příloh do e-mailu | [Přílohy](docs/specs/document-attachments.md) |
| Import | Obecný CSV/XLSX import kontaktů a historických faktur, mapování a náhled | [Přenos dat](docs/specs/data-migration.md) |
| Rozhraní | Čeština a angličtina, základní přehled, REST API a lokální MCP | [Lokalizace](docs/LOCALIZATION.md), [MCP](docs/COMPANIES.md#lokální-mcp) |
| Provoz | Vlastní server, první správce, zálohování a obnova | [Instalace](docs/INSTALLATION.md) |

## Současná omezení

- Podporovaný základ firemní agendy je české prostředí a CZK; další měny a daňové situace vyžadují navazující moduly.
- Importní předvolby pro konkrétní služby zatím nejsou ověřené adaptéry. Obecný import nepřenáší zdrojová PDF ani přílohy.
- Ceník zatím neeviduje skladové pohyby. Ruční úhrady nenahrazují bankovní párování.
- Desktop vyžaduje samostatný backend a PostgreSQL. Osobní režim bez ruční správy serveru a podepsané instalátory nejsou hotové.
- Pozvánky, rozšířená správa uživatelů a vzdálené MCP přihlášení přes OAuth se připravují.
- Výpočty mají vlastní [specifikaci](docs/specs/invoice-calculation.md); numerická shoda s externími systémy není potvrzená.

## Plánované rozšíření

| Oblast | Plánovaný rozsah |
| --- | --- |
| Prodej | Zálohy, daňové doklady k platbě, vyúčtování, dobropisy, cizí měny a kurzy |
| Automatizace | Pravidelná fakturace, upomínky, poděkování a další hromadné operace |
| Nákup | Přijaté faktury, účtenky, závazky a jejich úhrady |
| Finance | Banka, pokladna, import a párování pohybů, prodejky |
| Sklad | Příjmy a výdeje, vazby na doklady a historie pohybů |
| Příjem dokumentů | Inbox, ISDOC/QR a volitelné vytěžování dokumentů |
| Přehledy a integrace | Rozšířené přehledy, registry, účetní exporty, daňové podklady a integrační události |
| Správa agendy | Štítky, koš, uzávěrky, hromadné akce a úpravy vzhledu dokladů |
| Distribuce | Veřejná vydání, aktualizace, správa přístupů a osobní desktopový režim |

Celý rozsah zůstává součástí plánu; tabulka nepředstavuje termín dodání. Přesné požadavky, závislosti a kritéria dokončení jsou v [dodávkách P00–P20](docs/IMPLEMENTATION_PLAN.md).

## Podklady pro vývoj

Pravidla jednotlivých modulů popisují [specifikace](docs/README.md#specifikace-modulů). Historický průzkum trhu se zdroji je zachován v [referenčním soupisu](docs/research/FEATURES_REFERENCE.md); slouží k dohledání původu požadavků, nikoli jako seznam současných funkcí Open Faktury.
