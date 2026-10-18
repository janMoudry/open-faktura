> Aktivní dodávka od 15. 10. 2026: **celé P16–P20**. Stav zbývajících celků:
> [kontrolní seznam P16–P20](P16_P20_COMPLETION.md).

# Implementační plán Open Faktury

Aktualizováno 12. 10. 2026. Tento plán nahrazuje původních šest širokých etap.
Rozsah shrnuje [FEATURES.md](../FEATURES.md); pořadí je návrh dodávek, nikoli
vyřazení pozdějších funkcí. P00–P20 jsou stabilní ID pracovních balíků, jednotlivé
požadavky mají vlastní ID (INV-001…, SETUP-001…, MIG-01). P01–P08 a L10N-01 jsou implementované (výpočty podle vlastních pravidel); P09 má obecný import, ale dosud neověřené adaptéry zdrojových služeb. P12-01 přidává ceník, P12-02/P12-03 šablony a nabídky s převodem do faktury. Navazující dodávky přidaly odesílání faktur a nabídek, veřejné odkazy nabídek, nastavení SMTP, vlastní přílohy a export seznamů CSV/XLSX. Stav každé dodávky je uveden níže; souhrn dostupných funkcí je v [přehledu](../FEATURES.md). Projekt zatím není kompletní účetní systém.

Odkazy „FEATURES §…“ níže označují oddíly původního [referenčního soupisu](research/FEATURES_REFERENCE.md).

## Potvrzené rozšíření na všechny moduly (11. 10. 2026)

Uživatel výslovně zahrnul **celý plánovaný rozsah Open Faktury** z [přehledu funkcí](../FEATURES.md).
P10–P20 jsou autorizovaný rozsah implementace, nikoli jen volitelný seznam.
Vlastní brand Open Faktury (UI-REDESIGN-01), CS/EN a skutečné profilově izolované API/DB zůstávají.
Referenční chování ověřujeme po scénářích; vlastní pravidla a mezery mají explicitní stav.

Pořadí samostatně ověřitelných dodávek:

1. P12-01 ceník → výběr do faktury (implementace `tested`, CAT-001–004,
   [specifikace](specs/catalog.md), reference E-024).
2. P12-02 šablony a P12-03 nabídky → převod do faktury (`tested` v popsaném rozsahu); dále P12-04 měny/kurzy.
3. P13-01 zálohy → P13-02 daňové doklady k platbě → P13-03 vyúčtování → P13-04 dobropisy.
4. P10 soubory/SMTP/outbox → P14 opakování, upomínky, sdílení a historie odeslání.
5. P15 přijaté faktury a účtenky → závazky/úhrady; dodavatelská role adresáře.
6. P16 bankovní/pokladní evidence → import a párování → prodejky.
7. P18 ruční sklad → návaznost prodeje/nákupu → opravy a statistiky. P18-03e doplnilo pořizovací ceny počátečních stavů, ručních příjmů a inventurních přebytků; P18-04 doplnilo přehled prodeje položek. P18-05a doplňuje import Money S3 XML; P18-05c doplňuje obecný CSV/XLSX import ceníku. P18-05b Money S4/S5 čeká na konkrétní export a konfiguraci (E-059) (viz P16_P20_COMPLETION.md).
8. P17 inbox, ISDOC/QR a e-mailový příjem; AI jako oddělená volitelná schopnost.
9. P19 přehledy, registry, účetní exporty a daňové podklady; P20 zbývající
   funkce z matice FEATURES (štítky, koš, uzávěrky, hromadné akce, tisk/brand…).
10. P11 veřejná distribuce, aktualizace, správa přístupů a provozní ověření.

Každá dodávka končí referenčním důkazem, specifikací odchylek, DB migrací,
REST/MCP kontraktem, CS/EN UI a integračními scénáři. Nedokončená položka
zůstává označená jako nedostupná. Tento seznam sám o sobě nedokládá hotové moduly.

## UI-REDESIGN-01 — vlastní vizuální identita (17. 10. 2026)

Schválené obrázkové návrhy převedené do tokenů, společného shellu, Přehledu, seznamu a editoru faktur. Účetní chování a kontrakty zůstávají společné. Rozsah a limity ověření: [záznam dodávky](design/redesign-2026-10-17/README.md).

## Výchozí stav a hranice

- Hotová technická kostra: React/Tailwind, sdílený design systém, Tauri obal,
  Rust/Axum/SQLx, PostgreSQL, technická migrace, health/readiness, lokální CI konfigurace.
- Produktový seznam a editor ukládají návrhy faktur do PostgreSQL (P06).
  Vystavení, čísla a PDF jsou implementované v P07. Syntetický sandbox je oddělený.
- Katalog komponent prošel [auditem](design/design-system-audit.md). Zachovat
  jej jako vývojový nástroj a regresní testy; odstranit jej z běžného produktu.
- Přijaté technologie a hranice vrstev zůstávají podle
  [konvencí](architecture/DEVELOPMENT_CONVENTIONS.md). Mobil: responzivní web/PWA;
  desktop: aktuálně Tauri klient připojený k serveru. Zjednodušený osobní režim
  PERSONAL-01 je nově plánovaný; lokální backend a DB zatím nejsou balené.
- Vlastní návrh není doloženou shodou s iDokladem. Neuzavřená účetní pravidla
  v [evidenci Q-01 až Q-07](research/EVIDENCE.md) blokují jen související balík.

## Milníky

| Milník                       | Rozsah            | Co uživatel skutečně zvládne                                                                     |
| ---------------------------- | ----------------- | ------------------------------------------------------------------------------------------------ |
| A — Prázdný skutečný produkt | P00–P04 + L10N-01 | Nainstalovat, bezpečně založit správce a firmu, přihlásit se a uložit kontakt do DB              |
| B — První fakturace          | A + P05–P08       | Návrh → vystavení → PDF → ruční úhrada; data přežijí restart i obnovu                            |
| C — Veřejná self-hosted beta | B + P09–P11       | Ověřený přechod z podporovaných zdrojů, vlastní SMTP, zálohy, aktualizace a distribuovaný klient |
| D — Prodej a finance         | C + P12–P16       | Nabídky, zálohy, opravy, opakování, nákup, banka a pokladna                                      |
| E — Širší pokrytí            | D + P17–P20       | Inbox, sklad, přehledy, daňové podklady a ověřené integrace                                      |

Beta C smí deklarovat jen otestované importní adaptéry a podporované platformy.
Nedostupný export zdroje zaznamenat jako omezení, nikoli předstírat úplnou migraci.
První interní pilot je B; licence a provozní kontrola P11 jsou podmínkou veřejného
vydání. Časové odhady stanovit až po specifikaci balíku, ne podle počtu obrazovek.

## A — Od prototypu k vlastní agendě

### P00 — Produktové hranice a kontrakty

**Závislosti:** žádné. **Rozsah:** FEATURES §19–21, INV-001 až INV-009.

- Zaznamenat schválený rozsah prvního pilotu: česká agenda, CZK, běžné vydané
  faktury a ruční platby; plátce i neplátce až s ověřenými výpočtovými scénáři.
  Ostatní měny a daňové režimy postupně v P12/P19, v UI je do té doby nenabízet.
- Specifikovat vlastní role (správce instalace oddělený od vlastníka firmy),
  relace, revize, chybové odpovědi, idempotenci a generování OpenAPI klienta.
- Licence zvolena 12. 10. 2026: [GNU AGPL v3](../LICENSE), `AGPL-3.0-only`, bez zákazu komerčního použití či prodeje. Při vydání zachovat licenční oznámení a zajistit odpovídající zdrojový kód podle [licenčních podmínek](LICENSING.md).
- Získat anonymizované exportní vzorky iDokladu a Faktury online pro MIG-01.
- Q-01 až Q-04 uzavírat pro P05–P08; vlastní odchylky výslovně evidovat.

**Hotovo:** konkrétní backlog pro P01–P04, návrh onboarding kontraktu a matice
oprávnění; otevřené účetní otázky mají scénáře a dotčené balíky. Chybějící živá
reference neblokuje přihlášení a konfiguraci.

### P01 — Produktový vstup a oddělení sandboxu

Implementace: `implemented` (10. 10. 2026). Samostatný vývojový HTML vstup,
produkční routy a kontrola build grafu jsou připravené. Účty, autorizovaný
workspace a setup endpoint zůstávají P02/P03; současné routy nesimulují přístup.
Ověření: `tests/e2e/product.spec.ts` a zachované sandbox regresní scénáře.
Vlastní instalační vstup je návrh Open Faktury; referenční UI E-008 až E-010
zůstává zachované v sandboxu, bez nového tvrzení o shodě s iDokladem.

