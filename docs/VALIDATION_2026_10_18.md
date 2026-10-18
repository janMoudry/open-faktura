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

## Navazující ověření oznámení, adresáře a PWA

- Anonymizace adresáře: **5/5** izolovaných DB testů, původní kontakty **3/3**; browser kontakty **4/4** (CS/EN, desktop/mobil), včetně zrušení, neplatného potvrzení, skutečného zápisu a reloadu.
- Oznámení příjmu: **2/2** DB scénáře. Browser příjmu a stabilní navigace **8/8** po opravě skutečného mobilního překryvu zvonku a avatara. Navigační fixture nyní izoluje také nový požadavek na koš profilů a zachytává lazy chunk v dev i produkčním buildu.
- Přenosný archiv po aktualizaci explicitního seznamu na **113 tabulek**: všechny **3/3** DB scénáře správy profilu prošly, včetně binárních příloh a vyloučení tajemství; zbývající neignorovaný test touto DB sadou nebyl spuštěn.
- Produkční PWA: jednotkové testy **6/6**, browser **4/4** CS/EN desktop/mobil. Samostatná CI konfigurace lokálně také **4/4**, bez přeskočení. Chromium CDP nehlásí chyby instalovatelnosti; cache obsahuje přesně tři statické soubory s ověřeným SHA-256. Offline navigace zobrazí obecnou 503 obrazovku, soukromé API/PDF se z cache nevrací. Čtyři screenshoty vizuálně zkontrolované. Nejde o potvrzení instalace na fyzickém Android/iOS či do operačního systému a vzdálené Actions dosud neběžely.
- Před migracemi 090/100 zastaven starý API, vytvořena privátní záloha; nová vývojová DB má úspěšné migrace a endpoint readiness je připravený. Migrační skripty nepovolují příjem ani neanonymizují žádný kontakt automaticky.

- Navazující browser faktur, inboxu a správy profilu **20/20** (CS/EN desktop/mobil): volba cen dodacího listu a skutečné PDF stažení, uložení předvolby, odpojení originálu/zrušení potvrzení/reload/opětovné připojení ke stejnému dokladu. Izolovaná DB sada inboxu **5/5**, tiskové výstupy **1/1** a dokumentový brand **3/3**. CS/EN dodací listy s cenami rasterizované a vizuálně zkontrolované bez ořezu; CS používá desetinnou čárku, EN tečku.

## Uzavírací regrese navazujících funkcí

- Odeslání původního inbox souboru: **3/3** izolované DB scénáře, včetně fronty, replay a historie s jediným DB spojením. SMTP konfigurace se čte ve stejné transakci; historie nenačítá binární obsah všech zásilek. Browser inboxu **4/4** CS/EN desktop/mobil ověřil skutečný loopback SMTP příjem, originální název a historii po reloadu.
- Koš nabídek/šablon a související ochrany: sériová DB regrese **21/21** (attachments, delivery, exportní zámky, sales, sales_trash, share). Browser sales **4/4** CS/EN desktop/mobil.
- Oprava nových vazeb anonymizovaného kontaktu: **36/36** DB scénářů (anonymizace6, faktury10, opakování8, sales4, vyúčtování8). Nová vazba je odmítnutá, ponechání historické identity a ověřený replay původního příkazu zůstávají možné.
- Neplátcovské vyúčtování různých kurzů: browser **4/4** CS/EN desktop/mobil. Zálohy zachovaly kurzy25/24, konečný doklad kurz26 a doplatek100EUR. Jde o vlastní testovací příklad, nikoli numerické pozorování iDokladu. Omezení různých ekonomických kurzů DDPP u plátce nebylo odstraněno.
- Přenosný archiv: explicitní seznam **116 tabulek**, `company_admin` **4/4**, včetně neignorovaného testu. Před migracemi101/103 zastaven API a vytvořena privátní záloha; readiness nové instance připravená.
- Desktop updater UI a nápověda: **12/12** browser scénářů CS/EN desktop/mobil. IPC je syntetický: vypnutá konfigurace, explicitní kontrola, potvrzení uložené práce, zrušení, chyba podpisu a textové vykreslení poznámek. Tento test nic nestahuje/neinstaluje a nedokládá instalaci na cílovém OS.

- Regrese společného SMTP resolveru `pnpm test:smtp`: **4/4**, izolovaná DB a loopback odesílání.

- Nativní updater **3/3**: podpis a podepsaná verze, zákaz downgrade, lokální IPC/TTL, metadata512KiB včetně chunked, HTTP204. Node fixture ověřila podpisy a sestavení čtyř syntetických cílů; nejde o instalátory ani skutečný upgrade.
- Nová textová tisková předvolba: browser správy profilu **12/12**, CS/EN desktop/mobil, uložení/reload víceřádkového textu před položkami.
- Závěrečný web build, shoda CS/EN katalogů, Prettier, konvence tokenů a Rust workspace clippy všech targetů s `-D warnings` prošly. PWA jednotkové testy znovu **6/6**.
- Text před položkami: závěrečná DB sada `document_style` **4/4**, včetně limitů, původního JSON replay, byteové neměnnosti originálu, nabídky/zálohy/DDPP/dobropisu a nepřítomnosti textu na pokladním PDF. Víceřádkové PDF nebylo samostatně vizuálně kontrolováno a fyzický tisk nebyl proveden.
