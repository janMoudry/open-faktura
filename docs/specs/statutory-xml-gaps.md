# Oficiální XML DPH, KH a SH: ověřený kontrakt a zbývající data

Ověřeno 18. 10. 2026 přímým stažením oficiálních XSD, nikoli odhadem z účetního programu. [Seznam struktur EPO](https://mojedane.gov.cz/dpr/adis/idpr_pub/epo2_info/popis_struktury_seznam.faces) uvádí DPHDP3 03.01.04, DPHKH1 03.01.15 a DPHSHV 02.01.05 ze dne 11. 8. 2026. Strojově čitelný inventář URL, SHA256, velikostí a všech atributů jednotlivých vět je v `docs/research/epo-schema-inventory.json`. XSD jsou samostatná, neimportují další schémata. Žádný z uložených metadatových souborů není vzorovým daňovým podáním.

## Společná hlavička

Všechny tři struktury vyžadují VetaD a VetaP. Povinné atributy zahrnují dokument, k_uladis, rok, druh podání (dapdph_forma/khdph_forma/shvies_forma), c_ufo, dic a typ_ds; DPH navíc typ_platce. Profil obsahuje část identity, ale neověřený převod právní formy na typ_ds ani správce daně a pracoviště. Chybí explicitní období podání, řádné/opravné/dodatečné/následné podání, datum zjištění a návaznost na původní podání. Nelze je doplnit výchozími odhady podle adresy.

## Mapování věcných vstupů

| Podání / věty | Dostupný podklad | Chybějící doložený vstup nebo pravidlo |
| --- | --- | --- |
| DPHDP3 Veta1/Veta2 | uložené sazby, native částky a historický kurz vydaných dokladů | právní klasifikace tuzemského plnění, pořízení/dovozu, reverse charge, osvobození a jednotlivých řádků; samotná sazba ani VIES platnost tuto klasifikaci neurčuje |
| DPHDP3 Veta4 | explicitní částka uživatelského nároku v CZK | rozpis základu a nároku podle sazeb a sloupců plný/krácený, investiční majetek, korekce; scalar claimed_input_vat_czk nestačí pro odp_tuz23/5, odp_tuz23_nar/5_nar a další atributy |
| DPHDP3 Veta5/Veta6 | orientační součty | koeficienty a jejich vypořádání, korekce/úpravy odpočtu, výsledné řádky a zákonné zaokrouhlení |
| DPHKH1 A1/B1 | doklady s textovou poznámkou a uloženými částkami | explicitní režim přenesené povinnosti, kod_pred_pl, správný duzp a rozpad příslušné části; nelze odhadnout z nulové DPH |
| DPHKH1 A2/A3 | část identity partnerů | zahraniční režim, právní datum, investiční zlato, zvláštní identifikace; běžný kontakt nedokládá zařazení |
| DPHKH1 A4/B2 | snapshot partnera/čísla/data a položkové sazby tam, kde existují | kompletní klasifikace režimu, c_evid_dd dodavatele (nikoli naše interní číslo), dppd, pomer a zdph_44; výslovná alokace částečného nároku do sazeb |
| DPHKH1 A5/B3/C | orientační součty všech zdrojů | doložené zařazení zdroje do agregované/detailní části, zvláštní případy limitů a oprav, odpovídající kontrolní součty |
| DPHSHV VetaR | EU DIČ kontaktu, měna, částky | k_pln_eu (právní druh dodání), rozhodné období, členský stát odběratele pro konkrétní plnění, pln_pocet a agregace/correction k_storno; lookup VIES je pouze evidence registrace |
| DPHSHV VetaS | žádný samostatný call-off-stock model | k_cos, původní/nové DIČ a změny skladu ve zvláštním režimu |

Průchod XSD potvrzuje strukturu a lexikální omezení, nikoli úplnost účetnictví, správnou daňovou kvalifikaci ani úspěšnou věcnou kontrolu EPO. [Finanční správa k XML KH](https://financnisprava.gov.cz/cs/dane/dane/dan-z-pridane-hodnoty/kontrolni-hlaseni-dph/struktura-pro-podani-kontrolniho-hlaseni-xml) výslovně poskytuje XSD pro testování exportů. Žádné podání nebylo odesláno ani nahráno do EPO.

## Přijaté rozhraní před navazující implementací

Klasifikace `tax-classification.md` zůstává explicitním uživatelským vstupem a její CSV/XLSX podklady nejsou nativním podáním. Nativní generátor potřebuje samostatný model hlavičky podání, klasifikaci řádků/plnění a alokace nároků, snapshot vstupů, formální i věcná pravidla a testy nad oficiálním schématem. Vydané formuláře musí mít revize a následné opravy oddělené od proměnlivého reportu. Záznamům se nesmí domyslet sazby nebo partner podle současného kontaktu. Tato tabulka konkretizuje zbývající práci; není tvrzením, že celý daňový modul je dokončený.
