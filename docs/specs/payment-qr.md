# P20-04a — QR Platba v PDF

Podklad E-055, SPAYD 1.0. QR se generuje lokálně ze snapshotu vystavené
faktury nebo zálohy. Pouze bankovní převod v CZK, platný český IBAN
(nebo převod domácího čísla účtu s kontrolou), kladná splatná částka
nejvýše 9 999 999,99 Kč. Neplatné/nesplněné údaje QR nevytvoří, nikdy
nevznikne kód s náhradním účtem nebo zaokrouhlenou částkou.

Částka odpovídá původnímu PDF při vystavení, nikoli dnešnímu zůstatku po
částečných úhradách. PDF cache a historické snapshoty se nemění. Kód není
na nabídce, dobropisu ani DDPP. VS maximálně 10 číslic, splatnost YYYYMMDD.
Ochrana chyby M a bílý okraj čtyř modulů. Žádné síťové volání generátoru.
CS/EN vysvětlení u kódu upozorňuje na původní částku dokumentu.

QR Faktura a QR Platba+F, skenování/import i volba v tiskových preferencích
jsou další otevřený rozsah P20-04/P17, nikoli tato první implementace.
Akceptace: obsah kódu a hranice částek/účtů; skutečné dekódování z PDF;
CS/EN PDF a zachování cache, REST/MCP společný renderer.

Ověřeno 16. 10. 2026: dvě doménové regrese, izolovaný PostgreSQL scénář
`qr_payment_pdf_uses_exact_issued_snapshot`, render CS/EN PDF přes Poppler
150 DPI a dekódování pomocí zxing-cpp 2.3.0 (přesná shoda celého payloadu).
To není ověření konkrétní mobilní bankovní aplikace.

P16-13i (18. 10. 2026) umožňuje zahraniční IBAN v profilu a na faktuře,
ale nezvětšuje rozsah QR Platby. Pro non-CZ IBAN se QR nevygeneruje ani
při částce v CZK. Test ověřuje toto oddělení; důvod v E-070.

## P20-04b — QR Faktura a QR Platba+F, 18. 10. 2026

V návrhu faktury je explicitní volba `qr_format`: payment (výchozí), invoice,
payment_invoice. Nezadaná hodnota je payment a při serializaci se výchozí
hodnota vynechává pro kompatibilitu historických snapshotů a replay příkazů.
Volba je součástí uloženého návrhu a vystaveného snapshotu; web/REST/MCP
používají společný renderer. PDF cache starých dokladů se nepřepisuje.

Podklad QR Faktura SID 1.0 a integrace X-INV jsou veřejné specifikace KDP ČR:
https://www.kdpcr.cz/informace/qr-faktura/popis-formatu a
https://www.kdpcr.cz/informace/qr-faktura/integrace-s-qr-platbou,
přečteno 18. 10. 2026. Nejde o pozorování nebo potvrzení aplikace iDoklad.

QR Faktura z běžné CZK faktury od roku 2024 obsahuje původní číslo dokladu,
datum vystavení/plnění/splatnosti, celkovou splatnou částku a uložené základy
+ DPH ve skupinách 21 %, 12 % a 0 %. Nic se nepřepočítává podle aktuálního
profilu. IČO/DIČ, VS a účet se přidají jen v odpovídajícím formátu. Úhrady
po vystavení tento kód nemění. Číslo dokladu má limit standardu 40 znaků.
Text se escapuje procentovým kódováním; kód neobsahuje URL a nevyžaduje síť.
Není to ověřený podpis ani garance daňové správnosti plnění.

QR Platba+F vkládá SID do SPD X-INV, společné AM/DT/ACC/VS přesune do
vnější QR Platby a vnitřní oddělovače/escapy zakóduje znovu. Zdědí všechny
limity bezpečné QR Platby (včetně platného CZ účtu, metody a částky).
QR Faktura samotná není platební příkaz a nevyžaduje bankovní převod.
Nedaňová běžná faktura neplátce má TD:0; standard TD:0 sám nerozlišuje
zálohu od jiné nedaňové faktury, import proto musí být vždy kontrolovaný.

Nabídky, dobropisy a DDPP tento kód nemají. U záloh a vyúčtování s uloženými
odpočty zůstává podporována původní volba payment; invoice/payment_invoice
se nevygenerují, protože daňové odpočty vyžadují odlišné mapování. Cizí měna,
starší sazby, příliš dlouhé číslo nebo nepodporovaná skupina kód nevytvoří;
nikdy se nenahradí částka nebo účet. UI tyto limity uvádí u volby.

Doménový test kontroluje výchozí serializaci, přesné částky a dvojí escapování.
Roundtrip přes skutečný SID/SPD importer ověřuje zachování původního čísla
včetně hvězdičky, základu a DPH. PostgreSQL QR test renderuje payment,
invoice a payment_invoice v CS/EN ze skutečně vystavených snapshotů a ověřuje
opakované čtení stejné cache. Živá shoda konkrétní mobilní aplikace není
potvrzená.
