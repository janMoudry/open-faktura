# P16-13c — cizoměnové bankovní účty a importy

17. 10. 2026. Navazuje na E-042 (dokumentované měnové údaje výpisů),
[bankovní importy](bank-statements.md) a [saldo](bank-balances.md).
Jde o vlastní rozšíření Open Faktury, nikoli ověřenou numerickou shodu s iDokladem.

Účet podporuje CZK, EUR, USD, GBP, CHF, PLN, HUF, JPY, SEK, NOK, DKK, CAD, AUD.
Měna je po založení neměnná (i bez pohybů kvůli počátečnímu saldu a synchronizaci).
IBAN zůstává v profilu jedinečný. Zachováváme dosavadní validaci českého IBAN;
jde o cizoměnové účty českých bank, zahraniční IBAN navazují samostatně. Účet není multiměnová peněženka.
Částky pohybů a salda jsou přesné desetinné hodnoty v měně účtu, max. dvě
místa, stejně jako dosavadní bankovní kontrakt; žádné kurzové přepočty.
Nekombinovat salda různých účtů/měn do jediného součtu.

CSV, camt.053, Fio JSON a MONETA XML kontrolují měnu proti účtu.
camt/Fio kontrolují také shodu měny hlavičky a zaúčtované částky.
RB ABO zůstává pouze pro původní _CZK.gpc; zahraniční varianty nejsou doložené.
Import je atomický, jedna nesprávná měna zablokuje celý soubor. Deduplikace
zůstává podle profilu/účtu/externího ID. Synchronizace používá stejný parser
včetně kontroly měny prázdného výpisu. Syntetické fixtures nejsou exporty bank.

Migrace 054 rozšiřuje CHECK účtu a přidává generovanou měnu pohybu s firemním
FK na měnu účtu. Historické částky zůstávají CZK. Před upgradem záloha a zastavení
starého API (SELECT *). REST/MCP mají společnou validaci, saldo vrací currency.

Párování z cizoměnového účtu (ruční, dělené i automatické) je zatím odmítnuté;
UI vysvětlí omezení. Stávající párování CZK pohybu s cizoměnovou fakturou se nemění.
Navazuje párování v měně účtu a následně rozdílné měny/kurzy, zálohy a vratky.

Akceptace: EUR účet, import příjmů/výdajů, správné saldo EUR, opakování bez
kopií, nesprávná měna bez zápisu, neměnná měna účtu, oddělení profilů,
zamítnutí párování, CS/EN zobrazení a čistá/upgrade DB.

## Ověření 17. 10. 2026

- `pnpm test:bank`: 47 testů nad izolovaným PostgreSQL, včetně currency_tests
  a upgradu 053 → 054. Po zpřesnění rozlišení měn v dávce znovu 6 batch regresí.
- `cargo test -p faktura-infrastructure bank_statement --lib`: 5 parser testů.
- `pnpm check`, build webu a serveru: úspěch; web hlásí dosavadní velký chunk.
- Bank E2E: CS/EN desktop i mobile. Poslední EN mobile průchod zopakován
  samostatně po ztrátě otevřeného dialogu během průběžné změny UI; opakování prošlo.
  Testovací EUR účet používá syntetický český IBAN. Kontrola screenshotů CS
  desktop/mobile; soubory `docs/design/bank-currencies-{cs,en}-{desktop,mobile}.png`.
- Lokální dev DB zálohovaná před migrací, API restartované, migrace 054 a health OK.

Žádné skutečné bankovní přístupy ani živé exporty nebyly testovány.

## P16-13d — ruční párování ve shodné měně

17. 10. 2026, vlastní rozšíření nad E-068/E-067; bez nového pozorování iDokladu.
Příjem lze připojit k vydané faktuře, výdaj k běžné přijaté faktuře ve stejné
měně jako účet/pohyb. Kandidáti i zápis kontrolují přesnou shodu měny.
Ruční výběr, návrhy podle VS a rozdělení na více faktur sdílejí příkazy banky.
Úhrada vzniká v původní měně bez převodu; již evidovanou aktivní převodní
úhradu lze připojit pouze celou a právě jednou. Odpojení stornuje jen úhrady
vytvořené párováním. Existující úhrady zachová. Revize, UUID, firemní členství
ani transakční ochrany se nemění.

bank_match.amount je částka v měně pohybu. foreign_settlement=NULL nyní znamená
vazbu ve shodné měně, nikoli nutně CZK. Rozdílový report stále čte pouze
explicitní CZK/foreign snapshoty. Stejná měna neznamená účetně nulový kurzový
rozdíl; bez bankovního účetního kurzu žádný rozdíl neodvozujeme.
Bez migrace a změny tvaru příkazů; REST/MCP používají stejné příkazy.

Automatické a dávkové párování non-CZK zatím nepovolujeme. Zálohy/dobropisy
na non-CZK účtech a různé měny účet/doklad zůstávají odmítnuté. UI u ručního
výběru vysvětlí podporu shodné měny a formátuje kandidáty i rozdělení nativně.
Akceptace: příchozí/odchozí platby, existující plná úhrada, částečné rozdělení,
odpojení, souběh/opakování, jiné měny/profily/role, REST/MCP a CS/EN UI.

### Ověření P16-13d

