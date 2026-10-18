# P16-13a / P19-01k — CZK pohyb a cizoměnová faktura

Vlastní návrh Open Faktury, 17. 10. 2026; reference E-067. Rozsah:
ruční přiřazení CZK příjmu k vydané nebo CZK výdaje k přijaté běžné
cizoměnové faktuře. Nejde o celé cizoměnové bankovnictví.

## Příkaz a výpočty

foreign_candidates hledá nejvýše 50 faktur podle čísla/jména; filtr měny
probíhá před limitem. Nabízí jen vystavené/necancelované běžné faktury,
kladný zůstatek a datum vystavení nejpozději v den pohybu. Archivovaný
účet se nepáruje. Zálohy, dobropisy a účtenky tento příkaz nenabízí.

foreign_match obsahuje revize pohybu a úhrad/dokladu, cílový druh a ID,
bank_amount (CZK) a document_amount (měna faktury). Obě jsou kladné
desetinné částky nejvýše se dvěma místy a limitem 999999999999.99.
Bankovní částka nepřekročí nerozdělený pohyb; dokumentová částka nepřekročí
zůstatek faktury. Kurz ani jednu z částek neodhadujeme. Poplatky nejsou úhrada.

Vznikne nová bankovní úhrada společným platebním příkazem. Existující
ruční úhrady tento formulář nepřipojuje; UI to uvádí. Nezaměnit se způsobem
připojení existující CZK úhrady. Zápis vazby, platby, revize a auditu je
atomický, se stávajícím profilovým zámkem, UUID a fingerprintem vstupu.

book_value_czk = round(document_amount × uložený rate / units, 2),
od poloviny od nuly, Decimal. Pro příjem difference_czk =
bank_amount − book_value_czk; pro výdaj opačně. Kladné číslo znamená
zisk, záporné ztrátu. Jednotlivé alokace se zaokrouhlují zvlášť;
rozdíl není účetní zápis ani vyrovnání haléřového zbytku.
Příklad: 100 EUR kurzem 25, bankovní příjem 2490 CZK → úhrada 100 EUR,
hodnota 2500 CZK a rozdíl −10 CZK. Výdaj 2490 CZK → rozdíl +10 CZK.

Vazba uchovává snapshot měny, uhrazené částky, původního kurzu, CZK hodnoty
a rozdílu. bank_match.amount nadále znamená CZK, takže nerozdělená část
pohybu i bankovní saldo zůstávají správně. Cizoměnová úhrada ovlivní
zůstatek dokladu v jeho měně. Přehledy dokladů zachovávají dosavadní
přepočet kurzem dokladu; rozdíl zobrazujeme u bankovní vazby, nezahrnujeme
jej zatím do souhrnného výkazu kurzových zisků/ztrát.

Odpojení stornuje vytvořenou platbu ve stejné transakci, zachová snapshot
v historii. Oprava = odpojení a nové přiřazení. Automatické párování stále
vybírá jen CZK doklady a měnové částky nikdy neodhaduje.

## Data, práva a upgrade

Migrace 052 přidává nullable JSONB foreign_settlement k bank_match.
NULL znamená dosavadní CZK vazbu, historie se nedopočítává. Legacy odpovědi
idempotence bez pole se deserializují jako None. Před upgradem zastavit
staré API a zálohovat DB; migrace je aditivní.

REST bank_foreign_candidates a bank_execute/action=foreign_match, MCP
stejné názvy a společný BankStore. Čtení vyžaduje členství, zápis owner/editor.
Cizí profil/doklad nenabídne ani nespáruje. Směr, měnu, data i revize
ověřuje backend, ne klient. CS/EN sdílené komponenty a tokeny.

Akceptace: příjem/výdaj a znaménko rozdílu, částečné částky, zbytek
pohybu, opakování/souběh, chybná revize/částka/směr/role/profil,
odpojení a historie, upgrade051, REST/MCP a CS/EN desktop/mobile.

Zbývá připojení existujících cizoměnových úhrad, cizoměnové účty/importy,
zálohy a vratky, kurzy úhrady mimo banku, souhrnný výkaz kurzových rozdílů
a rozdílné kurzy DDPP/vyúčtování. Neprezentovat P16 ani P19 jako dokončené.

## Ověření 17. 10. 2026

- Bankovní sada: 40 scénářů prošlo při společném běhu; nový upgrade051
  nejprve odhalil použití starých připravených SQL dotazů po ALTER TABLE.
  Po doplnění obnovy poolu podle instalačního postupu prošel i tento test.
  Celkem ověřeno 41 bankovních scénářů včetně REST/MCP, rolí, replay a storna.
