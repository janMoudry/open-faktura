# TAX-RECEIPT-01 / P13-02 — daňové doklady k přijaté platbě

## Reference a rozsah

E-033: oficiální nápověda iDokladu potvrzuje samostatnou agendu, vystavení
z konkrétní úhrady, úpravu popisu a data před uložením, PDF a tisk. Není to
pozorování přihlášené aplikace ani důkaz numerické shody.

Implementujeme náhled a explicitní vystavení z aktivní úhrady zálohy plátce DPH,
samostatnou řadu DDPPYYYY####, neměnný snapshot, PDF, seznam/detail pod profilem,
REST/MCP a CS/EN. Dodavatel musí mít vyplněné DIČ. Neplátce, stornovaná úhrada a archivovaná záloha jsou odmítnuté.
Jedna úhrada má nejvýše jeden doklad. Opakovaný stejný příkaz stejného aktéra vrátí
původní výsledek; jiné použití UUID nebo úhrady je konflikt. Datum zdanitelného
plnění je den přijetí platby; datum vystavení nesmí předcházet platbě ani být
v budoucnosti (Europe/Prague). Popis a jazyk lze zvolit před vystavením.

Po vystavení nelze stornovat zdrojovou úhradu. Tím zůstává zachována blokace
editace/archivace zaplacené zálohy. Opravy daňových dokladů nejsou nahrazené
mazáním nebo stornem peněžní evidence. PDF se nikdy znovu negeneruje z kontaktu.
Vystavení nic neodesílá a nemění výnosy běžných faktur.

## Vlastní výpočet of-tax-receipt-v1

CZK, sazby původní zálohy 0/12/21 %. Platba se rozděluje poměrem kladných
hrubých částek původních daňových skupin po slevách. Každý podíl se nejprve
zaokrouhlí dolů na haléře, zbylé haléře dostanou skupiny s největším desetinným
zbytkem (při shodě nižší sazba). Součet rozpisu je přesně výše platby.
Základ skupiny = hrubá částka / (1 + sazba/100), na 2 místa half-away-from-zero;
daň je rozdíl. Žádné binární floaty. Zaokrouhlení proformy na celé Kč je zahrnuto
v částce platby, poměr sazeb vychází z nezaokrouhlených hrubých skupin.

Pravidlo není potvrzené iDokladem. V náhledu uživatel vidí částky před vystavením.
Přeplatkovou úhradu, nebo vydání nad celkovou výši zálohy, odmítáme; řešení
přeplatku a vypořádání patří do P13-03. Více dokladů může kvůli zaokrouhlení
mít jiný součet základů než původní záloha; vyúčtování musí použít uložené
skutečné daňové rozpisy, nikoli je přepočítat. Změny sazeb, cizí měny,
reverse charge a uživatelské členění DPH nejsou podporované.

## Uložení a ověření

Migrace 019 přidává tax_receipt a tax_receipt_sequence bez změny starých dat.
Složený FK váže úhradu i zálohu na stejný profil. Číslování, snapshot, PDF a audit
vznikají v jedné transakci; zámek zálohy serializuje vystavení se stornem úhrady.
Starý server před migrací zastavit — nezná novou blokaci storna.
Náhled nic nezapisuje; čtení dovoleno viewerovi, vystavení owner/editorovi.
Seznam je stránkovaný, hledá číslo, zálohu a odběratele. Kontrakt tax-receipt.openapi.json.

Testy: nezávislé číselné příklady, souběh/retry, role a cizí profil, storno proti
vystavení, PDF neměnnost, upgrade z 018 a CS/EN UI desktop/mobil.
E-mail, vlastní přílohy, editace vydaného dokladu, automatické vystavení a
upomínání lhůt jsou navazující práce, nikoli součást TAX-RECEIPT-01.

[Výsledky ověření](../design/tax-receipt-validation.md).