17. 10. 2026: `pnpm test:bank` 51 testů, `pnpm test:payments` 6,
`pnpm test:received` 10 nad izolovaným PostgreSQL; vše prošlo.
Nové scénáře v `crates/server/tests/bank/native_matching.rs` zahrnují REST/MCP,
nové i existující úhrady v obou směrech, rozdělení, souběh a profilová práva.
`pnpm check`, build webu/serveru a bank E2E ve všech 4 kombinacích CS/EN
Desktop Chrome/emulovaný iPhone 13 prošly. Screenshoty
`docs/design/bank-native-matching-{cs,en}-{desktop,mobile}.png`; CS desktop/mobile
vizuálně zkontrolovány. Lokální API restartované, health OK. Bez reálných bankovních
přístupů, živých referenčních exportů a potvrzení shody s iDokladem.

## P16-13e — dávka a automatika ve shodné měně

18. 10. 2026, vlastní rozšíření pravidel E-068/E-067, bez nového pozorování iDokladu.
Dávkový náhled a opt-in worker nově používají i non-CZK účty. Kandidát musí
mít stejnou měnu, normalizovaný VS a přesnou částku. Konkurenční pohyby se
hledají v celém profilu, ale pouze ve stejné měně. EUR 100 proto neblokuje
CZK 100; dvě EUR 100 se stejným VS automatickou volbu blokují i mezi účty.
Existující plná úhrada má přednost před vytvořením nové. Bez přepočtů kurzů.
Dávka nad různými měnami nesčítá částky; uvádí měnu u jednotlivých pohybů.

Zůstává náhled/fingerprint, atomický zápis, UUID, kontrola práv aktéra,
konzervativní pravidla pro částečné/nejasné shody a zákaz automatického opětovného
připojení ručně odpojených plateb. Migrace ani nové endpointy nejsou potřeba.
Přepínač je dostupný v CS/EN, výchozí stav vypnuto; není závislý na API tokenu.
Zálohy/vratky z non-CZK účtů a rozdílné měny účet/doklad nadále navazují.
Akceptace: směs CZK/EUR se stejným VS a částkou, konkurenční EUR pohyb mimo
výběr, existující úhrady bez duplicity, souběh workerů, odpojení a ztráta práv.

### Ověření P16-13e

18. 10. 2026: `pnpm test:bank` 54 testů nad izolovaným PostgreSQL, včetně
nového native_automation (smíšená měna, konkurence mimo výběr, stale fingerprint,
existující úhrada, souběh workerů, ruční odpojení a odebrání práv).
`pnpm check`, build serveru/webu a 4 bank E2E kombinace CS/EN desktop/mobile prošly.
E2E potvrzuje EUR dávku a zapnutí, skutečné provedení i vypnutí EUR automatiky
bez bankovních tokenů; používá pouze izolovanou DB. První E2E pokus opravil
klikání testu na skrytý input na kliknutí viditelného popisku přepínače.
Screenshoty `docs/design/bank-native-automation-{cs,en}-{desktop,mobile}.png`.
Lokální API restartované a health OK, bez migrace. Shoda s živými bankami ani
přihlášeným iDokladem nebyla tímto ověřena.

## P16-13f — zálohy a vratky ve shodné měně

18. 10. 2026, vlastní rozšíření společné platební vrstvy, bez nového pozorování
iDokladu. Ruční i dělené párování banky zahrnuje zálohy (příjem) a vratky
dobropisů (výdaj) ve shodné měně účtu. Měna se filtruje před limitem kandidátů
a znovu ověřuje při zápisu. Existující převodní platba se připojuje celá,
bez duplicity; odpojení ji zachová. Vytvořenou platbu odpojení stornuje,
pokud tomu nebrání DDPP nebo rezervace vyúčtování; konflikt vrací celou transakci.
Pokladna zůstává CZK. Automatika nadále páruje pouze běžné faktury.
Bez migrace, nových endpointů či kurzových přepočtů. CS/EN vysvětlení výběru
zahrnuje všechny ručně podporované typy. Různé měny účet/doklad nejsou podporované.
Akceptace: nové i existující platby, opakování, odpojení, jiné měny/profily,
a atomická ochrana rozdělení s DDPP.

### Ověření P16-13f

18. 10. 2026: `pnpm test:bank` — 58 testů nad izolovaným PostgreSQL,
včetně čtyř nových scénářů native_treasury (EUR příjem i vratka, existující
platby, opakování, profil, jiná měna a atomické odpojení s DDPP).
`pnpm check`, build serveru a webu prošly; web nadále hlásí velký chunk.
`pnpm exec playwright test tests/e2e/bank.spec.ts --workers=1` — 4 průchody
CS/EN desktop/mobile, skutečné spárování EUR zálohy a odkaz na její detail.
Screenshoty `docs/design/bank-native-treasury-{cs,en}-{desktop,mobile}.png`;
CS desktop/mobile vizuálně zkontrolovány. Lokální API běží, health OK.
Testy používají syntetická data; nepředstavují nově ověřenou shodu s iDokladem.
Regrese `cargo test --locked -p faktura-server --test cash --test advance
--test credit_note --test tax_receipt --test advance_settlement -- --ignored
--test-threads=1` s testovacím DATABASE_URL: všech 37 DB testů prošlo.

P16-13g (18. 10. 2026) doplňuje ruční párování různých měn účtu a běžné
vydané/přijaté faktury, včetně CZK faktur. Vyžaduje explicitní bankovní kurz
do CZK, uchovává částky a ocenění obou stran. Migrace 055; podrobnosti
v [bank-foreign-matching.md](bank-foreign-matching.md#p16-13g--rozdílné-měny-z-cizoměnového-účtu).
Rozdílné měny záloh/vratek, jejich automatika a zahraniční IBAN stále zbývají.