**Závislosti:** P00. **Rozsah:** produktová navigace, routy a stavy.

- Oddělit vstup aplikace, přihlášení, setup, výběr firmy a chráněný workspace.
- `DesignSystemPage`, galerie a syntetické faktury přesunout do explicitního
  vývojového vstupu/režimu. Produkční build je nesmí importovat ani routovat;
  samotné schování odkazu nestačí. Zachovat vizuální regresní testy.
- `invoice-demo.ts` a demo kalkulátor zůstanou pouze u fixtures. Produkční editor
  připojit až v P06; žádný fallback na lokální demo při výpadku API.
- Odstranit smyšlenou firmu, částky, dashboard a nefunkční navigační akce z produktu.
  Použít skutečné prázdné stavy, chybu připojení a opakování požadavku.
- Nesmazat automaticky uživatelský localStorage. Demo koncepty nepřenášet do DB
  jako skutečné faktury; případnou likvidaci testovacích dat řešit explicitně.

**Hotovo:** produkční bundle a přímé URL neobsahují sandbox, vývojový katalog
funguje dál. Po refreshi fungují routy; výpadek serveru nikdy nezobrazí demo.
Do P02 je vstup pravdivý stav nepřipravené instalace, nikoli falešná hotová agenda.

### P02 — Instalace, bootstrap a přihlášení

Implementace: `tested` v rozsahu P02 (10. 10. 2026). Rust/PostgreSQL bootstrap, auth,
obnova správce a instalační konfigurace viz [provozní návod](INSTALLATION.md).
HTTP/DB testy v `crates/server/tests/auth.rs`, E2E v `tests/e2e/auth.spec.ts`.
Firemní průvodce a členství nejsou součástí tohoto stavu; navazují P03.

**Závislosti:** P00–P01. **Rozsah:** SETUP-001 až SETUP-005, FEATURES §14/19.

- Produkční Compose: aplikace + DB, trvalé volumes, konfigurace originu a URL,
  tajemství, health/readiness, jednorázově koordinované migrace. Zachovat zvlášť dev Compose.
- Jednorázový bootstrap řízený tajemstvím provozovatele; atomicky založit prvního
  správce, vypnout veřejný bootstrap. Žádný výchozí účet ani veřejná registrace.
- Lokální přihlášení, odhlášení, expirace a revokace relací, bezpečné heslo,
  omezení pokusů, CSRF ochrana pro cookie relace a testy autorizace.
- Obnova přístupu bez SMTP přes administrátorský CLI postup; SMTP není povinné.
- Základ zálohy DB a souborů a obnova na prázdnou instanci před prvními reálnými daty.

**Hotovo:** čistá instalace projde setupem; dva souběžné bootstrapy nezaloží dva
první správce; restart nepovolí setup znovu. Neoprávněný uživatel nezíská agendu.
Výpadek DB se nikdy neinterpretuje jako nová instalace.

### P03 — Firma, nastavení a hranice přístupu

Implementace `tested` (10. 10. 2026): PostgreSQL firmy/členství, obnovitelný
průvodce CS/EN, nastavení, audit/účinnost DPH, revize a autentizovaný read-only
MCP stdio. [Rozsah, oprávnění a provozní návod](COMPANIES.md),
[specifikace](specs/company-setup.md). Fakturace, pozvánky a vzdálené MCP OAuth
nejsou hotové funkce tohoto balíku.

**Závislosti:** P02. **Rozsah:** SETUP-006 až SETUP-010, FEATURES §14.

- Průvodce první firmou podle [specifikace](specs/installation-onboarding.md):
  identita → účetní nastavení → platby a číslování → kontrola → prázdná agenda.
- Perzistentní firmy, členství, přepínání firem a oprávnění v application vrstvě.
- Firemní nastavení: kontaktní údaje, profil DPH s účinností, měna, časová zóna,
  splatnost, bankovní účet, číselná řada. Údaje lze následně editovat oprávněnou rolí.
- Audit prvních změn a konflikty revizí. Správce instalace nezískává automaticky
  aplikační členství ve všech firmách; provozovatel má samozřejmě fyzický přístup k DB.
- Základ MCP autentizace, výběr firmy a read-only nástroj kontextu; stejné autorizace jako REST.

**Hotovo:** průvodce lze přerušit a obnovit; první firma nemá demo obsah. Testy
přímých ID, seznamů i vazeb dokazují izolaci dvou firem ve web/API/MCP.

### P04 — Adresář jako první skutečný CRUD

**Závislosti:** P03. **Rozsah:** FEATURES §8.

**Stav: implementováno a ověřeno 11. 10. 2026.** Specifikace a hranice:
[contacts.md](specs/contacts.md), referenční základ E-013. Web CS/EN, REST i MCP
používají PostgreSQL a oprávnění aktivního profilu. Ověřeny souběžné zápisy,
audit, archivace/obnova, izolace profilů a role; UI scénáře prošly v obou jazycích
na desktopu i mobilu včetně konfliktu editací a režimu pouze pro čtení.

- Kontakt, fakturační/dodací adresy, kontaktní osoby, příjemci, platební preference.
- Seznam, detail, hledání, stránkování, editace, archivace a prázdný stav.
- Generovaný API klient; MCP čtení/zápis přes stejné příkazy a audit.
- Doplnění názvu, DIČ a sídla podle IČO z ARES implementováno 11. 10. 2026
  na přání uživatele již v P04; ověření plátcovství/DIČ zůstává P19.
  Ruční založení funguje bez externí služby.

**Hotovo:** kontakt přežije restart, konflikt dvou editací je viditelný, archivace
nezničí historii; přístup cizí firmy a její ID jsou odmítnuty. Milník A.

## B — První kompletní fakturační průchod

### P05 — Kalkulátor a model faktury

**Stav 11. 10. 2026:** technický základ implementován podle explicitních vlastních
pravidel `of-czk-v1` (Decimal, model/snapshoty, REST/MCP a generovaný klient).
Specifikace: [invoice-calculation.md](specs/invoice-calculation.md).
Referenční shoda zůstává otevřená v Q-01/R-01–03; neoznačovat celý balík za
reference-verified. UX podklad pro P06: [editor](design/invoice-editor-ux.md).

**Závislosti:** P03, uzavření Q-01 a minimální Q-02/Q-03. **Rozsah:** INV-001/002/005/006.

- Rust hodnotové typy, přesnosti Decimal, validace, verze pravidel a snapshoty.
- Návrh režimů ceny, množství, slev, DPH a rekapitulace podle ověřených příkladů.
- API/MCP přepočet bez vystavení; desetinné hodnoty v kontraktech beze ztráty přesnosti.

**Hotovo:** nezávislé očekávané částky pro hraniční množství, slevy a zaokrouhlení;
stejný výsledek přes všechny adaptéry. Demo TypeScript kalkulátor není autorita.

### P06 — Skutečný seznam a editor návrhů

Implementace: `implemented` (11. 10. 2026). PostgreSQL, REST/MCP a CS/EN editor
seznamu a návrhů: [specifikace](specs/invoice-drafts.md). Databázové a UI
regrese v `crates/server/tests/invoice.rs` a `tests/e2e/invoices.spec.ts`.
Žádné vystavení ani přidělování konečných čísel; to zůstává P07.

**Závislosti:** P04–P05. **Rozsah:** INV-001/006, FEATURES §1–2.

- DB návrhy, detail a editor navázané na existující UI; výběr kontaktu, data,
  položky, texty, objednávka a platební údaje. Rozpracovaný návrh může být neúplný.
- Uložit/otevřít/kopírovat návrh, ochrana neuložených změn a konflikt revize.
- Serverové filtry, hledání, stránkování a řazení; loading/error/empty stavy.
- Souběžný přepočet nesmí přepsat novější vstup starší odpovědí.

**Hotovo:** návrh znovu otevřu po restartu a na jiném klientu; bez serveru nelze
předstírat uložení. Čtení i zápis mají integrační a uživatelský scénář.

### P07 — Vystavení, čísla a PDF

Implementováno 11. 10. 2026 pro tuzemské CZK faktury. Kontrakt, vlastní pravidla
a hranice referenční shody: [specifikace P07](specs/invoice-issue.md).
REST/MCP, CS/EN UI, zamčený detail a serverové PDF jsou propojené s PostgreSQL.

