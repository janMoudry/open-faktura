# P17-03b — nativní IMAP příjem

18. 10. 2026, vlastní návrh Open Faktury. Sdílí originály, limity a
transakční příjem z [inboxu](inbox.md). Nejde o tvrzení živé shody iDokladu
nebo o předem nakonfigurovanou podporu konkrétní e-mailové služby.

## Instalace a explicitní připojení

Provozovatel uloží privátní `INBOX_IMAP_PROVIDERS_JSON`: objekt názvů
konfigurací s `host`, volitelným `port` (výchozí 993), `username`, `password`
a `mailbox`. Příklad se syntetickými hodnotami:
`{"own-mailbox":{"host":"imap.example.test","port":993,"username":"inbox@example.test","password":"replace-in-private-environment","mailbox":"INBOX"}}`.
Tento JSON patří do soukromého prostředí instalace, nikdy do repozitáře,
DTO, logu, auditu nebo screenshotů. Klient volí pouze existující název;
nemůže poslat síťový endpoint, heslo, schránku ani libovolnou URL.
Instalační konfigurace je důvěryhodné síťové oprávnění správce, obdobně
jako SMTP nebo bankovní gateway; neodvozuje se z e-mailového obsahu.

Firma zvlášť povolí nativní IMAP a vybere konfiguraci. Výchozí stav je
vypnutý. Je nezávislý na opt-in vlastní JSON e-mailové brány. Člen viewer
smí nastavení a historii číst; editor/owner smí nastavit a explicitně
spustit kontrolu. Každá změna má revizi a neměnný audit aktéra a změny.
Změna poskytovatele resetuje UID kurzor a UIDVALIDITY; vypnutí stejného
poskytovatele kurzor zachová. Konfigurace zruší claim případného běžícího
příjmu a po jeho I/O už nedovolí zápis. Současná kontrola stejné firmy je
konflikt; lease po 5 minutách umožní obnovu po pádu procesu.

## IMAP a TLS

Produkční transport naváže implicitní TLS s ověřením certifikátu a jména
serveru proti web PKI kořenům. Není možnost vypnout ověřování, přejít na
plaintext ani zapnout neověřený STARTTLS downgrade. LOGIN používá
standardní knihovnu s escapováním argumentů, debug je vypnutý. Chybové
odpovědi serveru ani přihlašovací argumenty se nevracejí do diagnostiky.

Používá se EXAMINE (read-only), UID SEARCH a UID FETCH BODY.PEEK[];
neoznačuje zprávy jako přečtené, nemaže, neodesílá ani nemění příznaky.
Vyhledávají se UID nad uloženým kurzorem mimo příznak Deleted. Změna
UIDVALIDITY znamená nový UID prostor a čtení od nuly; deduplikace originálů
a Message-ID v inboxu dál chrání před duplicitním přijatým dokladem.
První kontrola může načíst i starší zprávy, po nejvýše 10 za kontrolu.
Ruční synchronizace nevyžaduje automatiku; naplánované kontroly jsou zvláštní opt-in popsaný níže.

Před BODY fetch se čte RFC822.SIZE. Zpráva nad 10 MiB se nestahuje; ukládá
se odmítnutí a kurzor pokračuje. Batch má nejvýše 10 zpráv a 20 MiB těl.
Jednotlivé IMAP odpovědi mají budget (greeting/login 16 KiB, EXAMINE 64 KiB,
SEARCH 512 KiB, velikost 16 KiB, tělo 11 MiB). Síťové sockety mají timeout,
read wrapper navíc 120s deadline; připojení má timeout 10s na nejvýše čtyři
adresy. DNS rozlišení závisí na systémovém resolveru. CPU/synchronní IMAP
běží přes spawn_blocking a síť neprobíhá pod SQL zámkem.

## Nedůvěryhodný MIME obsah

MIME má omezenou hloubku/počet částí před i po parsování. Podporují se
přílohy s názvem PDF/PNG/JPEG/ISDOC, včetně base64 a quoted-printable přes
mailparse. Inline obrázky (např. tracking pixel) a jiné typy se nepřebírají.
Podporovaná příloha projde stejným snifferem, názvem a velikostí jako ruční
upload; cesta nebo škodlivý název není oprávnění k zápisu souboru.
Hlavičky From/Subject jsou neověřená metadata. Chybějící Message-ID dostane
stabilní identitu provider/UIDVALIDITY/UID; není to ověřený podpis odesílatele.
Tělo e-mailu není příkaz pro AI ani zdroj účetní správnosti.

