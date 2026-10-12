# Vydané faktury — první specifikace

Specifikace: `open`. Implementace: `partial` — P05 vlastní verzovaný kalkulátor a model, P06 [persistované návrhy](invoice-drafts.md); viz [pravidla](invoice-calculation.md). Rozsah: FEATURES §1–3 a §10; tato verze pokrývá jen základ vydané faktury. Zdroje E-NNN a otázky Q-NN jsou v [evidenci](../research/EVIDENCE.md), scénáře v [průzkumu](../research/SCENARIOS.md).

| Požadavek | Kontrakt / navržené chování | Důkaz | Akceptace |
| --- | --- | --- | --- |
| INV-001 | Partner, měna, číselná řada, způsob úhrady, datum vystavení/splatnosti/zdanění a neprázdné položky | E-001; povinnost interních polí ještě rozhodnout | Validní vstup projde; chybný vstup vrátí chybu konkrétního pole, R-04 |
| INV-002 | Položka s množstvím, jednotkovou cenou, typem ceny, slevou a DPH | E-002, E-003; Q-01 | Přepočet odpovídá referenčním fixture R-01 až R-03 |
| INV-003 | Oddělené stavy dokladu a úhrady, prodlení jako odvozený údaj | E-004 + náš návrh; Q-02, Q-04 | Částečná úhrada nepřepíše životní cyklus, R-07 |
| INV-004 | Jedinečné číslo v definovaném firemním rozsahu, transakční přidělení | Návrh; Q-02 | Souběžné vystavení nezíská stejné číslo, R-05 + vlastní souběhový test |
| INV-005 | Snapshot účastníků, položek a platebních údajů | Návrh; Q-03 | Pozdější změna adresáře nepřepíše vystavený obsah; reference R-06 |
| INV-006 | Přepočet dostupný bez vystavení | E-006 | Stejný vstup a verze pravidel dávají stejné součty přes web/API/MCP |
| INV-007 | Vazby na zálohy a odpočtové položky | E-007; Q-06 | R-09; plná specifikace před balíkem P13 |
| INV-008 | Opakované vystavení se stejným klíčem nevyrobí druhý doklad | Návrh Open Faktury | Stejný klíč a vstup vrátí původní výsledek; jiný vstup s týmž klíčem konflikt |
| INV-009 | PDF odpovídá uložené revizi a měně dokladu | Návrh; veřejný rozsah FEATURES §2 | Částky, účastníci, číslo a data se shodují s uloženým detailem |

## Operace a chyby

Navržené aplikační operace: `CreateDraft`, `UpdateDraft`, `CalculateInvoice`, `IssueInvoice`, `GetInvoice`, `RenderInvoice`, `RecordPayment`. Později `CorrectInvoice`, operace koše a záloh. Názvy jsou interní návrh, nikoli iDoklad endpointy.

Příkaz obsahuje firmu, aktéra, vstup a u zápisu identifikátor požadavku; aktualizace také očekávanou revizi. Čtení i zápis ověřují přístup k firmě a všem předaným vazbám. Vystavení atomicky přidělí číslo, uloží snapshot, součty a audit. Odeslání je samostatný příkaz.

Chyby rozlišují neplatné pole, nepovolený přechod, konflikt revize/čísla, chybějící oprávnění a neznámý číselník. Konkrétní HTTP mapování zvolit při návrhu API. Důvěrné údaje cizí firmy nesmí uniknout přes chybu.

## Neuzavřená pravidla

Neznáme přesné pořadí slev a zaokrouhlování, okamžik přidělení čísla v referenci, povolené změny již vystavené faktury ani význam všech daňových režimů. Proto zatím nelze napsat referenční očekávané součty či kompletní přechodovou tabulku. SDK anotace splatnosti a délek polí lze použít jako kandidáty, serverové chování je nutné ověřit.

## P07 — implementovaný podporovaný rozsah

INV-004/005/008/009 mají vlastní ověřenou implementaci popsanou v
[invoice-issue.md](invoice-issue.md): atomické vystavení uložené revize,
číslování v profilu, audit, zákaz přepsání, CS/EN detail a PDF. Dřívější otevřené
body výše se týkají referenční shody a dalších daňových/opravných scénářů;
netvrdí, že aplikace stále nemá žádné vystavení.
