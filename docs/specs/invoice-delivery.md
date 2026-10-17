# INVOICE-DELIVERY-01 — faktura e-mailem (P10-01)

Implementováno 11. 10. 2026. Reference E-029 (živý dialog + veřejná podpora),
společná infrastruktura s QUOTE-DELIVERY-01. Nejde o dokončení celého P10.

## Chování

V detailu vystavené faktury lze otevřít dialog s příjemcem, upravitelným předmětem
 a textem v jazyce dokladu. Odesílatel vychází ze SMTP instalace; Reply-To z uloženého
 e-mailu dodavatele. PDF je pevná příloha, totožná s uloženým PDF vystavené faktury.
Historie u dokladu obsahuje příjemce, čas, stav a detail zprávy včetně přesné přílohy.
UI je CS/EN a používá společný monochromatický DocumentDelivery pro nabídky i faktury.

Pouze vlastník/editor odesílá, čtenář má historii a PDF. Každá operace ověřuje profil.
Návrh ani historický import bez nativního PDF nelze odeslat. Vystavení ani úhrada
nikdy automaticky neposílají zprávu. Odeslání nemění revizi, číslo ani úhrady faktury.

## Konzistence a provoz

Migrace 015 přidává invoice_delivery se složeným FK na fakturu stejného profilu.
Ukládá obsah zprávy, snapshot, přesné PDF, stabilní Message-ID a request_id.
Stejné request_id a obsah vrací stejný záznam, změněný obsah je konflikt.
UI zachová klíč při ztrátě odpovědi; úmyslné nové odeslání má nový klíč.

Worker claimuje s SKIP LOCKED; SMTP běží mimo DB transakci, s limitem 45 sekund.
Stavy: queued → sending → sent / failed / uncertain. Přerušený sending po dvou
minutách přechází na uncertain; automaticky se neopakuje. Sent znamená převzetí
poštovním serverem, nikoli doručení nebo přečtení. Smazaný profil ruší čekající
zprávy; již probíhající externí odeslání nelze vzít zpět.

Bez SMTP funguje PDF/tisk/historie; nové odeslání není dostupné. Konfigurace a
záloha jsou popsány v INSTALLATION.md. REST a MCP používají stejný příkaz.
Kontrakt: contracts/invoice-delivery.openapi.json; MCP invoice_send,
invoice_history, invoice_deliverypdf. invoice_send vyžaduje výslovný pokyn
uživatele k externímu odeslání, není to náhled.

## Ověření a hranice

Izolované DB + loopback SMTP: idempotence/souběh, přesné PDF MIME, role a profilová
izolace, CSRF, REST/MCP, odmítnutí SMTP, nejistý výsledek, smazání profilu,
upgrade zachovávající PDF. Browser scénář CS/EN, desktop/mobil ověřuje dialog,
historii, PDF, přístupnost a ztrátu odpovědi následovanou retry bez druhé zprávy.

V tomto kroku jeden explicitní příjemce, prostý text a historie u faktury.
Více příjemců/kopie, veřejné odkazy faktur, hromadné odesílání, globální historie,
uživatelské přílohy a konfigurační SMTP UI nejsou součástí tohoto původního kroku.

Aktualizace 13. 10. 2026: navazující P10-02/03 již dodaly SMTP UI a vlastní
přílohy. P14-03 přidává volitelné `share_id`: e-mail obsahuje veřejný odkaz
místo PDF přílohy, přesné PDF zůstává v interní historii. Podrobnosti a omezení
jsou ve [specifikaci sdílení](invoice-sharing.md).
Žádný test neposílá do skutečné schránky. Referenční doručený e-mail nebyl ověřen.
