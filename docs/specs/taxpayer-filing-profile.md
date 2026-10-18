# Explicitní údaje daňového subjektu pro EPO

Stav 18. 10. 2026, vlastní návrhové rozhodnutí Open Faktury. Navazuje na `statutory-xml-gaps.md`, není nativním podáním ani změnou plátcovství faktur.

## Doložené definice

Aktuální oficiální DPHDP3 XSD 03.01.04 definuje c_ufo jako nejvýše třímístné celé číslo a c_pracufo jako volitelné nejvýše čtyřmístné celé číslo. DIČ je jeho kmenová číselná část délky 1–10; uchováváme ji jako řetězec, včetně úvodních nul. typ_ds má dle dokumentace volby F/P (fyzická/právnická osoba; u skupiny typ zastupujícího člena), typ_platce P/I/S/N/R/D (plátce, identifikovaná osoba, skupina, neplátce §108, pořízení/dodání nového dopravního prostředku). Tyto kódy patří do EPO adaptéru; neztotožňují se s invoice tax_mode.

[Dokumentované rozhraní číselníků](https://mojedane.gov.cz/dpr/adis/idpr_pub/epo2_info/rozhrani_ciselniku.faces) umožňuje dotaz C a datum PL. Byly přímo staženy `https://mojedane.gov.cz/dpr/epo_ciselnik?C=ufo&PL=2026-10-18` a stejný dotaz pro pracufo. Snapshot `crates/application/assets/epo-offices.json` obsahuje zdrojové URL/verze/SHA256 a 16 platných úřadů, 124 pracovišť. Převod je doslovný výběr kódu, názvu a nadřízeného úřadu; neplatná pracoviště s d_zaniku se nepřenášejí. Klientský `epo-offices.ts` je odvozený ze stejného snapshotu. Ověření 18. 10. 2026, verze ufo 51.3.00670 a pracufo 51.3.00210. Při změně číselníku aktualizovat obě reprezentace a opakovat asociace.

Server ověřuje skutečnou existenci kódu ve snapshotu a vazbu pracoviště na zvolený úřad, nikoli pouze regex. Neověřuje přidělení DIČ ani místní příslušnost konkrétního poplatníka. Uživatel je zadává výslovně; úřad se neodhaduje z adresy. Nejsou ukládány rodné číslo navíc, plné moci ani autentizační údaje EPO. DIČ je neveřejný firemní obsah se stejnou izolací jako doklady.

## Persistované nastavení

Migrace 072 vytváří samostatný firemní profil, audit a UUID replay. Čtení vyžaduje členství, zápis editační roli; viewer pouze čte. První stav má revision=0,data=null. Nastavení podléhá reviznímu konfliktu, atomickému zápisu a idempotenci stejného příkazu. Replay ověřuje aktéra a celý původní vstup. Historická nastavení zůstávají v auditu; následné změny nesmí přepsat budoucí snapshot vydaného podání.

REST `/api/companies/{company}/reports/tax/filing-profile` GET/POST, MCP taxpayer_filing_profile_get/set a web používají společný příkaz. Kontrakt `taxpayer-filing-profile.openapi.json`; modal je dostupný u daňového přehledu, v obou jazycích. Nepodporuje výběr konkrétního období/formy podání, zastupování, výpočty přiznání ani jeho odeslání.

## Ověření

Izolovaný `tax_classification` integrační target obsahuje test taxpayer_profile_official_codes_replay_and_isolation: uložení identifikované fyzické osoby, zachování nul DIČ, validní pracoviště, chybná vazba úřadu/pracoviště, neexistující úřady, současný UUID replay, změněný replay, izolace firem, audit a shoda MCP dispatcheru. Před migrací zastavit staré API a zálohovat DB.
