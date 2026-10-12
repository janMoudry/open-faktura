# Dokumentace Open Faktury

Vyberte si podle toho, co potřebujete udělat. Základní představení je v [README](../README.md), dostupné možnosti v [přehledu funkcí](../FEATURES.md).

## Používání a provoz

- [Instalace](INSTALLATION.md) — vlastní server, první přihlášení, SMTP, zálohy a obnova.
- [Firmy a nastavení](COMPANIES.md) — fakturační profily, jejich údaje a oprávnění.
- [Licence](LICENSING.md) — GNU AGPL v3 a podmínky použití.

## Vývoj a integrace

- [Lokální vývoj](DEVELOPMENT.md) — závislosti, spuštění webu a desktopu, kontroly a testy.
- [Vývojové konvence](architecture/DEVELOPMENT_CONVENTIONS.md) — technologie, struktura, UI a lokalizace.
- [Lokalizace](LOCALIZATION.md) — katalogy češtiny a angličtiny a formátování hodnot.
- [Sdílené UI](../packages/ui/README.md) — komponenty a jejich použití.
- [REST a MCP kontrakty](../contracts/) — OpenAPI a schémata nástrojů; klient se obnovuje přes `pnpm generate:api`.
- [Lokální MCP](COMPANIES.md#lokální-mcp) — přihlášení a spuštění stdio adaptéru.
- [Pravidla práce v repozitáři](../AGENTS.md) — požadavky na implementaci a ověřování.

## Specifikace modulů

Tyto dokumenty určují chování, validace, omezení a testovací scénáře. Nejsou návodem k obsluze každé obrazovky.

| Oblast | Dokumenty |
| --- | --- |
| První spuštění a účty | [Instalace](specs/installation-onboarding.md), [účet](specs/account-settings.md), [firma](specs/company-setup.md) |
| Kontakty a ceník | [Kontakty](specs/contacts.md), [ceník](specs/catalog.md) |
| Fakturace | [Přehled](specs/issued-invoices.md), [výpočty](specs/invoice-calculation.md), [návrhy](specs/invoice-drafts.md), [vystavení a PDF](specs/invoice-issue.md), [úhrady](specs/invoice-payments.md) |
| Nabídky a šablony | [Prodejní dokumenty](specs/sales-documents.md) |
| Komunikace | [Odesílání faktur](specs/invoice-delivery.md), [SMTP](specs/smtp-settings.md), [přílohy](specs/document-attachments.md) |
| Přenos dat | [Import a migrace dat](specs/data-migration.md) |

## Plán a architektura

- [Implementační plán](IMPLEMENTATION_PLAN.md) — stav dodávek, zbývající práce a kritéria dokončení.
- [Konceptuální datový model](DATA_MODEL.md) — původní návrh entit; skutečné schéma určují [migrace](../migrations/).
- [Technický průzkum](architecture/STACK_RESEARCH.md) — historické srovnání variant; přijatá rozhodnutí určují vývojové konvence.
- [Návrhy rozhraní a validační záznamy](design/) — podklady k jednotlivým změnám a jejich ověření.

## Výzkumné podklady

- [Referenční soupis funkcí](research/FEATURES_REFERENCE.md) — původní průzkum s datem a odkazy na zdroje.
- [Evidence zdrojů a pozorování](research/EVIDENCE.md) — dokumentované chování, přečtené kontrakty, experimenty a vlastní návrhy.
- [Průzkumné scénáře](research/SCENARIOS.md) — otázky a postupy pro doplnění chybějících důkazů.

Odkazy na jiné produkty slouží k doložení výzkumu a interoperability. Nevyjadřují spojení s jejich provozovateli ani oprávnění přebírat jejich kód, grafiku či texty. Při úpravách zachovávejte zdroje, data pozorování a rozlišení mezi důkazem a vlastním návrhem.

## Jak dokumentaci udržovat

U nové funkce aktualizujte specifikaci a implementační plán; po dokončení také přehled funkcí. Návod pro provoz doplňte, pokud se mění konfigurace, instalace nebo obnova. README má zůstat krátkým vstupem do projektu. Historické záznamy označujte datem a nepoužívejte je jako tvrzení o aktuálním stavu aplikace.
