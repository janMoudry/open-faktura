# Instalace a provoz

Návod pro vlastní server, první účet, odesílání dokladů a správu dat. Aplikace je v aktivním vývoji; veřejná stabilní distribuce ještě není připravená. Dostupné moduly a omezení uvádí [přehled funkcí](../FEATURES.md).

- [Výběr způsobu použití](#který-způsob-použití-zvolit) a [vlastní server](#vlastní-server).
- [První přihlášení a firma](#první-přihlášení-a-firma).
- [Nastavení SMTP v aplikaci](#nastavení-smtp-v-aplikaci) a [konfigurace SMTP na serveru](#odesílání-faktur-a-cenových-nabídek-smtp).
- [Veřejné odkazy nabídek](#veřejné-odkazy-na-nabídky) a [přílohy](#vlastní-přílohy-dokladů).
- [Záloha a obnova](#záloha-a-obnova) a [obnova přístupu](#obnova-přístupu-bez-smtp).
- [Technický kontrakt](#kontrakt-a-bezpečnostní-rozhodnutí) a [ověřování](#ověřování).

## Který způsob použití zvolit

Jednotlivec s vlastním serverem používá současný serverový setup. Desktopový
klient připojený k existující instalaci potřebuje adresu serveru a účet, nikoli
nový instalační klíč. Osobní režim „Používat na tomto počítači“ s automaticky
spravovaným lokálním backendem a PostgreSQL je plánovaný jako PERSONAL-01;
aktuální Tauri instalátor ho ještě neobsahuje. Podrobnosti a hranice jsou ve
[specifikaci režimů](specs/installation-onboarding.md#osobní-použití-a-desktop).
Agentům při setupu pomáhá repo skill `installation-setup`.

## Vývoj

Pro lokální vývoj použijte [samostatný návod](DEVELOPMENT.md#web-a-api), který obsahuje závislosti, vytvoření `.env` a spuštění služeb. Vývojový Compose s veřejným heslem nepoužívejte pro veřejné nasazení.

## Vlastní server

Potřebujete Docker Compose a doménu směrovanou na server s porty 80/443.
Z kořene repozitáře:

```sh
node scripts/init-secrets.mjs
# Vytvořte infra/production.env, například:
# APP_DOMAIN=faktury.example.cz
# APP_VERSION=local

docker compose --env-file infra/production.env -f infra/compose.production.yaml up -d --build
```

Build používá lockfiles, obsahuje backend i produktový web a žádný sandbox.
Caddy zajišťuje HTTPS, databáze ani aplikace nepublikují vlastní host port.
Doména musí být skutečná a dostupná pro vydání certifikátu; místní test image
nedokládá funkční veřejné DNS/certifikát konkrétního provozovatele.
PostgreSQL a soubory mají trvalá volumes. Compose secrets jsou lokálně
připojené soubory, nikoli šifrovaný trezor. Startup je načte a spustí aplikaci
pod UID 10001 bez dalších oprávnění; bootstrap soubor má soukromou pracovní kopii.

`APP_ENV=production` vynucuje HTTPS originy a cookie `Secure`. Chybějící DB URL,
neplatný origin nebo chybějící bootstrap tajemství u nové instalace zabrání
spuštění. `DATABASE_URL_FILE` má přednost před `DATABASE_URL`. Po dokončení
bootstrapu jej DB nedovolí znovu, ani když soubor stále existuje. Server může
po dokončení běžet bez bootstrap souboru (u Compose je pak nutné odebrat i jeho
mount a proměnnou). Dev credentials z lokálního Compose se zde nepoužívají.

Migrace jsou dopředné SQLx migrace s databázovým migračním zámkem; server je
aplikuje před nasloucháním. Při chybě nepřejde do ready stavu. Před updatem vytvořte zálohu;
nekombinujte neslučitelné verze API nad jedním schématem.

## První přihlášení a firma

1. Otevřete web aplikace. U nové instalace zobrazí založení správce; výpadek backendu není nová instalace.
2. Zadejte instalační klíč ze souboru `.secrets/bootstrap`, e-mail a heslo alespoň 12 znaků. Klíč čtěte pouze lokálně, nevkládejte jej do issue, logů ani URL.
3. Po potvrzení založení účtu se přihlaste. Založení správce je jednorázové; další pokus vrátí konflikt a je potřeba použít přihlášení.
4. Vytvořte fakturační profil a doplňte firemní údaje. Neúplný profil lze uložit a dokončit později; vystavení faktury vyžaduje úplné nastavení.

Podrobný postup a podporovaná nastavení najdete v [návodu pro firmy](COMPANIES.md). Správce instalace nemá automatický přístup k firemním dokladům bez členství. SMTP ani import nejsou pro první ruční fakturu nutné.

Desktop používá účet existující instalace. Nastavení adresy backendu a omezení Tauri jsou v [návodu pro desktop](DEVELOPMENT.md#desktop); nativní přihlášení není tímto serverovým návodem ověřené.

## Obnova přístupu bez SMTP

Jde o provozní příkaz dostupný člověku s přístupem k serveru/DB, nikoli veřejný
endpoint. Heslo neposílejte jako argument procesu. Připravte soukromý soubor
s novým heslem (12+ znaků) nebo ho předejte na stdin:

```sh
# Lokálně, heslo je v souboru s oprávněním 0600:
target/debug/faktura-server recover-admin admin@example.cz /private/path/new-password

# V Compose, obsah souboru předaný na stdin:
docker compose --env-file infra/production.env -f infra/compose.production.yaml run --rm -T app recover-admin admin@example.cz - < /private/path/new-password
```

Příkaz mění jen existujícího instalačního správce, odvolá všechny jeho relace,
zvýší verzi přihlašovacích údajů a zapíše audit bez hesla. Neotevře bootstrap.
Soukromý soubor po použití odstraňte vlastním provozním postupem. SMTP obnova,
pozvánky, 2FA a další uživatelé navazují v P11.

## Záloha a obnova

```sh
mkdir -p backups
sh scripts/backup.sh backups/first
```

Záloha na chvíli zastaví aplikaci (DB zůstává spuštěná), uloží DB dump,
soubory a provozní konfiguraci/tajemství a aplikaci znovu spustí. Cíl musí být
nový adresář. Výsledný adresář je citlivý; mimo server jej přenášejte šifrovaně.
Nezapomeňte také chránit přístup k zálohovacímu úložišti. Doklady, úhrady,
importní dávky a vlastní přílohy jsou v PostgreSQL; podrobnosti jsou v oddílech níže.

Obnovujte do čistého Compose projektu/volume se stejnou verzí image, před
spuštěním aplikace. Nastavte odpovídající `infra/production.env` a tajemství
(pro nový PostgreSQL lze použít nová DB tajemství; původní konfiguraci ponechte
pro porovnání). Nekopírujte staré DB připojení přes novou databázi bez kontroly.

```sh
sh scripts/restore.sh backups/first
docker compose --env-file infra/production.env -f infra/compose.production.yaml up -d
```

Obnova odmítne neprázdnou cílovou databázi. Browserové relace v dumpu vždy
zneplatní, aby záloha neobnovila již odhlášené přístupy. Při selhání obnovy
nezačínejte používat neúplný cíl; opakujte na novém čistém projektu. Ověřte účet,
stav instalace a soubory. Rollback upgradu znamená odpovídající image + obnovu
zálohy do čistého cíle, nikoli spuštění staré aplikace nad novým schématem.
Pro izolované provozní testy lze použít `PRODUCTION_ENV_FILE` a jiné jméno
Compose projektu v env souboru.

## Kontrakt a bezpečnostní rozhodnutí

Autorita transportu: `contracts/auth.openapi.json`, generování:
`pnpm generate:api` → `packages/api-client/src/auth.generated.ts`.
Runtime wrapper navíc kontroluje vrácený profil a stav setupu. Změny Rust DTO
musí upravit schéma a projít HTTP testy; schema není automaticky odvozené z Rustu.

- `GET /api/auth/setup`: pouze boolean `setup_required`; žádné údaje správce.
- `POST /api/auth/setup`: jednorázový secret + e-mail + heslo, 201 po commitu,
  explicitní přihlášení poté. Commit a zavření setupu tvoří jednu transakci.
- `POST /api/auth/login`: profil a cookie; žádný token/heslo v JSON.
- `GET /api/auth/session`: profil nebo 401; nedostupná DB je 503.
- `POST /api/auth/logout`: odvolání DB relace a vymazání cookie, idempotentní 204.
- Všechny zápisy vyžadují přesně povolený `Origin` a `X-Faktura-Request: 1`.
  JSON endpointy odmítnou formulářový transport. Chybějící Origin se odmítá,
  CORS sám není CSRF ochrana. Auth odpovědi mají `Cache-Control: no-store`.
- Hesla: Argon2id v0x13, parametry výchozí Argon2 0.5 (19 MiB, t=2, p=1),
  náhodná sůl, 12 znaků až 1024 bajtů pro nové heslo. Hashování mimo async
  executor s maximálně čtyřmi souběžnými úlohami.
- Relace: 256bit náhodný token, v DB pouze SHA-256 hash, absolutní expirace 12h,
  HttpOnly / SameSite=Lax / Path=/, v produkci Secure a prefix `__Host-`.
  Žádný token v localStorage. Recovery a login serializují změnu verze hesla.
- PostgreSQL limit: 10 pokusů na normalizovaný e-mail za 15 minut, 500 celkově,
  bootstrap 20; zahrnuje i úspěšné pokusy. Neznámý účet vrací stejnou chybu a
  provede hash. Při přetížení/omezení 429; ochranu proti objemovému DoS doplní proxy.
- Správce instalace a firemní členství jsou oddělené. Přístup k agendě kontrolují [firemní oprávnění](COMPANIES.md#oprávnění).

Podklady k rozhodnutím (ověřeno 10. 10. 2026):
[RustCrypto Argon2](https://docs.rs/argon2/0.5.3/argon2/),
[OWASP CSRF](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html).

## Ověřování

```sh
DATABASE_URL=postgres://faktura:faktura@127.0.0.1:54329/faktura pnpm test:auth
pnpm test:e2e
pnpm build:web
cargo clippy -p faktura-server --all-targets -- -D warnings
```

SQLx testy vytvářejí samostatné testovací databáze a vyžadují CREATEDB. E2E auth
scénář vytváří vlastní dočasnou databázi/server a po testu je odstraní; nikdy
nezaloží správce ve vaší běžné DB. Před E2E sestavte `cargo build -p faktura-server`.
Ostatní UI testy předpokládají standardní lokální dev API/DB a port 5173.

### Výsledek ověření P02 (10. 10. 2026)

- 5 izolovaných PostgreSQL integračních scénářů prošlo; také původní health test.
- 46 Playwright scénářů prošlo (desktop + mobilní Chromium), včetně skutečného
  bootstrapu, neplatného hesla, přihlášení, refresh, HttpOnly a odhlášení.
- Web build, formátování, UI konvence a backend Clippy prošly.
- Čistý Docker build `open-faktura:p02` prošel na Linux ARM64. Opraveny dvě
  typové neshody sdíleného UI odhalené čistou instalací závislostí.
- `python3 scripts/test-installation.py` prošel: neprivilegovaný proces, Secure
  cookie, restart, recovery CLI, záloha DB + syntetického souboru, obnova do
  čistého Compose projektu, odvolání obnovených relací a odmítnutí neprázdného
  cíle. Dočasné projekty a jejich volumes byly odstraněny.
- Veřejné DNS/HTTPS vydání certifikátu, x64 image a Tauri přihlašování nebyly
  ověřeny. Lokální instalaci jsme ponechali bez správce pro vlastní založení.

Relace nepoužívají JWT ani podpisový klíč: autoritou je databáze a náhodný token,
jehož hash se kontroluje při každém autentizovaném požadavku.

### P08: účetní data v zálohách

Doklady, PDF cache, příjmy, storna a audit úhrad jsou součástí PostgreSQL dumpu.
P08 obnovovací test (`pnpm test:payments`, jen izolované dev databáze) ověřuje
shodu dokladu, úhrad, zůstatku a PDF bytes po pg_restore do čisté DB a zrušení
relací. Souborový archiv je zatím ověřen syntetickou přílohou. Postup ověřte
také na vlastní infrastruktuře; test nenahrazuje provozní obnovovací zkoušku.

### Importní dávky (P09)

SQL záloha musí zahrnovat import_batch, import_row a import_identity: obsahují
zdrojovou agendu, identifikátory, staging, historii výsledků a ochranu před duplicitami.
Neodstraňovat je po úspěšném importu. Po obnově lze pokračovat v nedokončené dávce
v Nástroje → Import dat. Pilotní import nepřenáší zdrojové PDF ani přílohy;
archiv původní služby uchovat zvlášť. Automatický rollback importu není dostupný.

## Odesílání faktur a cenových nabídek (SMTP)

PDF a tisk nepotřebují SMTP. Pro skutečné odesílání nastavte na backendu:

```dotenv
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_SECURITY=starttls
SMTP_FROM=fakturace@example.com
SMTP_USERNAME=fakturace@example.com
SMTP_PASSWORD_FILE=.secrets/smtp-password
```

Heslo uložte do souboru přístupného pouze provozovateli/backendu (`chmod 600`),
ne do repozitáře ani veřejných VITE proměnných. Pro implicitní TLS použijte
`SMTP_SECURITY=tls`, port 465. Neověřená certifikace ani vypínání TLS nejsou
podporované; `plain` je povoleno pouze na loopback pro místní testovací SMTP.
Bez autentizace vynechte uživatelské jméno i soubor hesla, pokud ji server nevyžaduje.
Restartujte backend po změně konfigurace. Doménu From a její SPF/DKIM/DMARC
nastavuje provozovatel u svého poštovního poskytovatele.

Backend zpracovává trvalou frontu i po restartu. MCP stdio sám worker nespouští;
pro jeho odesílání musí běžet HTTP backend nad stejnou DB a SMTP konfigurací.
Předáno poštovnímu serveru neznamená potvrzené doručení zákazníkovi.
U nejasného výsledku nejprve ověřte SMTP logy; automatické opakování by mohlo
poslat duplicitu. Nové odeslání z detailu faktury nebo nabídky tvoří další historický záznam.
Migrace 015 přidává frontu faktur při startu; návrhy a historické importy bez PDF
nelze odesílat. Faktury používají stejnou SMTP konfiguraci jako nabídky.
Záloha PostgreSQL nyní zahrnuje také přesné PDF přílohy, adresy a obsah e-mailů.

## Veřejné odkazy na nabídky

Nastavte backendové PUBLIC_WEB_URL na veřejnou adresu webové aplikace
(např. https://faktury.example.cz); restartujte API i MCP. Povoleno HTTPS,
HTTP pouze pro localhost/127.0.0.1/[::1] při vývoji. Bez query, fragmentu
a přihlašovacích údajů. Bez konfigurace lze staré odkazy číst/zrušit,
nové vytvářet nelze. Migrace 014 se aplikuje při startu.

Web i API musí být dostupné příjemci; nastavte VITE_API_URL při buildu
a CORS_ORIGINS podle nasazení. Lokální http://127.0.0.1:5173 funguje
pouze na stejném počítači, není to odkaz pro zákazníky na internetu.
Tauri potřebuje pro příjemce hostovanou webovou aplikaci.

Odkazy obsahují tajný token ve fragmentu, API ho přijímá v POST těle.
Proxy/APM nesmí logovat obsah těchto těl; vynechte externí analytiku na
veřejné stránce. Fragment se neposílá v HTTP URL, stránka používá no-referrer.
DB a zálohy obsahují plnou URL kvůli kopírování a historii e-mailů: chraňte
je jako jiné důvěrné účetní údaje. Při změně domény zůstávají staré odkazy
na původní adrese; udržte ji dostupnou nebo vytvořte a rozešlete nové.
Veřejné API chraňte obvyklými limity požadavků reverzní proxy. Vestavěný
limit uložených zobrazení chrání růst historie, nenahrazuje ochranu HTTP.

Odeslat odkaz využívá stejný SMTP setup jako příloha. Testy používají pouze
loopback SMTP capture a syntetické adresy; neodesílat testy zákazníkům.

## Nastavení SMTP v aplikaci

Správce instalace: avatar → **Nastavení e-mailu (SMTP)** → **Vlastní SMTP**.
Vyplňte server, port, TLS/STARTTLS, odesílatele a případné přihlášení, uložte a
odešlete test na vlastní adresu. Nastavení platí ihned pro všechny profily,
faktury i nabídky. Ověřte příjem včetně spamu; přijetí SMTP není důkaz doručení.
Nastavení domény SPF/DKIM/DMARC stále patří poskytovateli pošty.

Před prvním uložením hesla spusťte `node scripts/init-secrets.mjs`. Vytvoří navíc
`.secrets/smtp-key` (stávající soubory zachová). Backend používá
`SMTP_ENCRYPTION_KEY_FILE`, výchozí cesta `.secrets/smtp-key` relativně k pracovnímu
adresáři. Produkční Compose připojuje klíč jako secret. Vlastní instalace musí
soubor zpřístupnit backendu i MCP, se stejným klíčem pro všechny repliky.

Heslo se ukládá šifrovaně a z API se nevrací. Prázdné pole zachová uložené heslo,
nové jej nahradí. Odebrání uživatelského jména vypne přihlášení a smaže heslo.
**Zálohujte klíč odděleně a bezpečně spolu s DB zálohami.** Jeho ztráta vyžaduje
znovu zadat heslo. Klíč nerotujte prostým přepsáním souboru; automatická rotace
není implementovaná. Bez dostupného klíče nelze uložené heslo použít.

Režim Konfigurace serveru zachovává dosavadní SMTP_* proměnné; Vypnuto pozastaví
čekající zprávy. Již probíhající zpráva se může odeslat. Změna serveru ovlivní
budoucí zpracování fronty; původní obsah a odesílatel uložených zpráv se nemění.
Migrace 016 zachovává starou konfiguraci jako výchozí a přidává testovací historii
v DB. Nová verze workeru je nutná na všech replikách; starý worker UI nastavení nezná.

## Vlastní přílohy dokladů

Faktury a nabídky mají vlastní soubory uložené přímo v PostgreSQL. Záloha a obnova
DB proto zahrnuje přílohy i odeslané historické verze; další filesystem mount
pro tuto funkci není nutný. Limit je 5 aktivních příloh na doklad, 2 MiB na soubor
a 256 MiB bytů na profil. Odstranění neodeslaného souboru uvolní místo; soubor
navázaný na e-mail zůstane zachován pro frontu a historii.

Proxy musí povolit JSON upload do 2 800 000 bajtů (base64 obálka souboru).
Přílohy se do e-mailu vybírají explicitně; poštovní poskytovatel může mít vlastní
nižší limit celé zprávy. Veřejné odkazy nabídek vlastní přílohy nevystavují.
Migrace 017 je aditivní, existující přílohy PDF/outbox nemění. Před použitím nové
funkce aktualizovat všechny backendy/workery, jinak starý worker vlastní soubory
nepřiloží. Přijaté soubory nejsou antivirově skenované ani vykonávané serverem.

## Pravidelné faktury

Plánovač P14-01 běží s API serverem. Pro automatizaci musí server a PostgreSQL běžet trvale; prohlížeč může být zavřený. Po výpadku doplní zmeškané termíny postupně. Nový vzor je pozastavený, aktivace samostatně potvrzuje volbu vystavení a případného odeslání. Pro PDF e-mail nastavte SMTP. Chyba generování zastaví vzor s viditelným stavem; po opravě obsahu/oprávnění/roku řady jej znovu aktivujte. Zařazení e-mailu při dočasné chybě zkouší znovu po 5 minutách, nejistý výsledek skutečného SMTP odeslání automaticky neopakuje. Pozastavení vzoru nemaže vytvořené doklady ani neruší již zařazené zprávy. Před upgradem zastavte staré API.

### Automatické upomínky a poděkování

Po konfiguraci SMTP otevřete nastavení fakturačního profilu → Upomínky a poděkování.
Vlastník nastaví odstupy a limit a výslovně potvrdí odesílání. Upomínky zahrnou
existující faktury po splatnosti. Poděkování sleduje jen nové ručně zapsané platby
od zapnutí. Obě volby jsou standardně vypnuté. API server musí běžet; prohlížeč
nemusí být otevřený. V detailu faktury lze automatizaci pozastavit. Historie
rozlišuje běžný e-mail, upomínku a poděkování, včetně selhání. Změna úhrady či
nastavení zastaví dosud nezahájené zprávy. Již předaný e-mail nelze odvolat.
[Omezení a přesná pravidla](specs/invoice-communication.md).

## Veřejné odkazy faktur

P14-03 používá stejné `PUBLIC_WEB_URL` jako veřejné nabídky. Nastavte veřejně
dostupnou HTTPS adresu webu, který dosáhne na API; localhost je jen pro vývoj.
SMTP je nutné pouze pro odeslání odkazu e-mailem. Samotné vytvoření a kopírování
funguje bez SMTP. Odkaz opravňuje držitele ke čtení PDF a stavu úhrady: nesdílejte
jej v logu ani veřejném screenshotu. Odvolání zneplatní i již odeslaný odkaz,
neodstraní stažené PDF. Před upgradem zastavte starý server; nový aplikuje
migraci 024. Nastavení automatických e-mailů tento upgrade nemění.
[Specifikace](specs/invoice-sharing.md).

## Evidence přijatých faktur

P15-01 nevyžaduje další službu. Před upgradem zastavte starý API proces;
nový aplikuje migraci 025 včetně rozšíření vazeb příloh. Existující doklady,
platby a přílohy zůstávají zachované. Originální soubory jsou v PostgreSQL
stejně jako ostatní přílohy; zahrnuje je běžná DB záloha. Ruční úhrada pouze
eviduje provedený výdaj, nepřipojuje banku a neposílá peníze. [Rozsah](specs/received-invoices.md).

## Účtenky a výdajové přehledy

Před nasazením P15-02 zastavte starý API proces; nový provede migraci 026.
Starý server účtenky nerozlišuje, proto nesmí běžet souběžně. Upgrade zachová
původní faktury, jejich číslování, opakování příkazů a přílohy. Není potřeba
další služba. Účtenka eviduje zaplacený nákup; nenahrazuje bankovní import
ani pokladní doklad. [Rozsah](specs/purchase-receipts.md).

### Bankovní evidence — P16-01

Před upgradem zastavte starý API proces. Nový provede migraci 027; starý proces
by nechránil storno úhrad propojených s bankou. Další služba ani bankovní přístupové
údaje nejsou potřeba. V aktivním profilu otevřete Finance a evidence → Banka,
přidejte CZK účet a zvolte import CSV nebo nový pohyb. Náhled musí být bez chyb;
duplicity se přeskočí. Potom otevřete pohyb a ručně vyberte doklad/úhradu.
Vlastní [formát CSV a hranice](specs/banking.md) nejsou nativní bankovní konektor.
Údaje i audit se zálohují společně s PostgreSQL. Žádné peníze se neposílají.

## Vlastní bankovní napojení

Po upgradu spusťte `node scripts/init-secrets.mjs` pro doplnění `bank-key`;
původní klíče se nepřepisují. API používá `BANK_ENCRYPTION_KEY_FILE` (výchozí
`.secrets/bank-key`), produkční Compose má vlastní secret. Klíč zahrňte do
chráněné zálohy; necommitujte jej. Bez něj nelze uložit ani použít bankovní token.

V Bance otevřete „Bankovní napojení“, vyberte účet a poskytovatele, uložte
vlastní token, spusťte test a následně synchronizaci. Fio je vestavěné; jiné
poskytovatele přidá správce přes `BANK_GATEWAYS_JSON` podle
[specifikace gateway](specs/bank-connections.md). Ta popisuje i omezení formátů,
intervalu, bezpečnost, obnovu přístupů a neověřené živé integrace.

Automatickou synchronizaci zapněte explicitně u konkrétního účtu a zvolte
interval. Worker běží s API, nepotřebuje cron. Po změně přístupů se vypne;
při odebrání oprávnění aktérovi se pozastaví a vyžaduje nové zapnutí.
Po delší odstávce se načte pouze posledních 30 dní, starší mezery doplňte
importem výpisů. Automatika neposílá platby ani je automaticky nepáruje.
