# Evidence reverse-engineeringu

Průchod 10. 10. 2026. Není proveden živý API request ani průchod přihlášenou aplikací. Původní `FEATURES.md` zůstává soupisem s odkazy, nikoli potvrzením všech pravidel.

## Typy důkazů

- `documented`: tvrzení nápovědy.
- `sdk-contract`: staticky přečtený kontrakt/validace klienta v konkrétním commitu.
- `observed`: reprodukované chování aplikace/API s uloženým vstupem a výstupem.
- `proposal`: vlastní rozhodnutí Open Faktury; nedokládá referenci.

## Zdroje

Oficiální [SDK](https://github.com/Solitea/IdokladSdk/tree/f76c7e16bc45f4fa601bf2889dc81b8d45a9dafc), commit `f76c7e16bc45f4fa601bf2889dc81b8d45a9dafc`, získán 10. 10. 2026. Níže jsou cesty relativní k tomuto neměnnému odkazu. SDK bylo čteno, nikoli spuštěno. [API dokumentace](https://api.idoklad.cz/Help/v3/cs/) při textovém načtení vrátila jen „Loading…“; její kontrakt zde není považován za přečtený.

| ID | Typ | Zdroj v SDK | Zjištění a omezení |
| --- | --- | --- | --- |
| E-001 | sdk-contract | `IdokladSdk/Models/IssuedInvoice/Post/IssuedInvoicePostModel.cs` | Faktura má partnera, měnu, číselnou řadu, způsob platby, data, položky a kurz včetně množství kurzu. Anotace požadují alespoň jednu položku. Neprokazuje DB ani úplné serverové validace. |
| E-002 | sdk-contract | `IdokladSdk/Models/IssuedInvoice/Post/IssuedInvoiceItemPostModel.cs` | Položka má množství, jednotkovou cenu, typ ceny, slevu, typ i hodnotu sazby a volitelný ceník/členění DPH. Přesnost a algoritmus zaokrouhlení nejsou z toho odvoditelné. |
| E-003 | sdk-contract | `IdokladSdk/Enums/PriceType.cs` | `WithVat=0`, `WithoutVat=1`, `OnlyBase=2`. Význam OnlyBase v konkrétních daňových situacích vyžaduje ověření. |
| E-004 | sdk-contract | `IdokladSdk/Enums/PaymentStatus.cs` | `Unpaid=0`, `Paid=1`, `PartialPaid=2`, `Overpaid=3`. Neobsahuje stav po splatnosti; způsob výpočtu stavů není doložen. |
| E-005 | sdk-contract | `IdokladSdk/Models/Contact/ContactPostModel.cs` | Kontakt má IČO, DIČ, adresu, dodací adresy, bankovní údaje, individuální splatnost, slevu a nastavení upomínek. Neprokazuje povinnou unikátnost IČO. |
| E-006 | sdk-contract | `IdokladSdk/Clients/IssuedInvoiceClient.cs` | Klient nabízí vytvoření, změnu, smazání, default, kopii, přepočet a podklady pro opakování. `RecountAsync` posílá POST na `/IssuedInvoices/Recount`; výsledek lze využít při ověřování kalkulátoru. |
| E-007 | sdk-contract | `IdokladSdk/Clients/IssuedInvoiceClient.cs`, metoda `ValidatePost` | Bez navázané zálohy klient povoluje pouze normální položky; s navázanou zálohou normální a odpočtové. Server a více záloh vyžadují samostatné ověření. |

## UI průzkum 10. 10. 2026

| ID | Typ a prostředí | Zdroj | Zjištění a omezení |
| --- | --- | --- | --- |
| E-008 | documentation, veřejná česká nápověda | [Vydané faktury](https://www.idoklad.cz/podpora/faktury-vydane), [snímek filtrů](https://www.idoklad.cz/wp-content/uploads/2022/12/fv-filter.png) | Dokumentace popisuje seznam, stavové záložky, hledání a filtry; snímek ukazuje rozšířený filtr nad tabulkou. Přihlášená aplikace nebyla pozorována, aktuálnost vizuálu snímku neověřena. Podklad UX-001. |
| E-009 | documentation, veřejná česká nápověda | [Karta faktury](https://www.idoklad.cz/podpora/karta-faktury) | Karta obsahuje zákazníka, data a položky, otevření přes číslo dokladu. Neprokazuje serverové validace. Podklad UX-002. |
| E-010 | observation, veřejný nepřihlášený web, české UI | [FakturaOnline editor](https://www.fakturaonline.cz/faktura) | Bez zadání a odeslání dat otevřen výchozí formulář: základní údaje, obecné nastavení, dodavatel/odběratel, položky s množstvím/cenou, přidání položky, poznámka, součet a akce Stáhnout. Datum ve formuláři 10. 10. 2026. Na veřejné cestě seznamu nebyly pozorovány žádné uložené doklady; chování seznamu s účtem neověřeno. Podklad UX-002. |

Výstup: [vlastní návrh seznamu a editoru](../specs/invoice-design.md). Syntetická data prototypu nejsou referenční účetní fixture.

## Otevřené otázky

| ID | Otázka | Jak uzavřít | Dopad |
| --- | --- | --- | --- |
| Q-01 | Fáze a přesnost zaokrouhlení, pořadí slev | R-01, R-02, R-03 | Kalkulátor a typy částek |
| Q-02 | Návrh/vystavení, přidělení čísla, editace a smazání | R-04, R-05 | Životní cyklus a transakce |
| Q-03 | Změna kontaktu po vystavení a snapshot | R-06 | Historický obsah a PDF |
| Q-04 | Úhrady, vratky, nulové a záporné doklady | R-07 | Saldo a stavy |
| Q-05 | Oprávnění uživatele a API ve více firmách | R-08 | Izolace dat |
| Q-06 | Vyúčtování více záloh, sazby a omezení SDK | R-09 | Typované vazby dokladů |
| Q-07 | Úplné modely ostatních modulů | Inventura klientů, modelů, enumů a operací SDK | Další specifikace |

## Zápis dalšího důkazu

Přidej ID, typ, datum, URL/commit nebo verzi aplikace, otázku, konfiguraci firmy, anonymizované vstupy, kroky, skutečný výsledek, odkaz na artefakt a dotčený požadavek. U konfliktu uchovej oba důkazy. Neukládej tokeny, cookies ani skutečné osobní údaje; export anonymizuj před commitem. Nepřítomnost pole v jednom modelu není důkaz nepřítomnosti funkce v produktu.

## E-011 — nastavení firmy (10. 10. 2026)

Typ `documentation`, [iDoklad: Nastavení – Firma](https://www.idoklad.cz/podpora/nastaveni-firma),
veřejná nápověda, uvádí aktualizaci 6. 5. 2026. Popisuje identitu, sídlo, oddělený
firemní e-mail a volbu plátcovství. Podklad pro pole P03; nebyly pozorovány interní
validace, role ani transakce. Průvodce, účinnost profilů, role owner/editor/viewer,
revize a idempotence jsou vlastní kontrakt Open Faktury, nikoli uzavření Q-05.

## E-012 — rozvržení nastavení a appbar iDokladu (11. 10. 2026)

Typ `documentation` + vizuální kontrola veřejného screenshotu v prohlížeči.
Zdroje: https://www.idoklad.cz/podpora/ovladani-aplikace a
https://www.idoklad.cz/podpora/nastaveni-firma; obrázek
https://www.idoklad.cz/wp-content/uploads/2024/04/nastav-firma-zaklad.png.
Screenshot ukazuje záložky nad souvislým bílým formulářem a účetní ovládání vpravo
nahoře; nápověda popisuje hlavní moduly vlevo, globální Nový a přepínač společností.
Jde o publikovanou referenci, nikoli pozorování aktuální přihlášené aplikace.

Vlastní adaptace Open Faktury: neutrální barvy, Manrope/Lucide, kompaktní sidebar
s rozbalovacími podsekcemi, appbar s výběrem fakturačního profilu, jazykem a odhlášením.
Formulář má sekce přepínané sdíleným FilterTabs; rozepsaná data přetrvají přepnutí.
Chyba otevře odpovídající sekci. Výpočty, oprávnění a API se nemění. Nedostupné
funkce zůstávají disabled, bez převzetí loga, barev či tarifních limitů reference.

## E-013 — Adresář a karta kontaktu (11. 10. 2026)

Typ: veřejná dokumentace. Zdroj:
https://www.idoklad.cz/podpora/adresar-kontakty (stránka aktualizována 3. 6. 2024,
přečtena 11. 10. 2026). Karta obsahuje základní údaje, dodací adresy a související
doklady; popisuje kontaktní osobu, další e-mailové příjemce, individuální splatnost
a bankovní spojení, více dodacích adres s jednou výchozí. P04 navazuje na toto
členění. Dokumentace nedokládá serverové validační limity, transakce, archivaci
ani interní DB; ty jsou explicitně vlastní návrh v specs/contacts.md.

## E-014 — FakturaOnline: veřejný editor (11. 10. 2026)

Typ: přímé UI pozorování, nepřihlášený veřejný formulář.
Zdroj https://www.fakturaonline.cz/faktura. Otevřen formulář, přidána prázdná
položka, rozbaleno obecné nastavení a dialog slevy. Bez vystavení, registrace
nebo zadání reálných údajů. Pořadí sekcí, podmíněné bankovní údaje, globální
net/gross přepínač, řádkové součty a volitelné slevy jsou doložené UI.
Konkrétní adaptace a hranice: docs/design/invoice-editor-ux.md.
Nejde o důkaz účetního algoritmu ani autentizovaného adresáře.

## E-015 — Výpočty: hranice veřejné nápovědy (11. 10. 2026)

Typ: veřejná dokumentace, https://www.idoklad.cz/podpora/karta-faktury a
https://www.idoklad.cz/podpora/nastaveni-prodej. Karta popisuje procentní slevu
z celkové ceny položky včetně daně; prodejní nastavení popisuje net/gross a
zaokrouhlení podle platby. Text nastavení zmiňuje čtyřmístný koeficient;
bez kontrolního experimentu ho nelze považovat za aktuální serverový algoritmus.
Q-01 zůstává otevřené. Vlastní pravidla of-czk-v1 viz specs/invoice-calculation.md.
Rust Decimal: https://docs.rs/rust_decimal/1.43.0/rust_decimal/struct.Decimal.html.

## E-016 — Seznamy a filtry iDoklad / FakturaOnline (11. 10. 2026)

Typ: veřejná oficiální nápověda, nikoli pozorování přihlášené aplikace.
Zdroje: https://www.idoklad.cz/podpora/faktury-vydane a
https://www.fakturaonline.cz/navody/faktury/vystaveni-a-sprava-faktur/hromadne-stazeni-vystavenych-faktur.
iDoklad dokumentuje textové hledání, samostatný podrobný filtr a viditelné
aktivní podmínky s možností jednotlivého i společného zrušení. FakturaOnline
popisuje filtrování před výběrem dokladů. Open Faktura přebírá členění lišty,
rozbalitelné filtry a odnímatelné podmínky; P06 nabízí pouze rozpracované návrhy
s hledáním a filtrem odběratele. Stavy úhrad a hromadné stahování nejsou
implementované a nejsou předstírané. Vizuální provedení je vlastní adaptace.

## E-017 — P07: vystavení, tisk a hranice shody (11. 10. 2026)

Typ: veřejná oficiální nápověda, nikoli autentizovaný experiment.
https://www.idoklad.cz/podpora/faktury-vydane popisuje oddělené úhrady, tisk,
export PDF a odeslání; https://www.idoklad.cz/jak-zacit-s-idokladem/vystaveni-prvnich-dokladu
uvádí číslování nových dokladů podle nastavené řady. Zámky, idempotence a přesný
okamžik rezervace čísla veřejná stránka nedokládá. P07 proto výslovně zavádí
vlastní pravidla v specs/invoice-issue.md a ponechává referenční Q-02/Q-03 otevřené.
Renderer: https://docs.rs/genpdf/0.2.0/genpdf/ — font embedding a stránkování;
volba genpdf je implementační rozhodnutí, nikoli součást reference.

## E-018 — P08: ruční úhrady a stavy (11. 10. 2026)

Typ: oficiální veřejná nápověda, https://www.idoklad.cz/podpora/faktury-vydane
(sekce Ikony v seznamu a Ikony označující stav). Dokumentuje zadání částky a
data úhrady, částečnou úhradu, neuhrazené/uhrazené/přeplacené doklady a prodlení.
To podporuje oddělení lifecycle, úhrady a splatnosti. Stránka nedokládá interní
transakce, idempotenci, nulové doklady ani přesný auditní mechanismus oprav.
Tyto části P08 jsou vlastní pravidla popsaná v specs/invoice-payments.md;
nejde o potvrzenou úplnou shodu s přihlášenou aplikací iDoklad.

## E-019 — P09: dostupné exporty a neověřené schéma (11. 10. 2026)

Typ: veřejná oficiální nápověda, nikoli autentický export nebo přihlášený experiment.
https://www.idoklad.cz/podpora/adresar-kontakty a
https://www.idoklad.cz/podpora/faktury-vydane potvrzují CSV/XLSX export seznamu
s volitelnými sloupci. Není tím doložen obsah úplné položkové faktury.
https://www.fakturaonline.cz/navody/pokrocile-funkce/statistiky-vystavenych-faktur
ve vyhledávacím výstupu potvrzuje export kontaktů XLSX; přímé textové načtení
vracelo jen navigaci/patičku. Přesná záhlaví, source ID, účetní typy a vztahy nejsou
ověřené. Neexistuje zatím testovací export, proto předvolby P09 zůstávají návrhem.

Vlastní CSV/XLSX parser: https://docs.rs/csv/1.4.0/csv/ a
https://docs.rs/calamine/0.36.1/calamine/. Připnuté verze ověřené Cargo.lock,
syntetické soubory testují implementaci, nikoli shodu se zdrojovou službou.

## E-020 — FakturaOnline: přihlášený průchod UI (11. 10. 2026)

Typ: přímé UI pozorování v Chrome, přihlášený profil neplátce DPH, build neuveden.
Zdroje: https://www.fakturaonline.cz/faktura,
https://www.fakturaonline.cz/vystavene-faktury, https://www.fakturaonline.cz/kontakty.
Kroky, výsledky, odchylky a hranice jsou v
[průchodovém auditu](../design/reference-audit-2026-10-11.md), UX-01 až UX-07.
Důležitá korekce E-014: po načtení přihlášeného profilu se dodavatel zobrazuje
jako souhrn, odběratel začíná hledáním; teprve explicitní akce odhalí ruční pole.
Seznam má samostatný stavový sloupec. Zápisy a účetní pravidla nebyly ověřovány
na skutečných dokladech. Citlivá data reference se do repozitáře neukládají.

## E-021 — iDoklad: syntetická faktura a částečné úhrady (11. 10. 2026)

Typ: přímý zápisový UI experiment, Chrome, česká legislativa, CZK, neplátce DPH,
fyzická osoba, nová oddělená testovací agenda. Výchozí řada, bankovní převod,
žádná banka ani e-mail odběratele. Build aplikace není zveřejněný v UI.
Zdroj https://app.idoklad.cz/, `/IssuedInvoice`, `/IssuedInvoice/Preview/66832512/Info`.

1. Registrace nové syntetické agendy, kontakt z tlačítka plus přímo v editoru.
   U kontaktu stačil název; po uložení se vybral do faktury bez opuštění editoru.
2. Nový editor již zobrazuje číslo 20260001. Položka Testovací služba, 3 hod,
   jednotková cena 100 Kč, popis Syntetický test UX – neodesílat. Náhled 300 Kč.
3. Uložit zobrazí varování chybějící banky; Přesto uložit vytvoří vydanou fakturu
   20260001 s cenou 300 Kč v seznamu. To nedokládá okamžik serverové rezervace
   čísla ani chování souběžných požadavků. Samostatný draft krok zde nebyl použit.
4. Detail → Úhrady → ikona úhrady. Dialog: částka, datum, způsob úhrady, průběh
   již uhrazené částky; nepovinné poděkování ponecháno vypnuté. Zápis 100 Kč
   vytvoří řádek a stav Částečně uhrazeno.
5. Znovu otevřený dialog ukazuje 100 z 300 Kč a předvyplní 200 Kč. Potvrzení
   vytvoří druhý řádek; stav Uhrazeno. Odeslání nebylo provedeno.

Screenshoty a popisky: [index](screenshots/2026-10-11/README.md).
Numerická shoda tohoto jednoduchého neplátcovského případu nedokládá shodu DPH,
zaokrouhlení, přeplatků, oprav nebo souběhu. R-07 je ověřen pouze v uvedené části.
Rozdíly a navazující opravy: [audit](../design/reference-audit-2026-10-11.md).

## E-022 — adresář, detail a editor kontaktu v iDokladu

Pozorování aplikace v Chrome 11. 10. 2026, stejná syntetická agenda jako E-021.
Zdroj https://app.idoklad.cz/Contact a detail `/Contact/Preview/25851526/Info`.
Otevření kontaktu „Syntetický odběratel Open Faktura“, následně akce Upravit.
Pouze čtení; žádné nové kontakty ani úpravy nebyly uloženy. Verze aplikace v UI neuvedena.

- Seznam: Všechny / Odběratelé / Dodavatelé, kompaktní hledání a panel akcí;
  sloupce Firma/Jméno, IČ, DIČ, Kontaktní osoba, E-mail, Telefon, Akce;
  řazení názvu vzestupně, rozsah výsledků, stránkování a velikost stránky.
- Kliknutí na název otevře čitelný detail. Úprava je samostatný režim přes Upravit.
- Detail má Základní údaje / Dodací adresy / Související doklady. Editor má první
  dvě záložky; upozorňuje, že změna kontaktu neovlivní vystavené faktury.
- Identita a adresa nahoře, kontaktní informace a kontaktní osoba v šedé sekci,
  podrobnosti a bankovní účet pod rozbalovací akcí. Editor končí Zrušit / Uložit.

Snímky 12–14 v [indexu](screenshots/2026-10-11/README.md). Toto pozorování nedokládá
serverové validace, import/export, mazání ani vztah dodavatel/odběratel.

## E-023 — společný rám, přehled a nastavení iDokladu

Pozorování aplikace 11. 10. 2026 v Chrome, stejná syntetická agenda jako E-021.
Pouze čtení, žádná změna nastavení. Verze aplikace v UI neuvedena.
Zdroje: https://app.idoklad.cz/, https://app.idoklad.cz/Settings/Company,
https://app.idoklad.cz/Settings/User/Login.

- Desktop: vnější tmavý rám, zaoblený pracovní prostor, bílé postranní menu,
  světlá plocha obsahu, oválné akce a aktivní položky. Nastavení patří do
  rozbalovací skupiny; kontakty jsou pod Adresářem. Aktivní skupina je otevřená.
- Firma má záložky Základní nastavení firmy, Účetní údaje, Logo a podpis,
  Správa firem a Správa uživatelů. Základní údaje obsahují identitu, sídlo
  a kontaktní informace; účetní záložka typ plátce a obchodní rejstřík.
- Uživatelský účet odděluje Přihlašovací údaje a Osobní údaje. Změna e-mailu
  a hesla má samostatnou akci; pozorována pouze vstupní stránka, nikoli změna.
- Přehled obsahuje fakturaci za období, bilanci prodeje/nákupu, limit DPH
  a stáří nezaplacených pohledávek/závazků. Naše současné API poskytuje pouze
  součty úhrad, zůstatků a počty faktur; nejde o ekvivalent tohoto přehledu.

Snímky 15–19 v [indexu](screenshots/2026-10-11/README.md). Monochromatické barvy,
Manrope/Lucide a self-hosting bez předplatného zůstávají rozhodnutím Open Faktury.

## E-024 — ceník a editor položky iDokladu

Pozorování v Chrome 11. 10. 2026, stejná syntetická agenda neplátce jako E-021.
Zdroj https://app.idoklad.cz/PriceList, následně tlačítko Nová položka ceníku.
Verze aplikace v UI neuvedena. Snímky 20–22 v [indexu](screenshots/2026-10-11/README.md).

- Prázdný seznam: Všechny, hledání, akce Nová položka ceníku; sloupce Název
  položky / Stav skladu / Jednotka / Cena / Akce. Ceník/Sklad obsahuje i Skladové pohyby.
- Nový editor: název (povinný), katalogové číslo, čárový kód EAN, prodávat po,
  jednotka; sekce Nastavení cen s cenou a měnou Kč.
- Evidence položky má switch Sledovat stav skladu. Zapnutí odhalí Množství,
  Datum naskladnění a Popis. Pouze přepnuto pro pozorování, nic nebylo uloženo.
- Zrušit otevře potvrzení opuštění neuložených změn; potvrzeno a editor opuštěn.

Neověřeno: uložení, limity, zaokrouhlení, plátce DPH, změna historie a skladové
transakce. P12-01 má vlastní dokumentovaná pravidla v [specifikaci](../specs/catalog.md).
Sklad nesimulujeme nulovým zůstatkem; do P18 zůstává nedostupný.

## E-025 — šablony a cenové nabídky: vstupní UI

Pozorování 11. 10. 2026, stejná syntetická agenda neplátce jako E-024, Chrome.
Zdroje https://app.idoklad.cz/IssuedDocumentTemplate a https://app.idoklad.cz/SalesOrder.
Verze UI neuvedena. Pouze prázdné seznamy a nové editory, bez vyplnění či uložení.
Snímky 23–27 v [indexu](screenshots/2026-10-11/README.md).

- Šablony: Název / Typ dokladu / Popis / Odběratel / Vytvořeno / Cena / Akce.
  Nová šablona má typ dokladu (výchozí Faktura vydaná), povinný název,
  odběratele, objednávku, popis, splatnost ve dnech, platební údaje a položky
  včetně přidání z ceníku. Dále řada, štítky a daňový příznak. Uložit/Zrušit.
- Nabídky: Číslo / Stav / Příznak / Popis / Odběratel / Vystaveno / Platnost /
  Cena / Akce; hledání, filtr, nastavení seznamu a hromadné akce.
- Nová nabídka ukazuje návrh čísla CN20260001, šablonu, štítky, Doklad/Přílohy,
  povinného odběratele a popis; objednávku/smlouvu, vystaveno, platnost do,
  platební údaje, položky z ceníku, cenu celkem a Uložit/Další akce/Zrušit.
- Rozbalený stav nabídky: Vytvořena, Odeslána, Objednána, Vyfakturována, Zamítnuta.
  Otevření seznamu stavů nezměnilo hodnotu; editor opuštěn bez uložení.

Neověřeno: rezervace čísla, ukládání, PDF, přechody stavů a převod nabídky na
fakturu/zálohu. Text prázdného seznamu deklaruje převod, ale není důkazem
skutečného chování převodu. P12-02/P12-03 zatím nejsou implementované.


## E-026 — uložení nabídky/šablony a otevření faktury z nabídky

Živé UI Chrome, 11. 10. 2026, syntetická agenda neplátce CZK, verze neuvedena.
Zdroje https://app.idoklad.cz/SalesOrder a https://app.idoklad.cz/IssuedDocumentTemplate.
Uložené vstupy: syntetický odběratel, popis „Test Open Faktura — nabídka“,
položka „Testovací konzultace“, 2 × 1 200 Kč, vystavení 11. 10., platnost 25. 10.
Výsledek: CN20260001, 2 400 Kč, Vytvořena, detail s Upravit a akcí faktury.
Akce faktury otevřela **nový editor** faktury 20260002 s převzatým odběratelem,
popisem, položkou, množstvím/cenou a stejnými datumy. Editor poté zrušen;
nevznikla uložená faktura. Uložená nabídka zůstává v testovací agendě.

Šablona „Test Open Faktura — šablona“ se stejnou položkou a bez odběratele
byla uložena. Registr ukazuje typ Faktura vydaná, popis Testovací konzultace,
2 400 Kč a vytvoření 11. 10. 2026. Snímky 28–30 v indexu.

Doloženo: uložení neplátcovské nabídky/šablony, částka jednoduchého příkladu,
převzetí do editoru faktury. Nedoloženo: dokončené referenční vystavení po převodu,
opakovaný převod, interní transakce/číslování, PDF a plátcovské varianty.
Naše deduplikace, archivace a oddělený uložený návrh jsou vlastní pravidla.

## E-027 — PDF příloha nabídky a historie e-mailů

11. 10. 2026, živý Chrome, stejná syntetická neplátcovská agenda jako E-026.
Verze UI neuvedena. Detail `https://app.idoklad.cz/SalesOrder/Preview/1375354/Info`:
CN20260001, 2 × 1 200 Kč, stav Vytvořena. Toolbar obsahuje e-mail, PDF a tisk.
Otevřen e-mailový dialog, zvolena „Příloha PDF v e-mailu“, následně Zrušit.
**Žádná referenční zpráva nebyla odeslána.**

Pozorováno: pevný odesílatel neodpovidat@idoklad.cz, zákazník (bez e-mailu disabled),
další příjemci, přepnutí odkaz/příloha, upravitelný předmět a text s proměnnými,
uzamčená příloha `Cenová nabídka - CN20260001.pdf`, Odeslat/Zrušit.
`https://app.idoklad.cz/MailHistory`: prázdná historie, sloupce Odesláno,
Typ e-mailu, Příjemce, E-mail, Přečteno, Způsob odeslání, Číslo dokladu;
hledání, filtr a nastavení seznamu. Snímky 31 a 32 v indexu.

Veřejná podpora přečtená stejného dne:
- https://www.idoklad.cz/podpora/cenove-nabidky — automatický stav Odeslána po odeslání.
- https://www.idoklad.cz/podpora/karta-cenove-nabidky — příloha/odkaz, primární e-mail zákazníka.
- https://www.idoklad.cz/blog/cenove-nabidky-odkazem — odkaz a sledování zobrazení.

Naše odchylky: jeden explicitní příjemce, prostý upravitelný text v jazyce dokladu,
historie přímo u nabídky, PDF příloha bez veřejného odkazu/sledování přečtení.
Fronta, idempotence, neměnný snapshot a stavy SMTP jsou náš návrh, nikoli zjištěná
implementace iDokladu. Vzhled PDF je vlastní monochromatická sazba; referenční
PDF ani doručený referenční e-mail nebyly v tomto scénáři porovnány.

## E-028 — veřejná nabídka odkazem a zobrazení

11. 10. 2026, veřejná dokumentace, nikoli pozorování doručené referenční nabídky:
- https://www.idoklad.cz/blog/cenove-nabidky-odkazem (21. 8. 2026):
  volba odkazu/přílohy, online náhled, tisk a PDF; příznak odeslání/zobrazení
  u hlavního e-mailu kontaktu. Odkaz ukazuje aktuální obsah a stav i po převodu
  na fakturu. Zrušená/smazaná nabídka již není dostupná. Zmíněna diskuse.
- https://www.idoklad.cz/podpora/karta-cenove-nabidky (aktualizace 3. 9. 2026):
  interní poznámka se netiskne, text před/za položkami je samostatný obsah.
- Živý dialog volby způsobu odeslání již doložen E-027. Referenční e-mail ani
  veřejný příjemcovský náhled jsme nyní neotevřeli; bez odesílání třetím osobám.

Implementace QUOTE-SHARE-01: aktuální nabídka, download/tisk, link e-mail,
oddělená historie zobrazení. Vlastní pravidla: několik odvolatelných odkazů,
expirace 1–365 dní, 256bit token, technické návštěvy bez identifikace čtenáře,
limit 60 událostí/h/odkaz. Neslibujeme důkaz přečtení či shodu interního
trackingu iDokladu. Archivace blokuje přístup; obnova jej vrací, pokud odkaz
nevypršel/nebyl zrušen. Odvolání odkazu je trvalé. Diskuse není v tomto kroku.
Změna oproti E-027: interní poznámka se již nepřidává do nově renderovaných PDF;
historické přílohy zůstávají neměnné. Monochromatický veřejný náhled je vlastní
adaptace doložené funkce, nikoli pixelově ověřená kopie příjemcovské stránky.

## E-029 — vystavená faktura e-mailem s PDF

11. 10. 2026, živý Chrome, syntetická neplátcovská agenda, verze UI neuvedena.
https://app.idoklad.cz/IssuedInvoice/Preview/66832512/Info — faktura 20260001,
„Syntetický test UX – neodesílat“, syntetický odběratel, 300 Kč, již uhrazená.
Z detailu otevřen e-mailový dialog, přepnuto na PDF přílohu, po snímku Zrušit.
Žádná zpráva odeslána, doklad ani úhrady nezměněny.

Pozorování: Od neodpovidat@idoklad.cz; zákazník/účetní, kopie a další příjemci;
volba odkazu nebo PDF, upravitelný předmět a text s proměnnými; pevná příloha
„Vydaná faktura - 20260001.pdf“. Snímek 33 v indexu.

Veřejná podpora https://www.idoklad.cz/podpora/faktury-vydane
(aktualizace 18. 4. 2024, přečteno 11. 10. 2026): odeslání ze seznamu/detailu,
šablony zprávy, PDF příloha bez sledování přečtení. Historii dokládá také E-027.

Naše adaptace: jeden explicitní příjemce, historie přímo u faktury, vlastní
monochromatický dialog společný s nabídkami. Uložená neměnná PDF příloha,
fronta/idempotence a rozlišení SMTP stavů jsou naše implementační rozhodnutí,
nikoli tvrzení o interní architektuře iDokladu. Referenční doručení neověřeno.

## E-030 — vlastní přílohy faktur a nabídek

12. 10. 2026, documented, veřejná česká nápověda:
- https://www.idoklad.cz/podpora/karta-faktury — záložka Přílohy, výběr souboru
  nebo přetažení, přílohy se zasílají spolu s fakturou; pět souborů, každý 2 MB;
  PNG/JPEG/JPG/PDF/ZIP/RAR/XLSX/DOCX/ODT/ODS/TXT.
- https://www.idoklad.cz/podpora/karta-cenove-nabidky — pět příloh, každá 2 MB.

Neověřeno živým uploadem v referenční agendě; text „MB“ nedokládá přesný počet
bajtů. Naše adaptace ATTACHMENTS-01: 2 MiB/soubor a 256 MiB/profil, explicitní
výběr do e-mailu, neměnná historie, soubory v PostgreSQL se scoped autorizací.
Tato architektura, quota a pravidla historického odstranění jsou náš návrh.