**Závislosti:** P06, Q-02/Q-03. **Rozsah:** INV-004/005/008/009.

- Transakční číslování, validace vystavení, neměnný historický obsah, audit a idempotence.
- Detail vystaveného dokladu, stažení/tisk PDF z uložené revize, první česká šablona.
- Vybrat PDF renderer, ověřit české fonty, více stránek a dlouhé položky.
- Nepovolené opravy jasně odmítnout; kompletní opravný tok přijde v P13.

**Hotovo:** souběh neduplikuje čísla, opakovaný příkaz vrací původní doklad,
změna kontaktu/nastavení nepřepíše PDF. Vystavení nic automaticky neodesílá.

### P08 — Ruční úhrady a základní přehled

Implementováno 11. 10. 2026 pro pilotní CZK rozsah. Viz [P08](specs/invoice-payments.md)
pro pravidla, REST/MCP, CS/EN UI, migraci 009 a obnovovací scénář.

**Závislosti:** P07, Q-04 pro podporované případy. **Rozsah:** INV-003, FEATURES §10/12.

- Plná a částečná úhrada, přeplatek, oprava/storno úhrady s auditní stopou.
- Oddělit stav vystavení, úhrady a splatnosti; historie alokací a plateb.
- Reálné součty neuhrazených/po splatnosti, filtry a základní přehled nad DB.
- Zopakovat obnovu DB + příloh/PDF do čisté instance a porovnat doklady a platby.

**Hotovo:** kontakt → návrh → vystavení → PDF → úhrada funguje ve web/API/MCP;
částky a izolace ověřeny. Podporovaný pilotní rozsah explicitně popsaný. Milník B.

## C — Přenos dat a použitelná distribuce

### P09 — Migrační průvodce

11. 10. 2026: implementované pilotní jádro, CSV/XLSX kontakty, kanonický JSON
        historických CZK faktur, náhled, konflikty, pokračování a report. **P09 zůstává
        rozpracované**: nejsou ověřené autentické exporty obou služeb ani jejich úplné
        fakturační adaptéry. Rozsah a ověření: [specifikace](specs/data-migration.md).

**Závislosti:** P04, P07–P08, ověřené vzorky P00. **Rozsah:** MIG-01, FEATURES §15/19.

Společný importní model, dry-run, mapování, konflikty, dávky a bezpečné pokračování.
První adaptéry iDoklad/Faktura online podle skutečně dostupných exportů; záložní
CSV/XLSX s mapováním. Historický import nepoužívá běžné nové vystavení, nepřiděluje
nová čísla a tiše nepřepočítává původní částky. Přesný rozsah níže v MIG-01.

**Hotovo:** porovnání počtů/částek/měn, opakování a výpadek bez duplicit,
nezaměnitelné zdrojové agendy, nepodporovaná data vysvětlená před zápisem.

### P10 — Odesílání a soubory

**Závislosti:** P07. **Rozsah:** FEATURES §2/11/19.

Vlastní SMTP, test nastavení, šablona, PDF příloha, historie a oddělené odeslání.
Worker/outbox, retry a viditelný nejasný výsledek doručení; neprohlašovat přesně
jedno odeslání při neurčité odpovědi SMTP. Souborové úložiště, přílohy a limity,
oprávnění stažení. Základní CSV/XLSX export seznamů a kontaktů.

**Hotovo:** fakturace funguje bez SMTP; selhání odeslání nevrací vystavení zpět;
cizí firma nestáhne soubor. Záloha zahrnuje soubory i potřebná tajemství.

### P11 — Self-hosted beta, PWA a desktop

**Závislosti:** P08–P10. **Rozsah:** FEATURES §14/16/19.

- Verzionované obrazy, instalace pro provozovatele, HTTPS/proxy návod, zálohy,
  obnova, aktualizace schématu, rollback obnovou kompatibilní verze a dat.
- Pozvánky (i bez SMTP předáním jednorázového odkazu), role, odebrání členství,
  obnova účtu, změna hesla/e-mailu a 2FA včetně recovery před veřejnou betou.
- PWA instalace a online chování; offline obrazovka bez nepravdivých stavů uložení.
  Offline fakturace/synchronizace není součástí prvního release.
- Tauri volba serveru, bezpečné relace a odhlášení, kompatibilita API, CSP/originy,
  instalátory a podpisy pro ověřené platformy, assets/checksumy v GitHub Releases.
- Licence, CONTRIBUTING, SECURITY, changelog, podporované verze a provozní diagnostika.
  Základní účetní tok bez centrálního účtu, licence serveru či placeného API.

**Hotovo:** jiný provozovatel podle návodu nainstaluje, obnoví a aktualizuje
instanci; otestované cílové instalátory, žádná release tajemství v PR jobech.
MCP má omezené/revokovatelné přístupy a read-only režim. Milník C.

## D/E — Navazující funkční balíky

Každý řádek rozdělit při zahájení na samostatné dodávky v uvedeném pořadí.
Před účetní implementací vlastní specifikace, migrační návrh a reference; nejde
o jeden velký PR ani o pouhé přidání obrazovek. Implementace všech: `not-started`.

| ID  | Pořadí dílčích dodávek a rozsah FEATURES                                                                    | Závislosti                | Podmínka dokončení                                                                                     |
| --- | ----------------------------------------------------------------------------------------------------------- | ------------------------- | ------------------------------------------------------------------------------------------------------ |
| P12 | Ceník/služby → šablony → nabídky a převod → cizí měny, kurzy a jazyky (§1/4/5/9)                            | P08                       | Převod zachová obsah; změna ceníku nemění historii; kurz/množství/zaokrouhlení mají fixture            |
| P13 | Zálohy → daňové doklady k platbě → vyúčtování více záloh → dobropisy (§3, INV-007)                          | P08, Q-06                 | Souběžně nelze přečerpat zálohu; opravy a měnící se sazby zachovají vazby a audit                      |
| P14 | Pravidelné faktury → upomínky/poděkování → sdílené odkazy a historie (§5/11)                                | P10, P12; zálohy také P13 | Jeden výskyt nevytvoří více faktur; časová zóna a retry ověřeny; odkazy lze odvolat                    |
| P15 | Přijaté faktury → účtenky → úhrady závazků a nákladové přehledy (§6)                                        | P08/P10                   | Originál a dodavatel dostupné; storno/oprava platby zachová správný zůstatek                           |
| P16 | Bankovní/pokladní evidence → import/notifikace → ruční/automatické párování → prodejky (§10)                | P08/P15                   | Duplicitní pohyb není druhá platba; nejednoznačnost vyžaduje řešení; odpojení opraví saldo             |
| P17 | Upload inbox → ISDOC/QR → příjem e-mailu → volitelné AI vytěžení (§7)                                       | P10/P15                   | Originál zachován, vytěžení je návrh k potvrzení; základ funguje bez AI poskytovatele                  |
| P18 | Ruční sklad → pohyby z prodeje/nákupu → opravy a statistiky (§9)                                            | P12/P15                   | Jedna operace nevytvoří duplicitní pohyb; oprava má dohledatelný dopad na zásobu                       |
| P19 | Rozšířené grafy → ARES/DIČ → účetní exporty po systémech → daňové režimy/formuláře po verzích (§8/12/13/15) | P13/P15/P16 dle výstupu   | Graf lze vysvětlit doklady; export proti cílovému schématu; daně podle aktuálních oficiálních podkladů |
| P20 | Pokročilá správa dokladů → nastavení/přístupnost → provozní úplnost (§2/14/16/19)                           | P11 a relevantní moduly   | Viz kontrola úplnosti níže; každá položka má scénář, test nebo přiznanou mezeru                        |

P20 zahrnuje štítky, koš/obnovu a pravidla mazání, omezení editace po exportu,
revize exportů, hromadný tisk, RTF, dodací listy, potvrzení úhrad, QR Platba/
Faktura/Platba+F podle ověřených podmínek; branding dokumentů, podpis/razítko,
tmavý režim, jazyk a notifikace; převod vlastnictví, opuštění/zrušení firmy a
anonymizaci s ohledem na zachování historie. Dále úplný datový export,
rozšířené migrační adaptéry, dokumentace/nápověda a automatické aktualizace
klienta. Nejde o rezervu pro nepojmenované funkce: při zahájení je rozdělit na
P20-01… a navázat jednotlivé odrážky FEATURES.

## Pokrytí funkčního soupisu

