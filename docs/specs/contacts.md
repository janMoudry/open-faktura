# P04 — Adresář odběratelů a kontaktů

11. 10. 2026. Referenční základ E-013 (veřejná nápověda iDokladu);
model, autorizace, limity a archivace jsou vlastní návrh Open Faktury.
Nejde o tvrzení shody s neveřejným API nebo databází iDokladu.

## Požadavky

- CONTACT-01: profilový adresář, seznam/detail/založení/úprava; jméno povinné,
  IČO nepovinné i pro fyzické osoby, textové identifikátory zachovávají nuly.
- CONTACT-02: fakturační adresa, max. 20 dodacích adres se stabilními lokálními
  ID a výchozí adresou; null znamená fakturační adresu. Dvouznaková země,
  ostatní adresní údaje mohou zůstat prázdné, před vystavením validuje P06/P07.
- CONTACT-03: hlavní e-mail/telefon, max. 20 osob a dalších příjemců.
  Osoba vyžaduje jméno, další e-maily nejsou prázdné; normalizované duplicitní
  příjemce slučujeme. NIC se neodesílá. SMTP a historie odeslání jsou P10.
- CONTACT-04: splatnost null nebo 0–365 dnů, metoda platby ''/cash/bank_transfer;
  null a '' dědí profil. Bankovní účet/IBAN jsou volitelné textové údaje,
  nikoli ověřené bankovní spojení. P06 doplní použití preferencí na faktuře.
- CONTACT-05: hledání názvu/IČO/e-mailu, stránka 1–10000, velikost 1–100,
  UI 25; stabilní pořadí lower(name),id, celkový počet. Hledání je doslovný
  podřetězec, nerozlišuje velikost písmen; % a _ nejsou wildcards.
- CONTACT-06: archivace i obnovení, revize a neměnné auditní snapshoty.
  Archiv není smazání; detail zůstává dostupný členům profilu.
- CONTACT-07: owner/editor zapisuje, viewer čte, instal. admin nemá bypass.
  Cizí company/contact ID i smazaný profil vrací 404; viewer zápis 403.
- CONTACT-08: REST a MCP používají stejné aplikační příkazy. Zápisy s revizí,
  idempotentní create přes request_id; stejný klíč + jiná data/aktér = 409.
- CONTACT-09: CS/EN, zachovaný shell, přepnutí profilu ponechá adresář,
  upozornění před ztrátou neuložených změn. Konflikt ponechá rozepsaná data,
  načtení serverové verze vyžaduje potvrzení zahození.

## Kontrakt a uložení

`contracts/contact.openapi.json` je zdroj REST typů a schémat MCP nástrojů.
`pnpm generate:api` regeneruje TS a `contracts/contact.tools.json`.
GET/POST /api/companies/{company}/contacts; GET/PUT …/{id}.
Create {request_id,data}; update {revision,archived,data}. Úspěch 200 včetně
create replay. 400/413/415 jsou framework transport chyby; strukturované
401/403/404/409/422/503 vrací code, validační chyby navíc fields.
REST zápis vyžaduje session, Origin a X-Faktura-Request=1; všechny odpovědi
no-store. MCP contacts_list/contact_get/contact_create/contact_update
vyžadují explicitní company_id a na každé volání znovu ověří session.

Migrace 202610110006 přidává contact a contact_audit; stávající data nemění.
FK (company_id,contact_id) v auditu brání vazbě do jiné firmy. Index adresáře
začíná company_id,archived; fulltext/trigram optimalizace je budoucí krok při
větších adresářích. Oprávnění se ověřují v transakci pod SHARE zámkem profilu
+ členství, update zamkne kontakt a porovná revizi. Audit je ve stejné transakci.
Kontakt se nemaže fyzicky; budoucí faktury uloží snapshot, ne živý odkaz na
aktuální adresu. Kontakty mohou vzniknout i v rozpracovaném profilu.

## Ověření a hranice

PostgreSQL testy: cizí profil, admin bez členství, viewer/editor, souběžný create
bez duplicity, konfliktní editace, archivace/obnova/audit, validace, stránkování,
hledání, REST session/origin a živý MCP SDK klient včetně odvolání session.
UI: CS/EN × desktop/mobil, skutečné API a dočasná DB, vytvoření adres a osoby,
reload, úprava, archiv/obnova, hledání a přepnutí na prázdný druhý profil,
zachování rozepsaných dat při konfliktu a režim viewer bez zapisovacích akcí.

Ověření DIČ/VIES a bankovního účtu zůstává P19. Související faktury a vytvoření
faktury z kontaktu přijdou s P06; exporty s importy/exporty. Slevy a automatické
upomínky z širšího FEATURES §8 se připojí až k příslušným fakturačním modulům.

## Doplnění z ARES — 11. 10. 2026

Rozšíření P04 na přání uživatele. GET …/contacts/ares/{ico} a MCP
contact_lookup_ares: přihlášený člen explicitního profilu; validace osmi číslic
včetně kontrolního součtu. Nezapisuje do DB. Backend volá pouze pevnou URL
oficiálního ARES, timeout 8 s, nejvýše 4 souběžné požadavky, odstup 250 ms
na instanci adaptéru a odpověď maximálně 1 MiB. Bez klíče a bez nové migrace.
Název, DIČ (jen vrácené registrem), sídlo; žádné odvozování DIČ z IČO nebo
plátcovství. Prázdné/nezjištěné údaje se nevymýšlejí.

UI načítá po blur pole IČO nebo tlačítkem Doplnit z ARES. Vyplní pouze prázdná
pole; existující jméno, DIČ a adresa zůstávají. Změna IČO zruší starý dotaz;
změna profilu/editoru jej odpojí. Při nesouladu země se údaje nedoplní.
Zápis je stále explicitní uložení kontaktu, s běžnou validací a auditem.
Při 404/422/503 zůstane formulář použitelný, s přeloženou zprávou a retry.
ARES je externí služba, nikoli podmínka založení odběratele.

Zdroj: https://ares.gov.cz/swagger-ui/ a živé veřejné API ekonomicke-subjekty,
ověřeno 11. 10. 2026 na veřejném záznamu Ministerstva financí (00006947).
Testy: checksum, mapování adresy, API autentizace a profil, UI CS/EN desktop/mobil
s deterministickou odpovědí registru, zachování vlastních údajů a retry po 503.

Stejný blok AresRegistration a společná logika načtení se používají také při
založení a úpravě vlastního fakturačního profilu (CompanyPage). Profil smí měnit
jen owner; ARES nemění typ osoby, režim DPH ani datum jeho účinnosti. Údaje se
uloží až standardním uložením profilu. Dotaz se zruší při změně routy profilu.
IČO je na začátku identifikačních údajů, stav načtení u pole, ruční formulář
zůstává přístupný. Přihlašovací účet není fakturační profil.

## UI sjednocení RE-01 (11. 10. 2026)

Reference E-022: oddělený čtecí detail a editor, základní údaje / dodací adresy,
kontaktní informace a osoby ve společné sekci, rozbalovací bankovní údaje.
Seznam má sloupce podle reference, serverové stránkování s volbou velikosti,
archiv a hledání v URL. Detail odkazuje na faktury filtrované kontaktem.
Rušení neuložené editace vyžaduje potvrzení. Viewer vidí čtecí detail bez akcí
úpravy a archivace. Přijaté odchylky a snímky: reference-audit-2026-10-11.md.
Regrese: tests/e2e/contacts.spec.ts (CS/EN, desktop/mobile, izolovaná DB).
