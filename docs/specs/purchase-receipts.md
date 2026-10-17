# P15-02 — účtenky a přehled výdajů

13. 10. 2026, reference E-040. Rozšíření P15-01; vlastní pravidla níže nejsou
ověřenou numerickou nebo vizuální shodou s iDokladem.

## Účtenky

Společný nákupní model received_invoice získává neměnný kind invoice/receipt.
Stará data i klienti bez kind znamenají invoice. Účtenka má samostatnou řadu
UC + rok vystavení + 6 číslic, dodavatel a externí číslo jsou nepovinné.
Popis v UI při prázdné hodnotě zastupuje první položka. CZK a přesné zdrojové
základy/DPH/zaokrouhlení z P15-01 zůstávají; žádný výpočet daňového odpočtu.

Účtenka eviduje již zaplacený nákup, method cash/card (výchozí card), datum
úhrady je datum vystavení. Vytvoření atomicky uloží doklad a plnou výdajovou
úhradu. Úprava s revizí stornuje původní úhradu a vloží novou; historie zůstane.
Zrušení s důvodem stornuje aktivní úhradu, doklad zůstává čitelný.
Samostatné pay/reverse příkazy nejsou pro účtenku povolené. Vazba na banku
ani pokladní doklad nevzniká; zaplaceno neznamená spárováno.

Účtenka používá stejné originální přílohy, autorizaci, audit a UUID ochranu.
Nemá VS, účet ani splatnost; v kompatibilním Data jsou tyto texty prázdné
a received_on/due_on shodné s issued_on. Receipt method nesmí být na invoice.
Při nasazení zastavit starý server: stará verze nerozlišuje typy nákupu.

## Seznam úhrad a přehled

REST/MCP výdajové úhrady: profil, období od/do včetně podle paid_on,
text (číslo, dodavatel, popis a poznámka), metoda, stav active/reversed/all,
strana 25 řádků. Řazení datum sestupně, ID dokladu a ID platby.
Odkaz vede do původního dokladu; storno faktury přes existující detail,
účtenka se opravuje jako celek. I stornované platby jsou dohledatelné.

Přehled v CZK počítá PostgreSQL NUMERIC:
- nákupy: nezrušené faktury i účtenky podle issued_on v období,
- zaplacené výdaje: pouze aktivní úhrady podle paid_on v období,
- aktuální závazky: nezrušené faktury total-paid bez omezení období,
- po splatnosti: aktuální závazky se splatností před dneškem Europe/Prague.
Časová období jsou záměrně odlišná a UI je vysvětluje. Storno odečítá původní
událost z jejího období; není to bankovní cash-flow ani účetní výsledovka.
Agregace nejsou omezené stránkováním. Čtení i součty jsou autorizované
stejným firemním členstvím; reader nemůže měnit doklady.
Dotazy mají validní kalendářní období 1900–9999, od<=do.

## Ověření a meze

Testy: čistá DB/upgrade025, oddělené číslování, replay, revize/souběh,
oprava a zrušení účtenky bez dvojího výdaje, přesné agregace, období,
storna, role/cizí profil, REST/MCP; CS/EN desktop/mobile včetně originálu.
Cizí měny, bankovní párování, pokladna, daňové exporty, tisk/e-mail
rekapitulace a konfigurovatelné číslování zůstávají další prací.

Výsledky: [ověření P15-02](../design/purchase-receipts-validation.md).
