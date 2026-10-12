# P12-02 / P12-03 — šablony a cenové nabídky

Podklad E025: živé UI iDokladu 11. 10. 2026, neplátce CZK; seznam, editor,
pět stavů nabídky a šablona se splatností ve dnech. E026 dokládá uložení a předvyplněný editor faktury.

Implementační pravidla Open Faktury (neprohlašovat za interní pravidla iDokladu):
- Profilově izolované šablony a nabídky, vlastník/editor zapisuje, čtenář čte.
- Šablona má název, splatnost 0–365 dní a vlastní kopii obsahu faktury. Použití
  obnoví datum vystavení/splatnosti a vymaže variabilní symbol. Zdroj se nemění.
- Nabídka má popis, odběratele, vystavení/platnost, položky, součty a stav
  created/sent/ordered/invoiced/rejected. Změna stavu sent nic neodesílá.
- Číslo CN<rok><pořadí alespoň 4 číslice> se přiděluje atomicky při uložení
  nabídky v profilu/roce. Šablony nemají účetní čísla.
- Převod nabídky založí nejvýše jeden rozpracovaný doklad s vlastním snapshotem,
  který uživatel zkontroluje a vystaví standardním příkazem. Odkaz je uložen
  v téže transakci. Vystavení navázané faktury automaticky zobrazí stav invoiced; ručně evidovaný
  stav nabídky sám o sobě žádnou fakturu nevystavuje.
- Revize chrání před přepsáním, UUID před opakováním vytvoření/převodu.
- Archivace je vratná. Historické kopie se při úpravě zdroje nemění.
- Výpočty a hranice CZK jsou stejné jako P05, nikoli nová JS aritmetika.

Akceptace: CRUD, hledání/stavy/archiv/stránkování, použití šablony, kopie částek
při převodu, souběžný převod bez duplicit, role/profilová izolace, REST/MCP,
CS/EN a kompaktní editor na desktopu/mobilu. Zálohová faktura a e-mail patří
navazujícím modulům; tento příkaz je nevytváří ani neodesílá.


## Ověření implementace 11. 10. 2026

`crates/server/tests/sales.rs`: 4 izolované PostgreSQL scénáře (číslování,
souběh a opakování převodu, reálné vystavení navazující faktury, role/profily,
archivace, snapshot, REST/MCP parita, upgrade z 011 bez změny existující faktury).
`tests/e2e/sales.spec.ts`: CS/EN × desktop/mobile, šablona → nabídka → návrh,
uložená vazba, profilová izolace a kontrola závažných a11y chyb seznamu.
Regresní `tests/e2e/invoices.spec.ts`: 4 scénáře. Vlastní částka 2 400 Kč je
zároveň doložená jednoduchým neplátcovským příkladem E-026; obecná numerická
shoda s iDokladem tím potvrzena není.

## QUOTE-DELIVERY-01 — PDF, tisk a e-mail

Reference E-027, implementace vlastních transakčních pravidel:

- `GET sales/{id}/pdf`: čtenář/editor/vlastník aktivního profilu může vygenerovat
  PDF uložené nabídky v jazyce dokladu (CS/EN), včetně archivované. Nealokuje číslo
  faktury, neposílá zprávu. Náhled/tisk používá stejné PDF. Šablona PDF nemá.
- `POST sales/{id}/deliveries`: editor/vlastník, aktivní nabídka, shodná revize,
  UUID požadavku, jeden platný příjemce, předmět 1–200 znaků, tělo do 20 kB.
  Odesílatel je serverová konfigurace; Reply-To je uložený e-mail dodavatele.
  Bez SMTP je příkaz odmítnut. UI nenabízí falešné odeslání.
- PDF se renderuje mimo transakci, maximálně dva rendery současně. Před uložením
  se znovu ověří role a zamčená revize. PDF, snapshot, příjemce, obsah, autor,
  stabilní Message-ID a příkaz se uloží atomicky do `quote_delivery`.
  Tentýž UUID/autor/vstup vrací tutéž historii; jiné použití UUID vrací konflikt.
- HTTP server zpracovává DB frontu; procesy se koordinují `SKIP LOCKED`.
  SMTP I/O běží mimo DB transakci s celkovým limitem 45 s. TLS/STARTTLS,
  nešifrovaný transport pouze výslovně pro loopback lokální capture.
- Stavy: queued → sending → sent / failed / uncertain. `sent` znamená kladné
  potvrzení SMTP serveru, nikoli doručení či přečtení. Odmítnutí SMTP je failed;
  timeout/nejasná síťová chyba uncertain. Opuštěný sending po 2 minutách přejde
  na uncertain. Žádné automatické opakování nejasného výsledku. Nové vědomé
  odeslání má nový UUID a vlastní historický záznam.
- Při úspěchu změníme created → sent, jen pokud se mezitím nezměnila revize,
  archivace ani vazba na fakturu. Změna dokumentu a audit jsou atomické s uložením
  výsledku. Historie odeslání nealokuje revizi nabídky; vlastní záznam je auditem
  požadavku. Stav odeslání je vidět i při pozdější úpravě nabídky.
- `GET sales/{id}/deliveries?page=1`: 25 zpráv, nejnovější první, vždy pod profilem.
  `GET sales/{id}/deliveries/{delivery}/pdf`: přesné neměnné bajty přílohy.
  Historie obsahuje autora, revizi, adresy, předmět/tělo, čas požadavku a dokončení,
  stav a bezpečný kód chyby. Neobsahuje SMTP hesla ani surové serverové chyby.
