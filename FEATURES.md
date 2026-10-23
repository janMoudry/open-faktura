# Funkce Open Faktury

Stav zkontrolován 23. 10. 2026 proti implementaci v repozitáři. Open Faktura má implementovanou fakturaci i navazující prodejní, nákupní a finanční agendu; pokračuje dokončování a ověřování rozsahu P16–P20 a veřejné distribuce. Nejde zatím o kompletní účetní systém ani dokončenou veřejnou betu.

[Podrobný checklist](docs/FEATURE_CHECKLIST.md) eviduje jednotlivé funkce a mezery. [Aktuální stav P16–P20](docs/P16_P20_COMPLETION.md) shrnuje návaznosti; [implementační plán](docs/IMPLEMENTATION_PLAN.md) obsahuje i historii dodávek a kritéria milníků.

## Dostupné funkce

„Dostupné“ znamená implementované v popsaném rozsahu. Konkrétní omezení a dosavadní ověření určují odkazované specifikace.

| Oblast | Co aplikace umí | Podrobnosti |
| --- | --- | --- |
| Účty a profily | První správce, přihlášení, více firem, profilové role, správa existujících členů, převod vlastnictví a obnova profilu | [Nastavení](docs/COMPANIES.md), [správa](docs/specs/company-administration.md) |
| Kontakty | Adresář, ARES, archivace a anonymizace; ověření DIČ přes RWS/VIES | [Kontakty](docs/specs/contacts.md), [RWS](docs/specs/vat-registry.md), [VIES](docs/specs/vies.md) |
| Faktury a úhrady | Návrhy, vystavení, číslování, neměnné PDF, plné/částečné úhrady, přeplatky a storna úhrad | [Vystavení](docs/specs/invoice-issue.md), [úhrady](docs/specs/invoice-payments.md) |
| Ceník, nabídky a šablony | Ceník a výběr položek, opakované šablony, nabídky s PDF a převodem do návrhu faktury | [Ceník](docs/specs/catalog.md), [prodejní dokumenty](docs/specs/sales-documents.md) |
| Zálohy a opravy | Zálohové faktury, DDPP, vyúčtování plně uhrazených záloh, dobropisy se zápočty a vratkami | [Zálohy](docs/specs/advance-invoices.md), [DDPP](docs/specs/advance-tax-receipts.md), [vyúčtování](docs/specs/advance-settlement.md), [dobropisy](docs/specs/credit-notes.md) |
| Automatizace | Pravidelné faktury, volitelné vystavení/e-mail, upomínky a poděkování; cizoměnové opakování s uloženým nebo výslovně zvoleným kurzem ČNB | [Opakování](docs/specs/recurring-invoices.md), [komunikace](docs/specs/invoice-communication.md), [kurzy](docs/specs/exchange-rates.md) |
| Odesílání a sdílení | Vlastní SMTP, fronta a historie, přílohy, veřejné odkazy na nabídky i faktury s expirací/odvoláním | [E-mail](docs/specs/invoice-delivery.md), [sdílení faktur](docs/specs/invoice-sharing.md), [přílohy](docs/specs/document-attachments.md) |
| Nákup | Přijaté faktury a účtenky, originály, závazky, oddělené výdajové úhrady a přehled nákladů | [Přijaté faktury](docs/specs/received-invoices.md), [účtenky](docs/specs/purchase-receipts.md) |
| Banka | Účty a salda, importy, ruční/dělené/dávkové párování včetně existujících úhrad, záloh a vratek; opt-in automatika | [Banka](docs/specs/banking.md), [měny](docs/specs/bank-currencies.md), [automatika](docs/specs/bank-auto-matching.md) |
| Bankovní napojení | Fio a vlastní gateway, ruční/plánovaná synchronizace, autentizovaná notifikační brána | [Napojení](docs/specs/bank-connections.md), [notifikace](docs/specs/bank-notification-gateway.md) |
| Pokladna a prodejky | CZK příjmy/výdaje, saldo, automatické doklady, samostatná plnění s DPH, kombinované úhrady prodejek, souhrnné zaúčtování a PDF | [Pokladna](docs/specs/cash-register.md), [prodejky](docs/specs/sales-receipts.md) |
| Inbox | Nahrávání a náhled originálů, ISDOC/QR návrhy, příjem přes gateway/IMAP, volitelný poskytovatel vytěžení, oznámení a předání účetnímu | [Inbox](docs/specs/inbox.md), [IMAP](docs/specs/inbox-mailbox.md), [předání](docs/specs/inbox-delivery.md) |
| Sklad | Ruční i dokladové pohyby, inventury, stavy k datu, ruční ocenění a průběžný průměr, vratky, cenové opravy a vedlejší náklady | [Sklad](docs/specs/stock.md) |
| Přehledy | Měsíce/kvartály/roky, kumulace a meziroční srovnání, bilance, dlužníci, prodeje položek a uložené bankovní kurzové rozdíly | [Období](docs/specs/period-report.md), [položky](docs/specs/item-sales.md), [kurzové rozdíly](docs/specs/exchange-difference-report.md) |
| Daňové podklady | Evidence DPH, orientační obrat, CSV/XLSX, explicitní uživatelská klasifikace podkladů a profil daňového subjektu | [Přehled](docs/specs/tax-dashboard.md), [klasifikace](docs/specs/tax-classification.md), [profil](docs/specs/taxpayer-filing-profile.md) |
| Import a export | CSV/XLSX s mapováním a náhledem, Money S3 základní ceník XML, export seznamů a běžných faktur ISDOC 6.0.2 | [Import](docs/specs/data-migration.md), [ceník](docs/specs/catalog-import.md), [export](docs/specs/register-export.md), [ISDOC](docs/specs/isdoc-export.md) |
| Správa a výstupy | Štítky, omezený vratný koš, exportní snapshoty/zámky, přenosný archiv profilu, logo/podpis, hromadné PDF, RTF, dodací list a potvrzení úhrady | [Stav P20](docs/P16_P20_COMPLETION.md#p20--úplnost-produktu) |
| Rozhraní a provoz | CS/EN, světlý/tmavý/systémový vzhled, REST a lokální MCP, vlastní server, zálohy/obnova, online PWA a Tauri klient | [Instalace](docs/INSTALLATION.md), [PWA](docs/specs/pwa.md), [desktop](docs/DESKTOP_RELEASES.md) |

## Současná omezení a zbývající práce

- Podporované cizí měny jsou implementované u dokladů, banky i přehledů s uloženými kurzy. Pokladna/prodejky zůstávají CZK. Vyúčtování neplátce bez DDPP dovoluje různé obchodní kurzy; různé daňové kurzy DDPP plátce zůstávají nepodporované. [Měnový rozsah](docs/specs/report-currencies.md), [vyúčtování](docs/specs/advance-settlement.md).
- Zbývají pravidelné zálohy, rozšíření nákupního editoru a výstupů, další hromadné operace, typy koše a tiskové preference. [Checklist](docs/FEATURE_CHECKLIST.md).
- Evidence DPH a uživatelská klasifikace nenahrazují přiznání DPH, kontrolní/souhrnné hlášení ani právní posouzení obratu. Úplné daňové režimy a podání zůstávají otevřené.
- Importní předvolby služeb nejsou ověřené nativní adaptéry. Obecný import nepřenáší zdrojová PDF/přílohy; přenosný profilový archiv není záloha pro obnovu PostgreSQL ani automaticky importovatelná migrace. Neověřené Money S4/S5 a další formáty řeší obecné mapování nebo [vlastní adaptér](docs/INTEGRATION_ADAPTERS.md).
- Parsery bank a bankovní napojení mají syntetické testy; živé ověření konkrétních bank/exportů, další nativní adaptéry a externí terminálový hardware zbývají.
- Výpočty a skladové ocenění mají dokumentovaná vlastní pravidla; úplná numerická shoda s iDokladem není potvrzená.
- Správa členství pracuje s existujícími účty. E-mailové pozvánky, dvoufázové ověření a vzdálené MCP přes OAuth zbývají.
- Desktop vyžaduje samostatný backend a PostgreSQL. Existuje build matice a opt-in podepsaný updater; produkční platformní podpisy/notarizace, skutečné OS instalace/upgrady a osobní desktopový runtime zbývají. PWA nepodporuje offline fakturaci; instalace na fyzických zařízeních zatím není ověřená.
- Veřejná beta vyžaduje dokončení provozního, bezpečnostního a zátěžového ověření. Rozsah funkcí nelze zaměňovat s připraveností vydání.

## Podklady pro vývoj

Pravidla určují [specifikace](docs/README.md#specifikace-modulů). Historický průzkum trhu se zdroji zůstává v [referenčním soupisu](docs/research/FEATURES_REFERENCE.md); dokládá původ požadavků, nikoli současný stav implementace. Celý přijatý rozsah zůstává v [plánu P00–P20](docs/IMPLEMENTATION_PLAN.md); tento přehled nemění jeho akceptační kritéria.
