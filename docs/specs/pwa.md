# Online PWA a bezpečná obrazovka bez připojení

P11, stav 18. 10. 2026. Vlastní implementace Open Faktury; offline fakturace ani synchronizace nejsou v rozsahu. Mobilní aplikace používá stejný web a API, ne samostatnou účetní databázi.

## Doložené požadavky

[MDN Making PWAs installable](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Guides/Making_PWAs_installable), ověřeno 18. 10. 2026: manifest, name/short_name, skutečné192/512 ikony, start_url, standalone display a HTTPS (pro lokální ověření také loopback). Instalaci a její nabídku řídí prohlížeč; service worker sám není důkaz instalovatelnosti. [WebKit](https://webkit.org/blog/13878/web-push-for-web-apps-on-ios-and-ipados/), 16. 2. 2023, popisuje standalone manifest a přidání webu na plochu iOS/iPadOS. Neimplementujeme push ani automatický instalační dialog.

## Distribuce a registrace

`apps/web/public/manifest.webmanifest` má stabilní id/start_url/scope `/`, standalone, CS metadata a existující brand. PNG512 je kopie desktopové ikony, PNG192 a Apple180 jsou skutečně převedené velikosti pomocí sips. Purpose=any; maskable safe zone není doložena a není deklarována.

`registerOnlinePwa` se volá z main pouze v produkčním zabezpečeném webovém kontextu. Tauri (interní globální objekt nebo jiný než HTTP/S protokol) a Vite dev service worker neregistrují. Registrace nezastavuje aplikaci při chybě. Aktualizace SW nevolá skipWaiting ani nespouští reload editoru. Nová verze se aktivuje standardním životním cyklem po uvolnění původního workeru. Pro vlastní instalaci HTTPS a správně obsloužené statické soubory; nerozšiřovat scope na jiný origin. Service worker servírovat s revalidací (doporučení Cache-Control: no-cache), generované content-hashované fallback cache odděluje verze.

`node scripts/generate-pwa-offline.mjs` generuje tři veřejné statické offline soubory z CS/EN offline katalogů a sdílených design tokenů. Generuje také worker z template s hashem obsahu fallbacků; změna textu/tokenů proto vyvolá novou instalaci. Spustit před webbuild. Generované soubory jsou součástí repozitáře i distribučního webu. Nepoužívá externí font, obrázek ani síťovou službu.

## Cache a chování při výpadku

SW při install stáhne pouze `/pwa/offline.html`, `.css`, `.js`, bez přihlašovacích cookies, s no-store a bez redirectů. Ověřuje úspěch, content type i SHA256 přesných generovaných bytů před prvním zápisem. Chybné SPA přesměrování na index s HTTP200 proto nezapíše app shell do fallback cache. Cache API ukládá pouze tyto tři pevné veřejné klíče. Nikdy neukládá index, aplikační bundle, navigační URL, API odpověď, firemní obsah, PDF, přílohu ani sdílený bearer odkaz. Nemá background sync ani replay zápisů.

Fetch handler se nedotýká jiných originů, POST či jiných metod, query URL, API a přímých PDF/sdílených cest. Pro hash-router kořen `/` nebo `/index.html` používá síť bez HTTP cache; pouze při skutečném selhání sítě vrací obecnou obrazovku se statusem503/no-store. HTTP chybové odpovědi serveru nejsou maskované úspěšným cached shellem. Při nezachyceném požadavku funguje běžné síťové chování prohlížeče.

Fragment hash-routu není součástí síťového požadavku. SW tedy nerozpozná veřejný `#/shared/...` od privátního route; žádnou z těchto URL ani jejich obsah neukládá. Při offline navigaci mají jen stejný obecný fallback bez identity, dat a tajemství. Případný URL fragment už existující v historii prohlížeče není novou PWA cache.

Statická obrazovka má CS/EN volbu, čte a zapisuje pouze jazykovou preferenci. Nečte relaci ani účetní data; nevypisuje původní URL. Neoznamuje úspěch uložení. Odkaz Zkusit znovu obnoví původní navigaci; bez JS jde na bezpečný kořen. Již otevřený online editor nadále používá své společné API validace/chybové stavy. Výpadek během zápisu není možné vyložit jako úspěch ani neúspěch podle offline screen; po připojení je nutné ověřit stav, opakované příkazy mají serverové UUID replay.

Aktivace maže jen staré cache se specifickým prefixem open-faktura-offline-, nikoli cizí cache stejného originu. Odhlášení nemusí mazat osobní doklady z PWA cache, protože tam nikdy nejsou ukládány. Běžnou HTTP cache řídí stávající serverové hlavičky; tvrzení této specifikace se týká explicitního SW/Cache API chování.

## Ověření a limity

`node --test scripts/test-pwa.mjs` ověřuje skutečný generovaný worker ve VM: přesný allowlist, credentials omit, no-store, nesouvisející cache, API/PDF/bearer/query/POST/external bypass, offline503 a skutečné rozměry PNG proti manifestu. Ověřuje také transpilovaný registrační helper a odmítnutí nesprávného obsahu statických souborů. Test je syntetický technický scénář, není instalací do OS.

Dne 18. 10. 2026 prošly čtyři produkční browser scénáře nad koordinovaným Vite preview na http://127.0.0.1:5173 (CS/EN, desktop/mobile Chromium). Desktop CDP vrátil prázdné installabilityErrors a validní manifest. Skutečný SW kontroloval stránku, Cache API obsahovala právě tři statické soubory; po odpojení sítě se zobrazila lokalizovaná503 obrazovka a API/PDF/public URL nevrátily cached odpověď. Všechny čtyři screenshoty byly vizuálně přečteny: čitelné texty, žádné přetečení ani účetní obsah. Dev a Tauri nesmějí registrovat worker. Instalace na fyzickém Android/iOS nebo do desktopového OS zatím není ověřená; emulace ani CDP tuto skutečnost nedokládají.

### CI a opakovatelnost

`pnpm test:pwa` je zapojený do CI checks po produkčním sestavení. Samostatný CI job pwa používá `playwright.pwa.config.ts`, spouští vlastní produkční preview na4173 a čtyři CS/EN scénáře (desktop/mobile Chromium), bez DB. Tato konfigurace nastaví PWA_BASE_URL sama; běžná dev E2E sada bez tohoto parametru PWA scénář přeskočí. Produkční konfigurace proto zabrání falešnému zelenému CI způsobenému přeskočením dev testu.

Příkaz: `pnpm build:web && pnpm exec playwright test --config=playwright.pwa.config.ts`. PWA_WEB_PORT umožňuje koordinovaný jiný port. Výsledky zahrnují offline screenshoty a při selhání trace. Zapojení CI není tvrzením, že vzdálený GitHub Actions běh už proběhl. Lokální běh `PWA_BASE_URL=http://127.0.0.1:5173 pnpm exec playwright test tests/e2e/pwa.spec.ts --workers=1` prošel4/4 (2,9s). Unit testy6/6 a generátor z apps/web cwd i formatcheck byly ověřené.

Samostatná konfigurace byla rovněž skutečně spuštěna: `PWA_WEB_PORT=5173 pnpm exec playwright test --config=playwright.pwa.config.ts`, výsledek4/4 (2,9s), žádný skip. Tento lokální běh použil již koordinovaně spuštěné produkční preview. Automatický start vlastního preview na4173 je zapojený v konfiguraci/CI; vzdálený GitHub Actions běh není tímto lokálním výsledkem potvrzený.