- REST a MCP sdílejí příkazy. MCP `sales_send` skutečně zařazuje e-mail;
  klient/agent vyžaduje uživatelský pokyn k odeslání. Běžící HTTP server je worker
  i pro MCP požadavky. Bez něj fronta čeká. PDF MCP vrací base64.

Akceptace: izolované PostgreSQL testy `crates/server/tests/delivery.rs` (upgrade,
role/profily, opakování/souběh, MIME příloha, neměnnost, SMTP odmítnutí, restart
rozpracovaného pokusu, REST/MCP/CSRF) a `tests/e2e/sales.spec.ts` (CS/EN,
desktop/mobil, download, náhled, lokální SMTP, historie). PDF fixture:
`output/pdf/quotation-cs.pdf`, `output/pdf/quotation-en-multipage.pdf`.

Veřejné odkazy a tracking doplňuje QUOTE-SHARE-01 níže. Mimo tento krok: více příjemců/CC/BCC,
globální historie všech typů dokladů, editor SMTP v UI, vlastní soubory v příloze.

## QUOTE-SHARE-01 — veřejný odkaz a historie zobrazení

Reference E-028 doplňuje QUOTE-DELIVERY-01. Implementace:

- Vlastník/editor vytváří odkaz k aktivní nabídce se shodnou revizí, UUID
  požadavku a expirací 1–365 dní (UI 30). Opakování téhož příkazu vrací
  tentýž odkaz; jiný obsah UUID je konflikt. Čtenář vidí odkazy/historii,
  ale nemůže vytvořit ani zrušit odkaz.
- Náhodný 256bit token je ve fragmentu URL, API jej přijímá v POST JSON těle.
  SHA-256 slouží k vyhledání; plná důvěrná URL je také uložena pro opakované
  kopírování a replay. Databáze/zálohy tedy obsahují přístupové údaje.
  Nejde o „pouze hash“ úložiště. Hlavičky no-store/no-referrer/noindex.
- Veřejný náhled nevyžaduje relaci; ukazuje aktuální uloženou nabídku a stav,
  CS/EN podle dokladu s přepínačem UI. Nabízí PDF/tisk. Nesděluje interní
  ID profilu/dokladu/faktury/autora, contact_id je null a interní note prázdná.
  PDF popis obsahuje veřejný název nabídky, nikoli interní poznámku.
- Revoked/expired/archived/deleted/unknown vrací shodně 404 bez obsahu.
  Obnovená archivovaná nabídka opět zpřístupní dosud platné odkazy.
  Zrušení odkazu je idempotentní a trvalé. Již stažený soubor nebo zobrazený
  obsah v prohlížeči nelze zpětně odebrat.
- Zobrazení se zapisuje samostatným POST až po vykreslení ve viditelné kartě.
  UUID návštěvy deduplikuje StrictMode/opakování požadavku. Nové načtení stránky
  má nové UUID. Resolve/PDF/HEAD se samy nepočítají. Čas určuje DB, revizi
  předává klient (kladná, nejvýše aktuální); nejde o ověření identity či čtení.
  Bez IP/cookie/fingerprintu. Nejvýše 60 uložených událostí za hodinu a odkaz;
  další návštěvy jsou dostupné, ale nezvyšují historii. Toto je ochranný limit,
  nikoli úplná analytika nebo obecný HTTP rate limiter.
- Detail: počet/první/poslední zobrazení, historie po 25 záznamech; odkazy
  samostatně po 25. „Obnovit“ načte aktuální statistiku. Nezaměňovat s potvrzením
  SMTP. Sdílený/přeposlaný odkaz nerozlišuje příjemce.
- SendQuote má volitelné share_id. Ověří stejný profil/nabídku, aktivní odkaz;
  server připojí URL k textu, SMTP neobsahuje PDF přílohu. Historický snapshot/PDF
  zůstává v DB pro audit. Bez share_id zůstává dosavadní PDF příloha.
  Historie uvádí způsob odeslání; originální UUID před migrací 014 lze opakovat.
- Worker před claimem ukončí queued zprávy s neplatným odkazem jako failed
  (link_unavailable). Rozběhnuté SMTP už nelze vzít zpět, ale veřejný přístup
  kontroluje platnost při každém čtení. Neděláme SMTP pod DB zámkem.
- REST authed /sales/{id}/shares, /{share}/revoke a /{share}/views sdílí
  aplikaci s MCP sales_sharecreate/list/revoke/views (skutečné názvy
  sales_sharecreate, sales_sharelist, sales_sharerevoke, sales_shareviews).
  Public POST /api/public/quotes/{resolve,pdf,view} je v samostatném OpenAPI.
  MCP vytváření odkazu uděluje přístup držiteli URL; samotné vytvoření
  nikomu neposílá zprávu.

Ověření: test:shares (PostgreSQL, upgrade 013→014, role/profily, souběh/replay,
revokace/expirace/archiv/smazaný profil, aktuální obsah, omezení/deduplikace
zobrazení, REST/MCP/CSRF, lokální SMTP bez přílohy), test:delivery regrese;
sales.spec.ts CS/EN desktop/mobil s anonymním náhledem a zrušením odkazu.
Mimo rozsah: diskuse, potvrzení nabídky příjemcem, identifikace čtenáře,
globální analytika a pixelové ověření veřejného iDoklad náhledu.
