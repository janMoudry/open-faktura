# Instalace a první spuštění

Návrh Open Faktury, 10. 10. 2026. Specifikace: `open` (pracovní kontrakt);
implementace: P02 `tested` pro SETUP-001 až 005 a základ SETUP-012;
P03 `tested` pro SETUP-006–010 v rozsahu [firemního kontraktu](company-setup.md); P11 dále `not-started`. Konkrétní auth rozhodnutí a omezení viz
[provozní kontrakt P02](../INSTALLATION.md). Nejde o reverse-engineering iDokladu.
Balíky [P02/P03/P11](../IMPLEMENTATION_PLAN.md) rozpracují tento návrh do konkrétních
endpointů, migrací a bezpečnostních testů. Lokální vývojový Compose není produkční instalace.

## Tři oddělené vrstvy konfigurace

| Vrstva | Kdo a kde | Obsah |
| --- | --- | --- |
| Instalace | Provozovatel, environment/secret files/CLI | DB, veřejná URL a povolené originy, klíče relací a šifrování, souborové úložiště, limity, HTTPS/proxy, zálohy, případně výchozí SMTP |
| Firma | Vlastník/oprávněný člen v aplikaci | Identita, adresy, profil DPH a jeho účinnost, měna, časová zóna, splatnost, číselné řady, bankovní účty, branding, komunikační preference |
| Uživatel | Přihlášený uživatel | Jméno, jazyk, vzhled, notifikace, heslo, 2FA a relace |

DB heslo ani serverové klíče nikdy nepřenášet do React buildu. Veřejná konfigurace
obsahuje pouze nezbytné schopnosti a verzi API. Nastavení SMTP v UI smí tajemství
zapsat/nahradit, nikoli vracet původní hodnotu. Přesnou prioritu firemního a
instalačního SMTP rozhodnout v P10; výchozí je explicitní firemní volba nebo
povolený instalační profil. Tajemství a zálohované klíče mají vlastní provozní návod.

## Cesta nového provozovatele

1. Zvolí verzi, nastaví produkční konfiguraci a spustí Compose. Dostane přehled
   chybějících povinných položek; žádné implicitní veřejné dev heslo.
2. Server ověří DB/migrace/úložiště. Provozovatel získá jednorázové bootstrap
   tajemství lokálně přes CLI/secret file; nepřidávat je do URL, logu ani analytiky.
3. Na setup obrazovce prokáže přístup tajemstvím a vytvoří správce s heslem.
   Bootstrap endpoint transakčně vytvoří účet a uzavře instalaci. Správce získá
   běžnou relaci až po úspěšném dokončení. Veřejná registrace je vypnutá.
4. Založí první firmu. Tentýž člověk může být správcem i vlastníkem firmy,
   ale jde o oddělená oprávnění.
5. Zkontroluje nastavení a otevře prázdnou agendu s akcemi „Přidat kontakt“,
   „Vytvořit fakturu“ a po P09 „Importovat existující data“.
6. SMTP, import, logo, pozvánky a integrace jsou přeskočitelné. Bez nich lze
   ručně vystavit a stáhnout fakturu v podporovaném rozsahu.

Veřejné API stavu setupu vrací jen stav potřebný k navigaci, nikoli jméno správce,
konfiguraci nebo informace DB. Nedostupná DB vede na „Služba není dostupná“, nikdy
na založení nové instalace. Nulový počet firem není důkaz nedokončeného bootstrapu.

## Průvodce firmou

| Krok | Vstupy a chování |
| --- | --- |
| Identita | Název/jméno, země, sídlo, typ subjektu, IČO/DIČ podle příslušného režimu, kontaktní údaje; fyzická osoba bez IČO nesmí být odmítnuta univerzální validací |
| Účetní profil | Podporovaný daňový režim a datum účinnosti, základní měna, časová zóna; české výchozí hodnoty nabídnout k potvrzení, nikoli odvodit ze jména |
| Platby a číslování | Výchozí splatnost a metoda úhrady, volitelný účet; pro převod validovat účet. Formát řady, období a následující číslo s náhledem, který nespotřebuje číslo |
| Kontrola | Souhrn, chybějící údaje, potvrzení a přechod do agendy. Nepodporované režimy nelze aktivovat, dokud nejsou specifikované a implementované |

