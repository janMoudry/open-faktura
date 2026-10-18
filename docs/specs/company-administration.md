# Správa přístupů, obnova profilů a export dat (P20-07a/P20-08a)

18. 10. 2026. Podklad: FEATURES §14/15/19 a aktivní P20. Níže uvedená pravidla
jsou **návrh Open Faktury**, nikoli ověřená shoda s iDokladem. Navazují na
`company-setup.md`, existující jedinou roli owner a logické odstranění firmy.

## Přístupy a historie

Vlastník může přidat existující účet podle normalizovaného e-mailu jako editor
nebo čtenář, změnit tyto role a odebrat nevlastníka. Neposílá se pozvánka ani
zpráva. Vyhledání e-mailu není veřejný adresář: nejprve se ověřuje vlastnictví
firmy. Seznam členů dostupný členům firmy obsahuje ID, jméno a e-mail, nikoli
přihlašovací údaje. Oprávnění instalačního správce členství nenahrazuje.

Převod vyžaduje existujícího člena; starý vlastník se stává editorem. Firma má
vždy právě jednoho vlastníka. Vlastník nesmí firmu opustit ani být odebrán;
nejdříve musí předat vlastnictví. Nevlastník může odejít, tím ztratí přístup.
Staré doklady, vydavatelské snapshoty, autorství i audit zůstávají zachovány.
Tyto operace nemažou ani neanonymizují globální uživatelský účet.

Změny serializuje `company FOR UPDATE`, který koliduje s autorizací dokladů
`FOR SHARE` a s editací/odstraněním profilu. Převod je atomický. Každá změna
zvyšuje revizi firmy a zapisuje `company_audit`; strukturovaný vstup a výsledek
jsou v `company_admin_command`. `request_id` je UUID; opakování stejného
příkazu stejným aktérem vrátí uložený výsledek bez druhého zápisu. Rozdílný
vstup nebo aktér stejného klíče je konflikt. Aktuální členství se ověřuje i před replay. Po odchodu uživatel nezíská
původní odpověď ani údaje členů; vrátí se NotFound a žádný druhý zápis.
Vlastní replay při zachovaném členství (například po převodu vlastnictví)
vrací uložený výsledek bez nové správcovské operace. Cizí příkaz není dostupný.

Koš profilů obsahuje pouze logicky odstraněné firmy, jejichž vlastníkem je
aktuální uživatel. Obnova vyžaduje aktuální revizi a původní vlastnictví.
Bývalý vlastník po převodu nemůže profil odstranit nebo obnovit. Koš jednotlivých
dokladů a definitivní anonymizace nejsou součástí této dodávky.

## Přenosný datový archiv

Vlastník aktivní firmy může stáhnout ZIP `company-{id}-archive.zip` přes REST
nebo MCP. `manifest.json` uvádí verzi formátu 1, poslední SQLx migraci, firmu,
seznam tabulek, počty řádků, limity a výslovné označení, že nejde o zálohu pro
obnovu PostgreSQL. `tables/{table}.jsonl` obsahuje původní řádky jako JSON;
PostgreSQL bytea (originály, přílohy a PDF cache) je hexadecimální řetězec.
Desetinné JSON hodnoty vznikají v PostgreSQL bez převodu přes float.

Export používá explicitně zkontrolovaný seznam tabulek a sloupců
`contracts/company-archive-schema.json`; nové tabulky/sloupce se bez revize
neexportují. Pohledy jsou vyloučené. Tabulka company se vybírá podle id. Řádky jiné firmy
ani globální auth/SMTP konfigurace se neexportují. Sloupce `secret`, `password`,
`token_hash`, `url` se vynechávají (bankovní přístupy a bearer odkazy).
Archiv neobsahuje globální uživatelské účty; vazby autorů zachovává ID.
Známé share URL a jejich tokeny se rekurzivně odstraní ze všech textových
hodnot včetně vnořených command snapshotů a e-mailových těl. Nahrazují se
pouze konkrétní známé přístupové hodnoty, běžné poznámky/URL se zachovávají.
Manifest uvádí přesný povolený seznam sloupců.

Export běží v jediném REPEATABLE READ snapshotu, ověřuje owner před čtením dat
a drží sdílený zámek firmy/členství. Žádný doklad ani PDF nepřepočítává.
Limit je 100 000 řádků a 100 MiB nekomprimovaného JSONL; při překročení vrátí
validaci archive_limit, žádný neúplný soubor. Odpověď je no-store. Velké instalace
nad limit používají provozní PostgreSQL zálohu; automatický import tohoto archivu
ani další nativní migrační adaptéry tato dodávka neslibuje.

## Kontrakty a ověření

`contracts/company-admin.openapi.json`, generovaný TS a MCP tools; HTTP a MCP
volají stejné aplikační porty. CS/EN formulář ve správě firmy zahrnuje členy,
převod, odchod, obnovu a stažení archivu. `crates/server/tests/company_admin.rs`
ověřuje revize/replay, role, odchod, souběžný převod, obnovu, izolaci exportu,
originální binární hodnoty a vynechání přístupových tajemství.

Migrace 202610180080 je aditivní mimo rozšíření CHECK auditu. Před upgradem
zastavit staré API; historické řádky se nepřepisují a nic se trvale nemaže.
