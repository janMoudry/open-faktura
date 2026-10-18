# Návrh další implementace: omezený tuzemský DPHDP3

Návrh k 18. 10. 2026, dosud neimplementovaný. Rozsah směřuje k nativnímu pracovnímu XML DPHDP3 pro výslovně potvrzenou jednoduchou agendu. Není to přejmenování CSV ani tvrzení univerzální daňové správnosti. Oficiální kontrakt a SHA256 viz `statutory-xml-gaps.md` a `../research/epo-schema-inventory.json`.

## První dodávka: klasifikace a náhled, poté generátor

Samostatný modul `tax_filing`, rezervace nové migrace až po koordinačním checkpointu. Entity:

- FilingHeader: taxpayer profile snapshot včetně revize, rok, právě měsíc nebo čtvrtletí, formulář a verze, druh podání B (řádné), datum sestavení. V1 pouze typ_platce=P a české DIČ z výslovného profilu. Identifikované osoby a skupiny zůstávají uložitelným profilem, ale nejsou podporované tímto generátorem.
- SourceAssignment: kind/id/otisk, skutečné datum daňové evidence, explicitní účetním potvrzená kategorie domestic_output nebo domestic_input_full, případně excluded s důvodem. Rozpis jednotlivých základů a DPH podle uložené sazby a alokace nároku; nepřebírat scalar claimed_input_vat_czk jako rozpis podle sazeb. Zahrnutí do obratu je samostatné rozhodnutí.
- FilingPreview: přehled všech zdrojů, nepřiřazených, zastaralých a blokujících případů, přesné částky před zaokrouhlením, výsledné atributy XML a doložené vzorce. Excluded zdroje nejsou tiše přeskočené: ukázat počet, hodnoty a důvody.
- FilingSnapshot: neměnná hlavička, plné zdrojové snapshoty/otisky, uživatelská rozhodnutí, výsledné řádky a byty XML s hashem. Export nepodává dokument. Nový export nad změněnými zdroji je nová revize; původní artefakt nezměnit.

REST/MCP/web sdílejí příkazy assign/preview/finalize/export. Finalize vyžaduje poslední fingerprint náhledu, UUID replay a souhlas uživatele, že zahrnuté zdroje i výslovně vyloučené případy zkontroloval. Tento souhlas není náhradou chybějící validace. Oprávnění, firemní vazby, revize, audit a uzamčená období sdílet s P20.

## Úzce podporovaný zdrojový rozsah

V1 pouze běžné vystavené CZK faktury bez záloh a vyúčtování a přijaté CZK faktury s výslovně plným nárokem na odpočet. Sazby 21 a 12 jsou uložené údaje, nikoli přepočet historické sazby. Příslušnost k tuzemskému standardnímu režimu musí určit uživatel; adresa, české DIČ, sazba a RWS/VIES ji nedokládají.

Přítomnost DDPP, vyúčtování, dobropisu, samostatného pokladního plnění, prodejky, cizí měny, historické jiné sazby, reverse charge, zahraničního plnění, zvláštního režimu, osvobození, kráceného/poměrného nároku či majetkových korekcí blokuje finalize. Pouhé excluded s důvodem nesmí obejít nepodporovaný režim; vyloučení je možné jen u doložených podporovaných případů, například běžného vstupu bez uplatněného nároku. Neznámé nebo změněné přiřazení vždy blokuje. Žádné implicitní zatřídění podle zdrojového kind ani rate=0.

U vstupů nelze z plného source total odvodit daňový základ nároku při částečném nároku. Proto V1 podporuje pouze explicitní plný nárok nad přesně uloženým základem a DPH; částečný odpočet z existující klasifikace je blokující.

## Přesné nativní mapování z aktuálního XSD

| Zdroj / výsledek | DPHDP3 atribut |
| --- | --- |
| Standardní výstup 21: základ / DPH | Veta1.obrat23 / dan23 |
| Standardní výstup 12: základ / DPH | Veta1.obrat5 / dan5 |
| Tuzemský vstup 21: základ / plný nárok | Veta4.odp_tuz23 / odp_tuz23_nar |
| Tuzemský vstup 12: základ / plný nárok | Veta4.odp_tuz5 / odp_tuz5_nar |
| Součet plného nároku ř.40–45 (ve V1 pouze ř.40–41) | Veta4.odp_sum_nar |
| Daň na výstupu, při absenci všech dalších řádků | Veta6.dan_zocelk = dan23 + dan5 |
| Uplatněný odpočet, bez krácení a úprav | Veta6.odp_zocelk = odp_sum_nar |
| Kladný rozdíl výstup − vstup | Veta6.dano_da |
| Kladný rozdíl vstup − výstup | Veta6.dano_no |

Názvy obrat23/obrat5 jsou historické názvy XML atributů, nejsou interním identifikátorem sazby 23/5. DPHDP3 dokument=DP3, k_uladis=DPH; správný version atribut ověřit přímo kořenem oficiálního XSD při implementaci. Žádné daně nepočítat násobením sazby: číst uložené skutečné daňové částky.

## Zaokrouhlení: přesná hranice před generátorem

Aktuální XSD pro uvedené částky vyžaduje desetinné číslo s fractionDigits=0 (nejvýše 14 číslic). To samo neurčuje právní okamžik/metodu zaokrouhlení. Před přidáním automatického zaokrouhlení doložit aktuální pokyn a referenční příklady, zvláště součet řádků vs. jednotlivých dokladů, záporná půlkoruna a nulový rozdíl. Bez tohoto důkazu smí první generátor přijmout jen případ, kdy všechny výsledné agregované základy a DPH jsou již přesně celá CZK; jakákoli nenulová desetinná část blokuje export. Nevydávat obvyklé Decimal.round za zákonné pravidlo. Uživatel nesmí pouhým potvrzením obejít blokaci neimplementovaného zaokrouhlení.

## Validace a akceptační testy

Před vydáním generátoru uložit oficiální XSD s licenční/provenienční poznámkou nebo reprodukovatelným ověřeným stažením; testovat `xmllint --nonet --schema` nad syntetickými výstupy a negativními fixture. Každý syntetický expected výsledek se musí opírat o uvedené mapování, nikoli o stejné produkční funkce.

DB scénář: explicitní CZK21+12 prodej/nákup, celé agregované částky, výsledek daň/přeplatek/nula; snapshot zůstane neměnný po změně zdroje; preview změnu odhalí. Negativní scénáře: neúplná klasifikace, DDPP, částečný nárok, cizí měna, desetinný výsledek, zastaralý otisk, cizí firma, viewer, souběžné finalize a opakování UUID. UI CS/EN musí ukazovat blokující zdroj i důvod. XSD validita není věcnou kontrolou EPO ani důkazem úplnosti agendy.

KH a SH nejsou vedlejší výstup tohoto generátoru. KH potřebuje vlastní A/B klasifikace, evidenční čísla dodavatele, rozhodná data, limity a poměr; SH vlastní EU kódy a agregace. Přidat je až se samostatnými doloženými pravidly a testy. Předchozí explicitní klasifikace a taxpayer profil jsou využitelné předpoklady, nikoli již hotový formulář.