Účetní nastavení se ukládá jako rozpracované; definitivní připravenost k vystavení
ověřuje backend. Změna kroku nesmí znovu založit firmu. V P03 není import součástí
transakce založení firmy; pozdější import je samostatná dohledatelná dávka.
Změny identity/profilu po vystavení nepřepisují snapshoty historických faktur.
Změna číselné řady kontroluje existující čísla, včetně budoucí migrace.

## Požadavky a akceptace

| ID | Požadavek | Akceptační scénář |
| --- | --- | --- |
| SETUP-001 | Validovaná instalace bez výchozího hesla | Chybějící povinný secret zastaví start s bezpečnou zprávou; restart zachová data |
| SETUP-002 | Jednorázový bootstrap chráněný tajemstvím | Neplatné tajemství odmítnuto; dva současné validní požadavky vytvoří právě jednoho prvního správce; starý secret po dokončení nefunguje |
| SETUP-003 | Transakční stav instalace | Pád před commitem lze opakovat, po commitu nelze bootstrap znovu otevřít; žádná částečně vytvořená identita |
| SETUP-004 | Přihlášení, odhlášení a obnova | Zneplatněná relace již nefunguje; recovery bez SMTP vyžaduje přístup provozovatele a zneplatní dotčené relace; recovery není veřejný setup |
| SETUP-005 | Minimální veřejný stav a diagnostika | Výpadek DB vrátí chybu služby, ne `setup_required`; logy neobsahují tajemství |
| SETUP-006 | Obnovitelný průvodce firmou | Obnovení stránky/přihlášení naváže na uložený krok; opakovaný zápis nevytvoří duplicitní firmu |
| SETUP-007 | Oddělené instalační a firemní role | Správce bez členství nečte doklady firmy; člen jiné firmy nemění její nastavení |
| SETUP-008 | Prázdný produkční workspace | Nová firma nemá demo faktury, kontakty ani falešné částky; dostupné CTA vedou na implementovanou funkci |
| SETUP-009 | Volitelné externí služby | Bez SMTP/ARES/AI se dá založit firma i kontakt; odpojení integrace nemaže účetní data |
| SETUP-010 | Připravenost účetního profilu | Neúplný profil lze uložit, nelze s ním vystavit; UI ukáže konkrétní chybějící nastavení; historický doklad po změně nastavení zůstane stejný |
| SETUP-011 | Desktopový výběr serveru | Neplatná URL či nekompatibilní API mají řešitelnou chybu; odhlášení/přepnutí serveru odstraní starý kontext a nezpřístupní jeho data novému serveru |
| SETUP-012 | Obnova a upgrade | Zálohu DB i souborů obnovím do čisté kompatibilní instance a porovnám data; selhání upgradu má zdokumentovaný návrat ze zálohy |

Před veřejnou betou doplnit scénáře resetu hesla přes vlastní SMTP, pozvánek,
2FA/recovery, zneplatnění tokenů a odebrání členství. První vlastník firmy nesmí
odejít bez vyřešení vlastnictví; detail v pozdější specifikaci správy členství.

## Model a migrační dopad — návrh

- `InstallationState`: jediný serverový záznam se stavem bootstrapu a časem
  dokončení. Správcovské oprávnění je explicitní vazba na globálního uživatele,
  nikoli role uvnitř libovolné firmy. Stav nesmí odvozovat od prázdnoty tabulky firem.
- `UserCredential`, `Session`, recovery tokeny: globální identity a revokace;
  hash hesla/tokenů, expirace, žádná tajemství v auditu. Konkrétní auth schéma
  a knihovny rozhodnout před implementací P02.