| FEATURES sekce                | Balíky                                                 |
| ----------------------------- | ------------------------------------------------------ |
| §1 Vydané faktury             | P05–P07, P12                                           |
| §2 Doklady a výstupy          | P06–P10, P19–P20                                       |
| §3 Zálohy a opravy            | P13                                                    |
| §4 Nabídky                    | P12                                                    |
| §5 Šablony/opakování          | P12/P14                                                |
| §6 Nákup                      | P15                                                    |
| §7 Inbox                      | P17                                                    |
| §8 Adresář                    | P04/P19                                                |
| §9 Ceník/sklad                | P12/P18                                                |
| §10 Úhrady/banka/pokladna     | P08/P16                                                |
| §11 Komunikace                | P10/P14                                                |
| §12 Přehledy                  | P08/P15/P19                                            |
| §13 Daně                      | P05 základní výpočty, P19 rozšířené režimy a formuláře |
| §14 Firmy/uživatelé/nastavení | P02/P03/P11/P20                                        |
| §15 Import/integrace/API      | P00/P04 průběžně API, P09/P19/P20                      |
| §16 Mobil/služby              | P11/P20; PWA místo samostatných Android/iOS aplikací   |
| §17 Tarifní rozsah            | Všechny balíky bez tarifních limitů reference          |
| §18 Reverse-engineering       | Průběžně před dotčeným balíkem                         |
| §19 Open-source provoz        | P02/P03/P09/P11/P20                                    |
| §20 MCP                       | P03 základ, P04–P20 operace společně s modulem         |
| §21 Specifikace               | P00 a definice dokončení každého balíku                |

## Společná definice dokončení

1. Požadavek má ID, specifikaci, explicitní nepodporované případy a důkaz nebo
   označené vlastní pravidlo. Stavy specifikace: `open / specified / reference-verified`;
   implementace: `not-started / in-progress / implemented / tested`.
2. Vertikální dodávka obsahuje doménu, migraci, aplikační příkaz, REST kontrakt,
   generovaný klient, UI a relevantní MCP operaci. Žádná druhá účetní logika v UI/MCP.
3. Firemní izolace, revize, audit, validační chyby a idempotence/souběh tam, kde
   dávají smysl. Databáze a přílohy přežijí restart i dokumentovanou obnovu.
4. UI používá existující tokeny/komponenty; má empty/loading/error/disabled stavy,
   klávesnici, mobilní rozložení a reduced-motion. Nové stavy patří i do katalogu.
   Nové uživatelské texty včetně validací a přístupných názvů mají překlady `cs`/`en`;
   formátování respektuje locale a layout je ověřen v obou jazycích.
5. Relevantní doménové a integrační testy, E2E skutečné cesty, build a konvence.
   Účetní očekávání nejsou výstupem testovaného kalkulátoru. Reference se netvrdí
   jen na základě zeleného technického testu.
6. Aktualizované návody, stav požadavku a známé mezery. Bez skrytého napojení na
   SaaS; volitelné služby mají konfigurovatelný adaptér a srozumitelný chybový stav.

Pro forky zachovat sémantické brand tokeny, modulární Rust vrstvy a explicitní
porty pro SMTP, úložiště, registry, importy a AI. Agent workspace využije API/MCP,
firemní oprávnění a audit; univerzální plugin runtime není podmínkou první faktury.
Výchozí provoz je jeden modulární server + PostgreSQL, worker od P10; horizontální
škálování nesmí záviset na lokální relaci nebo nedostupném lokálním souboru.
Zátěžové cíle definovat a měřit v P11; mikroservisy nepřidávat před doloženou potřebou.

## Osobní desktop — PERSONAL-01

