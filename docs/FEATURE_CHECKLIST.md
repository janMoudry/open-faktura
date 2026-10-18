# Checklist funkcí Open Faktury

Stav k 18. 10. 2026. Aktuální souhrn P16–P20: [stav dokončení](P16_P20_COMPLETION.md). [Zpět na README](../README.md) · [Implementační plán](IMPLEMENTATION_PLAN.md) · [Úplný cílový rozsah](research/FEATURES_REFERENCE.md)

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
- [ ] Širší daňové režimy a různé kurzy záloh/DDPP/vyúčtování.
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
- [ ] Úplný přenositelný datový export včetně dokumentů a příloh.

Předvolba zdroje v importu není důkaz kompatibility s jeho exportem. Dostupnost CSV ve FaktuřeOnline není potvrzena. [Hranice importu](specs/data-migration.md)

## Další účetní moduly — P13–P20

- [x] P13-01: zálohové faktury, samostatná řada ZF, PDF a ruční úhrady/storna. [Rozsah a odchylky](specs/advance-invoices.md).
- [x] P13-02: daňový doklad z úhrady, náhled DPH, řada DDPP, neměnné PDF a seznam/detail. [Rozsah a odchylky](specs/advance-tax-receipts.md).
- [ ] Daňové doklady: e-mail/přílohy, automatizace, členění DPH a opravy vydaných dokladů.
- [x] P13-03: vyúčtování jedné/více plně zaplacených záloh, rezervace, uložené daňové odpočty, doplatek/nulová faktura, PDF, REST/MCP a CS/EN. [Rozsah a odchylky](specs/advance-settlement.md).
- [ ] Částečné vyúčtování, přeplatky a vrácení záloh; numerická shoda odpočtů s referenční aplikací.
- [x] P13-04: dobropisy z vydaných faktur, částečné opravy, PDF, zápočet, ruční vratky/storna, REST/MCP a CS/EN. [Rozsah a odchylky](specs/credit-notes.md).
- [ ] Rozšíření oprav: e-mail/přílohy dobropisů, přímé opravy DDPP, cizí měny a vrubopisy.
- [x] P14-01: pravidelné CZK faktury, kalendář/ukončení/náhled, pauza/aktivace, historie, volitelné vystavení a PDF e-mail, CS/EN a REST/MCP. [Rozsah a odchylky](specs/recurring-invoices.md).
- [x] P14-02: ruční upomínky/poděkování, náhled, PDF a historie; automatizace profilu, intervaly/limit, pauza faktury, kontrola aktuální úhrady před SMTP, CS/EN a REST/MCP. [Rozsah a odchylky](specs/invoice-communication.md).
- [x] P14-03: veřejné odkazy faktur, původní PDF, aktuální stav úhrady, expirace/odvolání, e-mail s odkazem a historie zobrazení, CS/EN a REST/MCP. [Rozsah a odchylky](specs/invoice-sharing.md).
- [ ] P14: pravidelné zálohy, proměnné v textech, více příjemců, editace automatických e-mailových šablon, pravidla na kontaktu.
- [x] P15-01: přijaté faktury v CZK, vlastní PF řada, snapshot dodavatele, přesné částky z originálu, přílohy, seznam/filtry, ruční výdajové úhrady/storno, revize/audit, REST/MCP a CS/EN. [Hranice](specs/received-invoices.md).
- [x] P15-02: účtenky v CZK, UC řada, hotově/kartou, atomická úhrada a auditované opravy, přílohy, společný seznam úhrad a základní přehled nákupů/výdajů/závazků, REST/MCP a CS/EN. [Hranice](specs/purchase-receipts.md).
- [ ] P15: dodavatelská role adresáře, množství/jednotkové ceny, ceník, přeplatky, cizí měny, tisk rekapitulace a e-mail účetnímu.
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
- [x] P16-13h: české i zahraniční IBAN bankovních účtů, kontrola formátu a checksumu, import se shodou účtu/měny; bez rozšíření IBAN na dokladech a QR ([rozsah](specs/bank-currencies.md#p16-13h--zahraniční-iban-bankovních-účtů)).
- [x] P16-04: CZK pokladny, počáteční stav, PP/VP doklady, saldo za období, storno, ruční párování nových/existujících hotovostních úhrad, REST/MCP a CS/EN. [Hranice](specs/cash-register.md).
- [x] P16-05: volitelná hlavní pokladna, automatické PP/VP ze skutečných hotovostních úhrad vydaných/přijatých faktur a účtenek, opravy/storna, PDF a náhled tisku v CS/EN. [Rozsah](specs/cash-register.md).
- [x] P16-06: jedna samostatná položka pokladního dokladu s DPH, cena net/gross, serverový náhled, uložený rozpis, filtr a PDF CS/EN. Bez členění DPH a daňových výkazů. [Rozsah](specs/cash-register.md).
- [ ] P16: dokončení měnových vazeb záloh/dobropisů, členění DPH, externí terminály a konfigurace tiskových sestav.
- [x] CZK prodejky: kombinované úhrady, terminál, číslování, historie a PDF CS/EN.
- [x] Dávkové zaúčtování: náhled, uložené rozpisy, pokladní příjem pouze hotovosti, replay a storno; PDF celé dávky včetně interních skupin doplňuje P16-08b.
- [x] Inbox základ: originály, deduplikace, archiv a atomické vytvoření přijaté faktury s přílohou v CS/EN.
- [ ] P17: dokončení inboxu, ISDOC/QR, e-mailový příjem a volitelné vytěžování.
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
- [ ] P19: další přehledy, daňové podklady a účetní exporty.
- [x] P20: štítky a serverové filtry, koš nepropojených návrhů, exportní snapshoty a zámky, hromadné PDF.
- [ ] P20: koš dalších dokladů, anonymizace kontaktů a zbývající hromadné akce.
- [x] P20: logo, podpis/razítko a lokalizované patičky nových faktur, nabídek, záloh, pokladních dokladů, DDPP a dobropisů.
- [x] P20: RTF faktury, dodací list a potvrzení úhrady.
- [ ] P20: širší tiskové preference a výstupy dalších typů dokladů.

## Rozhraní a distribuce — L10N-01, P11, PERSONAL-01

- [x] České a anglické UI, přepínání jazyka a zapamatování volby na zařízení.
- [x] Responzivní rozhraní se společnými tokeny a současnou rumělkovou identitou.
- [x] Vývojový katalog komponent mimo produkční vstup.
- [x] Tauri klient sdílející webové UI a připojený k serveru.
- [ ] Dokončení a ověření všech zbývajících rozdílů proti referenčnímu UX.
- [ ] Instalovatelná PWA a offline režim.
- [x] Matice CI pro instalátory do draft GitHub Releases; lokální macOS debug DMG ověřený sestavením a integritou.
- [ ] Ověření instalace na všech OS, produkční podpisy a automatické aktualizace.
- [ ] Osobní desktop s přibaleným lokálním Rust/PostgreSQL runtime.
- [x] Světlý, tmavý a systémový vzhled s uložením preference uživatele v DB.
- [ ] Další personalizace a preference oznámení.

[Matice shody s iDokladem](design/idoklad-parity.md) · [Lokalizace](LOCALIZATION.md) · [Plán](IMPLEMENTATION_PLAN.md)

- [x] P16-10: počáteční stav bankovního účtu s revizí a důvodem změny, saldo za období nad všemi pohyby, REST/MCP a CS/EN. [Pravidla](specs/bank-balances.md).
- [x] P16-16 (část): částka slovy v CS/EN na nově generovaných pokladních PDF; historická uložená PDF se nemění. Konfigurace tiskových sestav zbývá.

Úplný zbývající rozsah P16 sleduje [plán dokončení](P16_COMPLETION.md); P16 jako celek není dokončena.

- [x] P16-09a: správa označení prodejních terminálů (profil, revize, archiv/obnova, REST/MCP a CS/EN). Volba terminálu a jeho snapshot na prodejce jsou hotové; napojení platebního hardwaru zbývá. [Specifikace](specs/sales-terminals.md).

- [x] QR Platba v nových PDF faktur a záloh s platnými bankovními údaji.
- [ ] QR Faktura a kombinovaný formát Platba+F.

- [x] P19-01c: měsíční, kvartální a roční přehled CZK hodnot dokladů, kumulace a srovnání roku, REST/MCP a CS/EN. [Specifikace](specs/period-report.md).

- [x] P19-01d: bilance prodeje/nákupu, TOP 3 odběratelé a dlužníci, prodlení pohledávek/závazků; aktuální CZK zůstatky, REST/MCP, CS/EN. [Rozsah a pravidla](specs/dashboard-finance.md).

- [x] P19-01e: [Evidence DPH a orientační obrat](specs/tax-dashboard.md) — uložené daně, DDPP/odpočty, měsíce/kvartály a dohledání zdrojů, REST/MCP a CS/EN.
- [ ] Daňové členění pro zákonný obrat, nárok na odpočet a přiznání; měnové návaznosti záloh, oprav a bankovního párování.

- [x] P19-01f: samostatná pokladní plnění v evidenci DPH a orientačním obratu, podle uloženého rozpisu a DUZP, bez dvojího započtení úhrad.

- [x] P19-01g: měna a uložený kurz běžných vydaných/přijatých faktur, ruční bankovní úhrady v měně dokladu a přepočty přehledů do CZK. Rozsah a zbývající měnové moduly: [specifikace](specs/report-currencies.md).

P19-01h: cizoměnové dobropisy, zápočty a ruční bankovní vratky se snapshotem původního kurzu, přepočty přehledů a CS/EN. [Rozsah a omezení](specs/credit-notes.md).

P19-01i: cizoměnové zálohy, ruční bankovní úhrady, DDPP a vyúčtování se shodným uloženým kurzem. Rozdílné kurzy zůstávají nepodporované. [Pravidla a rozsah](specs/advance-settlement.md).

P19-01j: návrh kurzu ČNB na vyžádání v editoru, REST/MCP, kontrola profilu a CS/EN. Ukládá se vybraný kurz; automatika pravidelných dokladů a kurzové rozdíly zbývají. [Rozsah a omezení](specs/exchange-rates.md).

P16-13a / P19-01k: ruční párování CZK pohybu s běžnou cizoměnovou vydanou/přijatou fakturou, oddělené částky a uložený rozdíl vůči kurzu dokladu. Bez automatického odhadu a účetního zápisu. Cizoměnové účty, existující úhrady a širší měnové návaznosti zbývají. [Specifikace](specs/bank-foreign-matching.md).

P16-13b / P19-01l: připojení existujících cizoměnových převodních úhrad běžných faktur bez duplikace; odpojení zachová původní platbu. CS/EN, REST/MCP a migrace 053. [Pravidla a zbývající rozsah](specs/bank-foreign-matching.md).

P19-01m: souhrnný přehled uložených kurzových rozdílů z aktivních bankovních vazeb, zisky/ztráty/netto, období a stránkované zdroje s odkazy. CS/EN, REST/MCP; bez účetních zápisů a nové migrace. [Specifikace](specs/exchange-difference-report.md).

P16-13c: cizoměnové bankovní účty, ruční pohyby a CSV/camt/Fio/MONETA importy,
saldo v měně účtu, kontrola měny a migrace 054. CS/EN, REST/MCP.
Párování z cizoměnových účtů a zahraniční RB ABO stále navazují.
[Specifikace](specs/bank-currencies.md).

P16-13d: ruční párování a rozdělení cizoměnových pohybů s běžnými fakturami
ve stejné měně, existující převodní úhrady bez duplikace, odpojení a CS/EN.
Bez nové migrace; REST/MCP sdílejí bankovní příkazy. Automatiku doplňuje P16-13e,
zálohy/vratky P16-13f. Různé měny a zahraniční IBAN zbývají.
[Specifikace](specs/bank-currencies.md#p16-13d--ruční-párování-ve-shodné-měně).

P16-13e (18. 10. 2026): dávkové a opt-in automatické párování běžných faktur
ve shodné měně, konkurenční pohyby se porovnávají jen v téže měně. CS/EN,
stejné REST/MCP příkazy a worker, bez migrace. Ruční non-CZK párování záloh/vratek
doplňuje P16-13f; různé měny a zahraniční IBAN zbývají. [Specifikace](specs/bank-currencies.md).

P16-13f (18. 10. 2026): ruční a dělené párování záloh a vratek dobropisů
z účtů ve shodné měně, připojení existujících převodních úhrad bez duplicity,
ochrany DDPP a rezervací. CS/EN, společné REST/MCP, bez migrace.
Zbývá párování rozdílných měn z non-CZK účtů, zahraniční IBAN a automatika
záloh/vratek. [Specifikace](specs/bank-currencies.md).

P16-13g: ruční párování různých měn z cizoměnové banky s běžnou vydanou
a přijatou fakturou, včetně připojení existující úhrady. Explicitní bankovní
kurz do CZK, uložené hodnoty obou stran a rozdíl; migrace 055, REST/MCP a CS/EN.
Zbývají rozdílné měny záloh/vratek, jejich automatika a zahraniční IBAN.
[Specifikace](specs/bank-foreign-matching.md#p16-13g--rozdílné-měny-z-cizoměnového-účtu).

### Dodávka 18. 10. 2026 — P16-13i, P16-08b, P16-14b

- [x] Zahraniční IBAN profilů a vystavených dokladů; historické PDF beze změny. QR zůstává CZ/CZK.
- [x] PDF/tisk souhrnného zaúčtování prodejek v CS/EN, snapshot vystavitele, cache po revizích, označení storna (migrace 056).
- [x] Samostatný opt-in pro automatické párování záloh a vratek ve stejné měně, existující úhrady bez duplicity (migrace 057).

Specifikace: [měny](specs/bank-currencies.md), [prodejky](specs/sales-receipts.md),
[automatika](specs/bank-auto-matching.md). Neuzavírá celé P16–P20. Otevřené
zůstávají rozdílné měny záloh/vratek, notifikace bank, příjem e-mailů a
vytěžování, S4/S5, účetní/daňové exporty a zbývající funkce P20.
