# Integrační ověření P16–P20, 18. 10. 2026

Lokální macOS arm64, Rust workspace, Node 22.22, PostgreSQL z vývojového Compose. Databázové scénáře zakládají a mažou vlastní izolované databáze; nepoužívají účetní data vývojové agendy. SMTP, bankovní a IMAP transporty jsou syntetické nebo loopback.

## Dokončené kontroly

- Bankovní regresní sada před poslední integrací: 71/71; po změnách PDF/koše odhalila dvě zastaralé upgrade fixtures. Fixtures nyní zakládají historické řádky ve starém schématu místo volání současného issueru. Následná sada všech bankovních upgradů: **11/11**. Nejde o fallback do neúplného produkčního schématu.
- Playwright faktury, přijaté doklady a správa produktu: **20/20**, čeština/angličtina, desktop/mobil; koš/obnova, filtry štítků všech čtyř registrů, role a přístupnost.
- Následný Playwright přijaté doklady a dashboard na novém produkčním web buildu: **8/8**, včetně vložení originálu, PDF canvas náhledu a explicitní práce s dodavatelem.
- Nápověda: **4/4** browser scénáře; bankovní UI: **4/4** v posledním příslušném běhu.
- Daňová klasifikace a explicitní profil subjektu: **2/2** integrační scénáře, revize/replay, izolace a MCP.
- Nové produktové backendové sady ověřené agentem: správa profilů 4, štítky 3, branding 3, exportní zámky 2, tiskové výstupy 1.
- ISDOC export: oficiální XSD; QR formáty nezávisle dekódované z šesti vykreslených CS/EN PDF. Technická validace formátu není potvrzení numerické shody s iDokladem.
- Překlady: shodné CS/EN klíče a parametry; TypeScript, web build a statická kontrola tokenů/hranic prošly. Generované kontrakty aktualizované.
- macOS debug aplikace a DMG sestaveny; integrita DMG a SHA-256 ověřeny. Windows/Linux a systémová instalace neověřeny; podpisy a updater nejsou hotové.

- Společné databázové regrese: pokladna 16, koš návrhů 2, faktury 10, přehledy 10 a prodejní doklady 4 prošly. Přijaté faktury 9/10 a pravidelné 7/8 odhalily stejné zastaralé upgrade fixtures; oba jednotlivé upgrade scénáře po opravě prošly.
- Knihovní testy domény/aplikace/infrastruktury: **59 prošlo**, 2 vyžadují zvláštní referenční prostředí a jsou ignorované.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` a `pnpm format:check` prošly.
- Vývojová DB: před upgradem zastaven starý API proces a vytvořen privátní dump mimo Git; následný server aplikoval migrace a `/api/ready` odpověděl `ready`.

- Audit dalších historických fixtures: všech **10/10** cílených upgradů prošlo (úhrady, nabídky/sdílení, doručování, komunikace, dobropisy, zálohy, DDPP a vyúčtování). Sdílená příprava historických řádků je pouze v testech.
- Formulář daňového subjektu: dodatečné **4/4** UI scénáře CS/EN desktop/mobil ověřily uložení úřadu, zachování úvodních nul DIČ a načtení po reloadu.
- Přenosný archiv po aktualizaci explicitního seznamu na 110 tabulek: **4/4** testy správy profilu včetně vyloučení tajemství.

## Limity

Souhrnný přehled hotových funkcí a mezer je v [P16–P20](P16_P20_COMPLETION.md). Testy nepotvrzují univerzální kompatibilitu všech bank, nativní Money S4/S5, kompletní právní správnost daňových přiznání ani úplnou shodu s iDokladem. Osobní desktopový server a automatické aktualizace nejsou součástí ověřeného DMG.
