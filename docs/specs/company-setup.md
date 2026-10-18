# P03 — firmy a oprávnění

Specifikace `specified`, implementace `tested`, 10. 10. 2026.
SETUP-006–010. Podklad: veřejná [nápověda iDokladu](https://www.idoklad.cz/podpora/nastaveni-firma)
(ověřeno 10. 10. 2026) pro identitu, sídlo, oddělený firemní e-mail a profil DPH.
Níže je vlastní implementační kontrakt, nikoli tvrzení o interním chování iDokladu.

## Rozsah a invarianty

- Každý přihlášený uživatel může založit firmu a stává se jejím vlastníkem.
  Instalace nemá automatický přístup do cizí firmy. Role owner upravuje nastavení;
  editor/viewer je čtou. Pozvánky, předání vlastnictví a správa členů navazují později.
- Firma má interní bigint ID v API jako řetězec, revizi, stav draft/ready a uložený
  krok 0–3. Založení je idempotentní podle UUID klienta v rozsahu uživatele;
  opakování stejného klíče s jinými vstupy je 409. Aktualizace je compare-and-swap
  podle revize, stará revize vrátí 409; klient nabídne načtení aktuálního záznamu.
- Nastavení je typovaný JSON snapshot (žádná volná uživatelská metadata): název,
  typ subjektu individual/business, IČO/DIČ, české sídlo, kontaktní e-mail/telefon,
  daňový režim non_payer/vat_payer, datum účinnosti, CZK, IANA zóna, splatnost 0–365,
  metoda cash/bank_transfer, volitelný IBAN, prefix řady, rok a následující číslo.
  Prázdné volitelné řetězce znamenají nezadáno; nevracet null.
- Země CZ a měna CZK jsou explicitní podporované hodnoty. IČO není povinné pro
  individual; při zadání má 8 číslic (kontrola registru není součástí P03).
  IBAN se ověřuje podle země, délky, struktury a mod-97; od P16-13i podporujeme také zahraniční IBAN (knihovna iban 0.2.0). QR Platba má oddělené omezení CZ/CZK.
  Bez účtu je dostupná hotovost. Převod před dokončením účet vyžaduje.
- Rozpracované údaje lze uložit neúplné; neprázdné neplatné údaje jsou odmítnuty
  s kódem pole. Dokončení vyžaduje jméno, sídlo, IČO pro business, DIČ plátce,
  profil s datem účinnosti a validní platební nastavení. Pole invalid/missing jsou
  strojové kódy, klient je překládá. Krok nikdy nenahrazuje validaci backendu.
- Tax history přidává snapshot při změně profilu nebo data; staré revize se nemažou.
  Audit zachová každý snapshot nastavení, aktéra, akci a revizi v téže transakci.
  Číslování P03 je pouze konfigurace: prefix + rok + 6 míst, náhled nespotřebuje číslo.
  Skutečná alokace a ochrana již vystavených čísel navazuje v P07 před první fakturou.
- P07 vrací `can_issue` jako orientační připravenost nastavení a role; vlastní vystavení vždy znovu validuje snapshot konkrétního dokladu.
  Žádné demo doklady ani nefunkční CTA. Změna nastavení nebude přepisovat historické
  snapshoty dokladů, až budou existovat.

## API a MCP

GET/POST `/api/companies`, GET/PUT `/api/companies/{id}`. Cookie relace P02,
mutace mají kontrolu originu a x-faktura-request. Nepřístupné a neexistující ID
vrací shodně 404. Role a členství se kontrolují pro každý seznam/čtení/zápis;
při zápisu se členství i firma zamknou v transakci. Bez RLS: izolaci vynucují
aplikace a SQL port, negativní testy pokrývají oba adaptéry.

MCP P03 je autentizovaný lokální stdio server s oficiálním Rust SDK. Soubor relace
získaný explicitním přihlášením je chráněný OS oprávněními; každé volání znovu ověří
expiraci/revokaci. Nástroje companies_list a company_context(company_id) jsou pouze
pro čtení, volají stejné aplikační operace jako REST. Vzdálené HTTP OAuth není P03.

## INV-DEFAULT-DUE-01 — výchozí splatnost nových faktur (17. 10. 2026)

Vlastní UX Open Faktury: vlastník nastaví v záložce Fakturační nastavení
„Výchozí splatnost“. Společný výběr s editorem faktury nabízí Ihned,
7/14/21/30/60 dní. „Vlastní počet dní“ umožní zadat jinou hodnotu 0–365,
v souladu se stávající serverovou validací `settings.maturity_days`.
Uložené vlastní hodnoty zůstávají dostupné, výchozí hodnota nového profilu je 14.

Nová faktura nadále převezme uložený počet kalendářních dní od data vystavení;
Ihned odpovídá nule. Nepředvolený odstup se v editoru označí „Vlastní datum“.
Výběr kontaktu dál použije jeho `maturity_days`, případně výchozí nastavení firmy.
Změna firmy nemění uložené návrhy ani vystavené doklady. API, revizní kontrola,
oprávnění a uložení do databáze se nemění.

UI regrese `tests/e2e/redesign.spec.ts` se syntetickým API ověřuje uložení 30 dní,
Ihned a vlastních 19 dní, obnovení a správné datum/předvolbu v nové faktuře,
v CS/EN na desktopu a mobilu. Nenahrazuje databázové regresní testy P03/P06.

## Akceptace

Čistá a existující P02 DB přijme dopřednou migraci. Dva uživatelé/firmy nevidí
cizí ID ani seznamy; globální admin nemá výjimku. Viewer/editor nesmí měnit
nastavení, odebrané členství ihned přestane platit. Dvě souběžná založení se stejným
klíčem vytvoří jednu firmu; dvě revizní změny uspějí právě jednou. Audit a historie
vzniknou jen při úspěšném commitu. Průvodce přežije reload, otevření uložené firmy
obnoví krok; CS/EN přepnutí zachová rozpracovaná data. Test MCP přes SDK klienta.

## UX-P03-01 — osobní fakturace (10. 10. 2026)

Vlastní úprava na základě požadavku uživatele: živnostník s jednou fakturou měsíčně
nemá procházet firemním administračním průvodcem. UI používá „fakturační profil“,
zatímco interní company, členství a oprávnění zůstávají beze změny.

- Jednostránkový formulář místo čtyř kroků: Kdo fakturuje / Jak fakturujete.
- Kontakty jsou volitelné, číselná řada a lokální nastavení v rozbalitelné části.
  Chyba skrytého pole jeho sekci otevře; DPH a datum účinnosti zůstávají explicitní.
- Výchozí fyzická osoba zahrnuje živnostníka; business je v UI právnická osoba.
- Uložení rozpracovaného profilu i kompletního profilu používá stávající API.
  Klient ukládá step=0; starší rozepsané profily otevře se všemi uloženými údaji.
- Jeden profil se z kořenové stránky otevře přímo; více profilů ponechává výběr.
  Explicitní cesta /companies zobrazí seznam. Nepřístupné ID nesmí automaticky
  otevřít jiný profil ani ukázat jeho data.
- Přehled ukazuje jméno, adresu a platební/DPH režim. Úplná konfigurace a historie
  DPH jsou rozbalitelné, vlastník nevidí redundantní upozornění na vlastní roli.
- Faktury stále nejsou implementované: žádné smyšlené částky nebo aktivní tlačítko
  pro neexistující editor. Cílová domovská stránka faktur navazuje v P06.

Akceptace: tests/e2e/company.spec.ts (CS/EN, desktop/mobil, skutečné API/testovací DB,
reload, přímý návrat, souběžná editace, oprávnění, axe a přetečení). Vlastní UX návrh,
nikoli ověřená kopie onboarding toku iDokladu nebo Faktura Online.


## UX-P03-02 — plné rozhraní a více profilů (11. 10. 2026)

Výslovné rozhodnutí uživatele nahrazuje původní skrývání nehotových modulů.
Produktový shell zobrazuje navigaci (přehled, faktury, odběratelé, prodej, finance,
reporty, import, integrace, uživatelé) a akci Nová faktura. Neimplementované akce
jsou nativně disabled, doprovázené informací o vývoji; nepředstírají žádná data.
Fakturační údaje a správa profilů jsou aktivní. Mobil má rozbalovací navigaci.

Přepínač profilů a přidání dalšího jsou trvale dostupné, včetně jediného profilu.
Poslední úspěšně načtený profil se pamatuje v localStorage odděleně pro user ID;
při návratu se zvolí jen pokud je v aktuálním autorizovaném seznamu. Nedostupná
storage neblokuje aplikaci. Při chybě přímého ID se neotevře uložený jiný profil.
Explicitní /companies vždy umožňuje výběr ze všech dostupných profilů.
Data, oprávnění i revize zůstávají oddělené profilem. Faktury samotné stále nejsou
implementované. Ověření: company.spec.ts (přepínání dvou profilů, návrat, disabled
akce, izolace, CS/EN a mobil/desktop).

## UX-P03-03 — rozhraní podle vizuální reference iDokladu (11. 10. 2026)

Nahrazuje vizuální provedení UX-P03-01/02; reference E-012. Horní appbar obsahuje
profil, přidání profilu, jazyk a účet. Sidebar má hlavní oblasti a rozbalitelné
podsekce; disabled stav samotných operací zůstává. Formulář je souvislá pracovní
plocha s přepínači Základní údaje / Fakturační nastavení / Další nastavení.
Logo a podpis je disabled. Přepínače používají sdílené FilterTabs s aria-pressed,
nejde o nový čtyřkrokový průvodce. Ukládá se celý profil; přepnutí nic neukládá
ani nezahazuje. První chyba z backendu automaticky otevře příslušnou sekci.

Ověření: company.spec.ts ověřuje automatické otevření chybné sekce, dokončení,
přepnutí profilů a zachování oprávnění v CS/EN na desktopu i mobilu. Screenshoty
p03-company-* se přegenerují z izolované testovací DB. Sandbox zůstává vývojový;
změna se týká produktového shellu, nikoli nové identity nebo doménových kontraktů.

## Uživatelské menu v appbaru — 11. 10. 2026

Přihlášený účet má avatar odvozený z prvních dvou znaků e-mailu. Rozbalovací
panel obsahuje e-mail, disabled Nastavení účtu (dosud neimplementované), jazyk
CS/EN a skutečné odhlášení. Jazyk na přihlašovací stránce zůstává přímo dostupný.
Panel je disclosure s aria-expanded a pojmenovaným regionem; tabulátor prochází
akce, Escape zavře panel a vrátí focus avataru, kliknutí mimo jej také zavře.
Volba jazyka zachová formulář i uloženou preferenci. AccountMenu je lokální
molekula aplikace; neobsahuje vlastní autentizační API. Ověření: auth/company
E2E v obou jazycích a desktop/mobil, snímky docs/design/account-menu-*.

## UX-P03-04 — profilový kontext a smazání (11. 10. 2026)

Vlastní návrh Open Faktury podle zpětné vazby uživatele, nikoli převzetí přesného
mazacího workflow iDokladu. Appbar má rozbalovací seznam profilů s aktivním stavem,
akcemi Spravovat profily a Přidat profil. Sidebar obsahuje pouze nastavení
aktivního profilu; správa všech profilů je globální stránka /companies. Ta ukazuje
název, typ, roli, rozpracovanost a smazání pro owner. Aktivní kontext v appbaru
zůstane zachován i na správě profilů. Přepnutí s rozepsanými změnami vyžaduje
potvrzení zahození; zrušení zachová formulář.

DELETE /api/companies/{id} s JSON {revision} vyžaduje session, Origin,
X-Faktura-Request a owner pod zámkem company + membership. Bez členství 404,
viewer/editor 403, nesprávná revize 409. Úspěch 204; opakované mazání již smazaného
profilu jeho vlastníkem je 204 bez dalšího auditu. UI vyžaduje opsání názvu.

Migrace 202610110004 přidává deleted_at, rozšiřuje audit o delete. Jde o logické
smazání: nastavení, členství a historie zůstávají v DB; aktivní seznam, detail,
update a MCP kontext profil nevrátí. Opakování původního create klíče profil
neoživí. Starý server by deleted_at ignoroval, proto během nasazení nesmí souběžně
běžet stará verze; downgrade aplikace po prvním smazání není podporovaný.
Obnova ani fyzický purge zatím nemají UI/API. Před navázáním dalších dokumentových
modulů musí jejich přístupové kontroly respektovat deleted_at.

Testy: company Rust integrační scénář mazání (včetně REST originu, rolí, revize,
replay a auditu), company E2E (CS/EN, mobil/desktop: přepnutí, zrušení dialogu,
opsání názvu a zmizení profilu). Migrace je aditivní; nedotýká se existujících
nastavení ani přihlašovacích účtů.