- `Company` + `Membership`: existující koncept modelu. Přidat verzovaný
  postup onboardingu a účetní nastavení. Uložená fáze je pomocná pro UI;
  autorizaci ani validaci vystavení nesmí nahrazovat.
- `CompanyBankAccount`, `NumberSeries`, `AuditEvent`: navazují na
  [datový model](../DATA_MODEL.md); firemní vazby ověřovat i v DB.
- Každá změna má dopřednou SQLx migraci a test prázdné i existující instance.
  Žádná automatická demo seed data v produkčních migracích.

## Rozhodnutí před implementací

Upřesnit auth a úložiště relací web/Tauri, hashovací parametry, expirace/rotaci
bootstrap secretu, obnovu správce, schéma rolí a přesná pravidla povinných údajů
prvního účetního profilu. Jde o navazující technická rozhodnutí v P00/P02/P03,
nikoli důvod čekat s oddělením sandboxu. Licence projektu zůstává rozhodnutím
vlastníka před veřejným vydáním; průvodce nevyžaduje aktivaci u provozovatele projektu.

## Osobní použití a desktop

Upřesnění podle požadavku uživatele 10. 10. 2026. Počet uživatelů a způsob
instalace jsou dvě různé věci: i jednotlivec může provozovat vlastní server.
Následující zjednodušený osobní režim je **návrh, nikoli hotová funkce P02**.

| Režim | Cílový první průchod | Instalační klíč |
| --- | --- | --- |
| Vlastní server, jeden nebo více uživatelů | Provozovatel připraví server → jednou založí správce → uživatelé se přihlašují | Ruční zadání jen při prvním založení správce |
| Desktop připojený k existujícímu serveru | „Připojit se k serveru“ → adresa → přihlášení → firma | Běžný uživatel jej nepotřebuje; nový server má oddělený instalační tok |
| Osobní desktop na tomto počítači | „Používat na tomto počítači“ → lokální úložiště a profil → firma | Lokální instalátor zajistí důvěryhodné předání, uživatel nic ručně nekopíruje |

Osobní desktop nemá vyžadovat hledání provozovatele ani ruční práci s Dockerem.
Před implementací však musíme rozhodnout balení, start/stop a aktualizace lokálního
Rust serveru a PostgreSQL, vlastnictví dat v profilu OS, zálohu a obnovu při
odinstalaci. Zachováváme zvolený stack; SQLite se tímto návrhem nezavádí.

Důvěryhodný lokální bootstrap nesmí být veřejný endpoint bez klíče jen proto,
že klient tvrdí „desktop“ nebo požadavek přišel z loopbacku. Návrh musí řešit
předání jednorázového tajemství přes chráněný lokální kanál, vazbu na OS profil,
oprávnění ostatních procesů a oddělení dalších uživatelů počítače. Zda osobní
režim vyžaduje vlastní heslo nebo odemykání přes OS, zůstává rozhodnutím této
specifikace; automatické přihlášení ani offline práci zatím neslibujeme.

### Dílčí dodávky PERSONAL-01 (plánované)

1. Specifikace správy lokálních služeb a dat, bezpečného bootstrapu a přihlášení
   podle OS; instalační experiment na podporované platformě.
2. Desktopový výběr „tento počítač / server“, ověření kompatibility a jasný stav
   připojení. Nefunkční osobní režim v aktuálním klientu nenabízet jako hotový.
3. Osobní instalátor: runtime, první start, profil, firma a obnova po přerušení.
4. Test restartu, souběžného spuštění, cizího OS uživatele, ztráty spojení,
   zálohy/obnovy, aktualizace a odinstalace bez nečekané ztráty dat.

Dokončeno až když čistý počítač bez předchozího Dockeru/DB projde podporovanou
osobní instalací bez kopírování klíče. Serverová instalace a její ochrany tím
zůstávají zachované. P03 může mezitím dodat společný firemní průvodce.
