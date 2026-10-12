# Referenční scénáře

R-01 až R-06 a R-08/R-09 jsou **připravené, nespouštěné**. R-07 byl 11. 10. 2026
částečně proveden: 300 Kč → úhrada 100 Kč → doplatek 200 Kč, neplátce CZK;
viz E-021 a screenshoty. Přeplatek, odpojení, nulový a opravný doklad neověřeny. Očekávané částky doplnit z reference, nevypočítat vlastním budoucím kalkulátorem. Konkrétní daňové nastavení a sazby zachytit z testovací agendy; zde se neurčují právní pravidla.

Před každým během zachytit legislativu, režim DPH, měnu, nastavení desetinných míst, roli, verzi rozhraní a datum. Používat syntetické kontakty v testovací agendě. U výpočtů uložit vstup a odpověď přepočtu; pokud se ověřuje vystavení, také detail uloženého dokladu a PDF. Anonymizované artefakty patří do `docs/research/fixtures/<scenario>/<run>/` až po skutečném běhu.

| ID | Vstup a postup | Co zachytit |
| --- | --- | --- |
| R-01 | Jedna a pak tři položky, množství 3 a 0.125, cena 0.01 a 19.995; režim ceny s DPH i bez DPH; více povolených sazeb | Základ/DPH/celkem po položkách i sazbách; rozdíl náhledu, uložení a PDF |
| R-02 | Stejná faktura bez slevy, s položkovou slevou 10 %, s dokladovou 5 %, s oběma; hraniční hodnoty zvlášť | Pořadí aplikace, základ slevy, zaokrouhlení a zamítnuté hodnoty |
| R-03 | Cizí měna, kurz 25.123 s množstvím kurzu 1 a 100, více položek | Směr přepočtu, částky v obou měnách, přesnost a uložený kurz |
| R-04 | Uložit nový doklad, změnit, kopírovat, smazat a obnovit; zopakovat s úhradou a exportem | Stavy, přidělení čísla, dovolené operace, chybové odpovědi a vazby |
| R-05 | V testovací agendě dva souběžné pokusy o stejné číslo; opakovat identický požadavek | Zda vzniknou duplicity, konflikt, rezervace nebo mezera v řadě; bez tvrzení, že reference podporuje idempotency key |
| R-06 | Vystavit doklad a změnit adresu kontaktu, banku firmy i cenu ceníkové položky | Rozdíly uloženého detailu a nově generovaného PDF; zda existuje explicitní aktualizace údajů |
| R-07 | Kladná faktura: část úhrady, doplatek, přeplatek, odpojení; zvlášť nulový a opravný doklad | Stav úhrady, zbývající částka, datum úhrady, vazba na finanční pohyb a případné chyby |
| R-08 | Dvě testovací firmy, majitel a omezený uživatel; číst a měnit doklad i přílohu přes UI/API | Dostupné role, reakce na cizí ID, rozsah přístupového tokenu |
| R-09 | Dvě zálohy s částečnou úhradou, daňový doklad k platbě, konečná faktura; poté pokus o změnu zdroje | Povolené kombinace, odpočty, vazby, sazby, rozdíly SDK a webu |

Náš akceptační výsledek může být přísnější než reference (např. ochrana proti duplicitnímu požadavku). Takový rozdíl má status vlastního návrhu, nikoli důkazu funkční shody.
