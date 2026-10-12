---
name: desktop-release
description: Připravuje buildy a instalátory desktopové Open Faktury pro GitHub Releases. Použij při konfiguraci balení, podpisů, release CI a distribuce desktopu.
---

# desktop-release

Přečti sekci Desktop a GitHub Releases v `docs/architecture/DEVELOPMENT_CONVENTIONS.md`. Zvolený runtime je Tauri. Ověř jeho připnutou verzi, konfiguraci v apps/desktop a existující manifesty; nepiš workflow s neexistujícími příkazy.

Desktop používá společné React UI a připojuje se k backendu; instalátor automaticky neobsahuje databázi. Buildy odvozuj ze stejného tagu/commitu, eviduj verzi, OS a architekturu. Přilož kontrolní součty a kompatibilitu serveru.

CI pro PR nedostává podpisová tajemství. Podpis/notarizaci kontroluj podle platformy; chybějící konfiguraci ohlas konkrétně, nevydávej nepodepsaný soubor za podepsaný. Ověř instalaci, spuštění, login, PDF a odstranění na podporovaných cílech.

Nejprve připrav přezkoumatelný draft release s hotovými assets. Publikuj, pokud je konkrétní vydání autorizované; plán budoucí distribuce není pokyn publikovat nyní. Automatický updater řeš zvlášť a ověř podpis aktualizací. Zachovej uživatelská data při upgradu.
