# Firmy a nastavení

Fakturační profil obsahuje údaje firmy, účetní nastavení a oprávnění k její agendě. Níže najdete postup práce s profily a technické podrobnosti pro integrace. Specifikace a přijatá omezení jsou v [company-setup.md](specs/company-setup.md), aktuální rozsah aplikace v [přehledu funkcí](../FEATURES.md).

## Uživatelská cesta

Po přihlášení se jediný dostupný fakturační profil otevře přímo. U více profilů
se obnoví naposledy zvolený dostupný profil, jinak se zobrazí výběr. Přepínač
a přidání profilu jsou stále v produktové navigaci. Nehotové moduly jsou disabled. „Přidat fakturační profil“ vytvoří rozpracovaný profil s aktuálním
uživatelem jako vlastníkem. Jedna stránka obsahuje sekce Kdo fakturuje a Jak fakturujete;
kontakty, číslování a lokální nastavení jsou rozbalitelné. Živnostník je výchozí
fyzická osoba, další profily nejsou nutné.
„Uložit rozpracované“ zapisuje neúplný profil, „Uložit fakturační údaje“ ověří úplnost.
Po reloadu lze pokračovat v uložených údajích. Neodeslané změny nejsou autosave.
Přepnutí jazyka je zachová v paměti, přechod na jinou stránku je neukládá.

Dokončení vyžaduje validní základní profil. Firma potom otevře prázdnou agendu
bez demonstračních kontaktů/dokladů. Vlastník může nastavení upravit. Při konfliktu
revize se změny nepřepíšou; klient nabídne načtení aktuální verze. Daňová historie
zachovává původní profil a datum účinnosti. Číselná řada má pouze náhled, nikoli alokaci.

Podporována česká agenda, CZK, živnostník/fyzická osoba i právnická osoba, neplátce a plátce
DPH, IANA časová zóna, hotovost nebo převod s českým IBAN. Identifikovaná osoba,
zahraniční legislativa, více bankovních účtů a pokročilé číselné řady navážou později.
P07 umožňuje vystavení uloženého návrhu. `can_issue` signalizuje kompletní nastavení,
oprávnění owner/editor a nevyčerpané číslování; konkrétní doklad vždy prochází
samostatnou validací. Vystavení zvýší revizi profilu a čítač řady.

## Oprávnění

| Role | Seznam/čtení dostupné firmy | Změna nastavení |
| --- | --- | --- |
| owner | ano | ano |
| editor | ano | ne |
| viewer | ano | ne |
| správce instalace bez členství | ne | ne |

Pozvánky, přidávání účtů, změny rolí a převod vlastnictví nejsou v tomto UI.
Členství je perzistentní a kontroluje se při každém požadavku. Nepřístupné a
neexistující ID mají stejnou odpověď 404. Zámky členství při zápisu brání závodu
s jeho odebráním. RLS není zapnuté; aplikace nevystavuje přímý SQL přístup klientům.

## REST a migrace

Kontrakt `contracts/company.openapi.json`, klient `packages/api-client/src/company.ts`.
`pnpm generate:api` obnoví auth i firemní typy. Seznam a detail: GET `/api/companies`
a `/api/companies/{id}`. Založení: POST se `request_id` (UUID) a `settings`.
Úprava: PUT detailu s `revision`, `step`, `complete`, `settings`. Mutace používají
stejnou cookie a ochranu originu jako P02. Chyby obsahují stabilní `code` a případně
`fields`; CS/EN texty se skládají až v klientu.

Migrace `202610100003_companies.sql` je dopředná, vytváří pouze nové tabulky;
existující účty a relace zachová. Firma a vlastník vznikají v jedné transakci,
stejně tak změna nastavení + audit + změna daňové historie. Unikátní uživatel/UUID
brání duplicitnímu založení při retry. Nastavení defaultního účtu/číslování je
zatím typovaný JSON snapshot; před P07 normalizovat vazby potřebné pro doklady
a zavést transakční alokaci čísel. Starší server nové tabulky ignoruje, downgrade
schématu se neprovádí. Zálohování a obnova PostgreSQL zahrnují všechny nové tabulky.

## Lokální MCP

Oficiální Rust SDK `rmcp` 3.5.1; P03 nabízí stdio, nikoli veřejný HTTP endpoint.
Přihlašuje se skutečný existující uživatel. Heslo vložte do soukromého dočasného
souboru mimo repo; nepředávejte je na příkazové řádce ani v logu. Provozovatel
spouští adaptér s konfigurací DB stejně jako server.

```sh
# SESSION_FILE nesmí existovat; příkaz vypíše jen potvrzení, nikdy token.
faktura-server mcp-login EMAIL PASSWORD_FILE .secrets/mcp-session
faktura-server mcp .secrets/mcp-session
```

Soubor relace se na Unixu vytváří s 0600 a adaptér širší oprávnění odmítne.
Na Windows použít adresář s přístupem pouze pro příslušný OS účet. Heslový soubor
po přihlášení odstraňte. Relace expiruje za 12 hodin; recovery účtu ji okamžitě
zneplatní. Nové přihlášení vyžaduje nový cílový soubor. Nevkládat token do konfigurace
MCP klienta: argumentem je cesta k souboru. MCP stdout obsahuje pouze protokol,
provozní logy jdou na stderr.

Nástroje `companies_list` a `company_context` (`company_id` povinný) volají
stejné aplikační operace jako REST. Každé volání znovu ověřuje relaci a členství;
nejde o univerzální provozovatelský přístup. Přihlášení vzdáleného MCP klienta
přes HTTP/OAuth zůstává navazujícím integračním úkolem. SDK klient byl ověřen
v integračním testu, externí GUI klienty ani desktopový instalátor zde netestujeme.

## Ověření

- `DATABASE_URL=... pnpm test:companies`: izolované PostgreSQL testy migrace z P02,
  izolace firem a globálního admina, role, revokace členství, idempotence/souběh,
  revizní konflikty, audit, účinnost profilů, REST session/origin, MCP SDK klient
  a odvolání jeho relace. Potřebuje DB uživatele s CREATEDB, nikdy produkční URL.
- `pnpm exec playwright test tests/e2e/company.spec.ts`: skutečný Rust a samostatná
  dočasná DB pro každý test, CS/EN desktop/mobil, jednostránkový profil, reload, editace,
  dvě vlastní firmy, cizí firma, viewer a konkurenční změna, přístupnost.
- `pnpm check:locales`, build, rustfmt, Clippy a stávající regresní testy.

## Přepínání a mazání profilů

V horní liště otevřete přepínač „Fakturuji jako“. Volba profilu otevře jeho údaje;
Spravovat profily vede na seznam všech dostupných profilů. Přepínač zachová
aktivní kontext i na tomto seznamu. Sidebar odkazuje na nastavení aktivního profilu.
U neuložených úprav nabídne přepnutí zahození nebo návrat k formuláři.

Vlastník může profil smazat z jeho detailu nebo seznamu. Dialog vyžaduje opsání
názvu a vysvětluje dopad na ostatní členy. Profil se skryje z aplikace/API/MCP;
audit a původní data se zachovávají v DB. Obnovení v aplikaci není implementované.
Nasazení vyžaduje novou migraci a zastavení starých serverů, které deleted_at
neznají. Účty se nemažou. Uživatel s rolí editor/viewer smazání nemá dostupné.
