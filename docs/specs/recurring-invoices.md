# RECURRING-01 / P14-01 — pravidelné faktury

Reference E-036, veřejná nápověda; žádné nové živé pozorování ani numerická či
vizuální 1:1 shoda. CZK běžné faktury, CS/EN, společné REST/MCP příkazy.

## Vlastní rozhodnutí a rozsah

Vzor obsahuje kopii stran, plateb a položek; lze začít prázdný nebo z faktury.
Změny adresáře a ceníku ho samy nepřepisují. Název vzoru není na faktuře.
Den/týden/měsíc/rok, interval 1–120, první datum, volitelný poslední den měsíce,
ukončení datem (včetně) či počtem, splatnost 0–365 dní. Náhled 12 termínů počítá
Rust. Měsíce/roky se odvozují od původního data, krátký měsíc ořízne den bez
posunu budoucí kotvy (31. 1. → 28. 2. → 31. 3.). Roky 2000–2100.
Časové pásmo je explicitní IANA hodnota uložená ve vzoru, výchozí z profilu.
Změna pásma profilu nemění existující vzor. Splatnost a DUZP jsou kalendářní data.

Uložení vytvoří pozastavený vzor. Aktivace je explicitní souhlas se zvolenými
operacemi: návrh, automaticky vystavit, případně zvlášť odeslat PDF jednomu
příjemci. Samotné vystavení e-mail neposílá. Výchozí je pouze návrh.
Při aktivaci musí být vzor vystavitelný i v režimu návrhu. Chyba při generování
zastaví další výskyty a je vidět v UI; po opravě se znovu aktivuje stejný termín.
Owner/editor může spravovat, viewer čte. Worker ověřuje práva posledního
aktivujícího uživatele při každém generování; odebrání role nezpřístupní profil.

Každý výskyt má index, plánované datum, snapshot vzoru a jednu fakturu. Návrh,
případné číslo/vystavení, výskyt a posun kurzoru vznikají v jedné transakci.
Souběh a restart nesmějí vytvořit druhý výskyt. Po výpadku doplní worker zmeškané
termíny postupně (jeden výskyt za iteraci), s plánovaným datem vystavení.
Pozastavení nepřeskočí backlog; pokračování jej doplní. Již vytvořené doklady
ani zařazené e-maily pozastavení neruší. Po první faktuře se nemění kotva,
interval ani pásmo; obsah a budoucí ukončení měnit lze jen při pozastavení.
Historické výskyty uchovají své snapshoty. Žádné mazání historie.

Číslování a kontrola daňového režimu používají stejnou operaci jako běžná faktura.
Rok řady profilu musí odpovídat datu; při přechodu roku se vyžaduje nastavení
profilu a opětovná aktivace. Neprovádíme tichou změnu daní/číselné řady.
Volitelný e-mail se po commitu idempotentně zařazuje přes P10 outbox s PDF;
chyba SMTP nemění vystavení ani nevytváří další fakturu. Stav doručení je v
historii faktury i výskytu. Zařazení po přerušení znovu použije stejné UUID.
Stav SMTP uncertain se automaticky neopakuje (stejná pravidla jako P10).

P14-01 zatím neobsahuje pravidelné zálohy, cizí měny, proměnné v textech,
více příjemců, více číselných řad, kopírování příloh ani upomínky. To jsou
samostatná rozšíření, nikoli tvrzená plná shoda s referencí.

## Ověření

Kalendář včetně přestupného roku, konce měsíce a DST; souběh/replay; pozastavení,
ukončení a doplnění po výpadku; rollback chybného vystavení; izolace a role;
upgrade 021; oddělené idempotentní odeslání přes loopback SMTP; CS/EN desktop/mobil.

Výsledky a prostředí: [ověření P14-01](../design/recurring-validation.md).
