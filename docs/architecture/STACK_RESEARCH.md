# Stack, škálování a rozšiřitelnost Open Faktury

Průzkum: **10. 10. 2026**. Stav: **doporučení k rozhodnutí**, nikoli schválená implementační architektura. Rust pro backend a doménové jádro a PostgreSQL jsou již zvoleny uživatelem. React, Tailwind, atomic design a monorepo jsou rovněž potvrzeny; PWA je plánovaný směr. Novější [vývojové konvence](DEVELOPMENT_CONVENTIONS.md) mají přednost před návrhy v tomto průzkumu. Zdroje níže jsou primární dokumentace; doporučení a provozní scénáře jsou naše syntéza. Nebyl spuštěn prototyp ani benchmark a nejsou doloženy kapacitní sliby.

## Doporučení

Rozhodnutí po průzkumu, 10. 10. 2026: uživatel potvrdil Rust s tím, že úpravy budou provádět agenti. Bariéra pro lidské JS přispěvatele proto není důvodem měnit jazyk backendu. Níže uvedené alternativy zachováváme jako záznam porovnání; konkrétní Rust knihovny zůstávají doporučením. Pro agentní vývoj upřednostňujeme jasné moduly, typované kontrakty, krátkou zpětnou vazbu kompilátoru a reprodukovatelné testy.

React + TypeScript + Vite, TanStack Router/Query, Rust + Axum/Tokio, PostgreSQL + SQLx, REST/OpenAPI a oficiální Rust MCP SDK. Jeden modulární backend, samostatně spustitelný worker, PWA jako mobilní klient. Desktop: Tauri, potvrzeno uživatelem 10. 10. 2026; ověřit integračním prototypem.

Pro fork na agent workspace je klíčové oddělit účetní doménu od identity, oprávnění, příloh a úloh. Agentní běh má být volitelný modul nebo samostatný proces nad veřejnými operacemi. Pro jednoduchou fakturaci nemá být povinný LLM provider ani orchestrace agentů.

Rozhodovací kritéria v pořadí: správnost a dohledatelnost dokladů, snadná instalace a úpravy, stabilní rozhraní, měřitelné škálování, teprve pak samotná rychlost frameworku.

## 1. Frontend

| Volba | Posouzení pro tento projekt |
| --- | --- |
| React + Vite SPA | Doporučení. Samostatný statický klient pro Rust API, použitelný ve webu, PWA i desktopovém obalu. Produkční Node server pro samotné UI nepotřebujeme. |
| Next.js | Má smysl při potřebě SSR a serverových React funkcí. Zde už máme Rust backend a převážně přihlášenou aplikaci; přidával by další serverovou vrstvu. Není špatně škálovatelný, ale jeho přínos pro tuto aplikaci zatím nepřevažuje další provozní práci. |
| React Router vs TanStack Router | Preferujeme TanStack Router kvůli typovaným parametrům URL pro filtry a tabulky; React Router zůstává rozumnou alternativou. |

