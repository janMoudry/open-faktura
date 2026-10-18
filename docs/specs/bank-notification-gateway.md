# P16-15 — autentizovaný příjem bankovních oznámení

18. 10. 2026, vlastní integrační kontrakt Open Faktury. Navazuje na
[existující parsery výpisů](bank-statements.md) a jejich doložené limity.
Nejde o nativní adaptér textových notifikací libovolné banky a syntetické
fixtures nejsou skutečnými bankovními e-maily.

## Tok

Správce vlastní poštovní brány přijme e-mail a předá podporovanou přílohu
výpisu jako JSON přes přihlášenou relaci editora/ownera. Přenos používá
standardní autentizaci a CSRF ochranu REST, nebo MCP s firemním členstvím.
Poštovní přihlašovací údaje zůstávají v uživatelově bráně. Open Faktura
neprovozuje veřejný neautentizovaný SMTP endpoint ani neodvozuje oprávnění
z adresy From. Pole sender je pouze neověřené metadata.

Nastavení je zvlášť pro každý bankovní účet, výchozí vypnuté, s revizí a
jedním povoleným formátem `camt053`, `fio_json`, `moneta_xml` nebo `rb_abo`.
Povolení příjmu nemění nastavení automatického párování ani synchronizace.
Archivovaný účet nepřijímá nové pohyby. Neexistující účet v jiné firmě
neodhaluje svůj stav.

POST `/api/companies/{company}/bank/notifications` obsahuje `request_id` UUID,
`message_id`, `sender` a `file` ve stejném formátu jako `BankStatement`.
Příloha má nejvýše 2 MiB; skutečný formát, účet, měna, znaménka, datum,
identifikátory a případný konflikt obsahu se ověřují existujícím parserem.
Gateway nesmí syntetizovat chybějící bankovní identifikátory ani vydávat
svůj obecný formát za bankovní export. Notifikace bez podporované přílohy
vyžaduje explicitně doložený budoucí adaptér konkrétního textového formátu.

## Konzistence

`statement_import_in_tx` je společná transakční operace pro ručně potvrzený
výpis i oznámení. Firemní oprávnění, explicitní opt-in, import a diagnostika
jsou pod stejným firemním advisory lockem a v jedné transakci. Není externí
I/O pod SQL zámkem. Import nikdy sám nevolá platební příkaz ani matcher.
Pokud už uživatel samostatně povolil worker párování, ten funguje podle
vlastního oprávnění a pravidel; příjem jej nijak nezapíná.

Message-ID je unikátní pro firmu a účet. Hash zahrnuje účet, sender, formát,
název a hash dekódovaných bytů. Stejný Message-ID a obsah vrátí uložený
výsledek bez druhého importu; odlišný obsah je 409. Request-ID nelze znovu
použít pro jinou zprávu. Jiný Message-ID se stejnými bankovními pohyby
projede standardní deduplikací výpisů. Chyba jediného řádku vrátí celou
transakci a žádnou úhradu. Vypnutí příjmu odmítne i replay staré zprávy.

GET `.../bank/accounts/{account}/notification-settings` a
`notification-history` respektují firemní členství. Viewer smí číst, změny
a příjem potřebují editor/owner. Diagnostika vrací posledních 100 úspěšných
zpráv (datum, neověřený sender, Message-ID, nové/duplicitní počty).
Odmítnutý vstup vrací standardní chybu; neukládají se těla e-mailů, hesla
ani odmítnuté přílohy. Samotné názvy formátů nejsou důkazem živé shody
všech bank; platí dosavadní limity referenčních podkladů.

Migrace 061 přidává pouze nové tabulky; dosavadní účetní data se nemění.
Před nasazením zastavit staré API a zálohovat databázi. Testy používají
izolovaný PostgreSQL a syntetický CAMT, bez skutečné banky nebo poštovního
účtu. Kontrakt `bank-notification.openapi.json`, shodné MCP nástroje a UI
banky CS/EN.

Každá úspěšná změna konfigurace ukládá oddělený neměnný záznam
`bank_notification_settings_audit` s aktérem, revizí, účtem, enabled, formátem
a časem ve stejné transakci. Historie příjmu v UI zobrazuje úspěšné importy,
nikoli zamítnuté požadavky či úplné provozní logy. Brána musí zaznamenat
HTTP/MCP chybový kód, request_id a čas svého pokusu bez účetních těl.
Po opravě může zopakovat stejný request_id; obsah úspěšného pokusu neměnit.

## Provoz a obnovení autentizace

Brána používá běžný uživatelský účet s členstvím v příslušné firmě;
instalační admin nemá výjimku. Relace má omezenou platnost jako běžný login,
nejde o trvalý neexpirující bankovní API klíč. REST integrace uloží přihlašovací
cookie pouze do privátního cookie jaru a posílá povolený Origin a
X-Faktura-Request podle auth kontraktu. Na 401 musí příjem pozastavit,
obnovit relaci standardním loginem a opakovat nedokončenou zprávu se stejným
request_id/Message-ID. Na 403 příjem zastavit a ověřit členství/opt-in;
neobcházet omezení jiným firemním ID. Pro MCP použít `mcp-login` a nový
privátní SESSION_FILE podle instalačního návodu. Cookie, heslo ani obsah
SESSION_FILE nevypisovat do logu a nesdílet v diagnostice. Návrh trvalého
service-account tokenu není součástí tohoto adaptéru.
