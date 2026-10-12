---
name: installation-setup
description: Připravuje a diagnostikuje instalaci Open Faktury, první založení správce a připojení klienta. Použij při setupu pro osobní použití, vlastní server nebo vývoj; odlišuje hotové instalační možnosti od plánovaného desktopového režimu.
---

# Setup Open Faktury

Pracuj z kořene repozitáře. Přečti `docs/INSTALLATION.md` a pro výběr režimu
`docs/specs/installation-onboarding.md` (oddíl „Osobní použití a desktop“).
Příkazy a soubory ověř v aktuálním checkoutu, ne podle zapamatovaného stavu skillu.

## Nejdřív zvol správnou cestu

- **Vývoj:** lokální Rust + PostgreSQL + Vite. Existující `.env` zachovej;
  `node scripts/init-secrets.mjs` doplní chybějící tajemství, nerotuje existující.
  `pnpm db:up`, `pnpm dev`; běžící služby nejprve zkontroluj, neduplikuj porty.
- **Vlastní server (i pro jednoho člověka):** produkční Compose podle návodu,
  skutečná doména, HTTPS a trvalá volumes. Dev Compose není instalační náhrada.
- **Desktop připojený k serveru:** připojuje se ke stávající instalaci a používá
  její účet. Nezakládej druhého instalačního správce. Ověř aktuální stav výběru
  serveru, originů, cookies a CSP; konfigurace běžného browseru nemusí fungovat v Tauri.
- **Osobní desktop bez serveru:** automaticky spravovaný lokální backend a DB
  jsou zatím plánované. Nevydávej Tauri instalátor za kompletní lokální instalaci.
  PostgreSQL ani Rust potichu nenahrazuj jiným stackem a neobcházej bootstrap.
  Vysvětli dostupnou variantu self-hostingu a přesně označ chybějící část.

Záměr obvykle plyne ze zadání. Pokud není jasné, zda má jít o vývoj, existující
server, nebo samostatný desktop, zjisti jen tento rozdíl před změnami prostředí.

## První správce a tajemství

- Zkontroluj `GET /api/ready` a `GET /api/auth/setup`. Výpadek, 503 nebo nevalidní
  odpověď nejsou nová instalace. Bootstrap lze provést pouze při `setup_required=true`.
- Instalační klíč je doklad oprávnění pro jednorázové založení správce, nikoli
  heslo, API token nebo licence. Zdroj určuje `BOOTSTRAP_SECRET_FILE`; standardně
  `.secrets/bootstrap`, v Compose mount podle manifestu. Obsah neloguj,
  nevypisuj do chatu a nedávej do URL, Git ani frontendových proměnných.
- Uživateli sděl umístění klíče a otevři správný formulář. Nevymýšlej jeho e-mail
  nebo heslo. Pokud interaktivní nástroj vyžaduje předání zadání hesla uživateli,
  respektuj jeho pravidla. Ověření lze dokončit na izolovaném syntetickém účtu.
- 409 znamená již dokončený setup: pokračuj přihlášením nebo provozní recovery.
  Nikdy kvůli tomu nenuluj `installation_state`, nemaž uživatele ani volumes.
- Obnova existujícího správce je dokumentovaný CLI příkaz s heslem ze souboru
  nebo stdin; odvolává relace a neotevírá bootstrap. Reset neprováděj jako pouhý
  diagnostický krok. Tajemství v `.env` a `.secrets` zachovej při opakování setupu.

## Ověření výsledku

Instalace není hotová jen proto, že se zobrazí stránka: ověř DB readiness,
jednorázové založení, přihlášení, refresh a odhlášení v požadovaném klientu.
Pro nové změny využij `pnpm test:auth`, `tests/e2e/auth.spec.ts` a podle rozsahu
`scripts/test-installation.py`; čti jejich požadavky v provozním návodu. Tyto
scénáře používají dočasné DB/projekty, nikoli uživatelovu agendu.

Před obnovou dat použij dokumentovanou zálohu a čistý cíl. Při nejasném výsledku
bootstrapu nejprve znovu načti stav, neopakuj destruktivní inicializaci. Při stejné
provozní chybě postupuj podle konkrétní diagnostiky; bez nového důkazu neopakuj
instalaci nebo reset. Výsledek popiš jako ověřený režim, dostupnou adresu,
umístění konfigurace a zbývající omezení; nikdy nepředstírej test nativního desktopu.
