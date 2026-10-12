# P05 — model a kalkulátor návrhu faktury

11. 10. 2026. Technická implementace vlastní verze `of-czk-v1`.
**Není ověřena numerická shoda s iDokladem.** Q-01 zůstává otevřené pro
referenční srovnání. Výsledek je náhled, nikoli daňové posouzení nebo vystavení.

## Rozsah a přesná pravidla v1

- Pouze CZK, režim non_payer / vat_payer; sazby 0, 12, 21 jsou podporované
  hodnoty kalkulátoru, nikoli automatické určení sazby pro zboží nebo datum.
  Non-payer připouští pouze 0; rekapitulace uchovává režim, takže jej nezaměňuje
  se zdaněním nulovou sazbou. Historické sazby, osvobození, reverse charge,
  OSS, cizí měny, opravné doklady a zálohy čekají na další pravidla.
- Decimal (rust_decimal), žádné float. JSON vstupy jsou výhradně nezáporné
  desetinné řetězce s tečkou, bez exponentu a mezer. Množství 0–1 000 000,
  cena 0–1 000 000 000, nejvýše 4 desetinná místa. Sleva 0–100 %, 2 místa.
  Výstupní peníze mají vždy 2 místa. Záporný může být jen rozdíl zaokrouhlení.
- 0–500 položek; prázdný návrh může mít nulu a prázdný popis. ID položek
  neprázdná, unikátní a max. 64 bajtů; popis max. 2000 znaků, jednotka 30.
  Vystavení později přidá povinné položky, popis a ostatní náležitosti.
- Zdrojová cena řádku = množství × jednotková cena × (1 − položková sleva/100)
  × (1 − dokladová sleva/100). Mezikroky se na haléře nezaokrouhlují.
  Zdrojová cena se zaokrouhlí na 2 místa, poloviny od nuly.
- Net: zdrojová cena je základ, daň = round2(základ × sazba/100).
  Gross: zdrojová cena je celkem, základ = round2(celkem / (1+sazba/100)),
  daň je rozdíl. Součet základ + daň vždy odpovídá celkem.
- Rekapitulace a celkové částky jsou součet zaokrouhlených řádků, nikoli
  nový výpočet daně ze souhrnného základu. Žádné skryté rozdělování rozdílu.
- None ponechá haléře. Whole_czk matematicky zaokrouhlí částku k úhradě na
  koruny; samostatný rounding je payable − gross. Základ a daň se tím nemění.
  P07 musí omezit použití podle účelu a ověřených daňových pravidel.
- Verze pravidel je povinná, neznámá verze se odmítne; změna algoritmu vyžaduje
  novou verzi a nové očekávané výsledky. Net/gross je režim celého návrhu.

## Model a oprávnění

`domain::invoice` obsahuje vstup, řádky, výsledky a vlastněné kopie účastníků
(PartySnapshot), platebních údajů a DraftData. Bez odkazování na živou adresu
při renderu. P06 přidá persistenci a revize; P07 atomické přidělení čísla a
zmrazení vystaveného obsahu. Tyto životní cykly jsou návrh Open Faktury,
nikoli odpozorované chování iDokladu (Q-02/Q-03 nadále referenčně otevřené).

POST `/api/companies/{company}/invoices/calculate` vyžaduje relaci, povolený
Origin, X-Faktura-Request: 1 a členství aktivního profilu. Náhled může i viewer;
admin nemá bypass. Bez zápisu do DB, čísla, auditu vystavení nebo odeslání.
Tělo max. 2 MiB. Chyby 422 mají cesty např. lines.0.unit_price. Framework
odmítá neplatné JSON typy/enum a neznámá pole. No-store i pro chyby.
MCP `invoice_calculate` volá stejnou aplikační operaci s explicitním company_id.
Klient `calculateInvoice` přijímá AbortSignal pro ochranu před starou odpovědí
v P06; desetinné vstupy nesmí převádět přes Number.

## Ověření

Doménové testy obsahují ručně vypočtená očekávání, nikoli výstupy reference:
3 × 19.995 net/21 % → 59.99 + 12.60 = 72.59;
121 gross/21 %, slevy 10 % a 5 % → 85.50 + 17.96 = 103.46;
0.125 × 19.995 net/12 % → 2.50 + 0.30 = 2.80.
Testy kontrolují přesnost vstupů, maxima, duplicity, rekapitulaci, nulu a verzi.
Integrační test company používá skutečný PostgreSQL a MCP SDK, porovnává
celé JSON REST/MCP a kontroluje odmítnutí nepřihlášeného/cizího profilu.

Referenční R-01/R-02/R-03 nejsou splněné: nemáme pozorované náhledy, uložené
faktury a PDF iDokladu pro shodné vstupy. P05 proto nelze označit reference-verified.
