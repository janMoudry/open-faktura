# P16-15a — nastavení bankovního napojení

Rozhodnutí uživatele 14. 10. 2026: vlastní přístupy a konfigurace uživatele,
více bank, test spojení a stav synchronizace. Tento díl neuzavírá celou P16-15.
Doložený kontrakt Fio: E-048. Vlastní gateway, transakční pravidla a UI níže
jsou návrh Open Faktury, nikoli odvozená interní implementace iDokladu.

## Dostupné metody

- Přímé Fio API Banking: HTTPS `periods/{token}/{from}/{to}/transactions.json`,
  pouze čtení. Uživatel vytvoří token pro svůj účet v bankovnictví. UI nepřijímá
  heslo internetového bankovnictví. Nový token dle dokumentace může začít
  fungovat až po pěti minutách. Limit poskytovatele se vrací jako srozumitelná chyba.
- Vlastní poskytovatel: správce instalace registruje důvěryhodné HTTPS gateway
  přes `BANK_GATEWAYS_JSON`. Uživatel vybere poskytovatele a vloží svůj token.
  Gateway obdrží GET s query `iban`, `from`, `to` (ISO data, inkluzivní) a
  `Authorization: Bearer TOKEN`. Vrací přímo dokument `camt053`, `fio_json`
  nebo `moneta_xml` dle nastavení. Účet, CZK a období musí odpovídat požadavku.
  Gateway musí vrátit kompletní období, nikoli první stránku. Open Faktura
  neimplementuje její bankovní OAuth/PSD2 autorizaci ani automatickou obnovu tokenu.
  Jde o explicitní vlastní integrační kontrakt; libovolné PSD2 API mu automaticky neodpovídá.

Příklad instalace (bez skutečných přístupů):

```sh
BANK_GATEWAYS_JSON='[{"id":"my-provider","label":"Můj poskytovatel","url":"https://bridge.example.test/transactions","format":"camt053"}]'
BANK_ENCRYPTION_KEY_FILE=/run/secrets/bank_key
```

URL schvaluje provozovatel serveru, není zapisovatelná klientem REST/MCP.
Musí být HTTPS bez uživatelských údajů, query nebo fragmentu; HTTP redirecty
se nenásledují. Provozovatel odpovídá za důvěryhodnost gateway a její síťový
přístup, včetně interních adres. UI dostane jen ID a label, nikoli URL.

## Operace a oprávnění

`bank_connection_status` čte nastavení a poslední stav v aktivním profilu.
`bank_connection_execute`: save/disconnect/test/sync/schedule používají společný port
REST/MCP a vyžadují owner/editor; viewer pouze čte. Účet musí patřit profilu.
Archivovaný účet dovoluje pouze čtení a odpojení.

Uložení má očekávanou revizi. Token je vstupní, nevrací se ani v auditu;
`null` zachová existující token pouze u stejného poskytovatele. Změna metody
vyžaduje nový token. Opakovaný save nebo schedule se stejným request_id vrátí konflikt;
klient obnoví stav. Ostatní opakované příkazy stejného aktéra, revize a druhu
neopakují síťový požadavek. Změna parametrů pod stejným ID je konflikt.

Test stáhne dnešní data, ověří formát/IBAN/CZK/datum a nic neimportuje.
Synchronizace na vyžádání stáhne posledních 30 kalendářních dní včetně dneška,
atomicky vloží nové pohyby a přesné duplikáty vynechá. Konfliktní identita
pohybu blokuje celou dávku. Používá stejná pravidla jako import výpisů, včetně
zákazu míchání formátů na účtu. Starší data lze importovat výpisem stejného
formátu. Limit odpovědi je 1 MiB / 1000 pohybů; částečné výsledky se neimportují.
Prázdný validní výpis je úspěch, nikoli chyba připojení. Nevznikají automatické
úhrady ani párování. Automatické plánování popisuje níže P16-15b; notifikace zůstávají navazující práce.

Síťové I/O neběží pod databázovým zámkem, má timeout 20 s a spojení 8 s.
Běh rezervuje účet na 60 s; konfiguraci ani další běh nelze mezitím změnit.
Před commitem se znovu ověří oprávnění, účet a revize. Přerušený běh se po
vypršení rezervace zobrazuje jako neúspěšný a umožní nový pokus. Párování,
číslování a import používají společný profilový zámek. Odpojení odstraní
šifrovaný token, ale zachová pohyby, vazby a audit.

## Uložení a provoz

