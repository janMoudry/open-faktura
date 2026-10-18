# P19-01j — Načtení návrhu kurzu ČNB

Stav: implementováno; navazuje na uložené kurzy P19-01g–i. Důkazy E-066.
Žádná nová migrace: vybraný kurz ukládají existující příkazy dokladů.

## Chování

V editoru vydané/přijaté faktury a prodejních dokladů s cizí měnou je datum
a explicitní akce „Načíst kurz ČNB“. Datum se předvyplní podle data vystavení,
jinak dneškem v Europe/Prague. Změna měny sama kurz nestahuje. Uživatel může
kurz a množství ručně upravit. U uzamčené měny vyúčtování nelze načtením
obejít shodu s kurzem rezervovaných záloh.

Úspěch doplní CZK cenu za uvedené množství měny a ukáže skutečné datum
publikace. ČNB může pro nepracovní den vrátit předchozí publikaci. Chyba
zachová dosavadní hodnoty a dovolí ruční zadání. Změna profilu, data nebo
měny i odchod z obrazovky ruší čekající požadavek. CS/EN používají společnou
komponentu a lokalizované názvy, stav, chybu a formát data.

Načtení nic nezapisuje: až uložení dokladu persistuje rate/units ve stávajícím
JSON snapshotu a auditu. Zdroj ani datum publikace se nyní nepersistují;
zobrazená informace o ČNB platí pouze pro právě načtený návrh. Ruční úprava
ji odstraní. Historické doklady, PDF ani přehledy se novým kurzem nemění.

## REST, MCP a síť

GET /api/companies/{company}/exchange-rates?currency=EUR&date=YYYY-MM-DD;
MCP exchange_rate_lookup. Oba používají společný aplikační příkaz. Nejprve
ověřují členství ve fakturačním profilu; bez něj se provider nevolá. Návrh
může číst i člen s přístupem pouze ke čtení, zápis nadále ověřuje příkaz dokladu.

Podporované cizí měny odpovídají P19-01g; CZK návrh nepotřebuje. Rozsah
požadavku je 1. 1. 2002 až dnešek v Europe/Prague. Odpověď obsahuje currency,
requested_on, published_on, source=CNB a exchange_rate={rate,units}.

Produkční adaptér volá pouze pevnou HTTPS adresu ČNB s validovaným datem,
nepřebírá URL od klienta a nesleduje přesměrování. Timeout 8 sekund,
maximum odpovědi 64 KiB, nejvýše 4 současné požadavky a minimální rozestup
250 ms v instanci provideru. Přetížení vrací nedostupnost; nejde o distribuovaný
limit ani cache. Síťové I/O neudržuje databázovou transakci.

Parser kontroluje hlavičku, datum, kód měny, množství i kladnou desetinnou
hodnotu podle společné domény. Odmítá duplicitní měnu, HTML, budoucí publikaci
a publikaci starší než 7 dní proti požadavku. Sedmidenní tolerance a dolní
hranice data jsou naše ochranná pravidla. Chybějící měna vrací 404, chybná
doménová data 422, neplatný query 400 a nedostupný/poškozený zdroj 503.

## Ověření a hranice

Syntetický parser test ověřuje víkend, JPY za 100 jednotek a poškozená data.
Izolovaný PostgreSQL test exchange_rate ověřuje REST/MCP, přihlášení,
přístup k profilu, validaci před síťovým voláním a absenci zápisu dokladu.
Dashboard E2E používá zachycenou syntetickou odpověď: doplnění, selhání bez
ztráty kurzu, uložení a reload, vystavení a úhradu; CS/EN, desktop/mobile.
Automatické testy nevolají živou ČNB. Screenshoty exchange-rate-* jsou
syntetická testovací agenda.

Tato část nedělá automatické změny kurzů pravidelných faktur, kurzové rozdíly,
konverzi bankovních pohybů, sklad v cizí měně ani vyúčtování záloh s různými
kurzy. Datum návrhu není právním rozhodnutím o správném daňovém kurzu.
Explicitní načtení ke zvolenému datu je naše UX, nikoli doložená přesná
automatika iDokladu. P19 jako celek zůstává otevřené.

Ověřeno 17. 10. 2026: parser 1/1, izolovaný REST/MCP test 1/1, UI scénáře
4/4. Prošly pnpm check, build:web a build serveru. Mobilní screenshot byl
vizuálně zkontrolován. Lokální API bylo restartováno a vrací ready.
Web build nadále upozorňuje na některé chunky nad 500 kB.

## P19-01n — Volitelný kurz pravidelných faktur

Pravidlo má rate_policy=stored (výchozí) nebo cnb_issue_date. Staré snapshoty
bez pole znamenají stored; tato hodnota se při serializaci vynechává kvůli
zpětné kompatibilitě idempotentních příkazů. Volba ČNB je explicitní, pouze
pro cizí měnu a neaktivní upravitelnou šablonu. Změna měny na CZK vrací UI
ke stored. Načítá se datum plánovaného výskytu, nikoli datum opožděného
spuštění workeru. Správnost zvoleného daňového kurzu zůstává rozhodnutím
provozovatele; automatika není obecný účetní kurzový režim.

Společný Run příkaz pro REST/MCP i worker nejprve ověří práva zápisu,
replay, stav a revizi, uzavře čtecí transakci a pak načte ČNB. Síťové volání
nedrží firemní/číslovací SQL zámek. Po návratu společný transakční příkaz
znovu ověří členství, revizi a datum, teprve pak atomicky vytvoří doklad,
výskyt a případné vystavení. Žádný nový dokument při nedostupnosti zdroje,
změně revize ani odebrání práv během čekání. Worker použije existující
chybový stav a ruční obnovení; nepadá tiše zpět na kurz šablony.

Částky cizí měny se nemění. Nový doklad uloží načtené rate/units;
snapshot výskytu navíc uchová pozorování kurzu včetně requested_on,
published_on a source. Šablona i všechny starší doklady si ponechají
původní kurz. Opakovaný request vrací existující výsledek i tehdy, když je
později ČNB nedostupná, a nedělá další síťový dotaz. Policy stored síť
nevolá. Bez nové migrace, verzovaný JSON kontrakt a CS/EN editor.

Test `opt_in_cnb_rate_snapshot_failure_replay_and_revision_race` používá
injektovaný syntetický provider: výpadek bez dokladu, nový uložený kurz,
neměnná šablona, replay bez provideru a změna revize během I/O. Provider
v testu aktualizuje stejný SQL řádek pod timeoutem a ověřuje absenci zámku
v síťové fázi. Testy nepoužívají živou ČNB.