- Regrese payment 6/6, period_report 7/7, received 10/10 v izolované PostgreSQL.
- bank.spec.ts 4/4: CS/EN, Chromium desktop/mobile; skutečné vytvoření
  a vystavení EUR faktury, spárování CZK pohybu, správný zůstatek a reload
  uloženého rozdílu. Syntetické screenshoty bank-foreign-* v docs/design;
  vizuálně kontrolován český desktop a anglický mobil.
- Prošly pnpm check, následná kontrola typů/lokalizací/formátu po úpravě
  klientské validace, webový i serverový build. Stávající upozornění na
  velikost některých webových chunků zůstává.
- Lokální vývojová DB zálohována, staré API zastaveno, migrace 052 aplikována;
  nový proces vrací ready. Žádné skutečné bankovní přístupy ani externí
  zákaznické zprávy nebyly použity.

## P16-13b / P19-01l — Připojení evidované úhrady

17. 10. 2026, rozšíření vlastních pravidel E-067. Nahrazuje výše uvedené
omezení „existující úhrady tento formulář nepřipojuje“ pro běžné vydané
a přijaté faktury. Zálohy, dobropisy a účtenky nadále nejsou v rozsahu.

foreign_candidates nově nabízí i nestornované převodní úhrady v měně
dokladu, pokud nemají aktivní bankovní vazbu, včetně plně uhrazených faktur.
payment_id/payment_amount jsou nullable: NULL značí novou úhradu,
jinak jde o konkrétní existující platbu. UI obě možnosti označuje,
existující částku předvyplní a zamkne. Bankovní CZK alokaci zadá uživatel.

foreign_match má nepovinné payment_id. Při jeho zadání ověří backend
firemní a dokumentovou příslušnost, stav/metodu platby, aktuální revizi
a absenci jiné aktivní bankovní vazby. document_amount musí přesně
odpovídat celé původní úhradě, nikoli zůstatku faktury. Původní datum
platby se zachová i při rozdílném datu bankovního pohybu; datum není
podmínkou automatické shody (žádná automatika se zde neprovádí).

Připojení nezapisuje novou úhradu a nemění její revizi/částku/datum/poznámku.
bank_match.created_payment=false. Odpojení zachová původní platbu;
stornuje jen platby vytvořené samotným párováním. Dokud je platba připojena,
opravuje se až po odpojení. Znovupřipojení je možné po odpojení,
nikdy současně ke dvěma pohybům. Ochranu zajišťuje transakční profilový
zámek, opětovná validace a existující unikátní index bank_match na platbu.

Migrace 053 upravuje pouze CHECK bank_match_foreign_shape tak, aby povolil
snapshot i pro created_payment=false. Nemění historická data, částky ani
indexy. Při chybě je celá operace vrácena. Pro staré příkazy bez payment_id
se zachovává serializace a fingerprint idempotence; starý retry nesmí
vytvořit druhou platbu ani skončit konfliktem kvůli novému nullable poli.
Před upgradem zastavit API a zálohovat DB.

Akceptace: existující vydaná/přijatá úhrada, plně i částečně zaplacená
faktura, neměnnost původní evidence, chybná částka/profil/revize,
konkurenční pohyb, odpojení a opětovná dostupnost, REST/MCP a viewer,
upgrade052 s replay původního příkazu, CS/EN desktop/mobile.

Zbývá: cizoměnové účty a importy, zálohy/vratky, souhrnný výkaz rozdílů,
měnové úhrady mimo banku a rozdílné kurzy DDPP/vyúčtování.

Ověření P16-13b/P19-01l (17. 10. 2026):
- pnpm test:bank: 44/44 v izolované PostgreSQL, včetně existujících
  příchozích/odchozích plateb, REST/MCP, oprávnění a upgrade052/replay.
  Rozšířený test souběžného připojení ke dvěma pohybům následně také prošel.
  Počáteční paralelní běh byl omezen auth rate limiterem testovacího setupu;
  standardní sériový příkaz prošel bez změny produkční ochrany.
- bank.spec.ts: 4/4, CS/EN desktop/mobile, zamčená původní částka,
  připojení a odpojení bez změny původních payments ani jejich revize.
  Screenshoty bank-foreign-existing-* používají syntetická data;
  vizuálně ověřen CS desktop a mobil.
- pnpm check, build:web a serverový build prošly. Web nadále hlásí
  velikost některých chunků přes 500 kB.
- Lokální DB zálohována, API zastaveno a znovu spuštěno; migrace 053
  aplikována, ready potvrzeno. Bez skutečných bankovních přístupů.

