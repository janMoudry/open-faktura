# SMTP-SETUP-01 — ověření

12. 10. 2026. Vlastní instalační UX, návaznost na existující monochromatické dialogy.

Backend: test:smtp ověřuje role, REST/MCP shodu, CSRF, write-only heslo,
AEAD integritu (poškozený ciphertext nesmí spustit fallback), zachování/odebrání
hesla, revize, okamžité přepnutí transportu, SMTP odmítnutí, idempotenci a obnovu
přerušeného testu. Čtyři scénáře prošly. Regrese delivery/invoice_delivery/share:
12 scénářů prošlo v izolovaných databázích po přidání migrace 016.

UI: tests/e2e/smtp.spec.ts používá syntetického správce, loopback capture a
čtyři varianty CS/EN × desktop/mobil. Kontroluje uložení/reload, zákaz testu před
uložením, vypnutí SMTP, ztrátu odpovědi a opakování bez druhé zprávy, axe.
Screenshoty smtp-{cs,en}-{desktop,mobile}.png. Žádné skutečné e-maily odeslané.

Opraven společný Select/Autocomplete popover v Radix dialogu: portál je uvnitř
hranice modalu, aby fungoval pointer i focus. SMTP scénář vybírá myší, regrese
nabídek kontroluje stávající sdílené dialogy. Při testování s Vite neměnit zdroje
během běhu; HMR může zrušit transientní potvrzení právě dokončené registrace.

pnpm check a build:web prošly; build hlásí dosavadní upozornění na velikost chunku.
Testy nedokládají doručení do externí schránky ani konkrétní hosting SMTP.

Finální výsledek: 4 SMTP + 12 stávajících doručovacích integračních scénářů a
1 nový scénář skutečné faktury přes UI konfiguraci (vypnutí/obnovení fronty bez
restartu) prošly. Všech 8 browser variant SMTP/nabídky prošlo; jeden mobilní
regresní běh přerušený HMR byl zopakován samostatně úspěšně. Lokální backend
je aktualizovaný, readiness OK, migrace 202610120016. Reálná SMTP konfigurace
zůstala environment/revize 0; žádné syntetické nastavení v uživatelově DB.
