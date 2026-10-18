# P20 — Ručně potvrzené podepsané aktualizace desktopu

Vlastní návrh Open Faktury, 18. 10. 2026. Nevychází z interního updateru
iDokladu. Používá oficiální Tauri 2 updater; závislost je připnuta na
`tauri-plugin-updater=2.14.0`, Tauri v locku 2.12.3, CLI 2.12.1. Zdroje:
[Updater](https://v2.tauri.app/plugin/updater/),
[Update API](https://docs.rs/tauri-plugin-updater/2.14.0/tauri_plugin_updater/struct.Update.html),
[upstream konfigurace](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/src/config.rs),
[macOS podpis](https://v2.tauri.app/distribute/sign/macos/),
[Windows podpis](https://v2.tauri.app/distribute/sign/windows/).
Přečteno 18. 10. 2026; statický API/zdrojový kontrakt, nikoli vlastní ověření
produkční instalace jiných OS.

## Trust a konfigurace

Bez obou build proměnných `OPEN_FAKTURA_UPDATER_ENDPOINT` a
`OPEN_FAKTURA_UPDATER_PUBLIC_KEY` je updater vypnutý; při startu ani běžném
používání se nekontaktuje aktualizační server. Jedna proměnná bez druhé či
neplatná konfigurace je chyba sestavení/spuštění, nikdy fallback na unsigned
instalaci. Parametry se zapečou do binárky, nejsou UI ani serverový API vstup.
Endpoint je HTTPS bez uživatelského jména/hesla/fragmentu. Public key obsahuje
veřejný Tauri base64 key, nikoli cestu nebo privátní klíč. Jeho platnost se
kontroluje ještě před povolením pluginu. Provozovatel může mít vlastní
veřejný statický manifest nebo kompatibilní update server.

Privátní klíč a jeho heslo patří pouze do lokálního privátního souboru či
release secrets. Žádné přístupy nejsou v repozitáři, UI nebo síťové odpovědi.
Plugin přijímá pouze podepsané balíčky s ověřenou podepsanou verzí
(`requireSignedVersion=true`); odmítne neplatný podpis i podpis jiné verze.
Výchozí porovnání dovolí pouze novější verzi. Ztráta privátního klíče
vyžaduje řízenou změnu důvěry či ruční přeinstalaci; nekopírovat nový veřejný
klíč do nepodepsaného síťového manifestu jako obcházení ověření.

Podpis aktualizace je oddělený od macOS Code Signing/notarizace a Windows
Authenticode. Současné CI bez platformních certifikátů nesmí tvrdit, že
instalátor prošel těmito platformními podpisy. Přítomnost updater `.sig`
není důkazem notarizace ani úspěšné instalace cílového OS.

## Lokální IPC a UI

Pouze lokální webview `main` s původem Tauri (ve vývojovém buildu také přesný
Vite localhost origin) smí volat naše příkazy:

- `desktop_updater_status`: lokální enabled/current_version, žádná síť.
- `desktop_updater_check`: výslovná kontrola kanálu. Vrací null či
  version/current_version/notes/token. Metadata nejsou tvrzením, že byl
  balíček už ověřen; podpis se ověřuje až nad staženými byty.
- `desktop_updater_install {version,token,confirmed}`: použije pouze pending
  výsledek této kontroly, token/verze musí souhlasit, potvrzení true a stáří
  nejvýše 10 minut. Jeden souběžný check/install, žádné klientské URL,
  installer argumenty nebo záměna klíče. Obecné plugin updater IPC příkazy
  jsou v lokální capability výslovně denied. Pending se při pokusu spotřebuje;
  chyba vyžaduje novou kontrolu, nic se automaticky neopakuje.

CSP povoluje jen Tauri lokální IPC transport `ipc:` / `http://ipc.localhost`
vedle stávajícího backendu, nikoli libovolné vzdálené updater hosty v JS.
Aktualizační HTTPS požadavky běží v Rustu. Zdroj:
https://v2.tauri.app/security/csp/ (oficiální příklad lokálního IPC).

Webový Help tento panel nevykresluje. Desktopový Help má CS/EN kontrolu,
plain-text poznámky, výslovné potvrzení uložené práce a instalace dané verze.
Žádná kontrola nebo instalace neběží z mountu komponenty. Instalace může
aplikaci ukončit (zejména Windows); automatický restart je vypnutý.
Po úspěchu na macOS/Linux uživatel ručně zavře a otevře klienta. Veškeré
neúspěchy se zobrazí lokalizovaně bez syrových endpointů či výjimek.

## Síť, instalace a data

Vlastní klient načte důvěryhodný HTTPS manifest s timeoutem 30 sekund a
limitem 512 KiB skutečně přečtených bytů, včetně chunked odpovědi; nadlimitní
Content-Length odmítne předem. HTTP204 znamená žádná aktualizace. Teprve
potom `restore_update(raw_json)` pinned Tauri pluginu provede parser,
verzní comparator a výběr cíle, se striktním signed-version ověřením.
Síťové přesměrování nesmí přejít na HTTP. Poznámky jsou v UI omezené na
8 000 znaků, nevykresluje se jejich HTML.

Balíček se stáhne vlastním bounded klientem: HTTPS, nejvýše pět redirectů,
180 sekund a 256 MiB skutečně přečtených dat, nikoli jen Content-Length.
Následuje plugin verify podpisu i podepsané verze; teprve potom se volá
nativní install, který ověření opakuje. Neexistuje unsigned/insecure bypass.
Neinstaluje se nic z fixture testů ani z HTTP loopbacku v produkčním příkazu.

Identifikátor aplikace `cz.openfaktura.desktop` a cesty uživatelských dat
zůstávají stejné. Kód nemaže webview data, nastavení ani klientské úložiště,
nespouští SQL migrace a neaktualizuje server/PostgreSQL. Serverová data
zůstávají na serveru. Zálohy a kompatibilita serveru jsou stále samostatný
provozní krok; použít odpovídající vydání serveru. Zachování dat skutečnou
instalací je třeba potvrdit na každém distribučním OS a formátu.

## Build a release

`pnpm --filter @open-faktura/desktop build` používá wrapper se stejnými Tauri
argumenty. Default zůstává unsigned vývojový build bez update artefaktů.
Při obou veřejných parametrech wrapper navíc vyžaduje
`TAURI_SIGNING_PRIVATE_KEY` nebo `TAURI_SIGNING_PRIVATE_KEY_PATH` a případné
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Vytvoří dočasný config overlay s
`createUpdaterArtifacts=true`, nikoli změnu versioned configu či privátního
klíče. Tauri CLI podepíše artefakty s verzí automaticky. Manuální podpis musí
použít `tauri signer sign --app-version VERSION`; bez podepsané verze jej
collector i klient odmítnou.

CI release joby používají veřejné variables `DESKTOP_UPDATER_ENDPOINT` a
`DESKTOP_UPDATER_PUBLIC_KEY`, privátní signing secrets pouze v explicitně
zapnutém release build kroku. PR CI podpisová secrets nedostává. Stále se
vytváří jen draft release. Žádný workflow nyní neprovádí autorizaci publikace.

Collector sbírá běžné instalátory i updater archivy/signatury. V configured
režimu ověří Ed25519 podpis souboru, global signature trusted commentu i
verzi pomocí standardních Node crypto primitiv podle pinned minisign-verify
0.3.0 formátu. Bez skutečné správné `.sig` nesmí tvrdit updater podporu.
Závěrečný generátor znovu ověří assets a vyžaduje čtyři cíle před vytvořením
`latest.json`: darwin-aarch64-app, darwin-x86_64-app, windows-x86_64-nsis a
linux-x86_64-appimage. Používá klíče včetně typu balíčku; linux DEB se přidá
jen pokud je skutečně vyrobený a podepsaný, nikdy nedostane AppImage fallback.
Obyčejný nepodepsaný DEB dál slouží k ruční instalaci. macOS používá updater
`.app.tar.gz`, nikoli DMG. Default unsigned kolekce `latest.json` nevytváří.

## Ověření a jeho meze

Rust testy: disabled/partial config, lokální původ, verze/token/confirm/TTL;
mock Tauri s HTTP loopbackem a syntetickým podepsaným souborem; správný podpis,
pozměněné byty, jiná podepsaná/ohlášená verze a neplatný podpis. Používá
přímo shipped konfiguraci; validní legacy podpis bez podepsané verze musí
být odmítnut a downgrade manifest nesmí vrátit kandidáta. Samostatný test
ověřuje nadlimitní Content-Length, nadlimitní chunked JSON a HTTP204. Dočasný klíč
vzniká jen v soukromém temp adresáři při testu, child output se zachytí a
nezapisuje, klíč se po testu smaže. Test nikdy nevolá nativní install.

Node test ověřuje stejnou CLI fixture, podpis/pozměnění/verzi, kolekci čtyř
syntetických cílů a kompletní manifest, a zachování unsigned defaultu.
Nejsou to skutečné instalátory ani instalace na Windows/Linux. UI testy s
mock IPC jsou pouze ověřením textů/ovládání; nenahrazují podpisové testy.

Lokální compile/debug bundle macOS lze ověřit bez produkčních klíčů.
Skutečné podepsání/notarizace, spuštění nových release workflows, upgrade
již nainstalované aplikace, zachování dat a běh cizích OS nelze vydávat za
ověřené bez příslušných certifikátů, instalačních oprávnění a testovacích OS.