Originály a Message-ID přijímá stejný `mail_in_tx`, bez automatické faktury.
Každá zpráva, originály, výsledek a posun kurzoru se zapisují atomicky;
odmítnutý formát/příloha se vrátí do savepointu a místo dat se uloží
odmítnutí UID s bezpečným kódem. Chyba infrastruktury nebo ztráta oprávnění
příjem přeruší bez posunu aktuální zprávy. Už potvrzené předchozí zprávy
se při dalším pokusu neopakují. Oprávnění, revize konfigurace a vlastnictví
claimu se kontrolují po síti i před každým zápisem.

Request-ID synchronizace je UUID. Hotový běh se stejným aktérem a revizí
vrátí původní výsledek, bez dalšího připojení. Selhaný běh lze opakovat
se stejným klíčem a pokračuje od posledního potvrzeného UID. Souběžný replay
nesmí uvolnit claim prvního běhu. Diagnostika posledních 100 běhů rozlišuje
running/completed/failed a bezpečné kódy. Nepublikují se těla ani hesla.

Migrace 062 přidává stav, audit, běhy a UID diagnostiku. Před upgradem
zastavit staré API a zálohovat databázi. REST/MCP smlouva je
`inbox-mailbox.openapi.json`, UI CS/EN. Testy používají injektovaný transport
a loopback IMAP; produkční TLS test prokazuje odmítnutí plaintextu, nikdy
nepřihlašuje ke skutečné schránce.

Podklady protokolu: IETF RFC 9051 (srpen 2021),
https://www.rfc-editor.org/rfc/rfc9051.html, zejména UID/UIDVALIDITY,
EXAMINE a BODY.PEEK; veřejná specifikace přečtena 18. 10. 2026.
Použité knihovny a staticky čtené kontrakty: imap 3.0.0-alpha.15
https://docs.rs/imap/3.0.0-alpha.15/imap/ a mailparse 0.16.1
https://docs.rs/mailparse/0.16.1/mailparse/, dne 18. 10. 2026.
Tyto kontrakty nedokládají nastavení či dostupnost konkrétní služby.


## P17-03c — samostatně povolené automatické kontroly

Migrace 063 přidává `automatic` (výchozí false) a next_run_at. Vyžaduje
zapnutý IMAP a vybraného poskytovatele; nikdy se nepovoluje při prvním
nastavení instalace, ručním sync nebo příjmu JSON brány. Aktivace v UI/REST/MCP
je explicitní změna konfigurace s revizí, aktérem a auditní stopou.
Vypnutí příjmu nebo změna nastavení ruší in-flight claim; po I/O je stará
revize odmítnutá. Uložený aktér je uživatel, který automatiku nakonfiguroval.

Worker běží s API serverem, kontroluje termíny po 15 sekundách a jednu
firmu naplánuje nejdříve za dalších 5 minut. Termín zabere přes
FOR UPDATE SKIP LOCKED a transakci ihned ukončí, před připojením spustí
společný sync s připnutou revizí a ověřeným aktuálním oprávněním aktéra.
Po síti a před každou zprávou jsou stejná ověření jako u ručního sync.
Odebraná práva nevedou k připojení; bezpečný chybový kód je v historii.
Nové povolení či jiná konfigurace nezpůsobí, že rozpracovaný worker použije
nového poskytovatele pod starým aktérem.

Souběžné workery nevezmou stejný termín; souběh s ruční kontrolou je chráněn
jedinečným lease a kurzorem. Proces má nejvýše čtyři současné blokující
IMAP transporty; permit zůstává v transportu i při zrušení async requestu.
TLS socket má zbývající deadline také pod rustls, takže limit neplatí jen
pro dekódovaný IMAP obsah. Není SQL zámek během sítě. Shutdown čeká na
běžící kontrolu v rámci jejích síťových limitů; po pádu termín/lease umožní
obnovu. Zprávy se nadále nikdy nemažou ani neoznačují jako přečtené.

Kontroly: dvě skutečné PostgreSQL regrese pro příjem/epoch/replay/revize
včetně odebrání práv a vypnutí během I/O, současný replay a dva workery;
čtyři unit scénáře MIME a loopback IMAP/TLS. Nejde o test přihlášení do
živé zákaznické schránky ani potvrzení konkrétního mailového poskytovatele.