P19-01m doplňuje souhrnný provozní přehled těchto uložených rozdílů
v Reportech, včetně období a odkazů na zdroje. Viz exchange-difference-report.md.
Účetní zaúčtování, cizoměnové účty/importy a další měnové návaznosti zůstávají otevřené.

## P16-13g — rozdílné měny z cizoměnového účtu

18. 10. 2026, vlastní rozšíření E-067, bez nového pozorování iDokladu.
Nahrazuje omezení CZK účtu výše. Ruční foreign_match podporuje různé měny
bankovního účtu a běžné vydané/přijaté faktury, včetně CZK faktury.
Stejná měna patří do dosavadního match/split. Automatika kurzy neodhaduje.

bank_amount je nativní měna pohybu; document_amount je měna dokladu.
Pro non-CZK pohyb je povinný explicitní bank_rate {rate,units}: CZK za daný
počet jednotek bankovní měny. Stejná validace jako dokumentový kurz: rate
kladný, max. 6 desetinných míst, units kladné celé; obě hodnoty max. 1 000 000.
Pro CZK pohyb bank_rate chybí. Kurz dokladu se nemění a čte se ze snapshotu.
CZK faktura má identitu 1/1 ve výstupním document_rate kvůli kompatibilitě DTO.

bank_value_czk = round(bank_amount × bank_rate.rate / bank_rate.units, 2).
book_value_czk = round(document_amount × dokumentový rate / units, 2).
Pro CZK stranu je hodnota přímo částka. Decimal, poloviny od nuly.
Příjem: difference_czk = bank_value_czk − book_value_czk; výdaj opačně.
Syntetický příklad: příjem 110 USD kurzem 22,5 proti 100 EUR kurzem 25
→ 2475 CZK proti 2500 CZK, rozdíl −25 CZK. Výdaj 4 EUR kurzem 24,9 proti
100 CZK → 99,60 CZK proti 100 CZK, rozdíl +0,40 CZK. Nejde o účetní zápis.
Kurz určuje uživatel, ne poměr obou částek ani automaticky dnešní kurz ČNB.
Poplatky a nerozdělený zbytek se nestávají úhradou.

Nové snapshoty doplňují bank_currency, bank_rate a bank_value_czk. Legacy
snapshoty bez polí se čtou jako CZK banka, bank_rate=None a bank_value_czk=None;
UI v tom případě použije původní bank_match.amount. Historie se nepřepočítává.
Report sčítá jen uložené difference_czk; nativní částky různých měn nesčítá.
Příkaz bez bank_rate ho při serializaci vynechá, aby zůstal starý fingerprint.

Migrace 055 rozšiřuje CHECK, historická data nemění; test čisté DB i upgrade054.
Před nasazením záloha a zastavení starého API. Přístup, revize, UUID,
profilový zámek, připojení celé existující bank_transfer úhrady a odpojení
pouze vytvořených plateb zůstávají společné REST/MCP. CS/EN UI umožňuje zadání
bankovního kurzu a oddělených částek, existující částku zamyká.

Akceptace: příjem/výdaj, CZK i cizí měna dokladu, rozdíl se správným znaménkem,
částečná alokace, existující úhrada bez kopie, odpojení, replay, role/profil,
chybějící/neplatný kurz, vyloučení stejné měny, report, upgrade054 a CS/EN UI.
Zbývá rozdílná měna záloh/vratek, jejich automatika, zahraniční IBAN,
měnové úhrady mimo banku a rozdílné kurzy DDPP/vyúčtování.

### Ověření P16-13g

18. 10. 2026: `pnpm test:bank` — 62 testů, včetně čtyř cross_currency scénářů
(příjem USD/EUR, výdaj EUR/CZK, existující úhrada, validace kurzu, report,
REST/MCP s rolemi a upgrade054). Regrese payment 6, period_report 9,
received 10 — všechny prošly nad izolovaným PostgreSQL.
Prošly `pnpm check`, generování API, serverový a webový build; web hlásí
stávající velký chunk. Lokální DB zálohována před 055, API zastaveno a
spuštěno s novou migrací; health OK. Historické snapshoty se nepřepisují.

UI: `pnpm exec playwright test tests/e2e/bank.spec.ts --workers=1` — všechny
4 kombinace CS/EN desktop/mobile prošly. První průchody odhalily staré
očekávání skrytého tlačítka a skutečné odmítání CZK kandidátů v runtime validaci
klienta; obojí opraveno a celá sada zopakována. Kontrola klienta nově ověřuje
bankovní měnu a snapshot i v historii a reportu. Screenshoty
`docs/design/bank-cross-currency-{cs,en}-{desktop,mobile}.png`, CS desktop/mobile
vizuálně zkontrolovány. Syntetické scénáře nedokládají shodu s přihlášeným iDokladem.
