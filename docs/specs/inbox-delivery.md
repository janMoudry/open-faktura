# P17 — Explicitní předání originálu účetnímu

Doloženo veřejnou nápovědou https://www.idoklad.cz/podpora/inbox,
aktualizace 6. 5. 2026, znovu přečteno 18. 10. 2026, oddíl Zasílání dokladu
účetní: přílohy při předání obsahují původní inbox dokumenty; reference
zmiňuje dva roky platný odkaz. Není to pozorování současného API/SMTP.

Vlastní rozhodnutí Open Faktury: samostatná explicitní akce pošle **jeden
vybraný originál jako skutečnou e-mailovou přílohu**, bez veřejného odkazu,
bez změny/vystavení účetního dokladu a bez nového PDF renderu. Nejde o
imitaci dvouletého bearer odkazu ani hromadný export dokladů. Originál lze
předat před zpracováním i po něm, včetně archivovaného. Každé odeslání má
konkrétního adresáta, předmět, tělo a potvrzení přílohy `confirm_original`.
Akce nic neodešle jen při otevření dialogu ani při zpracování přílohy.

Kontrakt `inbox-delivery.openapi.json`, nástroje `inbox_delivery_history`
a `inbox_delivery_send`, migrace101. Web/REST/MCP volají stejný store.
Send je externí akce vyžadující výslovné oprávnění od uživatele odeslat
uvedený soubor uvedenému příjemci. Testy používají jen syntetický originál
a SMTP capture na 127.0.0.1. Žádná skutečná pošta nebo účetní data.

## Validace, snapshot a replay

Aktuální editor/owner v nesmazané firmě smí zařadit originál do fronty;
viewer pouze čte historii. Instalátor nepřeskakuje členství. Kontroluje se
původní revize inbox položky, UUID, skutečná mailbox adresa, délky a hlavičkové
řídicí znaky, neprázdný předmět/tělo a výslovný boolean potvrzení. Předmět
nejvýše 200 znaků, adresa 254 B, tělo 20 000 B. MIME neposkytuje klient.

Snapshot obsahuje původní byty, název, media type, SHA-256, velikost a revizi,
SMTP odesílatele a potvrzený recipient/subject/body. Příloha má nejvýše 2 MiB
včetně kontroly `octet_length(content)=size`; stejný limit jako ruční inbox
upload. MIME/base64 je transportní režie nad tímto původním limitem, nejde
o slib přijatelnosti pro libovolný mailserver. Názvy souborů a typy byly
validovány už při uploadu. Nejsou přikládány jiné soubory než tento snapshot.

Fronta se uloží atomicky; účetní doklad, originál ani přílohy dokladu se
nemění. Pozdější archivace/odpojení inboxu nemění potvrzené byty. Unikátní
company/request UUID a porovnání aktéra, zdroje i celého příkazu brání dvojímu
zařazení. Opakování vyžaduje i nadále aktuální práva; změna payloadu se
stejným UUID je konflikt. Při chybě odpovědi klient zachová UUID. V témže
otevřeném dialogu neopakuje novou zprávu; po úspěšném zařazení a novém
otevření lze výslovně vytvořit nový send. U nejasného výsledku se nejprve
ověří SMTP historie, nikoli automaticky opakuje.

## Outbox a historie

Sdílí instalační SMTP konfiguraci a bounded SMTP dispatch s již existující
fakturou/nabídkou: nejvýše 45 s celá SMTP akce, transport timeout 30 s. Worker
nárokuje řádek SKIP LOCKED, má jeden pokus, drží síť mimo SQL zámky.
Bez aktuálních editor/owner práv aktéra či při smazané firmě se před
odesláním uloží failed/permission_changed a SMTP se nekontaktuje. Hranice
kontroly je bezprostředně před dispatch; odvolání práv během již zahájené
SMTP komunikace nemůže odvolat přijatou zprávu.

Stavy queued/sending/sent/failed/uncertain. Sent znamená přijetí SMTP
serverem, ne doručení nebo přečtení příjemcem. Odmítnutí ukládá pouze pevný
smtp_rejected, ne raw SMTP odpověď. Nejasná síťová chyba/timeout znamená
uncertain/smtp_unknown. Opuštěný sending po dvou minutách přejde na
uncertain/interrupted. Žádné failed/uncertain se neopakují automaticky.

Historie vrací posledních 50 odeslání vybraného originálu v aktuální firmě,
včetně adresáta, souboru, hashe, velikosti, času a bezpečného výsledku.
Neobsahuje byty souboru ani přihlašovací údaje. Uživatel ji obnovuje ručně;
UI ukazuje průběh, upozornění na meze SMTP a dostupné CS/EN texty. Při
401/403/404 vymaže načtenou historii. Zálohy a historie nadále obsahují
snapshoty; tato akce není anonymizací ani automatickou retenční politikou.

## Ověření

`DATABASE_URL=isolated pnpm test:inbox-delivery` — syntetické loopback SMTP,
exact původní base64/filename/jedna příloha, žádný nový účetní render;
potvrzení/hlavičky/replay/změna payloadu/firemní izolace/REST+MCP,
souběžní workeři právě jednou, SMTP odmítnutí, odebrání práv před dispatch
bez síťového spojení a interrupted stav bez automatického retry.
Regrese sdíleného dispatch: test:delivery a test:invoice-delivery.
