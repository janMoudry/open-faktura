# P16-01 — bankovní účty, import a ruční párování

13. 10. 2026, E-041. Tuzemské CZK účty a pohyby, náš interní návrh.
Reference dokládá ruční výběr dokladu a oddělení pohybu/úhrady, ne náš CSV formát.

## Účty a import

Profilové účty: název, kontrolovaný IBAN, CZK, revize, archivace/obnova.
IBAN je v profilu jedinečný a po prvním pohybu neměnný. Účet se nemaže;
historie zůstává. Účet nemění platební snapshoty faktur ani výchozí IBAN profilu.
Nejde o připojení k bance a aplikace nikdy nezadává platební příkazy.

CSV UTF-8 (volitelný BOM), čárka nebo středník, hlavička
external_id,booked_on,amount,currency,variable_symbol,counterparty,description.
Datum YYYY-MM-DD, podepsaná nenulová částka s tečkou a max. 2 desetinnými místy,
kladná příjem/záporná výdaj, CZK, VS nejvýše 10 číslic. Limit 1 MiB/1000 řádků.
Částky max. 999999999999.99, data 1900 až dnešek. Žádné odhady locale nebo kurzu.
ID bankovní transakce je povinné; nezaměňovat s pořadím řádku nebo VS.
Stejné ID v účtu se stejným obsahem je duplicita, jiný obsah je konflikt.
Náhled ukáže nové/duplicitní/chybné řádky; potvrzení znovu ověří data a zapíše
celý import atomicky, nebo nic. Jiný UUID stejného souboru nevytvoří pohyby znovu.
Ruční pohyb má oddělený zdroj a serverem přidělené ID; import nic automaticky nepáruje.

## Párování

Jeden pohyb odpovídá jedné úhradě. Příjem → vystavená CZK faktura,
výdaj → přijatá CZK faktura nebo již zaplacená účtenka kartou.
Nabídka kandidátů má hledání; neprovádí automatický výběr podle VS.
Nová úhrada má celou absolutní částku pohybu, datum zaúčtování a převod.
Nesmí překročit aktuální neuhrazený zůstatek, datum nesmí předcházet vystavení.
Alternativně se připojí existující platná převodní úhrada se shodnou částkou
(u účtenky card); její datum a částka se nemění. Jedna úhrada nesmí mít dva pohyby.
Již ručně zaplacený doklad se propojuje touto cestou, ne další novou platbou.

Odpojení vyžaduje důvod. Pokud párování úhradu vytvořilo, stornuje ji stejná
transakce. U dřívější ruční úhrady/účtenky pouze zruší vazbu; úhrada zůstane.
Spárovanou úhradu nelze samostatně stornovat a spárovanou účtenku nelze
upravovat/zrušit. Nejprve odpojit v bance. Všechny změny mají aktéra a historii.
Transakce, revize pohybu i cílového dokladu, UUID a unikátní vazby chrání souběh.

## Rozhraní a hranice

Owner/editor zapisuje, viewer čte. REST a MCP sdílejí příkazy. Banka má seznam
s filtrem účtu/směru/párování/textu, detail s historií a odkazem na doklad.
CS/EN, monochromatické tokeny, společný trvalý layout.
Pohyb není další příjem/výdaj v přehledu dokladů; započítává se jen úhrada.

Rozdělení pohybu mezi doklady, přeplatky, zálohy/dobropisy, cizí měny, bankovní
notifikace/PSD2, nativní bankovní exporty, počáteční zůstatky a pokladna jsou
navazující P16 kroky. CSV je dokumentovaný formát Open Faktury, nikoli tvrzení
o importní shodě s bankou nebo iDokladem. Před migrací027 zastavit starý server.

## CSV příklad

```csv
external_id,booked_on,amount,currency,variable_symbol,counterparty,description
bank-transaction-123,2026-10-01,1500.00,CZK,20260001,Synthetic customer,Invoice payment
bank-transaction-124,2026-10-02,-200.00,CZK,,Synthetic vendor,Card purchase
```

Jde o syntetický příklad, nikoli export konkrétní banky. ID musí pocházet ze
stabilního identifikátoru transakce ve zdroji. Úprava popisu již importovaného ID
je konflikt; import neslouží k opravám uložených pohybů. Pohyby v tomto kroku
nelze měnit/mazat; párování lze odpojit a účet archivovat.

Nová fakturační úhrada používá společnou platební vrstvu a může spustit dříve
výslovně zapnuté poděkování P14-02. Samotný import ani připojení existující úhrady
novou platbu nevytváří. Banka automatizaci nezapíná a nezadává platební příkazy.

## Rozšíření P16-11

Původní match zůstává přiřazením celé částky. [Rozdělení](bank-split-matching.md)
přidává více částečných vazeb a nerozdělený zbytek. Unmatch nyní odpojí všechny
aktivní vazby ve stejné transakci. Seznam rozlišuje plně/částečně/nespárované
pohyby. Původní omezení jedné vazby výše popisuje historickou P16-01.