Návrh po P02: odlišit připojení k serveru od použití na vlastním počítači.
[Specifikace režimů a dílčích dodávek](specs/installation-onboarding.md#osobní-použití-a-desktop).
Specifikace `open`, implementace `not-started`. Před desktopovou distribucí P11
ověřit správu lokálního Rust/PostgreSQL runtime a bezpečný bootstrap bez ručního
klíče. Společný firemní průvodce P03 na tuto práci nemusí čekat.

## Lokalizace od začátku — L10N-01

Implementace `tested` (10. 10. 2026) pro současné produktové UI a sdílené komponenty.
[Návod, ověření a hranice](LOCALIZATION.md). Uložení do profilu navazuje v P03.
Čeština (`cs`, výchozí a fallback) a angličtina (`en`) jsou součástí milníku A,
ne až pozdějšího nastavení účtu. Web i Tauri sdílejí překlady a volbu jazyka.

Dodáno před P03; dále průběžně v každém balíku:

1. Zavést společnou konfiguraci i18n, katalogy `cs`/`en`, typované klíče,
   interpolace a pluralizaci; kontrolu shodných klíčů obou katalogů v CI.
   Použit i18next 25.6.3; kontrola `pnpm check:locales` je součástí CI.
2. Přeložit existující produktový vstup P01/P02, setup, přihlášení, chyby,
   nápovědy a uživatelské texty sdílených komponent včetně přístupných názvů.
   Vývojové popisy sandboxu a dokumentace mohou zůstat česky.
3. Přepínač „Čeština / English“ dostupný i bez přihlášení; volbu zapamatovat
   na zařízení. S uživatelskými preferencemi P03 ji ukládat i do profilu;
   explicitní volba má přednost, bez volby použít češtinu.
4. Centralizovat locale formátování čísel, měn a dat, nastavit HTML `lang`
   a locale komponent kalendáře. Přepnutí nesmí měnit hodnoty, měnu,
   časové pásmo ani účetní pravidla; kalendářní datum neposouvat přes UTC.
5. Ověřit setup a přihlášení v obou jazycích, přepnutí bez ztráty formuláře,
   zachování volby po reloadu, pluralizaci, fallback a mobilní layout.

**Hotovo:** oba jazyky pokrývají existující produktové UI, CI odhalí chybějící
klíče a oba jazykové scénáře projdou. Překlad PDF/e-mailů naváže na jazyk dokladu
v P07/P10/P12; jazyk rozhraní jej automaticky nepřepisuje. Anglické UI neznamená
podporu legislativy jiné země. Konvence: [lokalizace](architecture/DEVELOPMENT_CONVENTIONS.md#lokalizace).

## Nejbližší implementační zadání

P16-01 je implementováno: profilové CZK účty, evidence pohybů, dokumentovaný CSV formát s náhledem a deduplikací, ruční párování a odpojení úhrad. [Specifikace](specs/banking.md). P16-02 má adaptéry podle veřejných kontraktů a návrhy shod s potvrzením. Ověření anonymizovaných originálních výpisů zůstává otevřené; [hranice](specs/bank-statements.md). Dodavatelská role adresáře a rozšíření nákupního editoru zůstávají otevřené v P15. Rozšíření pravidelných záloh, proměnných a více příjemců zůstává výslovně otevřené.

## P15-02 — účtenky a přehled výdajů (13. 10. 2026)

Účtenka má vlastní UC řadu, volitelného dodavatele, hotově/kartou a přílohy.
Plná úhrada vzniká atomicky, oprava stornuje a nahradí původní výdaj,
zrušení zachová historii. Společný seznam úhrad má období/metodu/text/stav
a stránkování. Přehled v CZK odděluje nákupy podle vystavení, výdaje podle
zaplacení a aktuální závazky/po splatnosti za všechna období.
Migrace 026, společný backend REST/MCP, CS/EN a profilová izolace.
[Specifikace](specs/purchase-receipts.md), E-040; výpočty ze zdrojových částek.
Bankovní/pokladní párování, daňové výstupy a cizí měny nejsou součástí.

## P15-01 — přijaté faktury (13. 10. 2026)

Evidence běžných CZK závazků, interní řada PF oddělená od čísla dodavatele,
snapshot a volitelný kontakt, data a přesné částky z originálu. Sdílené přílohy,
vyhledávání/filtry a ruční platby/storna se zůstatkem. Zrušení uchovává historii;
revize, UUID a transakce chrání souběh. Výdajové platby nemění příjmy P08.
Migrace 025, REST/MCP a CS/EN, navigace Náklady → Přijaté faktury.
[Specifikace a odchylky](specs/received-invoices.md), reference E-039.
Účtenky, ceník/množství/jednotkové ceny, přeplatky, daňové výstupy, cizí měny
a tisk/e-mail rekapitulace zůstávají navazující prací.

## P14-03 — veřejné faktury (13. 10. 2026)

Vytvoření/zkopírování odkazu, expirace, odvolání, technická historie zobrazení,
ruční odeslání odkazu místo PDF přílohy. Anonymní stránka zobrazuje původní PDF
a aktuální stav úhrady, bez interních platebních poznámek a vlastních příloh.
Migrace 024, společné příkazy sdílení, REST/MCP a CS/EN. Automatizované e-maily
nadále přikládají PDF. [Specifikace](specs/invoice-sharing.md), reference E-038
(veřejná dokumentace; expirace/tokeny a konkrétní UI jsou naše rozhodnutí).

Průběžně zbývá ověřit skutečné exportní možnosti FakturyOnline a iDokladu pro
P09, uzavřít otevřené účetní otázky a distribuční a provozní podmínky veřejné bety podle zvolené AGPL v3.
[Checklist](FEATURE_CHECKLIST.md) rozlišuje dokončené funkce od těchto mezer.

## MIG-01 — Jednoduchý přechod z jiné fakturační služby

Požadavek uživatele, 10. 10. 2026. Specifikace: `pilot-defined`; implementace: `partial` (P09, viz specs/data-migration.md).
Cílem je provést uživatele přenosem existující agendy do Open Faktury bez ručního
přepisování dokladů. První cílové služby: **iDoklad a Faktura online
(fakturaonline.cz)**. Další služby přidávat přes samostatné adaptéry a společný
importní formát.

Jde o návrh Open Faktury. Dostupnost konkrétních exportů, API, polí a příloh
jednotlivých služeb ještě není ověřena; jejich podporu neslibovat před průzkumem
a testem na anonymizovaných datech.

- Průvodce: vybrat zdroj a cílovou firmu → zobrazit návod k získání dat →
  nahrát export nebo připojit ověřené API → zkontrolovat mapování a náhled →
  potvrdit import → stáhnout výsledný report.
- Předpřipravené mapování pro podporované služby, obecný CSV/XLSX import s
  mapováním sloupců jako záložní cesta. Konkrétní formáty každé služby ověřit.
- Postupný rozsah: kontakty a vydané faktury včetně položek a historických údajů;
  následně úhrady, zálohy, dobropisy, přijaté doklady, ceník, vazby a přílohy podle
  dostupnosti zdrojových dat a dokončení cílových modulů.
- Zachovat původní čísla dokladů, data, měny, částky a daňové údaje. Historické
  doklady nepřepočítávat aktuálními sazbami ani ceníkem. Rozdíly zaokrouhlení a
  kolize čísel předložit k řešení; nic tiše nepřepisovat.
- Náhled bez zápisu ukáže počty, chyby, duplicity a nepodporovaná či chybějící
  data. Nejednoznačné shody kontaktů a konflikty existujících dokladů řeší
  uživatel; pouhá shoda názvu nestačí pro automatické sloučení.
- Importní dávka eviduje zdroj, původní identifikátory a mapování na cílové
  záznamy v rámci firmy. Opakovaný import ani pokračování po výpadku nesmí
  vytvořit duplicity; změněný zdrojový záznam musí projít řešením konfliktu.
- Zobrazovat průběh a umožnit bezpečné pokračování po přerušení. Import přes
  web, API i MCP používá stejné oprávnění a aplikační validace.
- Migrace sama neodesílá faktury, upomínky ani nespouští pravidelnou fakturaci.
  Před začátkem nabídnout zálohu cílové agendy; návrat importní dávky navrhnout
  s ohledem na pozdější změny a vazby, nikoli plošným smazáním dat.
- Závěrečný report porovná počty, částky odděleně podle měn, úhrady, vazby a
  přílohy; uvede importované, přeskočené a chybové záznamy i důvody neúplnosti.

**Zařazení:** průzkum a vzorky v P00, první průvodce pro kontakty a vydané
faktury v P09 po dokončení P08. Rozšíření o úhrady a další typy dokladů podle
připravenosti adaptérů a etap P12–P17. Žádný konektor neoznačit za podporovaný
bez ověřeného vstupního vzorku a výsledného reportu.

**Podmínky dokončení:** uživatel podle průvodce přenese testovací agendu z obou
prvních cílových služeb; součty a vazby odpovídají zdroji nebo mají explicitně
popsanou odchylku. Ověřeno opakování stejného importu, pokračování po výpadku,
částečná úhrada, cizí měna, kolize čísel, chybějící kontakt/příloha a izolace
firem. Nepodporovaný obsah je viditelný před potvrzením i ve výsledném reportu.

### UX-P03-01 — jednoduchý profil pro živnostníky

Úprava P03: jeden formulář s volitelnými/pokročilými sekcemi, osobní terminologie,
přímé otevření jediného profilu, stručný přehled. Podrobnosti a akceptace v
[company-setup](specs/company-setup.md#ux-p03-01--osobní-fakturace-10-10-2026).
Pro P06 je cílovým vstupem seznam faktur s akcí Nová faktura; fakturační údaje
budou dostupné v nastavení. Rozhodnutí 11. 10. 2026: plná navigace je viditelná již nyní; neimplementované moduly jsou disabled a označené jako připravované. První editor
má podporovat jednoduchou měsíční fakturu živnostníka bez další konfigurace řad
nebo firemních rolí. Aktuální profil není hotový fakturační workspace.

## ACCOUNT-01 — Nastavení vlastního účtu (11. 10. 2026)

Implementováno: jméno, přihlašovací e-mail, změna hesla přes menu avatara;
CS/EN, vlastní select jazyka s vlajkami. Specifikace a bezpečnostní pravidla:
[account-settings.md](specs/account-settings.md). Vlastní návrh Open Faktury,
nikoli doložená přesná shoda s iDokladem. Ověření: auth PostgreSQL integrační
testy a company E2E (CS/EN, desktop/mobil).

## RE-01 — Náprava referenčního ověřování (11. 10. 2026)

Před rozšiřováním produktu používat [audit živých referencí](design/reference-audit-2026-10-11.md).
E-020 je přihlášené čtení FakturyOnline; E-021 skutečné vytvoření kontaktu,
vystavení a částečná úhrada/doplatek v syntetickém iDokladu. Screenshoty mají
číslovaný index. Implementované dílčí opravy: postupné zadání zákazníka, stavový
sloupec, řádkové PDF/kopie, souhrn v dialogu platby. Celková parita není hotová;
UX-04, UX-05, UX-06, UX-08 až UX-12 uvádějí konkrétní zbývající odchylky.

RE-01 / UI seznamu a detailu: 11. 10. 2026 implementována další iterace podle
E-021 screenshotů 04/05/06/11. Hlavičkové řazení, serverové intervaly data a
částky, velikost stránky, omezení hlavních pohledů a samostatné záložky
Doklad/Úhrady. CS/EN, URL stav, kompaktní tabulka úhrad a synchronizovaný stav
v hlavičce. Detaily a úmyslné odchylky v reference-audit-2026-10-11.md.
Přílohy, bulk akce a změny vystavených dokladů nejsou touto iterací dokončeny.

RE-01 / editor: sjednocen nový doklad, úprava návrhu a detail podle E-021
01/02/06. Sdílený souhrn, obsahové záložky, odběratel jako první, dvojice
Datumy / Moje platební údaje, kompaktní editovatelné řádky se serverovými
součty a mobilní rozložení. Ukládání a vystavení zůstávají samostatné operace.

### RE-01 — sjednocení odběratelů (11. 10. 2026)

Dokončeno: seznam, čtecí detail a samostatný editor podle nových přímých referencí
E-022 (screenshoty iDokladu 12–14). Kompaktní tabulka, hledání a velikost stránky,
společné členění detailu/editoru, dvě obsahové záložky, rozbalovací bankovní údaje,
bezpečné zrušení úprav a odkaz na faktury kontaktu. Monochromatické tokeny a CS/EN.
Čtyři kontaktní E2E scénáře desktop/mobile × CS/EN prošly nad izolovanou databází,
včetně ARES, konfliktu, archivu, stránkování, přístupnosti a profilových oprávnění.
Podrobnosti a přijaté odchylky: docs/design/reference-audit-2026-10-11.md, E-022.

RE-01, návazná oprava auditu: kompaktní horizontální formuláře kontaktů se
společným tokenem šířky labelu, shodné osy detail/editor, hlavní akce u názvu,
sekundární menu, integrované ukládání a přímá editace z tabulky. Kontaktům a
fakturám sjednocena hustota desktopové tabulky a šířka hledání. Viz závěrečné
porovnání s E-022 v design/reference-audit-2026-10-11.md a nové snímky.

### RE-02 — společný rám podle iDokladu

Implementován monochromatický rám, seskupená aktivní navigace, kompaktní desktopové
ovládací prvky a tabulky, sdílené stránkování a sjednocené sekce nastavení profilu.
Datumy/platba v editoru faktury převedeny na horizontální řádky stejné jako detail.
Výchozí přijatý rozsah nedostupných modulů zůstává zachován. Požadavek na úplné
1:1 není označen jako dokončený: další mezery a rozdíly jsou výslovně vedené
v [matici shody](design/idoklad-parity.md), podklady jsou E-021 až E-023.

### P12-01 — dodaný ceník

Implementace `tested` 11. 10. 2026: catalog/catalog_audit (011), REST/MCP příkazy,
CS/EN seznam/detail/editor, archiv/obnova, profilové role a výběr do faktury.
První prázdný řádek se výběrem nahradí; další položky se přidají. Rust převádí
net/gross podle dokumentovaných vlastních pravidel. Faktura ukládá kopii hodnot.

Ověření: 4 PostgreSQL scénáře (včetně upgradu z 010 a REST/MCP shody), 2 doménové
scénáře přesnosti/limitů, CS/EN desktop/mobile E2E ceníku a regrese faktur.
Build, TypeScript, locales, konvence, formát a server Clippy prošly. Vite stále
upozorňuje na sdílený chunk přes 500 kB. Reference E-024 pokrývá pouze UI;
numerická shoda a skladové pohyby zůstávají otevřené. Celý P12 není dokončený.

### P12-02/P12-03 — šablony, nabídky a převod

Implementace `tested` 11. 10. 2026. Migrace 012, profilové REST/MCP příkazy,
CS/EN seznamy/detail/editor, archiv/obnova, hledání, stavové filtry, stránkování.
Šablony lze vybrat při nové faktuře i nabídce nebo použít přímo z detailu.
Nabídka se čísluje CN/rok/pořadí a převede na jeden navázaný návrh faktury.
Vystavení využívá P07, automaticky promítne stav Vyfakturována. Kopie se nemění
při následné úpravě šablony. Web zůstává monochromatický a používá společné
položky, součty, datumy a výběr z ceníku.

Důkaz E-026: uložená nabídka a šablona, 2 × 1 200 Kč = 2 400 Kč, otevření
předvyplněné faktury. Referenční dokončení vystavení a opakovaný převod nejsou
ověřeny; naše transakční pravidla jsou označená ve specifikaci. Technické
ověření: 4 PostgreSQL scénáře, 4 CS/EN desktop/mobile E2E a 4 regresní E2E faktur.
PDF a odesílání nabídek doplňuje QUOTE-DELIVERY-01 níže. Zálohy, vlastní přílohy,
štítky, hromadné akce a jiné měny nejsou součástí této dodávky; celý P12 ani
úplná shoda s iDokladem hotové nejsou.

### QUOTE-DELIVERY-01 — nabídky: PDF, tisk a odesílání

Implementace `tested` v rozsahu specifikace `docs/specs/sales-documents.md`:
PDF CS/EN, náhled pro tisk, SMTP s PDF přílohou, upravitelný příjemce/předmět/text,
trvalá fronta a historie v detailu nabídky včetně uložené přílohy. Migrace 013,
REST/MCP, role a izolace aktivního profilu. Referenční důkaz E-027; SMTP fronta
je vlastní řešení, nikoli doložená interní architektura iDokladu.

Kontroly: 4 integrační scénáře delivery + 4 regresní sales, lokální SMTP capture,
CS/EN desktop/mobile E2E; česká a sedmistránková anglická PDF vizuálně zkontrolovány.
Tisk ověřen jako PDF náhled/export v prohlížeči, bez fyzické tiskárny.
Odeslání do externí schránky nebylo provedeno. Pro nasazení nastavit SMTP podle
instalačního návodu. Odkazy/tracking doplňuje QUOTE-SHARE-01 níže. Zbývají více příjemců, globální historie
napříč doklady a UI konfigurace SMTP; celý P10 tím není dokončen.

### QUOTE-SHARE-01 — veřejné nabídky a zobrazení

Doplněno k P12-03: migrace 014, veřejný aktuální náhled CS/EN, PDF/tisk bez
přihlášení, odvolatelné odkazy s expirací, e-mail odkazem, historie zobrazení
pod aktivním profilem. REST/MCP sdílejí oprávnění a příkazy. Setup vyžaduje
PUBLIC_WEB_URL. Reference a odchylky E-028, specifikace sales-documents.md.
Diskuse a identifikace čtenáře nejsou součástí tohoto kroku.

## P10-01 — faktury e-mailem s PDF a historií (11. 10. 2026)

Implementována první část P10: detail vystavené faktury → upravitelný e-mail
s neměnnou PDF přílohou → historie pokusů a stažení historické přílohy.
Backend/DB migrace 015, REST/MCP, CS/EN, monochromatický společný dialog s nabídkami.
Specifikace: specs/invoice-delivery.md; reference E-029. Fronta, ochrana proti
opakování stejného požadavku, role a aktivní profil; selhání nezmění vystavení/úhrady.

Celý P10 zůstává rozpracovaný: SMTP nastavení/test v UI, obecné souborové úložiště,
uživatelské přílohy a další exportní možnosti nejsou tímto krokem dokončené.
Více příjemců, hromadné odeslání a veřejné odkazy faktur jsou další rozsah.

## P10-02 — SMTP nastavení v UI (12. 10. 2026)

Implementace SMTP-SETUP-01: menu avatara správce instalace, společná konfigurace,
TLS/STARTTLS, šifrované heslo, ochrana souběhu, testovací zpráva s idempotencí a
limitem, CS/EN, REST/MCP. Změny převezme fronta faktur/nabídek bez restartu.
Specifikace specs/smtp-settings.md. Jde o vlastní self-hosted administrační UX.
Obecné souborové přílohy a další exporty P10 tím nejsou dokončené.

## P10-03 — vlastní přílohy (12. 10. 2026)

ATTACHMENTS-01: upload/drag-and-drop, scoped stažení/odstranění, výběr při odeslání
faktury nebo nabídky, neměnná historie souborů, REST/MCP, CS/EN a sdílený panel.
Migrace 017; soubory jsou v PostgreSQL, zálohované spolu s DB. Důkaz E-030,
specifikace specs/document-attachments.md. Tím je dokončen požadovaný souborový
rozsah P10; základní CSV/XLSX exporty seznamů a kontaktů z širšího plánu P10
zůstávají samostatnou navazující částí, nejsou označené jako hotové.

## P10-04 — export seznamů (12. 10. 2026)

Implementace `tested` v uvedeném rozsahu; [ověření](design/register-export-validation.md).
EXPORT-01: faktury a odběratelé do CSV/XLSX, výběr sloupců, aplikované filtry a
řazení, všechny stránky do limitu 10 000 řádků. CS/EN, profilová oprávnění,
společný REST/MCP příkaz; bez migrace. Specifikace [register-export.md](specs/register-export.md),
reference E-031 s uvedenými odchylkami. Nejde o úplnou zálohu, účetní export ani
ověřený importní adaptér FakturyOnline. Následuje P13-01 — zálohové faktury.
Průběžný přehled všech modulů: [checklist funkcí](FEATURE_CHECKLIST.md).

## P13-01 — zálohové faktury (12. 10. 2026)

ADVANCE-01: samostatný seznam/editor/detail, vlastní řada ZF, PDF CS/EN,
částečné/plné ruční úhrady a storna, profilová oprávnění, REST/MCP, migrace 018.
[Specifikace a odchylky](specs/advance-invoices.md), reference E-032.
[Výsledky ověření](design/advance-validation.md).
Původní rozsah P13-01 nezahrnoval daňový doklad k platbě (doplněný níže v P13-02) ani vyúčtování;
Vyúčtovat je disabled a API konverzi zálohy odmítá. E-mail a přílohy záloh
nejsou automaticky dostupné přes adaptéry nabídek/faktur.

## P13-02 — daňové doklady k přijaté platbě (12. 10. 2026)

TAX-RECEIPT-01: náhled a vystavení z konkrétní úhrady, vlastní řada DDPP,
neměnný snapshot/PDF, seznam a detail CS/EN, REST/MCP a migrace 019.
Blokace storna po vystavení, transakce/retry, role a izolace profilu.
[Specifikace a odchylky](specs/advance-tax-receipts.md), reference E-033.
[Výsledky ověření](design/tax-receipt-validation.md).
E-mail, automatizace a opravy dokladů zůstávají v checklistu. Další krok
P13-03: vyúčtování s odpočtem uložených daňových rozpisů.

## P13-03 — vyúčtování záloh (12. 10. 2026)

ADVANCE-SETTLEMENT-01 připravuje upravitelnou fakturu z jedné nebo více plně
uhrazených záloh stejného odběratele. Uložené daňové rozpisy DDPP se odečítají
beze změny, peněžní odpočet vychází z přijatých plateb. Rezervace brání dvojímu
použití a změnám zdrojových úhrad; návrh lze uvolnit, vystavenou vazbu nikoli.
Finální faktura používá standardní číslování, PDF, doručení a úhrady doplatku.
CS/EN editor, detail, výběr více záloh a propojení dokladů zachovávají náš brand.

Migrace 020, REST/MCP kontrakt a testy: [specifikace](specs/advance-settlement.md).
Důkaz E-034 vychází z veřejné nápovědy; přesná numerická a vizuální shoda není
potvrzená. Vlastní pravidla: max. 20 záloh, CZK, přesně zaplacené zálohy,
žádné záporné vyúčtování. Dobropisy, přeplatky, částečné vyúčtování a historické
sazby zůstávají otevřené, dodávka je neoznačuje za hotové.

## P13-04 — dobropisy (12. 10. 2026)

Implementováno vytvoření z vydané faktury, důvod a částečné opravy řádků,
číslování DB, uložené CS/EN PDF, zápočet, evidence vratek a storno událostí.
Oprava vyúčtované zálohy vychází z uložených částek plnění a zachovává DDPP.
Migrace 021, společné REST/MCP příkazy, revize/idempotence a profilová oprávnění.
Monochromatický seznam, editor/detail a vazba z původní faktury v CS/EN.
[Specifikace a hranice](specs/credit-notes.md), reference E-035 (veřejná nápověda,
ne nové živé pozorování); numerická shoda s iDokladem není potvrzená.
E-mail/přílohy, přímé opravy DDPP, vrubopisy a cizí měny zůstávají rozšířením.

## P14-01 — pravidelné faktury (12. 10. 2026)

Implementované běžné CZK faktury: vzor nově/z faktury, den/týden/měsíc/rok,
interval, konec měsíce, ukončení datem/počtem, náhled 12 termínů, uložené pásmo.
Pauza/aktivace, atomické generování a volitelné vystavení, samostatné zařazení
PDF e-mailu, historie a bezpečné doplnění po výpadku. Backendový worker běží
s API serverem; zavřený prohlížeč mu nevadí. CS/EN, monochromatické sdílené UI.
Migrace 022, recurring REST/MCP kontrakt a PostgreSQL/Playwright scénáře.
[Specifikace a limity](specs/recurring-invoices.md), reference E-036.
Přesná shoda s živou aplikací iDoklad není doložená; pravidelné zálohy,
více příjemců, textové proměnné a více číselných řad jsou následná rozšíření.

## P14-02 — upomínky a poděkování (13. 10. 2026)

Ruční náhled/příjemce/text, odeslání s PDF a rozlišená historie. Automatizace
je standardně vypnutá, povoluje ji vlastník profilu; první odstup, interval,
částkový limit a maximálně 10 upomínek. Poděkování po nové úplné ruční úhradě,
jedno automatické na fakturu. Pauza konkrétní faktury, revizní ochrana,
trvalé idempotentní úlohy a opětovná kontrola před SMTP. Migrace 023,
REST/MCP, CS/EN a sdílené monochromatické UI.
[Specifikace a odchylky](specs/invoice-communication.md), E-037. Bankovní
párování, více příjemců, pravidla kontaktu a veřejné odkazy nejsou vydávány
za hotovou funkční shodu s iDokladem.

## P16-01 — banka (13. 10. 2026)

Migrace 027, společné REST/MCP příkazy a CS/EN UI. Jeden pohyb → jedna úhrada; nová platba či propojení existující. Odpojení nově vytvořenou úhradu stornuje, původní zachová. Veřejná dokumentace E-041, interní transakční model a CSV jsou vlastní návrh. Není potvrzena shoda přihlášené bankovní obrazovky ani nativních bankovních exportů. Navazující hranice jsou ve specifikaci.

## P16-02 — výpisy a návrhy shod (13. 10. 2026)

Migrace 028, camt.053 / Fio JSON / MONETA XML / RB ABO, společný atomický import,
profilová izolace a návrhy podle VS a částky. UI vyžaduje potvrzení. E-042
dokládá veřejné formáty, nikoli skutečné exportní varianty. Implementace
adaptérů a technické testy nenahrazují chybějící anonymizované originály; toto
ověření zůstává otevřené. [Specifikace](specs/bank-statements.md).

## P16-03 — hromadné potvrzení bankovních shod (14. 10. 2026)

Implementováno: náhled nespárovaných pohybů aktuální stránky/filtrů (max. 25),
přesné jednoznačné shody, důvody přeskočení, kontrola konkurenčních pohybů i mimo
výběr, revize náhledu, atomické potvrzení a idempotentní audit přes společné
příkazy. REST/MCP, CS/EN. Bez nové migrace; využívá bank_command/bank_match.
[Specifikace](specs/bank-batch-matching.md), veřejná dokumentace a odchylky E-043.
Nejde o trvale běžící automatické párování bankovních notifikací.

Další samostatná část P16-04: pokladní evidence (příjmové/výdajové doklady a vazby
na hotovostní úhrady); pravidla nejprve doložit referencí. Ověření reálných exportů
P16-02, notifikace/PSD2, přeplatky a dělení pohybů nadále zůstávají otevřené.

## P16-04 — pokladní evidence (14. 10. 2026)

Implementováno: profilové CZK pokladny s počátečním stavem a datem, archivace,
PP/VP roční číslování, ruční příjmy/výdaje, neměnné doklady se stornem a důvodem,
saldo za období přes všechny stránky. Párování používá společné platební
příkazy; existující hotovostní úhrady včetně účtenek nepřidává podruhé.
Odpojení stornuje jen vlastní úhradu. REST/MCP, CS/EN, migrace 029.
[Specifikace](specs/cash-register.md), reference E-044.

P16-05 naváže tiskem/PDF a automatickými pokladními doklady při hotovostních
úhradách mimo pokladnu. Samostatně zbývají prodejky, daňové položky pokladních
dokladů a další dříve evidované limity banky. Úplná shoda s iDokladem není hotová.

## P16-05 — automatické pokladní doklady a PDF (14. 10. 2026)

Implementováno: volba hlavní pokladny v profilu (implicitně vypnuto), atomické
PP/VP při hotovostních úhradách vydaných/přijatých faktur a účtenek, storno
a nahrazení při opravách zdroje. Ruční párování nevytváří duplicitní doklady.
PDF, stažení a náhled tisku v CS/EN používají uloženého vystavitele a cache
podle revize/jazyka. REST/MCP sdílí operace a oprávnění. Migrace 030.
[Specifikace](specs/cash-register.md), [ověření](design/cash-validation.md), E-045.

Zbývají prodejky, DPH samostatných plnění, automatika záloh/dobropisů,
částka slovy, konfigurovatelné sestavy a cizí měny. Starší úhrady se zpětně
nepřevádějí. Historické doklady 029 zachytí vystavitele až při migraci.

## P16-06 — samostatné pokladní plnění s DPH (14. 10. 2026)

Jedna položka s cenou net/gross, sazbou 0/12/21, datem plnění a serverovým
náhledem. Kalkulátor of-czk-v1; uložený rozpis je neměnný, součást detailu
a PDF v CS/EN. Samostatné plnění nelze spárovat jako další úhradu faktury.
Číslování, audit a storno používají společné cash příkazy, REST/MCP; filtr
tax a označení S DPH. Migrace 031. [Rozsah](specs/cash-register.md), E-046.

Následující P16-07: prodejky (prodej na místě, položky, hotovost/karta,
nepovinný odběratel, PDF) a explicitní oddělení od následného souhrnného
zaúčtování hotovosti do pokladny a karet do interních dokladů. Automatické
zaúčtování jako obyčejná invoice_payment by neodpovídalo referenci. Zbývají
členění DPH, daňové výkazy, terminály, sklad a další již uvedené mezery.

## P16-10 a dílčí P16-16 (14. 10. 2026)

Bankovní počáteční stavy a saldo za období jsou implementované: přesné součty
nad všemi pohyby, revize a důvod změny, REST/MCP a CS/EN UI. Migrace 032
neodhaduje historický počátek; bez nastavení vrací neznámý stav.
[Specifikace](specs/bank-balances.md). Pokladní PDF nově uvádí částku slovy
v češtině a angličtině; existující uložené PDF se nemění.

Ověření: 15 bankovních a 15 pokladních DB testů, REST/MCP shoda, upgrade
031→032, 4 bankovní E2E scénáře (CS/EN × desktop/mobile), kontrola projektu.
Celá P16 zůstává otevřená podle [úplného rozpisu](P16_COMPLETION.md).

## P16-15 — vlastní konfigurace bankovního napojení (14. 10. 2026)

Rozhodnutí uživatele: připojení si nastavuje každý uživatel/provozovatel sám,
se svými přístupy. Návrh má pokrýt více bank, nikoli pouze Air Bank; konkrétní
API/poskytovatel/notifikace se nabízí podle implementovaného adaptéru.
Součástí je průvodce CS/EN, test spojení, diagnostika synchronizace a odpojení.
Osobní poskytovatel ani schránka zadavatele nejsou podmínkou vývoje.
[Úplná akceptace a hranice ověření](P16_COMPLETION.md#vlastní-bankovní-napojení-p16-15).
Implementace zůstává otevřená; nejde o potvrzení podpory všech bank.

### P16-15a — dodané nastavení a synchronizace (14. 10. 2026)

Implementováno přes backend/DB: vlastní přístupy, revize, AES-GCM, audit bez
tokenů, test spojení bez importu, ruční synchronizace s deduplikací a stavem,
odpojení bez smazání pohybů. Web CS/EN používá společný REST/MCP port.
Přímý adaptér Fio a vlastní HTTPS gateway podle dokumentovaného kontraktu.
Nejde o automaticky fungující připojení všech bank ani o implementaci
e-mailových notifikací; žádné živé přístupy nebyly testované.
Migrace 033, specifikace [bank-connections.md](specs/bank-connections.md),
DB testy `tests/bank/connection.rs`, transportní testy a bankovní CS/EN E2E.

### P16-15b — automatická synchronizace (14. 10. 2026)

Opt-in po účtech: 15 minut, hodina, 6 hodin, den. Worker API používá společný
příkaz synchronizace a respektuje práva aktéra aktivace i jejich odebrání
během síťového požadavku. Stav obsahuje příští termín a chybu plánovače.
Migrace 034 ponechává automatiku všude vypnutou. CS/EN a REST/MCP, bez
automatického párování. [Pravidla](specs/bank-connections.md#p16-15b--volitelná-automatická-synchronizace).

### P16-14 — průběžné automatické párování (14. 10. 2026)

Dodáno samostatné opt-in nastavení účtu, aktivující uživatel, worker nad
společným evaluátorem přesných shod, audit a stav poslední dávky v CS/EN.
Migrace 035; výchozí stav vypnutý. Dávka maximálně 25 pohybů, transakční
ávrat celé dávky, kurzor přes všechny stránky a kontrola konkurujících pohybů.
Ručně odpojené pohyby automatika znovu nepáruje. Ztráta oprávnění automatiku
vypne. Pravidla jsou vlastní konzervativní návrh, nikoli potvrzená úplná
shoda s iDokladem. [Specifikace](specs/bank-auto-matching.md).

### P16-11 — rozdělení bankovního pohybu (14. 10. 2026)

Dodána atomická dávka 1–25 přiřazení, přesné částky a nerozdělený přeplatek,
navazující doplnění a společné odpojení. Stornují se jen úhrady vytvořené
párováním, dříve existující platby/účtenky zůstávají. Migrace 036, společný
REST/MCP příkaz split a split_candidates, CS/EN detail i filtr částečných
pohybů. Automatické párování částečné pohyby neřeší. Vlastní rozšíření dle
E-049, nikoli doložená shoda více vazeb iDokladu.
[Specifikace](specs/bank-split-matching.md), testy bank/split.rs a bank.spec.ts.

P19-01c (17. 10. 2026): přehledy podporují kalendářní kvartály a roky, víceletý rozsah a společné srovnání/kumulaci. Další neblokovanou částí P19 jsou bilance a přehledy dlužníků/odběratelů; rozsah viz P16_P20_COMPLETION.md.

P19-01d (17. 10. 2026): bilance, největší partneři a rozpis prodlení dodané nad existujícími doklady/úhradami. Další práce: evidence DPH/obratu, měny a varianty widgetů. Viz specs/dashboard-finance.md.

P19-01e (17. 10. 2026): měsíční/kvartální evidence uložené DPH, orientační
roční obrat a stránkované zdroje s odkazy, REST/MCP a CS/EN. DDPP samostatně,
vyúčtování s uloženým odpočtem. Bez nové migrace. Viz specs/tax-dashboard.md.
Zákonný obrat a přiznání zůstávají otevřené do doplnění daňové klasifikace,
stejně jako cizí měny, samostatná pokladní plnění a widgetové varianty.

P19-01f (17. 10. 2026): samostatná pokladní plnění v tax_report, bez migrace.
Specifikace docs/specs/tax-dashboard.md; test:period-report a dashboard.spec.ts.
Směr dle cash_movement.amount, částky z tax.amounts, datum z JSON cesty tax.tax_on ve snapshotu;
storna, automatika a úhrady bez tax snapshotu jsou vyloučené.

P19-01g: měna a uložený kurz běžných vydaných/přijatých faktur, ruční bankovní úhrady v měně dokladu a přepočty přehledů do CZK. Rozsah a zbývající měnové moduly: [specifikace](specs/report-currencies.md).

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


## P16-13h — zahraniční IBAN bankovních účtů (18. 10. 2026)

Bankovní evidence přijímá zahraniční IBAN s normalizací, kontrolou země,
struktury a checksumu přes verzovaný validátor. Bez migrace; firemní unikátnost,
immutabilita po pohybu, shoda účtu/měny importu a REST/MCP zůstávají společné.
UI a validace CS/EN. [Specifikace a akceptace](specs/bank-currencies.md#p16-13h--zahraniční-iban-bankovních-účtů).
Navazují zahraniční IBAN dokladů/QR, automatika záloh/vratek a jejich různé měny.

### Dodávka 18. 10. 2026 — P16-13i, P16-08b, P16-14b

- [x] Zahraniční IBAN profilů a vystavených dokladů; historické PDF beze změny. QR zůstává CZ/CZK.
- [x] PDF/tisk souhrnného zaúčtování prodejek v CS/EN, snapshot vystavitele, cache po revizích, označení storna (migrace 056).
- [x] Samostatný opt-in pro automatické párování záloh a vratek ve stejné měně, existující úhrady bez duplicity (migrace 057).

Specifikace: [měny](specs/bank-currencies.md), [prodejky](specs/sales-receipts.md),
[automatika](specs/bank-auto-matching.md). Neuzavírá celé P16–P20. Otevřené
zůstávají rozdílné měny záloh/vratek, notifikace bank, příjem e-mailů a
vytěžování, S4/S5, účetní/daňové exporty a zbývající funkce P20.

Ověření dodávky: `pnpm check`, `pnpm build:web`, serverový build a 27 doménových
testů prošly. Izolované DB: company 6, invoice 9, retail 6; bank 65 scénářů
a rozšířená sada native_treasury 8 (překrývá část bank), následná kontrola
auditu vratky; regrese advance 3, cash 16 a credit_note 6. Chromium bank
a cash CS/EN desktop/mobile: všech 8 prošlo po opravě cílové URL testovacího
stahování PDF. PDF CS/EN renderované přes Poppler a vizuálně zkontrolované.
Lokální upgrade po záloze: migrace 056/057 úspěšné, API health OK. Nejde
o ověření skutečných bank, tiskárny či úplné shody iDokladu.

### Integrační dodávka P16–P20 (18. 10. 2026)

Aktuální dokončení a otevřené části uvádí [samostatný přehled](P16_P20_COMPLETION.md).
P16 se znovu neimplementuje. Paralelní práce doplňuje zbývající návaznosti:
kurzové párování záloh/vratek, příjem dokladů a notifikační brány, ISDOC/registry,
export evidence DPH, automatické návrhy kurzů a produktové funkce (členství,
štítky, výstupy, vzhled, exportní zámky, vratný koš návrhů a nápověda).

Rozhodnutí uživatele: neověřené Money S4/S5 a další účetní formáty může
provozovatel přizpůsobit [vlastním adaptérem](INTEGRATION_ADAPTERS.md).
Nativní kompatibilita těchto variant se bez důkazů nedeklaruje. Toto rozhodnutí
nenahrazuje ověření daňových pravidel ani podpisy/instalační testy desktopu.
