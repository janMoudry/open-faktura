# RECURRING-01 / P14-01 — ověření 12. 10. 2026

[Specifikace](../specs/recurring-invoices.md), reference E-036 (veřejná nápověda).
Žádný nový experiment v přihlášeném iDokladu; přesná numerická a vizuální shoda
není potvrzená. Automatické testy používají syntetická data, jednorázové DB a
loopback SMTP, nikoli zákazníky či skutečné účetní doklady.

## Doména a PostgreSQL

`cargo test --locked -p faktura-domain recurring`: 2 kalendářní testy prošly.
Kotva 31. ledna pokračuje po únoru opět 31. března; 29. února se při ročním
opakování vrátí v dalším přestupném roce. Ověřen konec měsíce, datum/počet,
neplatný interval a kalendářní dny přes změnu letního času.

`DATABASE_URL=… pnpm test:recurring`: 7 scénářů prošlo:

- Souběžné opakování příkazu vytváří jednu fakturu; historie zachová původní
  obsah po úpravě budoucího vzoru. Po prvním výskytu nelze přesunout kotvu.
- Pozastavení, doplnění zmeškaných termínů a ukončení; režim návrhu nepřiděluje číslo.
- Odebrání role zastaví worker, viewer nezapisuje, cizí profil nečte.
- Chybný daňový režim při vystavení vrátí celou transakci včetně návrhu a kurzoru;
  po opravě vznikne právě jeden doklad.
- Chybějící SMTP nevrací vystavení. Restart po zařazení, ale před uložením vazby,
  znovu použije stejné UUID; outbox má jednu zprávu a zachycené MIME obsahuje PDF.
- REST/MCP včetně skutečné deserializace příkazů, preview a CSRF.
- Upgrade 021 zachová existující fakturu a PDF; worker respektuje uložené pásmo
  Pacific/Kiritimati oproti Etc/GMT+12, kde stejný termín ještě není splatný.

Regrese `cargo test --locked -p faktura-server --test invoice --test sales
--test advance_settlement --test credit_note --test invoice_delivery -- --ignored
--test-threads=1`: 27 scénářů prošlo. Obzvlášť sdílené transakční vytvoření a
vystavení nyní používá stejná pravidla i plánovač; původní REST/MCP tok se nemění.

`pnpm test` prošel: 21 běžných testů, 90 DB scénářů standardně ignorovaných.
Relevantní databázové scénáře byly spuštěné samostatně výše.

## UI a provoz

Playwright `pnpm exec playwright test tests/e2e/recurring.spec.ts --workers=1`:
4/4 základní scénáře prošly. Po doplnění prázdného vzoru prošly oba mobilní
scénáře; po opravě role alert v dialogu prošly oba desktopové scénáře znovu
příkazem s `--project=desktop`. Konečný ověřený rozsah: CS/EN × desktop/mobil.
Vytvoření vzoru z faktury, konec po počtu, samostatné volby vystavení a odeslání,
aktivace, automatický běh skutečného serveru, PDF e-mail, historie/odkaz na doklad
a izolace profilu. Nový prázdný vzor zůstává pozastavený; aktivace zobrazuje
validaci a nevytvoří fakturu. Axe kontrola detailu bez nálezů. Test přepínačů používá
klávesu Space; skryté nativní inputy nejsou klikacím cílem vlastního switch.

Vizuálně zkontrolován desktopový seznam a editor/detail, mobilní editor.
Screenshoty (ignorované): `docs/design/recurring-{editor,detail,list}-{cs,en}-{desktop,mobile}.png`.
Sdílené monochromatické komponenty, zalamování na mobilu; opraveno oddělení
přepínačů a popiska faktury v historii. Náhled termínů odlišuje načítání od
skutečně vyčerpaného plánu. PDF používá běžný již ověřený fakturační renderer;
fyzický tisk ani Tauri tento krok netestoval.

`pnpm check` prošel (oba katalogy, TypeScript, Prettier, tokeny, rustfmt, workspace
Clippy). Build serveru a webu prošel; sdílený webový chunk nadále hlásí upozornění
na velikost nad 500 kB. Regenerace API beze změn, `git diff --check` bez chyb.

Lokální API bylo restartované, databáze má migraci 022 a `/api/ready` vrací ready.
Plánovač vyžaduje běžící server. Nepřebírá tarifní omezení; pravidelné zálohy,
textové proměnné, více příjemců/řad a upomínky zůstávají výslovná rozšíření.
