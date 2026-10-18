# Explicitní vstupy daňové evidence

Stav 18. 10. 2026. Vlastní návrhové rozhodnutí Open Faktury, nikoli doložená numerická shoda s iDokladem nebo elektronické daňové přiznání.

## Důkazy a hranice

Finanční správa v [informaci ke změnám od roku 2025](https://financnisprava.gov.cz/cs/financni-sprava/novinky/novinky-2025/informace-ke-zmenam-v-platcovstvi-dph-2025), ověřeno 18. 10. 2026, popisuje kalendářní rok a dva registrační limity. Z toho neplyne, že každý evidovaný výnos je zákonným obratem. Zdrojové doklady neobsahují úplnou klasifikaci místa plnění, osvobození, ekonomického použití ani podmínek nároku. Původní orientační přehled proto zůstává oddělený.

Nová evidence dovoluje uživateli výslovně určit zahrnutí celého původního čistého plnění do vlastního sledovaného obratu a částku uplatněné vstupní DPH s datem. Součty jsou označené jako uživatelské vstupy, nejsou rozhodnutím o splnění zákonných podmínek. Neobsahují řádky přiznání, kontrolního či souhrnného hlášení ani XML podání. Záporné opravy vstupního odpočtu, poměrné koeficienty, více období a částečný obrat na jednom dokladu zatím nejsou podporované; nulový nárok lze zapsat výslovně. Důvod je vždy povinný.

## Uložení a konzistence

Migrace 071 ukládá oddělenou firemní klasifikaci, revizi, aktéra, neměnný pozorovaný zdroj a audit. Před upgradem zastavit staré API a zálohovat databázi. Zápis nemění doklad, jeho kurz ani historickou DPH. Částky CZK pocházejí ze společných reportů a uložených kurzů, nikoli dnešního kurzovního lístku.

Otisk zahrnuje účetní obsah zdroje; provozní revize sama do něj nevstupuje. Běžná úhrada tedy klasifikaci nezneplatní. Změna obsahu nebo storno naopak označí klasifikaci jako zastaralou a vyřadí její částku ze součtu. Uživatel ji musí potvrdit nad novým otiskem. Opakování UUID vrátí původní rozhodnutí s aktuálním příznakem platnosti, nevytvoří další audit. Současná změna vyžaduje správnou revizi i otisk; přístup k cizí firmě je zakázaný, viewer pouze čte.

Nárok je nezáporná desetinná částka nejvýše evidované vstupní DPH, s datem nejdříve datum evidovaného plnění a nejpozději dnes. To je konzervativní aplikační omezení, nikoli implementace všech právních pravidel nároku. Souhrn používá jeden konzistentní databázový snapshot, přesné desetinné součty, počty nerozhodnutých a zastaralých zdrojů. Roční obrat se řídí datem původního plnění, vstupní nárok uživatelem zadaným datem uplatnění.

REST kontrakt `contracts/tax-classification.openapi.json`, MCP `tax_classification_get/set/summary` a web používají společný aplikační příkaz. Web nabízí klasifikaci u zdrojů daňového přehledu a samostatný roční souhrn. Úplnost podání nelze odvozovat ani z nulového počtu nerozhodnutých zdrojů: aplikace nemusí obsahovat všechny účetní události.

## Ověření

`DATABASE_URL=postgres://… CARGO_INCREMENTAL=0 cargo test -p faktura-server --test tax_classification -- --include-ignored --test-threads=1` používá izolovanou databázi a syntetické doklady. Ověřuje explicitní CZK nárok nad uloženým EUR kurzem, strop nároku, souběžný replay, role a izolaci firem, změnu/storno zdroje, nové potvrzení a audit. Syntetické výsledky nejsou referenčními výpočty Finanční správy ani iDokladu.