Vite podporuje React šablony. Next.js lze self-hostovat, ale při více instancích je třeba promyslet sdílení cache a související koordinaci. TanStack Router dokumentuje typovanou navigaci i vyhledávací parametry. [Vite](https://vite.dev/guide/), [Next.js self-hosting](https://nextjs.org/docs/app/guides/self-hosting), [TanStack Router](https://tanstack.com/router/latest/docs/overview).

TanStack Query pro serverová data; cache klíče musí zahrnovat firmu. Při odhlášení a změně identity cache vyčistit. Lokální formuláře držet lokálně, filtry v URL; nevytvářet druhou kopii serverových dat v globálním store. Query je knihovna pro asynchronní serverový stav, nikoli účetní zdroj pravdy. [TanStack Query](https://tanstack.com/query/latest/docs/framework/react/overview).

Pro UI doporučujeme upravitelné komponenty založené na shadcn/ui, design tokens a samostatný balíček UI bez závislosti na fakturaci. Výhodou pro fork je přímé vlastnictví komponentového kódu; cenou je vlastní údržba převzatých komponent. Výběr formulářové knihovny a tabulek uzavřít na prototypu faktury, ne přidáváním balíčků předem. [shadcn/ui](https://ui.shadcn.com/docs).

### PWA a desktop

PWA vyžaduje odpovídající manifest a bezpečný kontext; instalace se liší podle platformy. Instalovatelnost neřeší synchronizaci offline změn. První verze: online práce, cache statických assetů, jasný stav bez připojení; soukromé API odpovědi a doklady implicitně neukládat do service-worker cache. Aktualizace nesmí zahodit rozpracovaný formulář. [MDN](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Guides/Making_PWAs_installable).

Tauri používá systémový WebView a Rust; Electron přibaluje Chromium a Node.js. Po tomto porovnání uživatel 10. 10. 2026 zvolil Tauri. Před vydáním zbývá ověřit přihlášení, český tisk, stahování PDF a aktualizace na Windows/macOS/Linux. Electron zde zůstává pouze jako záznam posouzené alternativy. [Tauri](https://v2.tauri.app/concept/architecture/), [Electron](https://www.electronjs.org/docs/latest/tutorial/process-model).

Desktop je klient serveru. Lokální agent pracující se soubory by byl zvláštní komponentou s výslovně udělenými oprávněními; neznamená přesun účetní DB do desktopu.

## 2. Backend a data

| Varianta | Výhoda | Cena / doporučení |
| --- | --- | --- |
| Rust + Axum | Silné typy, async runtime Tokio, Tower middleware | Preferovaná varianta respektující směr uživatele; nutná znalost Rustu a vlastní skládání aplikačních komponent |
| Rust + Actix Web | Další zavedený Rust web framework | Relevantní alternativa; nemáme měření opravňující tvrdit, že Axum bude rychlejší |
| TypeScript + Fastify | Jeden jazyk s frontendem, jednodušší vstup pro JS přispěvatele | Posouzená, nevybraná alternativa; uživatel potvrdil Rust. Nižší bariéra je náš odhad, ne benchmark |

Axum dokumentuje integraci s Tokio/Tower, Actix i Fastify mají samostatné webové ekosystémy. Volbu Axum doporučujeme kvůli složení komponent a kompatibilitě s MCP SDK, ne na základě neprovedeného rychlostního srovnání. [Axum](https://docs.rs/axum/latest/axum/), [Actix](https://actix.rs/docs/), [Fastify](https://fastify.dev/docs/latest/).

### SQLx před ORM

SQLx umožňuje explicitní SQL, kontrolu statických dotazů při kompilaci a práci s migracemi. Doporučujeme jej pro účetní vazby, agregace a dobře viditelné transakce. Kontrola typů dotazu neověří oprávnění ani účetní správnost. Pro build bez běžící DB udržovat metadata offline dotazů a jejich aktuálnost kontrolovat v CI. Dynamické filtry psát parametrizovaně; názvy sloupců a sort směr vybírat z whitelistu. [SQLx](https://github.com/transact-rs/sqlx).

SeaORM je alternativa pro tým preferující entity a ORM abstrakce. Pro tento projekt zatím nevidíme důvod přidat ORM nad SQLx; přesné finanční dotazy a zámky stejně potřebují pozornost. [SeaORM](https://www.sea-ql.org/SeaORM/docs/index/).

PostgreSQL `NUMERIC` pro přesné hodnoty; Rust `rust_decimal` jako kandidát pro výpočty. Jejich rozsahy nejsou totožné. Nejprve určit limity množství, ceny, kurzu a mezivýsledků, potom nastavit SQL typy a checked operace. Nevybírat automaticky dvě desetinná místa pro všechno. JSON částky přenášet jako desetinné řetězce. [PostgreSQL numeric](https://www.postgresql.org/docs/current/datatype-numeric.html), [rust_decimal](https://docs.rs/rust_decimal/latest/rust_decimal/).

### Izolace firem

Pro první verzi společná databáze a schéma s `company_id`, složenými vazbami a aplikační autorizací. Navrhujeme doplnit PostgreSQL Row-Level Security jako další pojistku. Aplikační DB role nesmí být superuser, vlastník tabulek ani mít BYPASSRLS; migrace běží jinou rolí. PostgreSQL tyto výjimky z RLS výslovně popisuje. [RLS](https://www.postgresql.org/docs/current/ddl-rowsecurity.html).

Firemní kontext nastavovat pouze uvnitř transakce na získaném spojení a veškeré dotazy operace vést přes ni. Otestovat vrácení spojení do poolu, chybějící kontext a střídání firem. RLS nenahrazuje práva typu „smí vystavit fakturu“ a nechrání před plně kompromitovaným backendem, který nastavuje kontext. Samostatnou DB pro každou firmu nedoporučujeme jako výchozí provozní model.

## 3. API, MCP a agent workspace

REST/OpenAPI jako veřejný kontrakt. Kandidát: Rust DTO + `utoipa` → verzovaný OpenAPI artefakt → `openapi-typescript` a typovaný HTTP klient. Databázové modely nevystavovat přímo. Typy generované pro TypeScript nejsou runtime validace; vstupy musí ověřit backend. CI má kontrolovat změnu kontraktu a regeneraci klienta. [utoipa](https://docs.rs/utoipa/latest/utoipa/), [openapi-typescript](https://openapi-ts.dev/introduction).

MCP přes oficiální `rmcp`. SDK dokumentuje stdio a Streamable HTTP a serverovou integraci přes Tower. Preferovat Streamable HTTP pro server a volitelný stdio adaptér pro lokální nástroje; přesné verze připnout a integračně ověřit. REST i MCP volají stejné aplikační operace, aby se pravidla nerozcházela. [Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk).

MCP přihlašování nelze vyřešit jen existující webovou cookie. Aktuální autorizační specifikace řeší OAuth a kontrolu zamýšleného příjemce tokenu. OIDC login a kompatibilita autorizačního serveru s MCP jsou dvě samostatná ověření. [MCP authorization, revize 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization).

Navržené hranice:

- **Platforma:** identity, členství, oprávnění, audit, soubory, úlohy a události.
- **Fakturace:** firmy, kontakty, doklady, výpočty, platby, daně. Zachová vlastní silné typy a pravidla.
- **Volitelný workspace:** konverzace, běhy agentů, kroky, artefakty, použitá oprávnění a schválení.

`Workspace` není automaticky právnická osoba. Pro budoucí fork navrhujeme samostatný `workspace_id` pro pracovní prostředí a explicitně autorizované vazby na jednu či více `company_id`. Přístup do workspace sám neuděluje přístup do každé firmy. Zatím jde o návrh rozšíření, nikoli změnu stávajícího schématu.

U agentního běhu evidovat `run_id`, aktéra, případného delegujícího uživatele, firmu, scope, vstupy nástrojů, výsledky a nákladový limit. Dlouhé operace vracejí ID úlohy. Rozhraní nabídne návrh/náhled, vykonání a stav; citlivé úkony mohou podle nastavení vyžadovat schválení vázané na konkrétní vstup a revizi. Nová revize ruší platnost starého schválení. Každý zápis znovu kontroluje oprávnění, i po čekání ve frontě.

Agent nesmí spouštět libovolný kód uvnitř API procesu. Výkonné agenty provozovat odděleně s omezeným přístupem k souborům, síti a tajemstvím. Samotná fronta ani Docker automaticky netvoří dostatečnou izolaci nedůvěryhodného kódu. Fakturační jádro nepotřebuje konkrétní agent framework; fork může použít TypeScript/Python worker nad veřejným API.

Pro průběh běhu doporučujeme SSE, pro zadávání příkazů HTTP. SSE je jednosměrný stream; obousměrný terminál může později potřebovat WebSocket. Události ukládat s kurzorem pro navázání po reconnectu, ne jen vysílat z paměti jedné instance. [MDN SSE](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events).

## 4. Škálování krok za krokem

### První nasazení

Compose: reverzní proxy/TLS, aplikace, worker a PostgreSQL; renderer PDF a identita podle zvolené varianty. Aplikace může servírovat statický frontend. Jedno sestavení backendu může poskytovat samostatné režimy server/worker. Identita uživatele, rozpracované úlohy a podstatný stav nesmí žít pouze v paměti procesu.

Pro úlohy navrhujeme transakční outbox a databázovou frontu. Změna dokladu a zápis události proběhnou v jedné transakci. Worker krátce rezervuje práci, uloží lease a provádí externí volání až mimo DB transakci. Potřebuje timeout, retry s odstupem, konečný stav selhání a obnovu po pádu. `SKIP LOCKED` je PostgreSQL mechanismus vhodný pro více konzumentů fronty, nikoli hotový spolehlivý job systém. [PostgreSQL SELECT](https://www.postgresql.org/docs/current/sql-select.html).

Doručení úlohy je alespoň jednou; neopírat návrh o „exactly once“. Příjemce potřebuje deduplikaci. U e-mailu nebo externího API může po timeoutu zůstat výsledek nejasný; pokud poskytovatel nenabízí idempotenci, nelze garantovat nulové duplicity pouhým outboxem.

PDF generovat z uložené revize přes samostatný renderer, například Gotenberg s HTML/CSS šablonou. Je to snadno upravitelné pro fork, ale Chromium zvyšuje spotřebu a počet služeb. Renderer nemá dostat libovolné URL z neověřeného dokumentu. [Gotenberg](https://gotenberg.dev/).

Přílohy přes rozhraní blob storage: lokální volume pro jednu instalaci, S3 kompatibilní storage při více hostech. DB drží metadata, hash a oprávnění; filesystem není veřejný webový adresář. Sdílené úložiště souborů je podmínkou horizontálního provozu.

### Kdy co přidávat

| Měřený problém | Další krok | Co se tím nevyřeší |
| --- | --- | --- |
| Roste latence API a CPU, DB má rezervu | Více stateless API instancí za load balancerem | Pomale dotazy a zámky |
| Roste stáří nejstarší úlohy | Přidat workery; oddělit PDF, importy a agenty, zavést limity firmy | Limity externích služeb a DB |
| Čekání na DB spojení | Rozpočet poolů všech procesů, pak případně PgBouncer | Nedostatek výkonu DB |
| Nákladné přehledy | Indexy, profilování, agregované projekce; případně read replica | Čtení bez zpoždění po zápisu |
| Velký audit a historie běhů | Retence, archivace a podle měření partitioning | Chybné dotazy či autorizace |
| Fronta zatěžuje OLTP nebo workflows trvají dny | Vyhodnotit broker či Temporal za zachovaným job kontraktem | Automatické řešení vedlejších efektů |
| Jeden zákazník dominuje zátěži | Kvóty a plánování podle firmy; později izolované umístění | Nutnost zachovat datové vazby |

PgBouncer transaction pooling má omezení session funkcí a LISTEN; ten případně vyžaduje dedikované spojení. Připravené dotazy, RLS kontext a pooler musí projít společným testem. [PgBouncer](https://www.pgbouncer.org/features.html). Partitioning a repliky zavádět podle konkrétního profilu, nikoli podle počtu registrací. [PostgreSQL partitioning](https://www.postgresql.org/docs/current/ddl-partitioning.html). Temporal je kandidát pro trvanlivé workflow, ne počáteční povinná služba. [Temporal](https://docs.temporal.io/).

Zpočátku nepřidávat Kubernetes, Kafka, Redis, Elasticsearch ani vlastní plugin marketplace bez doložené potřeby. Mikroslužbu vyčlenit až když má samostatné provozní nároky a stabilní hranici. Rozdělení adresářů není důvod pro síťové volání.

### Měření a provozní kritéria

Prototyp musí měřit p50/p95/p99 API, chyby, zámky, dobu čekání na pool, délku fronty a dobu do zahájení úlohy, CPU/RAM rendereru, dobu kompilace a obnovy. Dataset: firmy s rozdílným počtem dokladů, více souběžných uživatelů, velký import, další běžná firma během importu. Přidat pád workeru po externím účinku, opakovaný příkaz a obnovení SSE.

Konkrétní cílové latence, objemy a hardware stanovit před měřením. Výstupem musí být reprodukovatelný benchmark a limit, nikoli tvrzení „Rust zvládne miliony uživatelů“.

Verzované migrace spouštět řízeně jednou. Pro více verzí aplikace používat postup přidat → převést → odstranit; nevracet destruktivní migraci automaticky s rollbackem binárky. Zálohování zahrnuje DB i přílohy a ověřenou obnovu do čisté instalace. PostgreSQL dokumentuje dumpy, fyzické zálohy a kontinuální archivaci; konkrétní strategii zvolit podle požadované ztráty dat a doby obnovy. [Backup and restore](https://www.postgresql.org/docs/current/backup.html).

## 5. Jak umožnit snadný fork

Fork musí zvládnout změnit branding, přidat stránku a nástroj bez zásahu do kalkulátoru. Veřejné kontrakty: aplikační příkazy, OpenAPI, události a UI komponenty. Vnitřní tabulky nejsou integrační API.

Preferujeme staticky registrované moduly ve zdrojovém kódu a externí integrace přes API/webhooky. Vlastní dynamické načítání Rust knihoven ani obecný low-code engine není součástí první architektury. Nový workflow může orchestrátor sestavit z existujících příkazů; změna finančního pravidla musí projít doménovou specifikací.

Navržený layout (dosud nevytvořen):

```text
apps/web/                     React aplikace; src/pages, components, hooks, services, utils
crates/domain/                peníze, doklady, pravidla; bez HTTP a SQLx
crates/application/           příkazy, oprávnění, transakční hranice a porty
crates/infrastructure/        PostgreSQL, soubory, e-mail, job repository
crates/server/                kompozice, REST a MCP adaptéry
crates/worker/                spouštění úloh nad aplikační vrstvou
packages/ui/                  sdílené UI a theme tokens
packages/api-client/          generované typy a HTTP klient
contracts/                    generovaný OpenAPI a verzované události
migrations/                   autoritativní SQL migrace
infra/                        Compose a příklady konfigurace
```

Moduly uvnitř vrstev členit podle domény; není nutný crate pro každou tabulku. Doména nezávisí na infrastruktuře, porty implementuje infrastruktura a vše spojuje server/worker. Port pro transakci musí umožnit atomicky uložit doklad, audit a outbox — samostatné autocommit repository metody nestačí.

Použít pnpm workspace pro TS a Cargo workspace pro Rust; nezavádět další monorepo orchestrátor, dokud nezrychlí změřený build. Lockfiles, toolchain a obrazy připnout; žádné produkční `latest`. Připravit jeden dokumentovaný vývojový vstup, syntetická demo data, CI, kontrolu generovaných kontraktů a krátký návod „přidej vlastní modul“. [pnpm](https://pnpm.io/workspaces), [Cargo](https://doc.rust-lang.org/cargo/reference/workspaces.html).

Agent workspace modul přidat až s konkrétním scénářem. Účetní produkt nebude podmíněn existencí chatových tabulek. Vlastní metadata modulu lze uložit verzovaně jako JSONB, ale částky, vazby a oprávnění zůstanou typované.

## 6. Identita, licence a otevřené volby

Pro přihlášení doporučujeme OIDC adaptér a oddělená aplikační oprávnění. Keycloak je ověřitelný self-hosted kandidát s OIDC, ale přidává službu a provozní nároky. PoC musí určit, zda jej dodáme v Compose jako výchozí, nebo zvolíme lehčího poskytovatele. Nevytvářet vlastní OAuth server jen kvůli snížení počtu kontejnerů. Web může používat backendovou session v HttpOnly cookie s CSRF ochranou, desktop systémový prohlížeč a PKCE; konkrétní klientské toky ještě otestovat. [Keycloak OIDC](https://www.keycloak.org/securing-apps/oidc-layers).

Pro snadné komerční i nekomerční forky je kandidátem Apache-2.0: umožňuje úpravy a redistribuci při dodržení podmínek a obsahuje patentový grant. To samo nezaručuje, že odvozené projekty zveřejní všechny změny. Licence zůstává samostatným produktovým rozhodnutím; před volbou projít také podmínky všech distribuovaných závislostí. [Text Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0).

## 7. Co ověřit před uzavřením stacku

| Ověření | Podmínka přijetí |
| --- | --- |
| React ↔ Axum ↔ PostgreSQL | Formulář, typovaný klient, serverová validace a transakce fungují z čistého checkoutu |
| Přesné výpočty | Referenční fixture a hraniční mezivýsledky projdou bez float konverze |
| Izolace firem | Negativní testy REST/MCP/příloh a recyklovaného spojení nepropustí cizí data |
| Idempotence a worker | Pád a opakování nevytvoří druhý doklad; nejasný externí účinek je viditelný |
| MCP a identita | Reálný klient projde přihlášením, scope kontrolou a odvoláním přístupu |
| PWA / desktop | Instalace na Android/iOS; desktopový tisk, login a download na cílových OS |
| Fork | Příklad přidá vlastní stránku a nástroj bez úprav účetního jádra |
| Provoz | Čistá instalace, migrace, backup/restore příloh i DB a změřená spotřeba |

Playwright umožňuje projekty pro více prohlížečů a konfigurací; nenahrazuje skutečnou zkoušku instalace PWA a systémového desktopového chování. [Playwright projects](https://playwright.dev/docs/test-projects).

Vývojové skills jsou nyní v `.agents/skills/`: `repo-structure`, `react-development`, `rust-backend`, `database-migrations`, `api-contracts`, `faktura-testing` a `desktop-release`. Mají odkazovat na přijatá rozhodnutí a skutečné příkazy. Tento průzkum je podklad; neinstaluje závislosti ani nezavádí předčasně scaffold aplikace.
