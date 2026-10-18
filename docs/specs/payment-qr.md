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
