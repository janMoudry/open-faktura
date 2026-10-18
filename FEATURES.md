# Funkce Open Faktury

Přehled dostupných funkcí a plánovaného rozšíření. Aktualizováno 18. 10. 2026.
Podrobný [checklist](docs/FEATURE_CHECKLIST.md) rozepisuje jednotlivé funkce. Stav dodávek, testů a otevřených pravidel sleduje [implementační plán](docs/IMPLEMENTATION_PLAN.md).

[Podrobný checklist hotových a chybějících funkcí](docs/FEATURE_CHECKLIST.md).

## Dostupné funkce

„Dostupné“ znamená implementované v popsaném rozsahu, nikoli dokončený účetní systém nebo veřejné stabilní vydání.

| Oblast | Co aplikace umí | Podrobnosti |
| --- | --- | --- |
| Fakturační profily | Založení firmy, její údaje, účetní nastavení, přepínání profilů a kontrola členství | [Firmy a nastavení](docs/COMPANIES.md) |
| Kontakty | Evidence, hledání a úpravy obchodních partnerů | [Adresář](docs/specs/contacts.md) |
| Faktury | Uložené návrhy, výpočty, vystavení, přidělení čísla a PDF | [Návrhy](docs/specs/invoice-drafts.md), [vystavení](docs/specs/invoice-issue.md) |
| Zálohy | Samostatná řada ZF, PDF, ruční úhrady a storna; DDPP a vyúčtování plně uhrazených záloh | [Zálohové faktury](docs/specs/advance-invoices.md) |
| Úhrady | Ruční úhrady, jejich opravy a zůstatek faktury | [Úhrady](docs/specs/invoice-payments.md) |
| Sklad | Pohyby z dokladů, inventury, souhrnné stavy k datu a ruční skladové ceny s historií | [Sklad](docs/specs/stock.md) |
| Ceník | Položky a jejich výběr do dokladu, archivace a obnova | [Ceník](docs/specs/catalog.md) |
| Nabídky a šablony | Opakované použití šablony, nabídka s PDF a převod do návrhu faktury | [Prodejní dokumenty](docs/specs/sales-documents.md) |
| Odesílání a sdílení | E-maily s historií přes vlastní SMTP; veřejné odkazy pro nabídky | [Odesílání faktur](docs/specs/invoice-delivery.md), [nabídky](docs/specs/sales-documents.md) |
| Přílohy | Vlastní soubory u faktur a nabídek, výběr příloh do e-mailu | [Přílohy](docs/specs/document-attachments.md) |
| Export | CSV/XLSX seznamy faktur a odběratelů, aplikované filtry, výběr sloupců, CS/EN | [Export seznamů](docs/specs/register-export.md) |
| Import | Obecný CSV/XLSX import kontaktů a historických faktur; ceník CSV/XLSX a Money S3 XML, mapování a náhled | [Přenos dat](docs/specs/data-migration.md) |
| Export seznamů | CSV/XLSX s filtry a výběrem sloupců; nejde o úplnou zálohu ani účetní export | [Exporty](docs/specs/register-export.md) |
| Daňové doklady k platbě | Náhled DPH z úhrady zálohy, řada DDPP, neměnné PDF, seznam/detail CS/EN | [P13-02](docs/specs/advance-tax-receipts.md) |
| Rozhraní | Čeština a angličtina, základní přehled, REST API a lokální MCP | [Lokalizace](docs/LOCALIZATION.md), [MCP](docs/COMPANIES.md#lokální-mcp) |
| Provoz | Vlastní server, první správce, zálohování a obnova | [Instalace](docs/INSTALLATION.md) |

## Současná omezení

- České prostředí je základ agendy. Vydané a přijaté faktury, zálohy, dobropisy a bankovní účty podporují také cizí měny s uloženými kurzy. Hotovostní doklady a některé návaznosti zůstávají omezené; viz [aktuální stav](docs/P16_P20_COMPLETION.md).
- Importní předvolby pro konkrétní služby zatím nejsou ověřené adaptéry. Obecný import nepřenáší zdrojová PDF ani přílohy.
- Sklad má ruční i dokladové pohyby, stavy k datu, ruční ocenění a konzervativní průměr z pořizovacích cen nákupních příjmů. Vratky, cenové opravy a vedlejší náklady jsou zahrnuté; pořizovací ocenění ručních příjmů, počátečních stavů a inventurních přebytků je zahrnuté. Přehled prodeje položek nabízí filtry, zdrojové doklady a CSV/XLSX. Money S3 má import základního ceníku XML s náhledem a deduplikací; S4/S5 bez doloženého kontraktu řeší obecné mapování nebo vlastní adaptér provozovatele.
- Desktop vyžaduje samostatný backend a PostgreSQL. Osobní režim bez ruční správy serveru a podepsané instalátory nejsou hotové.
- Správa členství pracuje s existujícími účty. E-mailové pozvánky a vzdálené MCP přihlášení přes OAuth se připravují.
- Výpočty mají vlastní [specifikaci](docs/specs/invoice-calculation.md); numerická shoda s externími systémy není potvrzená.

## Plánované rozšíření

| Oblast | Plánovaný rozsah |
| --- | --- |
| Prodej | Rozšíření daňových režimů a vyúčtování záloh s rozdílnými kurzy |
| Automatizace | Pravidelné faktury, upomínky a poděkování implementovány v rozsahu P14-01/02; zbývají pravidelné zálohy, proměnné a další hromadné operace |
| Nákup | Základ přijatých faktur, účtenek, závazků a výdajových úhrad je hotový; zbývá dodavatelská role, rozšířený editor, cizí měny a další výstupy |
| Finance | Základ banky, pokladny a prodejek je hotový; zbývají úplné daňové klasifikace a ověření konkrétních externích zařízení |
| Sklad | Pohyby, dokladové vazby a ocenění jsou hotové; neověřené externí formáty lze doplnit vlastním adaptérem |
| Příjem dokumentů | Inbox, ISDOC/QR a volitelné vytěžování dokumentů |
| Přehledy a integrace | Rozšířené přehledy, registry, účetní exporty, daňové podklady a integrační události |
| Správa agendy | Štítky, koš, uzávěrky, hromadné akce a úpravy vzhledu dokladů |
| Distribuce | Veřejná vydání, aktualizace, správa přístupů a osobní desktopový režim |

Celý rozsah zůstává součástí plánu; tabulka nepředstavuje termín dodání. Přesné požadavky, závislosti a kritéria dokončení jsou v [dodávkách P00–P20](docs/IMPLEMENTATION_PLAN.md).

## Podklady pro vývoj

Pravidla jednotlivých modulů popisují [specifikace](docs/README.md#specifikace-modulů). Historický průzkum trhu se zdroji je zachován v [referenčním soupisu](docs/research/FEATURES_REFERENCE.md); slouží k dohledání původu požadavků, nikoli jako seznam současných funkcí Open Faktury.


Aktuální rozlišení hotových funkcí a zbývající práce je soustředěné v [přehledu P16–P20](docs/P16_P20_COMPLETION.md). Historické poznámky v implementačním plánu popisují stav v době jednotlivých dodávek, nikoli dnešní omezení.