Migrace 033 přidává `bank_connection` a `bank_connection_run`, obě s profilovou
FK na účet. Existující data ani přístupy nemění. Před upgradem zastavit API.
`node scripts/init-secrets.mjs` vytvoří `.secrets/bank-key`, existující klíče
nemění. Produkční Compose připojuje klíč jako secret. AES-256-GCM používá náhodný
nonce a AAD vázané na profil a účet. Klíč zálohovat odděleně a chránit stejně
jako ostatní instalační tajemství; při ztrátě je nutné přístupy zadat znovu.
Ruční výměna souboru není migrace ciphertextů. Tokeny, plné Fio URL a těla
bankovních odpovědí se nesmějí logovat. Chyby mají pouze pevné veřejné kódy.

## Ověření a hranice

Testy používají izolovaný PostgreSQL, syntetické transportní odpovědi a
loopback HTTP. UI ověřuje CS/EN, desktop/mobile, šifrované uložení přes backend,
chybu nedostupné lokální gateway a odpojení. Žádné skutečné bankovní přístupy
ani zákaznické pohyby nebyly použity. Živá shoda Fio a jednotlivých gateway
zůstává neověřená. Nativní adaptéry všech bank, e-mailové notifikace a obecný
PSD2 klient nejsou touto dodávkou implementované; banky přes vlastní gateway
jsou podporované pouze tehdy, když pro ně provozovatel dodá odpovídající službu.

Ověřeno při dodání: 18 bankovních DB testů (včetně REST/MCP statusu a upgradu
032→033), čtyři parserové regrese, HTTP transportní test proti loopbacku,
4 E2E scénáře CS/EN × desktop/mobile s axe, `pnpm check` a web/server build.
Lokální API po restartu hlásí ready a má aplikovanou migraci 033.

## P16-15b — volitelná automatická synchronizace

Vlastní návrh Open Faktury, 14. 10. 2026. Automatika je na nových i existujících
účtech vypnutá. Owner/editor může explicitním příkazem `schedule` zvolit
15 minut, hodinu, 6 hodin nebo den. Uloží se aktér aktivace a revize. První běh
je naplánovaný za zvolený interval, okamžité načtení zůstává samostatná akce.
Změna přístupů nebo odpojení automatiku vypne; archivované účty se nezpracují.

Worker v procesu API zpracovává nejvýše jeden účet současně. Claim používá
FOR UPDATE SKIP LOCKED a atomický posun dalšího termínu, takže více serverů
nespustí stejný plánovaný termín. Následuje stejný příkaz synchronizace jako
REST/MCP, včetně opětovného ověření oprávnění před I/O i před importem.
Vypnutí nebo změna konfigurace invaliduje dosud nezahájený claim revizí.
Běžící synchronizaci nelze změnou nastavení přerušit; lze počkat na výsledek
nebo vypršení 60s rezervace. Ztráta oprávnění aktéra, archivace nebo smazání
profilu automatiku vypne a uloží bezpečný důvod. Pozdější vrácení oprávnění
ji samo nezapne.

Chyby poskytovatele jsou v posledním běhu; další pokus nejdříve za interval.
Chyba před vytvořením běhu má samostatný stav plánovače. Pád serveru po claimu
nevede k okamžitému opakování: další pokus je v následujícím intervalu; pohyby
se při opakování deduplikují. Po delší odstávce proběhne jeden catch-up běh,
ne smyčka všech zmeškaných termínů. Import stále čte jen posledních 30 dní.
Žádné automatické párování ani odesílání bankovních příkazů.
Migrace 034 nemění uložené přístupy a automatiku nikomu nezapíná.

Ověření P16-15b: bankovní regrese 20 testů a dodatečný test odebrání práv během
I/O (celkem 21 scénářů), upgrade033, souběh workerů, chyby poskytovatele a
vypnutí. E2E CS/EN desktop/mobile ověřuje plánovač přes skutečný API proces
a izolovanou DB, pouze s lokální gateway. Mobilní CS scénář navíc prošel
třemi opakováními. Živé bankovní účty ani skutečné přístupy nebyly použity.

## Nezávislé párování P16-14

Příkaz `matching {enabled}` na stejném revizním kontraktu ovládá
[automatické párování](bank-auto-matching.md). Nepotřebuje bankovní token;
status vrací matching_enabled/next_at/error a matching_last (čas a počty).
Odpojení bankovního API vypíná stahování, nikoli samostatně povolené párování
už uložených pohybů. To se vypíná vlastním přepínačem.
