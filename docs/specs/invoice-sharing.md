# P14-03 — veřejné odkazy faktur

Datum: 13. 10. 2026. Reference E-038; sdílené příkazy a pravidla tokenů z QUOTE-SHARE-01.

## Rozsah a reference

Nápověda iDokladu potvrzuje volbu PDF přílohy nebo odkazu, zobrazení/stáhnutí
faktury v prohlížeči a evidenci data otevření. Starší oficiální článek dokládá
štítek uhrazené faktury. Nepotvrzuje detaily tokenů, expirace a souběhu.

Náš návrh: vlastník/editor vytvoří pro vystavenou fakturu samostatný odkaz
s platností 1–365 dní (výchozí 30). Kdokoli s odkazem uvidí originální vystavené
PDF a stav úhrady při posledním načtení. Stejný odkaz zůstává platný při úhradě
nebo jejím stornu, PDF se nemění. Nezveřejňují se řádky úhrad, jejich poznámky,
aktéři, interní ID kontaktu, další faktury ani vlastní přílohy. Poznámka vytištěná
přímo na faktuře je součástí již vystaveného PDF, nikoli interní poznámka úhrady.

Veřejná stránka používá PDF jako autoritativní náhled i pro vyúčtování záloh.
Nevytváří druhý výpočet ani zjednodušenou HTML daňovou rekapitulaci. Nabízí
stažení a otevření PDF pro tisk, stav úhrady, obnovení a přepnutí CS/EN.
Jazyk PDF je jazyk vystaveného dokladu. Mobilní prohlížeč bez PDF náhledu může
soubor stáhnout. Jde o vlastní monochromatické UI, ne pixelově doloženou kopii.

## Příkazy a persistence

Migrace 024 přidává `invoice_share`, `invoice_view` a volitelný FK odkazu
v `invoice_delivery`. Firemní vazby jsou složené. U vytvoření se ukládá aktér,
UUID, revize, expirace, hash tokenu a důvěrná URL pro autorizované opakování.
Opakování UUID se stejným vstupem vrátí stejný odkaz; jiný vstup je konflikt.
Odvolání je nevratné a opakovatelné, s aktérem a časem. Čtenář profilu může
vidět odkazy a historii; pouze editor/vlastník mění přístup.

Odkaz je 256bit náhodný bearer token. Prefix výhradně z PUBLIC_WEB_URL,
nikoli Host. Token je ve fragmentu webové URL a těle POST; ne v cestě API ani
query. Odpovědi mají no-store, no-referrer, nosniff a noindex. Neplatný,
prošlý či odvolaný token a smazaný profil vracejí stejnou 404. Návrh faktury
ani historický import nelze sdílet. Při vytvoření se připraví neměnná cache PDF;
veřejné požadavky nespouštějí drahý render. Oprávnění tokenu se před vrácením
PDF kontroluje znovu.

Ruční e-mail může použít existující aktivní odkaz. Server připojí jeho URL,
PDF zůstává uložené pouze v interní historii tohoto odeslání. Zvolené vlastní
přílohy odejdou jen e-mailem. Před předáním SMTP se znovu ověří platnost odkazu;
neplatný odkaz zastaví čekající zprávu. Již zahájené SMTP ani stažené PDF nelze
odvolat. Automatizace P14-01/02 dál používá PDF přílohu; změna automatického
způsobu odesílání není součástí tohoto kroku.

REST/MCP používají společnou aplikační vrstvu `share`. Privátní kontrakt
`invoice-share.openapi.json`, veřejný `public-invoice.openapi.json`; rozšíření
`invoice-delivery.openapi.json` o nepovinné share_id. Staré příkazy bez odkazu
mají zachovaný serializovaný vstup a replay. Web využívá společnou správu odkazů
s nabídkami, nikoli druhou implementaci formuláře.

## Sledování a ověření

Záznam vznikne po načtení metadat a PDF na viditelné stránce. UUID návštěvy
zabrání dvojímu zápisu po opakování requestu; nové načtení je nová technická
návštěva. Nejvýše 60 záznamů za hodinu/odkaz, bez IP, cookies nebo fingerprintu.
Datum, počet a revize jsou technické události, nikoli důkaz přečtení či totožnosti.
Automatická kontrola odkazu může událost vyvolat; blokované JS ji může vynechat.

Testy: izolovaný PostgreSQL, loopback SMTP, REST/MCP, role/cizí profil,
opakování/souběh, expirace/odvolání/profil, neměnné PDF, změna úhrady, soukromí,
upgrade 023, CS/EN a desktop/mobile. Výsledky: [validační záznam](../design/invoice-sharing-validation.md).
