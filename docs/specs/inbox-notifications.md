# Soukromá oznámení příjmu

Vlastní návrhové rozhodnutí Open Faktury z 18. 10. 2026 pro požadavek upozornění
na nové dokumenty v `docs/research/FEATURES_REFERENCE.md` §7. Není tvrzenou
kopií interního modelu iDokladu. Kontrakt `inbox-notification.openapi.json`,
migrace 090, stejné aplikační příkazy přes REST a MCP.

Každý uživatel má ve firmě vlastní dvě kategorie: nové originály a chyby
příjmu. Obě jsou výchozí vypnuté. Čtení výchozího feedu nic nezapisuje;
virtuální nastavení má revision=1. Konfigurace ověřuje očekávanou revizi,
změní pouze vlastní nastavení a zvýší revizi. Opakování se starou revizí
vrátí konflikt (klient načte aktuální stav); nevytváří duplicitní události.
Zapnutí vypnuté kategorie nastaví její kurzor na aktuální maximum: starší
události lze vidět ve feedu, ale nevyvolají nové nepřečtené upozornění.

Události vznikají transakčními triggery pouze pro nově uložený originál,
odmítnutou uloženou IMAP zprávu a uložený přechod synchronizace/brány do
selhání. Duplicitní hash/Message-ID či replay nevytváří nový originál ani
událost. Chyba obsahuje jen pevný kód `message_rejected`, `mailbox_failed`
nebo `gateway_failed`, nikoli původní chybu, MIME, účetní data, endpoint či
přihlašovací údaje. Po rollbacku nevalidního gateway příjmu se novou transakcí znovu ověří
aktuální právo zápisu a zapnutý příjem. Potom se uloží pouze
`gateway_failed`, deduplikovaný podle request UUID. Nic z odeslaného těla,
adres či předmětu se neuchovává. Nepovolený přístup, vypnutá brána a
nevalidní UUID oznámení nevytvoří. Stejné UUID lze po opravě přílohy
úspěšně zpracovat; historická chyba zůstane jednou. Selhání samotné
diagnostiky nenahradí původní chybu a nelze je slibovat při výpadku DB. Žádná událost se
neodesílá e-mailem, nic neaktivuje mailbox ani automatiku párování.

Každá operace včetně počítadla, vlastního nastavení a označení přečteného
ověřuje současné členství v nesmazané firmě. Viewer smí číst a měnit vlastní
nastavení; nepotřebuje právo upravovat firemní doklady. Odebrané členství
zamezí dalšímu čtení i označení, ani správce instalace jej neobchází.

Feed vrací posledních nejvýše 50 událostí zapnutých kategorií a počet všech
nepřečtených v nich. `through` je největší ID vrácené stránky, nebo `0`.
Označení dosavadních událostí posune pouze zapnuté kategorie až k tomuto ID
(včetně starších než zobrazených 50). Kurzor je monotónní a opakovatelný,
novější souběžná událost zůstane nepřečtená. Nenulové ID musí patřit do téže
firmy a některé zapnuté kategorie. Firemní transakční lock před přidělením
identity události chrání před přeskočením později commitujícího nižšího ID.

Zvonek v appbaru je pojmenovaný i s počtem nepřečtených. Otevře dostupný
dialog se seznamem, kategoriemi, obnovením a výslovným označením přečteného;
otvírání samo nic neoznačí. Každá agenda má oddělenou komponentu/state.
Při viditelné stránce se zavřeným dialogem se počet obnovuje po minutě;
ruční obnova je dostupná vždy. Texty CS/EN, čas dle locale, názvy souborů
vykreslené jako text. Odkaz vede do inboxu stejné firmy.

Test `inbox_notification` používá izolovanou PostgreSQL a syntetické soubory:
default bez zápisu, opt-in/revize, replay/deduplikace, sanitizace chyb,
firemní i uživatelská izolace, částečný monotónní kurzor, odebrání členství,
viewer, REST/MCP shoda a odmítnutí neautentizovaného/CSRF zápisu. Žádné
skutečné schránky nebo externí odeslání.
