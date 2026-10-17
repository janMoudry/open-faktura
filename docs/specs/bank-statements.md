# P16-02 — bankovní výpisy a návrhy párování

13. 10. 2026. Navazuje na [banku](banking.md), důkazy E-042.
Implementace podle veřejných kontraktů, nikoli potvrzení všech skutečných exportů.

## Rozsah adaptérů

| Banka / služba | Volba importu | Hranice |
| --- | --- | --- |
| Air Bank podnikatelský účet | XML camt.053 | CZK zaúčtované jednotlivé pohyby, bankovní reference |
| Česká spořitelna George Business | XML camt.053 | SEPA XML výpis; běžný George CSV/JSON není tento formát |
| ČSOB CEB | XML camt.053 | ČBA XML, nikoli historický proprietární XML |
| Komerční banka | XML camt.053 | Bankovní AcctSvcrRef; MsgId/pořadí není stabilní ID |
| UniCredit firemní bankovnictví | XML camt.053 | Výpis, nikoli camt.052 přehled |
| Fio | JSON accountStatement | Export dle API kontraktu 16. 10. 2025; žádný API token |
| MONETA Internet Banka | XML statement | DTD revize 1.22; jen výslovný credit/debit marker |
| Raiffeisenbank | ABO ediční GPC | Původní soubor s koncovkou _CZK.gpc, účty v edičním pořadí |

Tabulka je mapou doložených formátů a implementovaných adaptérů, **ne certifikací
bankovních exportů**. Testovací soubory `tests/fixtures/bank` jsou syntetické.
Anonymizované originály ze všech bank stále chybí. MONETA DTD neurčuje lexikální
podobu data ani znaménka částky: přijímáme ISO datum nebo DD.MM.YYYY a vyžadujeme
jednoznačný credit/debit marker; ostatní varianty odmítneme. To je naše konzervativní
pravidlo, nikoli doklad plné kompatibility. Účty ostatních bank mohou použít
vlastní CSV Open Faktury; jejich nativní varianty nejsou automaticky podporované.

## Importní kontrakt a ochrany

`BankStatement`: account_id, format, filename, content_base64. REST
`POST /bank/statement-preview`, MCP bank_statement_preview jsou read-only.
`bank_execute action=statement` potvrzuje stejný obsah přes společný příkaz.
Původní CSV kontrakt zůstává kompatibilní. Zápis vyžaduje owner/editor,
preview je přístupný členům profilu; všechny entity jsou scoped company_id.

Limit 1 MiB dekódovaných dat, 1000 pohybů. XML má omezení hloubky/počtu uzlů,
neprovádí externí I/O ani expanzi vlastních entit. Externí DTD deklarace MONETY
je inertní; interní subset je odmítnut. UTF-8, pro MONETA/RB také Windows-1250.
JSON částky používají arbitrary_precision a desetinnou aritmetiku, nikoli f64.

Kontrolujeme účet výpisu proti zvolenému IBAN, CZK, datum, přesnost a stabilní
bankovní ID. camt používá AcctSvcrRef, Fio column22, MONETA transaction.id,
RB číslo dokladu. Žádné odvození ID z částky, VS nebo pořadí řádku.
Hromadné camt položky s více TxDtls, čekající pohyby, chybějící reference a
změněné/částečné RB položky odmítáme. Storno banky je nový podepsaný pohyb,
nepřepisuje historické párování. RB přebírá měnu z dokumentované přípony názvu;
soubor nepřejmenovávat. Dvoumístný RB rok vykládáme jako 2000–2099.

Náhled ukáže nové/duplicitní/konfliktní řádky. Zápis znovu ověří vše pod společným
profilem a transakčním zámkem. Jeden chybný řádek zablokuje celý import. Idempotence
UUID a unikátní (profil, účet, externí ID) chrání opakování i souběh.
Po prvním importu musí účet používat stejný formát; ruční pohyby tomu nebrání.
Změna formátu by mohla změnit bankovní ID a způsobit duplicitní úhrady. Toto
omezení je vlastní návrh, nikoli převzatý limit iDokladu.

## Návrhy párování

GET `/bank/movements/{id}/suggestions`, MCP bank_suggestions, žádný zápis.
VS je porovnán po odstranění úvodních nul (neprázdný, samé nuly jsou hodnota 0).
Vyhledává se v celém profilu před limitem, max. 100 dokladů se shodným VS;
překročení vrátí chybu, nikoli falešně jednoznačnou shodu z první stránky.
Následuje stejná kontrola částky, měny, stavu a data jako u ručního párování.
Doklad s vhodnou již zaznamenanou úhradou nabízí její připojení místo další úhrady.
Existující přesná úhrada nebo plná úhrada zůstatku má přednost před částečnou.
Při více výsledcích UI výslovně oznámí nejednoznačnost. Uživatel vždy vybere a
potvrdí konkrétní doklad; zápis znovu kontroluje revize a oprávnění.

Žádné automatické hromadné potvrzení, přeplatky, haléřové dorovnání ani párování
účtenek podle VS. Účtenky lze dál připojit ručně. Chybějící VS nabízí ruční hledání.
Návrhy nemění saldo a import neodesílá platební příkazy.

## Migrace a akceptace

Migrace 028 pouze rozšiřuje povolený zdroj bankovního pohybu. Před upgradem
zastavit starý API proces. Historická data a UUID zůstávají beze změny.

- Syntetické kontrakty každého adaptéru, kladné/záporné částky, haléře a nuly VS.
- Nesprávný účet, neznámá varianta, batch, XXE, chybná přesnost: odmítnout.
- Izolovaná DB: náhled nic nezapíše; opakování/souběh nezduplikuje pohyby;
  konflikt obsahu a mix formátů neprojde; jiné profily nemají přístup.
- Návrhy: shoda, nejednoznačnost, existující platba, jiné profily, žádný zápis.
- REST/MCP sdílejí příkazy; CS/EN desktop/mobile import a potvrzení návrhu.
