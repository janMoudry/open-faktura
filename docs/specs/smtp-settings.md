# SMTP-SETUP-01 — nastavení instalace (P10-02)

12. 10. 2026. Vlastní návrh pro self-hosting, nikoli zjištěná konfigurace iDokladu.
Navazuje na INVOICE-DELIVERY-01 a QUOTE-DELIVERY-01.

Správce instalace otevře v menu avatara Nastavení e-mailu (SMTP). Běžný uživatel
nemá přístup ani přes REST/MCP. Konfigurace platí pro všechny fakturační profily;
role vlastníka profilu sama o sobě neopravňuje měnit poštovní server instalace.

Režimy: environment (dosavadní konfigurace serveru), custom (uložené nastavení),
disabled (žádné nové odesílání, fronta čeká). Migrace 016 zachovává environment
jako výchozí; neimportuje stávající tajemství do DB. Změny jsou účinné bez restartu,
worker i vytvoření zprávy načítají aktuální nastavení. Běžící odeslání může doběhnout.
Již zařazené zprávy zachovají obsah a From; pro transport použijí aktuální server.

Vlastní nastavení: host, port, TLS/STARTTLS, adresa From, volitelné přihlášení.
Plain pouze pro loopback. Bez obcházení ověřování certifikátů. Změna je chráněna
revizí; souběžný starší zápis vrací 409. API nikdy nevrací heslo. null jej zachová,
prázdný řetězec jej smaže; odebrání username smaže heslo a vypne přihlášení.

Heslo je AES-256-GCM ciphertext s náhodným 96bit nonce a pevnými authenticated data.
Klíč 32 bytů v hexu je v SMTP_ENCRYPTION_KEY_FILE (výchozí .secrets/smtp-key),
mimo DB. init-secrets vytváří soubor s oprávněním 0600 a nikdy existující nerotuje.
Chybějící klíč blokuje uložení hesla/dešifrování; nezpůsobí fallback na jiný server.
Audit ukládá pouze aktéra, čas a akci, nikoli heslo/poštovní obsah.

Test používá uloženou revizi a explicitního příjemce. Prostá samostatná zpráva
CS/EN není faktura a nemění žádné doklady. Registrace pokusu se zapíše před SMTP;
network I/O mimo transakci, timeout 45 s. request_id chrání opakování stejného
pokusu; jiné parametry se stejným klíčem jsou konflikt. Max. 5 nových testů/minutu
za instalaci. Sending starší dvou minut je při dalším ověření uncertain. Žádné
automatické opakování nejasného pokusu. Sent znamená pouze přijetí serverem.

REST /api/installation/smtp GET/PUT a /test POST; MCP smtp_get/save/test sdílejí
příkazy a kontrolu správce. Test je externí akce vyžadující pokyn k odeslání na
konkrétní adresu. Testy používají výhradně izolované DB a SMTP na loopback.
