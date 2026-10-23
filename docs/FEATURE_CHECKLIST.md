# Checklist funkcí Open Faktury

Stav zkontrolován 23. 10. 2026 proti implementaci v repozitáři. Aktuální souhrn P16–P20: [stav dokončení](P16_P20_COMPLETION.md). [Zpět na README](../README.md) · [Implementační plán](IMPLEMENTATION_PLAN.md) · [Úplný cílový rozsah](research/FEATURES_REFERENCE.md)

Zaškrtnutí znamená implementaci v uvedeném rozsahu, nikoli úplnou shodu s iDokladem nebo připravenost celého produktu pro produkci. Nezaškrtnuté položky chybí nebo vyžadují dokončení a ověření. Podrobné podmínky a testy uvádějí odkazované specifikace. Checklist aktualizujte s každou dodávkou; nenahrazuje úplný soupis FEATURES.

## Vzhled — UI-REDESIGN-01

- [x] Vlastní logo a rumělková vizuální identita, společné tokeny a komponenty.
- [x] Responzivní shell, Přehled a seznam/editor faktur; CS/EN, světlý a tmavý režim.
- [x] INV-DUE-DAYS-01: splatnost faktury ihned, výběrem 7/14/21/30/60 dní nebo vlastním datem, zachování odstupu při změně vystavení ([pravidla](specs/invoice-drafts.md#inv-due-days-01--zadání-splatnosti-ve-dnech)).
- [x] INV-DEFAULT-DUE-01: výchozí splatnost firmy stejnými předvolbami včetně Ihned a vlastního počtu dní; automatické předvyplnění nové faktury ([pravidla](specs/company-setup.md#inv-default-due-01--výchozí-splatnost-nových-faktur-17-10-2026)).
- [x] Přehled používá existující API; chybějící data nenahrazuje ukázkovými hodnotami.

[Rozsah, snímky a limity ověření](design/redesign-2026-10-17/README.md)

## Instalace, účty a profily — P01–P03, P11

- [x] Produkt oddělený od vývojového sandboxu.
- [x] Docker Compose, PostgreSQL migrace a kontrola dostupnosti API/DB.
- [x] Jednorázové založení správce chráněné instalačním klíčem.
- [x] Přihlášení, odhlášení, serverové relace a CLI obnova přístupu.
- [x] Nastavení jména, přihlašovacího e-mailu a hesla.
- [x] Více fakturačních profilů, přepínání a nastavení.
- [x] Profilová izolace, role owner/editor/viewer a audit implementovaných operací.
- [x] Základní postupy zálohy a obnovy databáze.
- [x] Správa existujících členů podle e-mailu, role, převod vlastnictví a odchod; ochrana posledního vlastníka.
- [ ] E-mailové pozvánky dosud nezaregistrovaným uživatelům.
- [ ] Dvoufázové ověření a další správa účtu z úplného rozsahu.
- [ ] Veřejná beta: provozní, bezpečnostní a zátěžové ověření.
- [x] Projektová licence GNU AGPL v3 (`AGPL-3.0-only`), [znění](../LICENSE) a [vysvětlení](LICENSING.md).

[Instalace](INSTALLATION.md) · [Profily](specs/company-setup.md) · [Účet](specs/account-settings.md)

## Odběratelé — P04, P19

- [x] Seznam, hledání, stránkování, detail a editor kontaktů.
- [x] Fakturační a dodací adresy, kontaktní osoby a platební údaje.
- [x] Doplnění údajů podle IČO z ARES; ruční zadání bez registru.
- [x] Archivace/obnova a ochrana proti přepsání souběžné úpravy.
- [x] Ruční ověření českého DIČ přes oficiální RWS a EU DIČ přes VIES; výpadek služby není neplatné DIČ. [RWS](specs/vat-registry.md) · [VIES](specs/vies.md).
- [ ] Rozšíření pro nákupní/dodavatelskou agendu.

[Specifikace kontaktů](specs/contacts.md)

## Faktury a úhrady — P05–P08

- [x] Uložení a kopírování návrhů, editor položek a výběr odběratele.
- [x] Serverové hledání, filtry podle stavu, odběratele, data a částky; řazení.
- [x] Výpočty v desetinné aritmetice podle vlastních pravidel `of-czk-v1`.
- [x] Vystavení tuzemské CZK faktury, číslování, historická kopie údajů a audit.
- [x] PDF a náhled pro tisk; změna kontaktu nemění vystavený doklad.
- [x] Plné a částečné ruční úhrady, přeplatek a oprava/storno úhrady.
- [x] Základní přehled pohledávek a plateb aktivního profilu.
- [ ] Úplně ověřená numerická shoda s iDokladem.
- [x] Podporované cizí měny, uložené kurzy a přepočty přehledů podle [rozsahu](specs/report-currencies.md).
- [x] Různé obchodní kurzy plně uhrazených záloh a vyúčtování neplátce bez DDPP. [Rozsah](specs/advance-settlement.md#rozšíření-18-10-2026--různé-obchodní-kurzy-neplátce).
- [ ] Širší daňové režimy a různé daňové kurzy DDPP při vyúčtování plátce.
- [ ] Kompletní opravný tok vystavených dokladů.

[Výpočty](specs/invoice-calculation.md) · [Vystavení](specs/invoice-issue.md) · [Úhrady](specs/invoice-payments.md)

## Ceník, šablony a nabídky — P12

- [x] Ceník: seznam/detail/editor, archiv/obnova a výběr položky do faktury.
- [x] Opakovaně použitelné šablony faktur a nabídek.
- [x] Nabídky: seznam, filtry, editor, číslování a stavy.
- [x] Převod nabídky na jeden navázaný návrh faktury a následné vystavení.
- [x] PDF nabídky v CS/EN a náhled pro tisk.
- [x] Odvolatelné veřejné odkazy s expirací a historií zobrazení.
- [ ] Diskuse, identifikace čtenáře a širší spolupráce nad nabídkou.
- [ ] Cizí měny/kurzy a zbývající rozsah P12.

[Ceník](specs/catalog.md) · [Šablony a nabídky](specs/sales-documents.md)

## E-maily, přílohy a exporty — P10

- [x] Odeslání vystavené faktury s PDF a historie pokusů.
- [x] Odeslání nabídky s PDF nebo veřejným odkazem.
- [x] Trvalá fronta a zachování odeslané verze PDF.
- [x] SMTP nastavení v UI správce instalace a testovací odeslání.
- [x] Vlastní přílohy: nahrání, stažení, odstranění a výběr do e-mailu.
- [x] Historicky odeslané přílohy zůstávají dostupné po odstranění z dokladu.
- [x] CSV/XLSX export faktur a odběratelů: filtry, řazení, výběr sloupců a CS/EN.
- [ ] Více příjemců, hromadné odesílání a globální historie napříč doklady.
- [x] Veřejné odkazy na faktury, expirace/odvolání a historie zobrazení (P14-03).

Export má nejvýše 10 000 řádků, zachovává hodnoty jako text a není úplným datovým ani účetním exportem. Přílohy mají limit 5 souborů po 2 MiB na doklad. Přijetí SMTP serverem není potvrzení doručení nebo přečtení.

[E-mail faktur](specs/invoice-delivery.md) · [SMTP](specs/smtp-settings.md) · [Přílohy](specs/document-attachments.md) · [Export seznamů](specs/register-export.md)

## Migrace a integrace — P09, P19–P20

- [x] Obecný import CSV/XLSX s mapováním, náhledem a reportem.
- [x] Profilová izolace importu a ochrana proti duplicitnímu opakování.
- [x] REST kontrakty a generovaný klient pro implementované moduly.
- [x] Lokální MCP přes stdio se společnými příkazy a oprávněními.
- [ ] Ověřený zdrojový adaptér iDokladu na autentických anonymizovaných exportech.
- [ ] Ověřený zdrojový adaptér FakturyOnline podle skutečně dostupných formátů.
- [ ] Úplná migrace položek, úhrad, vazeb a příloh podle zdrojových možností.
- [ ] Účetní exporty do konkrétních systémů, webhooks a další integrace.
- [ ] Vzdálené MCP s OAuth.
- [x] Přenosný ZIP archiv profilu s explicitním manifestem, dokumenty/přílohami a redakcí tajemství; není záloha pro obnovu PostgreSQL. [Rozsah a limity](specs/company-administration.md#přenosný-datový-archiv).
- [ ] Automatický import profilového archivu a úplné migrační pokrytí všech zdrojů.

Předvolba zdroje v importu není důkaz kompatibility s jeho exportem. Dostupnost CSV ve FaktuřeOnline není potvrzena. [Hranice importu](specs/data-migration.md)

## Další účetní moduly — P13–P20

- [x] P13-01: zálohové faktury, samostatná řada ZF, PDF a ruční úhrady/storna. [Rozsah a odchylky](specs/advance-invoices.md).
- [x] P13-02: daňový doklad z úhrady, náhled DPH, řada DDPP, neměnné PDF a seznam/detail. [Rozsah a odchylky](specs/advance-tax-receipts.md).
- [ ] Daňové doklady: e-mail/přílohy, automatizace, členění DPH a opravy vydaných dokladů.
- [x] P13-03: vyúčtování jedné/více plně zaplacených záloh, rezervace, uložené daňové odpočty, doplatek/nulová faktura, PDF, REST/MCP a CS/EN. [Rozsah a odchylky](specs/advance-settlement.md).
- [ ] Částečné vyúčtování, přeplatky a vrácení záloh; numerická shoda odpočtů s referenční aplikací.
- [x] P13-04: dobropisy z vydaných faktur, částečné opravy, PDF, zápočet, ruční vratky/storna, REST/MCP a CS/EN. [Rozsah a odchylky](specs/credit-notes.md).
- [x] Cizoměnové dobropisy, zápočty a vratky s uloženým kurzem zdrojové faktury.
- [ ] Rozšíření oprav: e-mail/přílohy dobropisů, přímé opravy DDPP a vrubopisy.
- [x] P14-01: pravidelné faktury v CZK i podporovaných cizích měnách, kalendář/ukončení/náhled, pauza/aktivace, historie, volitelné vystavení a PDF e-mail, CS/EN a REST/MCP. [Rozsah a odchylky](specs/recurring-invoices.md).
- [x] P14-02: ruční upomínky/poděkování, náhled, PDF a historie; automatizace profilu, intervaly/limit, pauza faktury, kontrola aktuální úhrady před SMTP, CS/EN a REST/MCP. [Rozsah a odchylky](specs/invoice-communication.md).
- [x] P14-03: veřejné odkazy faktur, původní PDF, aktuální stav úhrady, expirace/odvolání, e-mail s odkazem a historie zobrazení, CS/EN a REST/MCP. [Rozsah a odchylky](specs/invoice-sharing.md).
- [ ] P14: pravidelné zálohy, proměnné v textech, více příjemců, editace automatických e-mailových šablon, pravidla na kontaktu.
- [x] P15-01: přijaté faktury v CZK, vlastní PF řada, snapshot dodavatele, přesné částky z originálu, přílohy, seznam/filtry, ruční výdajové úhrady/storno, revize/audit, REST/MCP a CS/EN. [Hranice](specs/received-invoices.md).
- [x] P15-02: účtenky v CZK, UC řada, hotově/kartou, atomická úhrada a auditované opravy, přílohy, společný seznam úhrad a základní přehled nákupů/výdajů/závazků, REST/MCP a CS/EN. [Hranice](specs/purchase-receipts.md).
- [x] Přijaté faktury v podporovaných cizích měnách s uloženým kurzem a přepočty přehledů do CZK.
- [ ] P15: dodavatelská role adresáře, množství/jednotkové ceny, ceník, přeplatky, tisk rekapitulace a další výstupy; předání originálu z inboxu účetnímu je implementované samostatně v P17.
- [x] P16-01: profilové CZK účty, archivace/obnova, ruční pohyby, vlastní CSV s náhledem a deduplikací, ruční párování vydaných/přijatých faktur a existujících karetních úhrad účtenek, odpojení/historie, REST/MCP, CS/EN. [Hranice](specs/banking.md).
- [x] P16-02: adaptéry podle veřejných kontraktů (camt.053, Fio JSON, MONETA XML, RB ABO), kontrola účtu, atomický náhled/import a návrhy shod VS/částka s potvrzením, REST/MCP, CS/EN. [Přesné varianty a omezení](specs/bank-statements.md).
- [ ] P16-02: ověření anonymizovaných originálů všech uvedených bank a doplnění nepokrytých variant; syntetické testy nejsou certifikace kompatibility.
- [x] P16-03: hromadný náhled a potvrzení přesných shod z aktuální stránky, vysvětlené přeskočení, konflikty i mimo výběr, atomický zápis/rollback, REST/MCP a CS/EN. [Rozsah](specs/bank-batch-matching.md).
- [x] P16-15a: vlastní přístupy k bankovnímu účtu, šifrování, test spojení, ruční synchronizace a její stav, odpojení, REST/MCP a CS/EN. Přímé Fio a instalačně registrovaná gateway; [kontrakt a omezení](specs/bank-connections.md).
- [x] P16-15b: volitelná automatická synchronizace (15 min / 1 h / 6 h / den), worker s profilovými oprávněními, příští běh a diagnostika, CS/EN. Bez automatického párování.
- [x] P16-15: autentizovaná vlastní brána bankovních notifikací se společným importem.
- [ ] Další nativní adaptéry/PSD2 poskytovatelé a živé ověření. Vlastní gateway není hotový adaptér všech bank.
- [x] P16-14: opt-in průběžné párování přesných shod, oprávnění aktéra, atomické dávky, audit a CS/EN stav. [Pravidla a omezení](specs/bank-auto-matching.md).
- [x] P16-11: rozdělení pohybu mezi více dokladů, přesný nerozdělený zbytek/přeplatek, atomické odpojení a CS/EN. [Rozsah](specs/bank-split-matching.md).
- [x] P16-12: ruční bankovní/pokladní párování záloh a vratek dobropisů, vlastní platební operace, existující úhrady bez zdvojení a ochrana DDPP/vyúčtování. [Rozsah](specs/treasury-documents.md).
- [x] P16-13: cizoměnové účty, importy, shodné i rozdílné měny faktur, záloh a vratek; explicitní kurzy a existující úhrady bez duplicity.
- [x] P16-13h/i: české i zahraniční IBAN bankovních účtů i profilů/dokladů, kontrola formátu a checksumu, import se shodou účtu/měny; QR má vlastní omezení ([rozsah](specs/bank-currencies.md#p16-13h--zahraniční-iban-bankovních-účtů)).
- [x] P16-04: CZK pokladny, počáteční stav, PP/VP doklady, saldo za období, storno, ruční párování nových/existujících hotovostních úhrad, REST/MCP a CS/EN. [Hranice](specs/cash-register.md).
- [x] P16-05: volitelná hlavní pokladna, automatické PP/VP ze skutečných hotovostních úhrad vydaných/přijatých faktur a účtenek, opravy/storna, PDF a náhled tisku v CS/EN. [Rozsah](specs/cash-register.md).
- [x] P16-06: jedna samostatná položka pokladního dokladu s DPH, cena net/gross, serverový náhled, uložený rozpis, filtr a PDF CS/EN. Bez členění DPH a daňových výkazů. [Rozsah](specs/cash-register.md).
- [ ] P16: úplné členění DPH, externí terminály a další konfigurace tiskových sestav.
- [x] CZK prodejky: kombinované úhrady, terminál, číslování, historie a PDF CS/EN.
- [x] Dávkové zaúčtování: náhled, uložené rozpisy, pokladní příjem pouze hotovosti, replay a storno; PDF celé dávky včetně interních skupin doplňuje P16-08b.
- [x] Inbox základ: originály, deduplikace, archiv a atomické vytvoření přijaté faktury s přílohou v CS/EN.
- [x] P17: omezené ISDOC/PDF a QR adaptéry, autentizovaná e-mailová brána, IMAP a volitelné vytěžování; [rozsah](specs/inbox.md).
- [x] P17: vlastní oznámení nových originálů a chyb příjmu, opt-in kategorií, historie a označení přečtení podle uživatele/profilu.
- [x] P17: odpojení zpracovaného originálu s historií; účetní doklad a příloha zachované, opětovné připojení ke stejnému dokladu bez další kopie.
- [x] P17: předání jednoho originálu účetnímu jako SMTP přílohy s explicitním potvrzením, historií a UUID replay; [rozsah](specs/inbox-delivery.md).
- [x] P18 základ: skladové karty, ruční pohyby, reverzy a inventury s revizemi, historií a CS/EN.
- [x] P18-03a: statistika vybrané karty za období, přesná salda, ruční příjmy/výdeje a oddělené inventurní/počáteční/opravné zápisy.
- [x] P18-02: skladové vazby faktur, potvrzených nákupů, prodejek a množstevních vratek dobropisů; atomické opravy/storna a dohledatelnost.
- [x] P18-03b: souhrnný stav k datu a ručně zadané skladové ceny s platností, historií, součty a označením neoceněných zásob.
- [x] P18-03c: explicitní pořizovací cena nákupního příjmu, snapshot a konzervativní automatický vážený průměr běžných příjmů/výdejů.
- [x] P18-03d: vratky v původní výdejové ceně, přepočet cenových oprav zásoby/výdeje, rozvržení vedlejších nákladů podle pořizovací hodnoty.
- [x] P18-03e: pořizovací ocenění ručních/počátečních příjmů a inventurních přebytků, storno původní hodnoty. Numerická shoda iDokladu není doložená.
- [x] P18-04: přehled prodeje ceníkových položek, období/typ, skutečné uložené částky, zdrojové doklady, sloupce a CSV/XLSX, REST/MCP, CS/EN. [Specifikace](specs/item-sales.md).
- [x] P18-05a: import základního ceníku Money S3 XML s náhledem, konflikty a deduplikací. [Rozsah](specs/catalog-import.md).
- [x] P18-05b: pro neověřené varianty Money S4/S5 je podle rozhodnutí uživatele dostupné obecné mapování a [vlastní adaptér](INTEGRATION_ADAPTERS.md); nejde o nativní podporu S4/S5 (E-059).
- [x] P18-05c: obecný import ceníku CSV/XLSX s mapováním a přesnými cenami, CS/EN.
- [x] Měsíční CZK přehled prodeje/nákupu, kumulace, meziroční srovnání a zdrojové doklady s profilovou izolací.
- [x] P19: evidence DPH a orientační obrat včetně samostatných pokladních plnění, export zdrojů CSV/XLSX. [Rozsah](specs/tax-dashboard.md).
- [x] P19: explicitní uživatelská klasifikace obratu/vstupní DPH a profil daňového subjektu; nejde o podání. [Klasifikace](specs/tax-classification.md) · [Profil](specs/taxpayer-filing-profile.md).
- [ ] P19: další přehledy, úplné daňové režimy, přiznání DPH/KH/SH a další nativní účetní exporty.
- [x] P20: štítky a serverové filtry, koš nepropojených návrhů, exportní snapshoty a zámky, hromadné PDF.
- [x] P20: pořadí, viditelnost a kompaktní/podrobná varianta widgetů přehledu.
- [x] P20: nevratná anonymizace adresáře a jeho historie vlastníkem; historické finanční snapshoty a původní dokumenty zachované. [Hranice](specs/contact-anonymization.md).
- [x] P20: koš nepoužitých nabídek a šablon; obnova a audit, ochrana odeslaných/sdílených/navázaných/exportovaných zdrojů.
- [ ] P20: další přípustné typy koše a zbývající hromadné akce.
- [x] P20: logo, podpis/razítko a lokalizované patičky nových faktur, nabídek, záloh, pokladních dokladů, DDPP a dobropisů.
- [x] P20: RTF faktury, dodací list s cenami/bez cen (předvolba profilu i jednorázová volba) a potvrzení úhrady.
- [x] P20 (část): firemní text před položkami CS/EN v uloženém vzhledu nových faktur, nabídek, záloh, DDPP a dobropisů; pokladna a historická PDF beze změny. DB 4/4, UI CS/EN desktop/mobile ověřeno v product-admin 12/12. [Rozsah](specs/document-style.md).
- [ ] P20: další tiskové preference (včetně nedoloženého data tisku) a výstupy dalších typů dokladů.

## Rozhraní a distribuce — L10N-01, P11, PERSONAL-01

- [x] České a anglické UI, přepínání jazyka a zapamatování volby na zařízení.
- [x] Responzivní rozhraní se společnými tokeny a současnou rumělkovou identitou.
- [x] Vývojový katalog komponent mimo produkční vstup.
- [x] Tauri klient sdílející webové UI a připojený k serveru.
- [ ] Dokončení a ověření všech zbývajících rozdílů proti referenčnímu UX.
- [x] PWA manifest, skutečné 192/512 ikony a online registrace mimo dev/Tauri; statický CS/EN fallback ukládá pouze tři veřejné soubory ověřené SHA-256, bez offline fakturace.
- [x] Produkční PWA: desktop Chromium CDP bez chyb instalovatelnosti, skutečný SW a pouze tři statické Cache API soubory; CS/EN offline scénáře prošly na desktopu i mobilní emulaci. Samostatná produkční CI konfigurace, bez offline fakturace.
- [ ] Ověření instalace PWA na fyzickém Android/iOS a do desktopového OS; CDP a mobilní emulace instalaci do OS nedokládají.
- [x] Matice CI pro instalátory do draft GitHub Releases; lokální macOS debug DMG ověřený sestavením a integritou.
- [x] [Signed desktop updater](specs/desktop-updates.md): výchozí vypnutý, trusted endpoint/public key při sestavení, explicitní CS/EN check/install, podpis i podepsaná verze, bounded manifest/balíček a oddělené ruční potvrzení. Fixture testy neinstalují OS balíčky.
- [ ] Skutečný upgrade a zachování dat na všech cílových OS, produkční platformní podpisy/notarizace a živé vydání aktualizačního kanálu.
- [ ] Osobní desktop s přibaleným lokálním Rust/PostgreSQL runtime.
- [x] Světlý, tmavý a systémový vzhled s uložením preference uživatele v DB.
- [ ] Další personalizace a preference oznámení.

[Matice shody s iDokladem](design/idoklad-parity.md) · [Lokalizace](LOCALIZATION.md) · [Plán](IMPLEMENTATION_PLAN.md)

## Další implementované návaznosti

- [x] P16-10: počáteční stav účtu, revize/důvod změny a saldo za období nad všemi pohyby. [Pravidla](specs/bank-balances.md).
- [x] P16-09a: profilová konfigurace terminálů, archivace a snapshot na prodejce; externí hardware zbývá. [Rozsah](specs/sales-terminals.md).
- [x] P16-16 (část): částka slovy CS/EN na nových pokladních PDF; širší konfigurace sestav zbývá.
- [x] QR Platba, QR Faktura a Platba+F pro podporované nové dokumenty; historická PDF beze změny. [Podmínky](specs/payment-qr.md).
- [x] P19-01c/d: měsíční/kvartální/roční přehledy, kumulace, meziroční srovnání, bilance, TOP 3 a prodlení v CZK. [Období](specs/period-report.md) · [Bilance](specs/dashboard-finance.md).
- [x] P19-01j/n: návrh kurzu ČNB na vyžádání a explicitní volba ČNB podle plánovaného data pravidelné faktury. [Rozsah](specs/exchange-rates.md).
- [x] P19-01m: přehled uložených bankovních kurzových rozdílů; neprovádí účetní zápisy. [Pravidla](specs/exchange-difference-report.md).
- [x] P16-08b: PDF souhrnného zaúčtování prodejek včetně interních skupin a storna. [Rozsah](specs/sales-receipts.md).
- [x] P16-13j: párování různých měn záloh/vratek včetně existujících úhrad a ochran DDPP. [Rozsah](specs/bank-foreign-matching.md).
- [x] P16-14b: samostatný opt-in automatického párování záloh/vratek ve shodné měně. [Pravidla](specs/bank-auto-matching.md).

Tento checklist popisuje současný stav. Historii jednotlivých měnových a integračních dodávek zachovává [implementační plán](IMPLEMENTATION_PLAN.md); tehdejší seznamy zbývající práce nejsou aktuální backlog. P16 ani P16–P20 jako celek nejsou uzavřené; viz [akceptace P16](P16_COMPLETION.md) a [aktuální souhrn P16–P20](P16_P20_COMPLETION.md).
