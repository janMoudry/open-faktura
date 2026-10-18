# Evidence zdrojů a pozorování

Úvodní průchod 10. 10. 2026 vycházel ze statického čtení zdrojů, bez živého API requestu a přihlášené aplikace. Pozdější pozorování mají vlastní datum a omezení v jednotlivých záznamech. Původní [referenční soupis](FEATURES_REFERENCE.md) zachovává odkazy; není potvrzením všech pravidel.

## Typy důkazů

- `documented`: tvrzení nápovědy.
- `sdk-contract`: staticky přečtený kontrakt/validace klienta v konkrétním commitu.
- `observed`: reprodukované chování aplikace/API s uloženým vstupem a výstupem.
- `proposal`: vlastní rozhodnutí Open Faktury; nedokládá referenci.

## Zdroje

Oficiální [SDK](https://github.com/Solitea/IdokladSdk/tree/f76c7e16bc45f4fa601bf2889dc81b8d45a9dafc), commit `f76c7e16bc45f4fa601bf2889dc81b8d45a9dafc`, získán 10. 10. 2026. Níže jsou cesty relativní k tomuto neměnnému odkazu. SDK bylo čteno, nikoli spuštěno. [API dokumentace](https://api.idoklad.cz/Help/v3/cs/) při textovém načtení vrátila jen „Loading…“; její kontrakt zde není považován za přečtený.

| ID    | Typ          | Zdroj v SDK                                                          | Zjištění a omezení                                                                                                                                                                                  |
| ----- | ------------ | -------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| E-001 | sdk-contract | `IdokladSdk/Models/IssuedInvoice/Post/IssuedInvoicePostModel.cs`     | Faktura má partnera, měnu, číselnou řadu, způsob platby, data, položky a kurz včetně množství kurzu. Anotace požadují alespoň jednu položku. Neprokazuje DB ani úplné serverové validace.           |
| E-002 | sdk-contract | `IdokladSdk/Models/IssuedInvoice/Post/IssuedInvoiceItemPostModel.cs` | Položka má množství, jednotkovou cenu, typ ceny, slevu, typ i hodnotu sazby a volitelný ceník/členění DPH. Přesnost a algoritmus zaokrouhlení nejsou z toho odvoditelné.                            |
| E-003 | sdk-contract | `IdokladSdk/Enums/PriceType.cs`                                      | `WithVat=0`, `WithoutVat=1`, `OnlyBase=2`. Význam OnlyBase v konkrétních daňových situacích vyžaduje ověření.                                                                                       |
| E-004 | sdk-contract | `IdokladSdk/Enums/PaymentStatus.cs`                                  | `Unpaid=0`, `Paid=1`, `PartialPaid=2`, `Overpaid=3`. Neobsahuje stav po splatnosti; způsob výpočtu stavů není doložen.                                                                              |
| E-005 | sdk-contract | `IdokladSdk/Models/Contact/ContactPostModel.cs`                      | Kontakt má IČO, DIČ, adresu, dodací adresy, bankovní údaje, individuální splatnost, slevu a nastavení upomínek. Neprokazuje povinnou unikátnost IČO.                                                |
| E-006 | sdk-contract | `IdokladSdk/Clients/IssuedInvoiceClient.cs`                          | Klient nabízí vytvoření, změnu, smazání, default, kopii, přepočet a podklady pro opakování. `RecountAsync` posílá POST na `/IssuedInvoices/Recount`; výsledek lze využít při ověřování kalkulátoru. |
| E-007 | sdk-contract | `IdokladSdk/Clients/IssuedInvoiceClient.cs`, metoda `ValidatePost`   | Bez navázané zálohy klient povoluje pouze normální položky; s navázanou zálohou normální a odpočtové. Server a více záloh vyžadují samostatné ověření.                                              |

## UI průzkum 10. 10. 2026

| ID    | Typ a prostředí                                 | Zdroj                                                                                                                                             | Zjištění a omezení                                                                                                                                                                                                                                                                                                                             |
| ----- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| E-008 | documentation, veřejná česká nápověda           | [Vydané faktury](https://www.idoklad.cz/podpora/faktury-vydane), [snímek filtrů](https://www.idoklad.cz/wp-content/uploads/2022/12/fv-filter.png) | Dokumentace popisuje seznam, stavové záložky, hledání a filtry; snímek ukazuje rozšířený filtr nad tabulkou. Přihlášená aplikace nebyla pozorována, aktuálnost vizuálu snímku neověřena. Podklad UX-001.                                                                                                                                       |
| E-009 | documentation, veřejná česká nápověda           | [Karta faktury](https://www.idoklad.cz/podpora/karta-faktury)                                                                                     | Karta obsahuje zákazníka, data a položky, otevření přes číslo dokladu. Neprokazuje serverové validace. Podklad UX-002.                                                                                                                                                                                                                         |
| E-010 | observation, veřejný nepřihlášený web, české UI | [FakturaOnline editor](https://www.fakturaonline.cz/faktura)                                                                                      | Bez zadání a odeslání dat otevřen výchozí formulář: základní údaje, obecné nastavení, dodavatel/odběratel, položky s množstvím/cenou, přidání položky, poznámka, součet a akce Stáhnout. Datum ve formuláři 10. 10. 2026. Na veřejné cestě seznamu nebyly pozorovány žádné uložené doklady; chování seznamu s účtem neověřeno. Podklad UX-002. |

Výstup: [vlastní návrh seznamu a editoru](../specs/invoice-design.md). Syntetická data prototypu nejsou referenční účetní fixture.

## Otevřené otázky

| ID   | Otázka                                              | Jak uzavřít                                    | Dopad                      |
| ---- | --------------------------------------------------- | ---------------------------------------------- | -------------------------- |
| Q-01 | Fáze a přesnost zaokrouhlení, pořadí slev           | R-01, R-02, R-03                               | Kalkulátor a typy částek   |
| Q-02 | Návrh/vystavení, přidělení čísla, editace a smazání | R-04, R-05                                     | Životní cyklus a transakce |
| Q-03 | Změna kontaktu po vystavení a snapshot              | R-06                                           | Historický obsah a PDF     |
| Q-04 | Úhrady, vratky, nulové a záporné doklady            | R-07                                           | Saldo a stavy              |
| Q-05 | Oprávnění uživatele a API ve více firmách           | R-08                                           | Izolace dat                |
| Q-06 | Vyúčtování více záloh, sazby a omezení SDK          | R-09                                           | Typované vazby dokladů     |
| Q-07 | Úplné modely ostatních modulů                       | Inventura klientů, modelů, enumů a operací SDK | Další specifikace          |

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

## E-031 — export seznamů CSV/XLSX (12. 10. 2026)

Typ: veřejná oficiální nápověda, ne autentický soubor ani přihlášený experiment.
Zdroje: https://www.idoklad.cz/podpora/faktury-vydane,
https://www.idoklad.cz/podpora/adresar-kontakty,
https://www.idoklad.cz/podpora/ovladani-aplikace,
https://www.idoklad.cz/blog/novinky-srpen-2020.
Nápověda potvrzuje export seznamu faktur a kontaktů do XLSX/CSV a volbu sloupců.
Příspěvek z roku 2020 popisuje předvýběr viditelných sloupců; aktuální živé UI
nebylo v tomto kroku ověřeno. Filtry pro export jsou výslovně popsané u ceníku:
https://www.idoklad.cz/podpora/seznam-polozek-ceniku.
EXPORT-01 přebírá volbu formátu/sloupců a dle zadání aplikované filtry faktur/kontaktů.
Vlastní odchylky: všechny sloupce předvybrané, 10 000 řádků, textové buňky a bezpečné CSV.
Přesná shoda schématu a formátování není ověřena. Nevypovídá o exportech FakturyOnline.

## E-032 — zálohové faktury (12. 10. 2026)

Veřejná oficiální nápověda https://www.idoklad.cz/podpora/zalohove-faktury
(uvádí aktualizaci 22. 4. 2024), https://www.idoklad.cz/podpora/prodej a
https://www.idoklad.cz/podpora/danove-doklady-k-platbe.
Doloženo: samostatná proforma agenda, nejde o daňový doklad, seznam má pohledy
všechny/uhrazené/neuhrazené/po splatnosti. Vyúčtování a daňové doklady jsou
navazující operace. Nápověda popisuje omezení editace po navázání daňového dokladu.
Nejde o živý experiment; účetní/numerická shoda nebyla ověřena. Naše číslování ZF,
blokace editace při jakékoli kladné úhradě, archiv a transakční pravidla jsou
vlastní rozhodnutí ADVANCE-01. Reference neslouží jako tvrzení o interní DB.

## E-033 — daňové doklady k přijaté platbě (12. 10. 2026)

Veřejná nápověda https://www.idoklad.cz/podpora/danove-doklady-k-platbe,
uvádí aktualizaci 18. 12. 2022, přečtena 12. 10. 2026. Popisuje samostatný
seznam, výběr dosud nezpracované platby, vystavení z řádku úhrady, změnu popisu
a data před uložením a export PDF/tisk. Popisuje také e-mail a automatizaci,
které TAX-RECEIPT-01 zatím nepřebírá. Neproběhl experiment v živé testovací agendě.
Nápověda neuvádí algoritmus rozdělení částečné platby mezi sazby ani přesné
serverové chování při přeplatku, souběhu a stornu. Naše of-tax-receipt-v1,
řada DDPP, immutable snapshot a blokace storna jsou vlastní pravidla popsaná
v docs/specs/advance-tax-receipts.md; numerická a vizuální shoda není potvrzená.
Právní lhůty z článku nejsou v této implementaci automaticky vyhodnocované.

## E-034 — vyúčtování záloh (12. 10. 2026)

Oficiální nápověda https://www.idoklad.cz/podpora/zalohove-faktury,
aktualizace stránky 22. 4. 2024, přečteno 12. 10. 2026. Sekce vyúčtování:
plná úhrada před přípravou, upravitelná konečná faktura s položkami a odpočty,
automatické zahrnutí DDPP, více nevyúčtovaných záloh stejného odběratele a měny.
Přidat odpočet lze i při tvorbě faktury. Stránka popisuje speciální chování při
historické změně sazeb, které aktuální CZK model 0/12/21 % nepřebírá.
Bez živého experimentu; rezervace už při návrhu, striktní odmítnutí přeplatku,
identita stran, normalizace rozdílných cen a pravidla souběhu jsou naše rozhodnutí.
Přesný odpočet uložených DDPP je výslovný požadavek uživatele a je popsaný v
specifikaci advance-settlement.md. Nepředstírá se numerická ani 1:1 vizuální shoda.

## E-035 — dobropisy (12. 10. 2026)

Veřejná oficiální nápověda https://www.idoklad.cz/podpora/dobropisy,
aktualizace 18. 12. 2022, přečteno 12. 10. 2026. Doložené: tvorba z běžné
faktury, editovatelný koncept, důvod opravy, dílčí oprava ceny/položek,
samostatný seznam a zápočet proti zdrojové faktuře pouze při dostatečném
neuhrazeném zůstatku. Neprokazuje databázi, číslování, přesnost výpočtu ani
konkurenci. Není proveden nový experiment v přihlášené testovací agendě.
CREDIT-NOTE-01 výslovně odlišuje vlastní kumulativní odpočty, životní cyklus,
vratky a návaznost na vyúčtované zálohy; numerická/vizuální shoda neověřená.

## E-036 — pravidelné faktury (12. 10. 2026)

Veřejné oficiální stránky https://www.idoklad.cz/podpora/pravidelne-faktury
(aktualizace 3. 1. 2025) a https://www.idoklad.cz/podpora/vzor-pravidelne-faktury
(18. 12. 2022), přečtené 12. 10. 2026. Vzor lze vytvořit nově nebo z faktury,
přenáší údaje odběratele, položky a ceny. Každá faktura má vlastní číslo.
Nápověda vzoru uvádí den/týden/měsíc/rok a interval, kotvu prvního vystavení,
volitelný konec měsíce, náhled 12 dat, konec datem/počtem, splatnost ve dnech,
samostatný výběr e-mailových příjemců. Dále podporuje zálohy a proměnné v textech.
Neprokazuje souběh, DB ani chování po výpadku/DST. Tyto mechanismy a pozastavené
uložení s explicitní aktivací jsou naše rozhodnutí v recurring-invoices.md.
Žádné nové pozorování přihlášené aplikace; tarifní omezení nepřebíráme.

## E-037 — komunikace a automatické upomínky

- Datum čtení: 13. 10. 2026; veřejná dokumentace, aktualizace stránky 4. 12. 2024.
- URL: https://www.idoklad.cz/podpora/nastaveni-e-maily-a-komunikace
- Zjištění: oddělené e-mailové texty, ruční odeslání; automatické upomínání
  běžných faktur při přihlášení, s povolením kontaktu, odstupem po splatnosti,
  intervalem, limitem celkové částky a maximálně deseti upomínkami. Automatická
  upomínka obsahuje odkaz na PDF. Poděkování může být ruční při úhradě nebo
  automatické při bankovním párování. Příjemci mohou zahrnovat další adresy.
- Žádné nové živé odeslání z referenční agendy nebylo provedeno. Serverové
  souběhy a idempotence nejsou touto stránkou doložené.
- Naše odchylky a ochrany: `docs/specs/invoice-communication.md`; zejména serverový
  worker, pauza faktury, jeden adresát, PDF příloha a ruční úhrady místo banky.

## E-038 — odkazy vystavených faktur

13. 10. 2026, veřejná dokumentace:

- https://www.idoklad.cz/podpora/faktury-vydane (aktualizace 18. 4. 2024),
  část Odesílání jedné faktury: volba PDF přílohy nebo odkazu na PDF v prohlížeči;
  u odkazu lze evidovat datum zobrazení, PDF příloha sama tuto informaci nedává.
- https://www.idoklad.cz/blog/novinky-duben-2021 : příjemce má odkaz na zobrazení
  a stažení; uhrazený doklad je označen štítkem Uhrazeno. Historický článek,
  nikoli potvrzení současné podoby stránky.

Živý příjemcovský náhled a referenční e-mail nebyly v tomto scénáři nově
pozorované. Odesílání zákazníkům neproběhlo. Expirace, více odvolatelných odkazů,
počítání technických návštěv a ochrana fronty jsou návrh Open Faktury podle
specifikace `docs/specs/invoice-sharing.md`, nikoli odvození interního modelu iDokladu.

## E-039 — evidence nákupu a výdajové úhrady

Ověřeno 13. 10. 2026, veřejná dokumentace:

- https://www.idoklad.cz/podpora/nakup-faktury-prijate (aktualizace 5. 5. 2026).
- https://www.idoklad.cz/podpora/nakup-uhrady (aktualizace 9. 12. 2024).

Doložené: interní číslování oddělené od čísla dodavatele, dodavatel a data,
příloha originálu, filtry, upravitelná DPH dle zdroje. Úhrady mohou být částečné,
mají částku/datum/metodu; přehled je na dokladu, zrušení platby je podporované.
Dokumentace popisuje také automatické zaplacení podle metody a přeplatky.

P15-01 se záměrně omezuje na explicitní ruční výdaje bez přeplatků, CZK,
řádky přesných částek ze zdroje a původní soubory. Transakční model, audit,
revize, UUID, limity, PF řada a konkrétní monochromatické UI jsou vlastní návrh.
Živá aplikace ani API nebyly v tomto kroku experimentálně ověřeny.
Specifikace: [přijaté faktury](../specs/received-invoices.md).

## E-040 — účtenky a nákupní úhrady (13. 10. 2026)

Typ: veřejná oficiální nápověda, nikoli živé pozorování API/UI.

- https://www.idoklad.cz/podpora/nakup-uctenky (aktualizace 6. 5. 2026):
  vlastní číslování, nepovinný dodavatel, externí číslo, popis z první položky,
  hotově/kartou, přílohy. Zaplacení a párování s bankou/pokladnou jsou
  rozdílné skutečnosti.
- https://www.idoklad.cz/podpora/nakup-uhrady (9. 12. 2024):
  společný seznam úhrad, filtrování dodavatele/metody/popisu/data a vazba na doklad.
- https://www.idoklad.cz/podpora/domovska-stranka (29. 4. 2025):
  nákup zahrnuje přijaté faktury a účtenky.

Náš návrh P15-02: CZK, pevná UC řada, přesné částky z originálu, atomická
evidence zaplacené účtenky bez bankovního párování, revizní opravy se stornem.
Definice období a metrik jsou explicitní ve specifikaci purchase-receipts.md;
nepotvrzují shodu s účetními nebo daňovými výstupy iDokladu.

## E-041 — banka a bankovní účty (13. 10. 2026)

Oficiální veřejná dokumentace:

- https://www.idoklad.cz/podpora/finance-banka (9. 12. 2024): příjmy/výdaje,
  ruční výběr nespárovaného dokladu, účet/VS/partner/popisek/data, filtry a odkaz na doklad.
- https://www.idoklad.cz/podpora/nastaveni-banka (29. 6. 2026): více účtů,
  neměnná měna po prvním pohybu, notifikace a rozlišení párování od samotného pohybu.
- E-040: účtenka kartou už eviduje zaplacení, bankovní párování je samostatné.

Bez živého pozorování této části. P16-01 používá vlastní CSV s povinným
externím ID, transakční propojení jedné úhrady, auditované odpojení a CZK.
Žádný skutečný bankovní export nebyl dodán; neoznačovat syntetické CSV za
bankovní fixture. Rozdíly a další rozsah: specs/banking.md.

## E-042 — bankovní formáty a návrhy párování (13. 10. 2026)

Druh důkazu: veřejná dokumentace, žádné přihlášené bankovní API/UI pozorování.
SDK ani databázový model bank se z těchto dokumentů neodvozuje.

- [Air Bank specifikace exportů](https://www.airbank.cz/archiv-dokumentu/technicke-pozadavky-a-specifikace-hromadnych-plateb-a-export-vypisu.pdf), dokument 22/01: XML camt.053.001.02, VS v EndToEndId.
- [Fio API](https://www.fio.cz/docs/cz/API_Bankovnictvi.pdf), verze 16. 10. 2025: accountStatement JSON, sloupce transakce a bankovní ID column22.
- [ČS George Business](https://www.csas.cz/en/george-help/george-business/accounts-and-loans/accounts/account-statements), bez uvedené verze: dostupnost SEPA XML camt.053.
- [ČSOB ČBA XML](https://www.csob.cz/documents/10710/420157/multicash-xml.pdf), verze neuvedena zde: XML standard pro firemní výpisy.
- [KB XML výpis](https://mojebanka.kb.cz/file/cs/KB-podminky_format_XML_vypis.pdf), 19 stran: bankovní reference AcctSvcrRef; pořadí MsgId není náhradou.
- [UniCredit elektronické bankovnictví](https://www.unicreditbank.cz/cs/financni-instituce/cash-management/elektronicke-bankovnictvi.html), verze neuvedena: zveřejněný formát camt.052/053.
- [MONETA XML DTD](https://www.moneta.cz/documents/20143/11740743/mmb-ib-struktura-vypisu-xml.zip), DTD revize 1.22: účet, transaction.id, date-post, amount, trn-messages type. Lexikální formáty částky/data a úplná pravidla směru nejsou z DTD doložené. Adaptér proto odmítá nejednoznačnost.
- [RB ABO/Gemini](https://www.rb.cz/attachments/elektronicke-bankovnictvi/27804-Popis-datove-struktury-ABO-a-Gemini-formatu-A.pdf), 19 stran, str. 6–9: ediční účty, stabilní číslo dokladu, haléře, kódy 1/2/4/5, měna v názvu souboru.
- [iDoklad nastavení banky](https://www.idoklad.cz/podpora/nastaveni-banka), aktualizace 29. 6. 2026: VS a částka při párování; [účtenky](https://www.idoklad.cz/podpora/nakup-uctenky), aktualizace 6. 5. 2026: ruční párování bez VS.

Vlastní návrh: ručně potvrzené návrhy, omezení CZK, zákaz mixu importních formátů
v účtu a odmítnutí batch položek. Přesná implementace/limity v
[specifikaci P16-02](../specs/bank-statements.md). Syntetické fixtures nejsou
skutečné exporty. Kompatibilita všech bankovních variant zůstává neověřená;
potřebuje anonymizované originály. Pokrytí formátu neslibuje dostupnost exportu
v každém osobním účtu nebo tarifu banky.

## E-043 — párování podle VS/částky a hromadné potvrzení (14. 10. 2026)

Veřejná dokumentace:
[Nastavení – Banka](https://www.idoklad.cz/podpora/nastaveni-banka),
[Finance – Banka](https://www.idoklad.cz/podpora/finance-banka),
[Nákup – Účtenky](https://www.idoklad.cz/podpora/nakup-uctenky).
Živé verze načtené 14. 10. 2026; nejde o SDK ani přihlášené pozorování.

Dokumentace popisuje shodu účtu, VS a částky, automatické zpracování bankovních
notifikací a ruční řešení neúspěšných shod. Účtenky bez VS nelze takto automaticky
párovat. Nedokládá náš hromadný dialog, fingerprint ani přesnou transakční hranici.

P16-03 je vlastní konzervativní rozšíření: uživatel potvrdí náhled aktuální stránky,
jen přesné jednoznačné shody, bez tolerancí a částečných automatických úhrad.
Kolize více pohybů vedou k ručnímu řešení, nikoli volbě prvního řádku.
[Specifikace](../specs/bank-batch-matching.md). Syntetické testy nedokládají
shodu celé bankovní automatizace iDokladu.

## E-044 — pokladna, veřejná dokumentace (14. 10. 2026)

Zdroje: https://www.idoklad.cz/podpora/finance-pokladna (aktualizace 9. 12. 2024)
a https://www.idoklad.cz/podpora/nastaveni-pokladna (2. 9. 2024).
Veřejná nápověda, nikoli živý experiment nebo potvrzení interní DB.
Pokladna eviduje příjmy/výdaje a párování s doklady; přehled lze omezit
pokladnou/obdobím. Nastavení zahrnuje počáteční stav a datum, číslování,
měnu a výchozí pokladnu. Dokumentace popisuje automatické vytvoření
pokladního dokladu při hotovostní úhradě a tisk. P16-04 dodává explicitní
evidenci a párování; automatizace a tisk zůstávají rozdíly. Pravidla storna
a neměnnosti počátku jsou vlastní návrh ve specs/cash-register.md.

## E-045 — hlavní pokladna a automatické doklady (14. 10. 2026)

Veřejná nápověda, znovu načtená 14. 10. 2026:
https://www.idoklad.cz/podpora/nastaveni-pokladna (aktualizace 2. 9. 2024),
https://www.idoklad.cz/podpora/nakup-uctenky (6. 5. 2026).
Hlavní domácí pokladna je cílem automatických dokladů. Hotovostní účtenka
vytváří pokladní doklad, jazyk tisku je samostatné nastavení. Nápověda není
experiment se stornem nebo souběhem; P16-05 tyto podrobnosti výslovně navrhuje
v cash-register.md. Samotné označení způsobu úhrady v našem editoru není platba.

## E-046 — pokladní položka DPH a návaznost prodejek (14. 10. 2026)

Veřejná nápověda načtená 14. 10. 2026 (nikoli přihlášený experiment):

- https://www.idoklad.cz/podpora/finance-pokladna (9. 12. 2024): jedna položka
  s názvem/cenou, typem ceny, sazbou a členěním DPH pro plátce; nepodporuje PDP.
- https://www.idoklad.cz/podpora/prodejky (21. 11. 2024): prodej na místě,
  odběratel není povinný, návazné zaúčtování.
- https://www.idoklad.cz/podpora/zauctovani-prodejek (18. 12. 2022): souhrnný
  pokladní doklad pro hotovost, interní doklad pro kartu/stravenku, storno
  zaúčtování ve zdroji. Proto nelze prodejky pouze převést na běžné úhrady faktur.

P16-06 dokončuje pouze samostatnou pokladní položku DPH. Vlastní návrh:
kalkulátor of-czk-v1, neměnnost/storno, zákaz párování daňového plnění, omezení
data, CZK, sazby a snapshot. Není ověřena numerická ani úplná UI shoda s iDokladem.
Členění DPH a souhrnné zaúčtování ještě nejsou implementované.

## E-047 — bankovní počátek, prodejky a terminály (14. 10. 2026)

Veřejná dokumentace načtená 14. 10. 2026:
https://www.idoklad.cz/podpora/nastaveni-banka (29. 6. 2026) výslovně popisuje
nastavení počátečního stavu k datu. Způsob započtení hraničního dne a audit
změn není doložen; bank-balances.md je definuje jako vlastní pravidla.
https://www.idoklad.cz/podpora/karta-prodejky (6. 1. 2023) popisuje kombinaci
hotovosti/karty/stravenky, nutnost plné úhrady, nepovinného odběratele a oddělení
interní poznámky od tisku. Prodejky tedy nelze vydávat za obyčejné faktury.
https://www.idoklad.cz/podpora/nastaveni-pokladna (2. 9. 2024) popisuje terminály
a tisk. Jde o nápovědu, nikoli běhovou či numerickou shodu.

## E-048 — Fio API a vlastní bankovní napojení (14. 10. 2026)

Zdroj: veřejná dokumentace Fio API Bankovnictví, verze 16. 10. 2025,
https://www.fio.cz/docs/cz/API_Bankovnictvi.pdf a
https://www.fio.cz/bank-services/internetbanking-api . Ověřeno čtením veřejné
dokumentace 14. 10. 2026; nešlo o volání skutečného bankovního účtu.
Kontrakt dokládá čtení pohybů za interval přes
`https://fioapi.fio.cz/v1/rest/periods/{token}/{from}/{to}/transactions.json`,
token vytvořený oprávněným uživatelem a prodlevu aktivace nového tokenu.

Open Faktura přidává vlastní gateway kontrakt a správu přístupů (P16-15a).
Tato část je návrhové rozhodnutí, není oficiálním společným API českých bank
ani dokladem plné shody s iDokladem. Testy jsou syntetické a loopback;
živé ověření jednotlivých poskytovatelů zůstává otevřené.

## E-049 — rozdělení bankovního pohybu (14. 10. 2026)

Veřejná dokumentace https://www.idoklad.cz/podpora/finance-banka,
aktualizace stránky 9. 12. 2024, načteno 14. 10. 2026. Popisuje ruční výběr
jednoho nespárovaného dokladu a jednu položku pohybu. Rozdělení mezi více
dokladů ani chování přeplatků z této stránky doložené není. Žádné nové
pozorování přihlášené aplikace/API. P16-11 je explicitně vlastní rozšíření,
nikoli potvrzená shoda s iDokladem; viz specs/bank-split-matching.md.

## E-050 — prodejky a souhrnné zaúčtování, opětovné ověření (15. 10. 2026)

Veřejná nápověda, nikoli pozorování přihlášené aplikace:
https://www.idoklad.cz/podpora/prodejky (aktualizace 21. 11. 2024) popisuje
prodej na místě s nepovinným odběratelem. Souhrnné zaúčtování odděluje hotovost
(pokladní doklady) od karty/stravenek (interní doklady). U kombinace přenáší
poměrnou část a sdružuje podle zařízení, případně odběratele. Storno zaúčtování
uvolní zdrojové prodejky pro nové zpracování. Skladový prodej má návazný výdej.

https://www.idoklad.cz/podpora/karta-prodejky (aktualizace 6. 1. 2023) popisuje
samostatný editor, číslování a sdílené cenové předvolby; nejde o obyčejný návrh
faktury. https://www.idoklad.cz/podpora/nastaveni-pokladna (aktualizace 2. 9. 2024) je podklad profilové evidence terminálů.

Neověřeno: konkrétní numerické rozdělení haléřů mezi způsoby úhrady, běhová
pravidla oprav zaúčtovaných prodejek a kompatibilita externích zařízení.
Daňová tvrzení nápovědy nejsou náhradou aktuálního oficiálního právního podkladu.
Terminál v Open Faktuře je zatím konfigurace; není dokladem funkčního propojení
s platebním hardwarem ani hotového modulu prodejek.

## E-051 — Inbox (veřejná nápověda, 15. 10. 2026)

Zdroj: https://www.idoklad.cz/podpora/inbox, poslední aktualizace stránky 6. 5. 2026, přečteno 15. 10. 2026. Jde o veřejnou dokumentaci, nikoli
pozorování přihlášené aplikace nebo znalost serverového modelu.

Inbox podporuje jednotlivé i hromadné nahrání souborů a příjem příloh přes
unikátní adresu agendy. Z dokumentu lze vytvořit přijatou fakturu či účtenku,
nebo jej připojit k existujícímu dokladu. Strukturovaný obsah ISDOC/QR umožňuje
předvyplnění; chybějící rozpoznání nesmí předstírat známý typ. Příjem bez příloh
se nezpracovává. Vyčtená data uživatel kontroluje. Tarifní omezení nepřebíráme.

Vlastní návrh Open Faktury: izolovaný originál v PostgreSQL, SHA-256 deduplikace
v profilu, potvrzení převodu v transakci a samostatný audit. Uživatelský upload
není totéž co ověřená bankovní notifikace. Žádná příloha sama nevytváří úhradu.

## E-052 — ISDOC 6.0.2 (statický standard, 15. 10. 2026)

Zdroj: https://isdoc.github.io/ a
https://isdoc.github.io/xsd/isdoc-invoice-6.0.2.xsd,
verze 6.0.2 z 25. 4. 2022. Přečteny názvy a významy polí řádku,
dodavatele, měny, plateb a LegalMonetaryTotal. Jde o formátový kontrakt,
nikoli skutečný export iDokladu. Syntetická fixture není certifikovaná
XSD/Schematron validní faktura.

První parser Open Faktury podporuje unsigned tuzemské CZK běžné faktury
6.0.1/6.0.2 bez záloh, přenesené daně a již provedených plateb. Zachovává
zdrojové částky; neshoda součtů je chyba. DTD a více namespace deklarací
odmítá, externí zdroje nikdy nenačítá. Neověřuje elektronický podpis ani
celou shodu XSD. ISDOC.PDF, ISDOCX, QR a další režimy zatím nejsou tímto
parserem implementované.

## E-053 — Skladové pohyby (veřejná nápověda, 15. 10. 2026)

Zdroj: https://www.idoklad.cz/podpora/karta-skladove-pohyby, aktualizace 23. 4. 2024; přečteno 15. 10. 2026. Dokumentuje ruční naskladnění/vyskladnění,
pohyby z vydaných/přijatých faktur a dobropisů, seznam s příjmy/výdaji,
detail položky a filtr období. E-025 doplňuje pozorovaný přepínač sledování
zásoby. Není ověřená přesnost množství, souběh ani interní způsob inventury.

Vlastní návrh: čtyři desetinná místa množství, neměnný pohyb s reverzním
záznamem, UUID replay, profilový zámek a inventurní rozdíl vůči očekávané
revizi. Záporná zásoba je evidovaná skutečnost, nikoli automatické omezení prodeje.

## E-054 — Přehledy a datace grafu (16. 10. 2026)

Veřejná nápověda: https://www.idoklad.cz/podpora/domovska-stranka, aktualizace 29. 4. 2025, přečteno 16. 10. 2026. Faktury/dobropisy používají u daňových
dokladů DUZP, jinak vystavení; nákupní faktury přijetí, účtenky/prodejky
vystavení. Bilance prodeje zahrnuje faktury, dobropisy a prodejky, nikoli
zálohy/DDPP. Nákup zahrnuje přijaté faktury a účtenky. Reference nabízí ceny
s/bez DPH, kumulaci a srovnání roků. Nejde o pozorování testovací agendy;
vlastní přesné obnovení částky vyúčtování není numericky ověřené vůči iDokladu.

## E-055 — QR Platba SPAYD 1.0 (16. 10. 2026)

Oficiální specifikace: https://qr-platba.cz/pro-vyvojare/specifikace-formatu/
(bez uvedené revize, přečteno 16. 10. 2026). Hlavička SPD*1.0, ACC je IBAN,
AM nejvýše 9 999 999,99 se dvěma desetinnými místy, CC, DT YYYYMMDD, X-VS
maximálně deset číslic. Pro tisk doporučená korekce M. Náš renderer používá
uloženou původní částku a lokální generování; neověřuje provedení platby.
QR Faktura/SIND byla pouze prozkoumána: https://www.kdpcr.cz/informace/qr-faktura/popis-formatu,
verze 1.0; její implementace a kombinace Platba+F zůstávají otevřené.

## E-056 — Stav skladů k datu, hranice ocenění (17. 10. 2026)

Veřejná dokumentace https://www.idoklad.cz/podpora/seznam-polozek-ceniku,
aktualizace 23. 4. 2024, přečteno 17. 10. 2026: sestava Stav skladů obsahuje
název, kód, množství a jednotku ke zvolenému datu. Cena v ceníku je prodejní
cena, nikoli doložená pořizovací cena. Nápověda
https://www.idoklad.cz/podpora/karta-skladove-pohyby (stejné datum aktualizace)
dokládá členění příjmů/výdejů a filtrování datem. V těchto zdrojích není
popsána metoda pořizovacího ocenění. Nebyl proveden srovnávací experiment.

Vlastní rozšíření P18-03b: časově platná ručně zadaná skladová cena CZK za
jednotku, historie a neoceněné položky; žádné tvrzení o FIFO či váženém průměru
v iDokladu. Tato metoda nepřebírá prodejní cenu ani automaticky nepovažuje
základ DPH z přijaté faktury za celkovou pořizovací cenu zásob.

P18-03c (17. 10. 2026) zavádí vlastní konzervativní průběžný vážený průměr
z explicitních pořizovacích cen příjmů. Syntetický příklad 10 × 20 + 5 × 50,
výdej 3 → množství 12, hodnota 360 CZK je test našeho pravidla, nikoli pozorování
iDokladu. Neznámé vstupy a nevyřešené opravy vedou k neznámé hodnotě;
vyprázdnění zásoby zahajuje nový průměr. Viz hranice v `docs/specs/stock.md`.

P18-03d (17. 10. 2026): vlastní rozšíření E-056 používá přepočet ceny
nákupního cyklu, původní cenu výdeje pro vratky a největší zbytky při
rozvržení vedlejších nákladů podle pořizovací hodnoty. Očekávané hodnoty
v doménových/DB testech jsou syntetické. Algoritmus ani účetní účtování
cenových oprav nebyly potvrzeny v iDokladu; žádná nová referenční shoda se
netvrdí. Skladová analytika není účetní deník P19.

## E-057 — Přehled prodeje položek (17. 10. 2026)

Veřejná dokumentace https://www.idoklad.cz/podpora/prehled-prodeje,
aktualizace zdroje 23. 4. 2024, přečteno 17. 10. 2026. Potvrzuje umístění
Prodej → Přehled prodeje, ceníkové položky, prodané množství a skutečné částky
bez/s DPH, filtr období a typu dokladu, výběr sloupců a CSV/XLSX export.
Neurčuje pravidla dobropisů, datum započtení, zaokrouhlení, měny či vyúčtování.
Nejde o pozorování přihlášené aplikace ani SDK. Vlastní pravidla a limity
Open Faktury jsou v `docs/specs/item-sales.md`.

## E-058 — Money S3 ceník XML (17. 10. 2026)

Veřejná dokumentace https://money.cz/navod/s3xmlde/ odkazuje na
https://money.cz/wp-content/uploads/2024/10/schemas.zip a
https://money.cz/wp-content/uploads/2024/10/vzorove_xml.zip.
Staticky přečteny Schemas/__Zasoba.xsd a Skladova_karta.xml (metadata exportu
2024-10-18). SHA256 archivů:
schemas: e0bed0d59a648e978b72dbaf623b7d72abd8255ccd9375dd454ac46d1ded277d;
ukázky: 3d5597a167ba1c2230d028be7b4718a48630fd0a1fc964c26a33a4fbd4955006.

XSD popisuje PC bez Hladina jako základní cenu. Ukázka má kartu Kolečkové
brusle, základní cenu 750 s DPH 21 %, jednotku ks, prázdný kód domácí měny
a jiné cenové hladiny 675 a 30 EUR. Parser ověřujeme přímo na této ukázce.
Nejde o živý export z uživatelovy agendy ani pozorování synchronizace iDokladu.

https://www.idoklad.cz/podpora/komunikace-s-ucetnimi-systemy a
https://www.idoklad.cz/podpora/cenik-sklad dokládají propojení ceníku s Money.
Neprokazují totožnost formátů S3/S4/S5. Vlastní omezený souborový import,
konfliktní politika, potvrzení CZK a vynechání skladových množství jsou
rozhodnutí Open Faktury: specs/catalog-import.md. S4/S5 nejsou implementovány.

## E-059 — Konfigurovatelný export Money ERP (17. 10. 2026)

Veřejná nápověda, verze 1.17.1:
https://help.moneyerp.com/versions/1.17.1/am_export_do_xml.htm
popisuje výběr objektů i jejich polí, filtry a volitelnou XSLT transformaci.
https://help.moneyerp.com/versions/1.17.1/am_import_z_xml.htm
popisuje export XSD karty importu, ID jako atribut a konfigurovatelná pravidla
spojování. Importní XSD není samo o sobě důkazem podoby libovolného exportu.

Přečteno 17. 10. 2026, pouze veřejná dokumentace; žádné pozorování přihlášené
aplikace, export z konkrétní instalace ani potvrzení shody historických S4/S5.
Zdroje neposkytly konkrétní kontrakt požadovaných ceníkových polí a cen.
Nativní P18-05b zůstává neověřený. P18-05c je naše mezicesta: obecný CSV/XLSX
import s mapováním, nikoli předvolba vydávaná za nativní adaptér S4/S5.

Pro pokračování nativního adaptéru získat anonymizovaný export katalogu a cen,
verzi aplikace, konfiguraci exportovaných objektů/polí a případnou XSLT.
Pokus má obsahovat dva záznamy, cenu s i bez DPH, základní a další cenovou
hladinu, nulovou cenu, chybějící cenu, opakovaný export a změnu jedné ceny.
Přesnou strukturu, význam ceny, měnu a stabilitu identity porovnat s aplikací.

## E-060 — Členění fakturace na kvartály a roky (17. 10. 2026)

Veřejná nápověda https://www.idoklad.cz/podpora/domovska-stranka,
sekce Fakturace za období, aktualizovaná 29. 4. 2025, přečtená 17. 10. 2026.
Dokládá měsíční, kvartální a roční zobrazení, ceny s/bez DPH, kumulaci
a srovnání předchozího roku. Nejde o pozorování přihlášené aplikace ani
numerické porovnání. Přesné zacházení s částečnými obdobími, technické
limity a tvar API jsou vlastní rozhodnutí v specs/period-report.md (P19-01c).

## E-061 — Bilance, dlužníci a prodlení (17. 10. 2026)

https://www.idoklad.cz/podpora/domovska-stranka, aktualizace 29. 4. 2025,
přečteno 17. 10. 2026. Veřejná nápověda dokládá bilanci prodeje/nákupu
a úhrad, letošní/loňský rok, základní částky s/bez DPH; do prodeje patří
faktury, dobropisy a prodejky, nikoli samostatné zálohy/DDPP. Dokládá
rozpis neuhrazených částek podle prodlení a přepnutí vydané/závazky,
letošní/celé období. TOP 3 zahrnuje běžné i zálohové faktury; dlužníci
jsou určeni částkou nezaplacenou po splatnosti.

Nejde o pozorování aplikace/API. Přesné intervaly prodlení, datum zahrnutí
do roční bilance, částečné dobropisy a identita anonymního odběratele nejsou
doložené. Vlastní pravidla a CZK omezení: specs/dashboard-finance.md.

## E-062 — Evidence DPH a obrat na přehledu (17. 10. 2026)

Veřejná nápověda https://www.idoklad.cz/podpora/domovska-stranka
(aktualizace 29. 4. 2025, čteno 17. 10. 2026) popisuje měsíční/kvartální DPH,
DDPP podle vystavení, běžné faktury podle DUZP a sledování ročního obratu
z faktur bez odpočtu záloh, dobropisů a prodejek. Nejde o pozorování API.
Finanční správa: https://financnisprava.gov.cz/cs/financni-sprava/novinky/novinky-2025/informace-ke-zmenam-v-platcovstvi-dph-2025
(čteno 17. 10. 2026) uvádí kalendářní rok a hranice 2 000 000 / 2 536 500 Kč.
Náš model nemá úplnou klasifikaci zákonného obratu; samotný součet proto
neoznačujeme jako potvrzení vzniku plátcovství. Vlastní pravidla a mezery:
`docs/specs/tax-dashboard.md`. Žádná numerická shoda nebyla pozorována.

## E-063 — Měnový přepočet přehledů (17. 10. 2026)

https://www.idoklad.cz/podpora/domovska-stranka (aktualizace 29. 4. 2025,
čteno 17. 10. 2026), veřejná nápověda: přehledy zahrnují cizí měny přepočtené
na domácí měnu kurzem na dokladu. Nepotvrzuje přesnost ani zaokrouhlení,
kurzové rozdíly či vnitřní databázi. Vlastní pravidla a omezení implementace:
`docs/specs/report-currencies.md`. Žádný živý numerický experiment.

## E-064 — Měna dobropisu (17. 10. 2026)

Veřejná nápověda https://www.idoklad.cz/podpora/dobropisy (aktualizace
18. 12. 2022) potvrzuje vznik z běžné faktury a samostatný zápočet.
Historický článek https://www.idoklad.cz/blog/jak-v-idokladu-vystavit-dobropis
zmiňuje možnost změny měny; aktuální nápověda přesná kurzová pravidla
nepopisuje. Přečteno 17. 10. 2026, bez pozorování účtu/API.
P19-01h proto nepřebírá možnost měnit měnu: zachovává měnu i uložený kurz
zdroje jako vlastní omezení pro přesné opravy a zápočty. Přepočet přehledů
navazuje na E-063. Numerická shoda ani aktuální editace měny v iDokladu
nejsou tímto potvrzeny.

## E-065 — Cizoměnové vyúčtování záloh (17. 10. 2026)

Veřejná https://www.idoklad.cz/podpora/zalohove-faktury potvrzuje stejného
odběratele a měnu všech společně vyúčtovaných záloh. Stránka
https://www.idoklad.cz/podpora/karta-faktury popisuje měnu a upravitelný kurz.
Přečteno 17. 10. 2026; nejde o experiment v aplikaci. Přesné použití kurzů
DDPP a vyúčtování nebylo doloženo. P19-01i proto podporuje pouze společný
uložený kurz zálohy/DDPP/vyúčtování jako vlastní omezení a neoznačuje je
za úplnou shodu s iDokladem nebo podporu všech daňových kurzových pravidel.

## E-066 — Návrh kurzu z ČNB (17. 10. 2026)

Veřejné zdroje, čteno 17. 10. 2026:
- https://www.cnb.cz/cs/casto-kladene-dotazy/Kurzy-devizoveho-trhu-na-www-strankach-CNB/
  dokumentuje TXT endpoint, parametr date a sloupce země/měna/množství/kód/kurz.
- https://www.cnb.cz/cs/financni-trhy/devizovy-trh/kurzy-devizoveho-trhu/kurzy-devizoveho-trhu/
  popisuje publikaci v pracovní dny kolem 14:30 a platnost přes nepracovní dny.
- https://www.idoklad.cz/podpora/karta-faktury
  popisuje načtení posledního kurzu národní banky a ruční změnu.

Veřejné HTTP pozorování 17. 10. 2026:
GET https://www.cnb.cz/cs/financni-trhy/devizovy-trh/kurzy-devizoveho-trhu/kurzy-devizoveho-trhu/denni_kurz.txt?date=16.10.2026
vrátilo hlavičku „16.10.2026 #200“, sloupce
„země|měna|množství|kód|kurz“ a například AUD množství 1, kurz 15,045.
Jde o formát veřejného zdroje, nikoli experiment s daňovými výpočty iDokladu.
Syntetické EUR/JPY testovací sazby nejsou skutečným kurzem z tohoto pozorování.

Vlastní rozhodnutí: explicitní tlačítko a datum, sedmidenní tolerance
publikace, limity sítě, ruční fallback a ukládání pouze rate/units přes
stávající příkaz dokladu. Viz specs/exchange-rates.md. Numerická a daňová
shoda s iDokladem tím není potvrzena.

## E-067 — Částečné úhrady a měnové bankovní vazby (17. 10. 2026)

Veřejná nápověda https://www.idoklad.cz/podpora/prodej-uhrady
(aktualizace 18. 4. 2024, čteno 17. 10. 2026) dokládá částečné úhrady,
datum/částku/metodu a historii na kartě Úhrady. Stránka
https://www.idoklad.cz/podpora/domovska-stranka dokládá přepočet dokladů
uloženým kurzem, viz E-063. Ani jeden zdroj nedokládá algoritmus křížové
bankovní úhrady, rozdělení jedné platby nebo zaúčtování kurzového rozdílu.
Živý experiment v iDokladu nebyl proveden.

P16-13a/P19-01k je proto explicitní vlastní návrh: oddělené skutečné částky
v CZK a měně faktury, snapshot rozdílu bez účetního zápisu, ruční potvrzení.
Vzorce a syntetické příklady jsou v specs/bank-foreign-matching.md; nejde
o pozorované referenční výsledky.

## E-068 — měny bankovních účtů a importů (17. 10. 2026)

Implementační rozšíření nad dříve přečtenými veřejnými formáty E-042 (URL a
verze tam). Měna hlavičky a zaúčtované částky se kontrolují proti účtu;
původní měna zahraniční transakce není měnou zaúčtovaného pohybu. Nejde o nové
pozorování bank ani iDokladu. Seznam podporovaných měn, neměnnost a zákaz
párování jsou naše pravidla. Testy EUR jsou syntetické transformace fixtures,
nikoli ověření skutečného exportu. RB zahraniční přípony zatím nepodporujeme.

## E-069 — zahraniční identifikátory bankovních účtů (18. 10. 2026)

Veřejná nápověda https://www.idoklad.cz/podpora/nastaveni-banka
(aktualizace 29. 6. 2026, přečteno 18. 10. 2026) uvádí měnu účtu a IBAN
pro mezinárodní platby. Nedokládá seznam přijímaných zemí ani validační algoritmus.
Živý experiment v testovací agendě nebyl proveden.

Technický primární zdroj: https://docs.rs/iban/0.2.0/iban/ a staticky přečtený
zdroj `iban` 0.2.0 z crates.io, `src/lib.rs`, implementace `FromStr`:
kontrola známé země, délky, znakové struktury BBAN a mod 97. Knihovna odkazuje
na registr Swift/ISO 13616. Seznam zemí je svázaný s verzí závislosti v Cargo.lock;
netvrdíme aktuální úplnost celosvětového registru ani validaci existence účtu.

P16-13h je vlastní návrh rozšíření bankovní evidence. Normalizace, unikátnost
v profilu, měna nezávislá na zemi a omezení na bankovní evidenci jsou v
specs/bank-currencies.md. Příkladové IBAN a syntetické XML testy nejsou
skutečnými exporty ani důkazem chování iDokladu či zahraničních bank.

## E-070 — zahraniční IBAN a QR Platba (18. 10. 2026)

Veřejný primární zdroj https://qr-platba.cz/pro-vyvojare/specifikace-formatu/
popisuje účet ACC jako IBAN a volitelný BIC.
https://qr-platba.cz/kontakt/ omezuje spolehlivou použitelnost na tuzemské
platby; samotná syntaxe IBAN nepotvrzuje podporu zahraniční bankovní aplikací.
Přečteno 18. 10. 2026; standard 1.2 (červen 2021) na
https://qr-platba.cz/wp-content/uploads/1645-standard-qr-v1-2-cerven-2021.pdf.
Vlastní rozhodnutí: přijmout zahraniční IBAN v profilu/dokladu, zachovat QR
jen CZ/CZK. Živé ověření iDokladu či mobilních bank nebylo provedeno.

## E-080 — ISDOC 6.0.2: formální exportní kontrakt (18. 10. 2026)

Primární veřejný standard Ministerstva vnitra, verze 6.0.2 z 23. 3. 2022:
https://isdoc.cz/6.0.2/, https://isdoc.cz/6.0.2/doc/isdoc.html,
https://isdoc.cz/6.0.2/xsd/isdoc-invoice-6.0.2.xsd.
Staticky přečtené XSD/Schematron, nikoli pozorování iDokladu nebo příjmu do
Money/Pohody. XSD je s původní licencí u syntetických testů, SHA-256
fb5de36fe7b5517acb8aa0c7950d91f8e28910e63b8cdcaaf7db0b0f4f0420bb.
Schéma rozlišuje typy dokumentů, lokální/foreign částky a rate/units;
AmountType je obecný decimal. P19-03a exportuje uložené běžné faktury,
neodvozuje z něj automaticky jiný účetní formát, podpis nebo daňové přiznání.
Náš explicitně omezený převod a validace: specs/isdoc-export.md.

## E-081 — Registr DPH RWS: V2 typ subjektu a účty (18. 10. 2026)

Primární zdroje Finanční správy:
https://financnisprava.gov.cz/cs/dane/dane-elektronicky/danovy-portal/registr-dph,
https://podpora.mojedane.gov.cz/cs/seznam-okruhu/rozhrani-pro-treti-strany/informace-o-webove-sluzbe-pro-zjistovani-4381,
WSDL https://adisrws.mfcr.cz/adistc/axis2/services/rozhraniCRPDPH.rozhraniCRPDPHSOAP?wsdl
(SHA-256 7e878ee0721507fc91f94e457ecb5ddca5c0ccc750d06035848cc185c35b3385).
Veřejná dokumentace uvádí rozšíření o identifikovanou osobu od 20. 6. 2023.
Statický kontrakt rozlišuje druh subjektu od příznaku nespolehlivosti,
technický status, data a standardní/nestandardní účty včetně ukončení.
Naše ruční ověření P19-02 podle specs/vat-registry.md nemění kontakt ani
daňový režim. Automatické testy používají pouze syntetický provider/XML;
žádná konkrétní firma nebyla živě dotázaná.

## E-082 — VIES: EU a Severní Irsko (18. 10. 2026)

Primární veřejný web Evropské komise
https://ec.europa.eu/taxation_customs/vies/?lang=en (v7.3.0-HF1) a
https://ec.europa.eu/taxation_customs/vies/checkVatService.wsdl
(SHA-256 a5de093d23585cb3ff4d6c2a4367ba6003de48b428e30bb99548f2d184829279).
Přečtený SOAP checkVat kontrakt obsahuje zemi, číslo, boolean valid, datum
a volitelný název/adresu. Technické chyby nejsou záporným potvrzením.
Oficiální UI uvádí ukončení podpory GB od 1. 1. 2021 a samostatnou XI.
P19-02b je ruční ověření pro fakturační použití podle specs/vies.md;
není určováním režimu DPH ani české spolehlivosti. Žádné živé dotazy
na obchodníka ani ukládání cizích registračních dat do fixture neproběhly.

## E-083 — ISDOC.PDF a QR Faktura SID (18. 10. 2026)

Primární veřejné specifikace ISDOC 6.0.2:
https://isdoc.cz/6.0.2/doc/isdoc.html a podpůrný oficiální projekt
https://github.com/isdoc/isdoc.pdf. KDP ČR, QR Faktura SID 1.0:
https://www.kdpcr.cz/informace/qr-faktura/popis-formatu a
https://www.kdpcr.cz/informace/qr-faktura/integrace-s-qr-platbou.
Přečteno 18. 10. 2026; jde o dokumentaci formátů, nikoli pozorování iDokladu.
ISDOC.PDF vkládá účetní XML do PDF; QR Faktura nese souhrnné účetní atributy,
zatímco samotná SPD QR Platba není účetním dokladem. X-INV vkládá SID do SPD
s přesunem společných platebních atributů. Plná konformita PDF/A, všechny
ISDOC režimy nebo podpisy z těchto podkladů nevyplývají.

P17 používá konzervativní klasický PDF subset, samostatné ISDOC 6.0.1/6.0.2
běžných faktur CZK/podporovaných měn s uloženým kurzem a QR v PNG/JPEG.
Zachovává původní částky, kontroluje součty, neověřuje digitální podpisy.
P20-04b generuje SID/Platba+F z immutable snapshotu podle specs/payment-qr.md;
šest CS/EN PDF (payment/invoice/combined) bylo 18. 10. 2026 vyrastrovaných
Popplerem 150 DPI a nezávisle přečtených zxing-cpp 2.3.0 s přesnou shodou
celého payloadu. Syntetický ISDOC export/import CZK+JPY a SID/SPD roundtrip
jsou technické testy našeho kontraktu, nikoli živá shoda účetního systému.

## E-084 — Nativní IMAP a MIME, omezené kontrakty (18. 10. 2026)

IETF RFC 9051 (srpen 2021), https://www.rfc-editor.org/rfc/rfc9051.html,
zejména UID/UIDVALIDITY, EXAMINE a BODY.PEEK; primární protokolová dokumentace
přečtena 18. 10. 2026. Staticky přečtené Rust kontrakty imap 3.0.0-alpha.15
https://docs.rs/imap/3.0.0-alpha.15/imap/ a mailparse 0.16.1
https://docs.rs/mailparse/0.16.1/mailparse/. LOGIN/app-password přes ověřený
implicitní TLS; XOAUTH2 a jiné specifické autentizační integrace nejsou
implementované. Není doloženo přihlášení ke konkrétní živé poštovní službě.

P17-03b/c jsou vlastní návrhová pravidla: privátní instalační reference
konfigurace, firemní opt-in, nezávislý opt-in pětiminutového workeru, původní
UID epochy/kurzor, limity těla/částí/sítě, atomický příjem každé zprávy a
příloh, explicitní odmítnutí UID, revize, claim a opakované ověřování práv.
Dva izolované PostgreSQL scénáře a čtyři unit/loopback scénáře pokrývají
replay, změnu epochy, souběh workerů i odebrání práv/vypnutí během I/O.
Žádný test nečetl skutečnou schránku; zprávy se nemažou ani neoznačují přečtené.

## E-085 — Vlastní gateway a vytěžovací kontrakty (18. 10. 2026)

`inbox/mail`, `inbox/{id}/extract-provider` a `bank/notifications` jsou
výslovné vlastní integrační návrhy Open Faktury, nikoli reverse-engineering
konkrétní mailové, AI nebo bankovní služby. Smlouvy a limity jsou v
specs/inbox.md, specs/inbox-mailbox.md a specs/bank-notification-gateway.md.
Gateway má standardní autentizovanou relaci, explicitní opt-in, Message-ID,
hash původních příloh, replay a diagnostiku. Bankovní přílohy znovu používají
stávající parsery podle specs/bank-statements.md; neexistuje univerzální
parser textů bankovních e-mailů ani implicitní párování.

Vytěžovací provider je privátní důvěryhodná instalační reference HTTPS,
nikoli URL z klienta. Explicitní síťová akce předá originál a očekává
validované ReceivedData jako návrh k ruční kontrole. Není to hotová
specifická integrace libovolného AI modelu. Loopback testy používají
syntetické requesty a ověřují velikost, redirect, validaci soukromých vazeb;
žádný skutečný účetní originál nebyl odeslán externímu poskytovateli.

## E-086 — Anonymizace adresářového kontaktu

- Přečteno 18. 10. 2026, veřejný článek iDokladu publikovaný 16. 5. 2018:
  https://www.idoklad.cz/blog/gdpr-v-idokladu-zadarmo-prehled-funkci-kterymi-vam-idoklad-usnadni-gdpr
- Doloženo veřejným textem: výběr subjektu, přehled jeho dokladů, nevratná
  anonymizace údajů v adresáři i na dokladech. Nejde o pozorování současného
  API/UI; článek nedokládá současné role, transakce, auditní payloady či replay.
- Záměrná odchylka Open Faktury: vlastník může explicitně anonymizovat
  samostatný kontakt a jeho adresářovou historii. Historické finanční
  snapshoty a původní PDF se nemění; importy/přílohy/exporty/zálohy zůstávají.
  Tento rozsah je před potvrzením výslovně uveden a není tvrzením o splnění GDPR.
- Vlastní rozhodnutí: owner-only, lokální opis názvu v UI, UUID+revize,
  sanitizace data/initial_data/contact_audit, konečný archivovaný stav,
  DB ochrana, aktuální oprávnění před replay; neutrální zástupný název.
- Specifikace docs/specs/contact-anonymization.md; migrace100; syntetické
  integrační testy contact_anonymization5/5 a contact3/3; reálná agenda nedotčena.

## E-P17-DETACH-01 — Odpojení dokumentu v inboxu

- Zdroj: https://www.idoklad.cz/podpora/inbox, veřejná nápověda,
  poslední aktualizace 6. 5. 2026, znovu přečteno 18. 10. 2026.
- Oddíl „Další úpravy v Inboxu“ výslovně dokládá odpárování pomocí tlačítka
  Odpojit dokument. Samostatně popisuje mazání originálu s dotazem na přílohu;
  toto není zaměňováno s odpojením. Nejde o přihlášené pozorování ani API.
- Chybějící pravidla (vlastní návrh): detach vymaže jen vazbu inbox originálu,
  zachová přijatý doklad, kopii přílohy, hash a archivaci, přidá revizi/audit;
  role editor/owner, UUID replay a souběžné revizní konflikty jako ostatní
  inbox příkazy. Reattach stejné entity reuse aktivní kopie stejného hashe,
  vždy v téže firmě. Žádné automatické mazání nebo nové účetní zpracování.
- Specifikace docs/specs/inbox.md §Odpojení originálu; syntetický test
  crates/server/tests/inbox.rs `detach_preserves_document_attachment_archive_replay_and_scoped_reattach`.

## E-P20-PRINT-01 — Dodací list s cenami

18. 10. 2026, veřejná nápověda iDokladu „Nastavení – Prodej“, poslední
aktualizace 23. 4. 2024:
https://www.idoklad.cz/podpora/nastaveni-prodej
Výslovně doložena volba dodacího listu s cenou / bez ceny. Přesné částky,
API, role, snapshot či replay nejsou dokumentem doloženy. Open Faktura
používá vlastní owner/revize/UUID nastavení, čtení aktuální preference při
vyžádání pomocného výstupu a explicitní override true/false. Ceny jsou
uložené net/VAT/gross jednotlivých původních řádků, bez přepočtu nebo
odečítání plateb/záloh. Původní fakturační PDF se nepřepisuje.

## E-P17-DELIVERY-01 — Předání původního originálu účetnímu

- Zdroj: https://www.idoklad.cz/podpora/inbox, veřejná nápověda,
  aktualizace6.5.2026, znovu přečteno18.10.2026; oddíl Zasílání dokladu účetní.
- Doloženo: původní inbox přílohy při e-mailovém předání, reference zmiňuje
  dva roky platný odkaz. Nejde o přihlášené pozorování či SMTP kontrakt.
- Vlastní Open Faktura návrh: samostatný explicitní original-only SMTP send
  s konkrétním adresátem a potvrzením jedné přílohy. Bez publicURL, bez
  kopírování referenčního TTL a bez generovaného účetního PDF. Immutable
  rawbytes snapshot, UUID/revision replay, currentcompany role před queue
  i dispatch, durable queued/sending/sent/failed/uncertain, žádný auto retry.
- Sdílí instalační SMTP a bounded dispatch existujících invoice/quote
  outboxů. Specifikace docs/specs/inbox-delivery.md, migrace101,
  kontrakt inbox-delivery.openapi.json; testy jen syntetické loopback SMTP.

## E-P20-UPDATER-01 — Tauri signed updater a vlastní explicitní tok

- Přečteno18.10.2026: https://v2.tauri.app/plugin/updater/,
  https://docs.rs/tauri-plugin-updater/2.14.0/tauri_plugin_updater/struct.Update.html,
  https://v2.tauri.app/distribute/sign/macos/ a
  https://v2.tauri.app/distribute/sign/windows/.
- Staticky ověřen pinned crate tauri-plugin-updater2.14.0 (Cargo.lock):
  restore_update používá stejný target/comparator a vynucuje signed-version;
  verify/install ověřují podpis, install jej kontroluje před OS operací.
  get_urls hledá os-arch-installer před os-arch. Nejde o pozorování instalace.
- Vlastní návrh: build-time důvěryhodný HTTPS endpoint/publickey, oba chybí
  znamená disabled, lokální RustcustomIPC místo genericpluginIPC; explicitní
  check a install s verzí/tokenem/confirm/TTL; žádný background check/install.
  Bounded512KiB manifest +256MiB package, ruční restart a zachováníidentifieru,
  žádné backend migrace nebo mazání klientských dat.
- Odděleny Minisign update podpisy a platformní CodeSigning/notarizace;
  chybějící certifikáty/cizíOS instalace se nevydávají za ověřené. Volitelné
  release CI, kryptograficky ověřené.sig+verze, unsigneddefaultbezlatest.json.
- Syntetický test dočasného klíče vtemp je výslovně autorizovaný; klíč ani
  childsecretoutput nejde do repa/logů, po testu odstraněn. Rust3/3 ověřuje
  podpisy/legacy/downgrade/tamper/limits/gates bez voláníOSinstall. Node
  collector fixture test ověřuje4targets/defaultunsigned/crypto, nikoli
  skutečné instalační balíčky. Spec docs/specs/desktop-updates.md.
