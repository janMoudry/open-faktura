# P14-02 — upomínky a poděkování za úhradu

Stav: implementováno v níže vymezeném rozsahu, 13. 10. 2026. Důkaz E-037; předchozí vrstva P10-01.

## Reference a rozsah

Veřejná nápověda iDokladu popisuje ruční komunikaci, automatické upomínky běžných
faktur, první odstup od splatnosti, interval opakování, minimální celkovou částku
(nikoli zůstatek) a nejvýše deset automatických upomínek. Automatické poděkování
je spojeno s bankovním párováním. Dokumentace není důkaz interní implementace.

Následující pravidla jsou rozhodnutí Open Faktury, nikoli tvrzení o přesné shodě:

- Automatizace je samostatná konfigurace profilu, ve výchozím stavu vypnutá.
  Změny povoluje vlastník; čtení všichni členové. Revize chrání před přepsáním.
- Běží na serveru bez přihlášení uživatele. Přijaté oprávnění je vázáno na autora
  konfigurace; ztráta členství nebo práva zápisu zastaví jeho automatické akce.
- Zapnutí upomínek zahrnuje existující nezaplacené faktury po splatnosti.
  První odstup i interval jsou 1–365 dní v časové zóně profilu. Nejvýše deset
  pokusů o automatickou upomínku na fakturu. Nedohání více upomínek najednou.
  Limit částky je v CZK, stejně jako aktuální podporované faktury.
- Namísto referenčního přepínače kontaktu poskytujeme pozastavení automatizace
  jednotlivé faktury. Ruční odeslání zůstává explicitní akcí editora/vlastníka.
- Automatické poděkování reaguje na **nové platby po zapnutí**,
  až když je zůstatek nulový a faktura obsahuje skutečnou peněžní úhradu.
  Historické úhrady ani samotný zápočet dobropisu jej nespustí. Od P16-01 sem patří i platba vytvořená ručním bankovním párováním; pouhé připojení starší platby nevytváří novou. Automatické poděkování je nejvýše jedno na fakturu,
  včetně zrušení a nové registrace úhrady. Ruční poděkování potlačí automatické.
- Příjemce automatizace je jeden e-mail z historické kopie odběratele na faktuře.
  Více příjemců a automatické použití aktuálního kontaktu nejsou implementované.
- Texty automatizace jsou vlastní CS/EN šablony podle jazyka dokladu. Před ručním
  odesláním lze upravit příjemce, předmět a tělo. Editace automatických šablon
  v nastavení není v tomto kroku. PDF je příloha; veřejné odkazy faktur navazují
  v P14-03. Zálohy a importované historické doklady nejsou zahrnuty.

## Persistované kontrakty a souběh

Migrace 023 je aditivní. `communication_settings` má profilový PK, revizi,
aktéra, přepínače, odstupy, desetinný limit a počátek sledování nových úhrad.
`communication_pause` má složený FK profilu a faktury. `communication_job`
ukládá jedinečný slot, UUID požadavku, neměnný payload, revizi nastavení a vazbu
na outbox. `communication_audit` eviduje změny konfigurace i pozastavení.
`invoice_delivery` přidává typ zprávy a revizi úhrad / automatizace. Staré
příkazy, PDF, čísla a úhrady se nemění.

Náhled vrací aktuální revizi úhrad, příjemce a text. Enqueue zamkne fakturu
proti zápisu platby a ověří aktuální způsobilost: upomínka jen při prodlení,
poděkování jen po úplné úhradě s peněžní platbou. Změna revize znamená konflikt.
UUID a celý payload chrání opakování, včetně restartu workeru.

Bezprostředně před předáním SMTP se znovu kontroluje revize úhrad, oprávnění,
aktivní profil a případně revize konfigurace a pozastavení. Změněná zpráva
končí v historii jako neodeslaná (`communication_changed`). Kontrola je hranice
zahájení odesílání; externí SMTP nemůže být atomické s databázovou platbou.
Platba potvrzená až po této hranici již nemusí zastavit odesílaný e-mail.
`uncertain` se nikdy automaticky neopakuje. SQL zámky nedržíme během SMTP.

REST a MCP používají společnou aplikační vrstvu. Odeslání používá existující
`invoice_send`, nově s `purpose` a `settlement_revision`; náhled, konfigurace a
pauza mají kontrakt `communication.openapi.json`. Náhled nikdy nic neodesílá.

## Ověření

Izolovaný PostgreSQL a pouze loopback SMTP: náhled CS/EN, nepovolené stavy,
částečná/úplná platba, storno, souběžné enqueue a workery, změna nastavení,
pauza, cizí profil, role, opakování, max. počet, minimální částka a upgrade 022.
UI: nastavení s potvrzením, ruční náhled a historie, čeština/angličtina,
monochromatické komponenty a mobilní šířka. Výsledky v [validačním záznamu](../design/communication-validation.md).
