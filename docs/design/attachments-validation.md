# P10-03 — ověření vlastních příloh

12. 10. 2026, ATTACHMENTS-01. Reference veřejné nápovědy E-030, vlastní
monochromatická adaptace a vlastní pravidla uložení/historie.

- attachment.rs: 3 integrační scénáře prošly, včetně rolí/izolace, souběhu,
  omezení souborů, idempotence, MIME, smazání a historických bytů, REST/MCP.
- delivery/invoice_delivery/share: dalších 13 regresních scénářů prošlo,
  včetně upgradu staršího schématu a opakování starých příkazů bez attachment_ids.
- E2E invoices/sales: všech 8 kombinací CS/EN × desktop/mobil prošlo.
  Upload/stažení, výběr do zprávy, lokální SMTP capture, odstranění a historické
  stažení. U faktur doplněno přepnutí URL na attachments (předtím pouze payments).
- pnpm check a build:web prošly; build nadále hlásí známou velikost JS chunku.

Snímky attachments-{invoice,quote}-{cs,en}-{desktop,mobile}.png zachycují pouze
syntetická data. Prohlédnut desktop faktury a mobil nabídky. Po této kontrole
upraveno zobrazení malých souborů z 0 KB na počet bajtů a pozadí panelu na surface.
Žádný test neposlal externí e-mail. Nativní systémový tisk/desktop se neměnil.

Přílohy jsou v DB záloze. P10 CSV/XLSX exporty nadále nejsou součástí tohoto kroku.
