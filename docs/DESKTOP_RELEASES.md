# Desktopové instalátory

Workflow `desktop-release.yml` připraví vývojové instalátory Tauri ze stejného
commitu: macOS Apple Silicon/Intel DMG, Windows x64 NSIS a Linux x64 DEB/AppImage.
Spuštění z větve (`workflow_dispatch`) uloží pouze CI artifacts. Tag `vX.Y.Z`
navíc po úspěchu celé matice vytvoří **draft** GitHub Release; existující veřejné
vydání neupraví. Workflow nic automaticky nezveřejňuje.

Před tagem sjednoťte verzi v Tauri konfiguraci, kořenovém Cargo workspace a
package manifestech. Sběr artefaktů odmítne tag odlišný od desktopové verze.
Manifest u každé platformy obsahuje commit, verzi, názvy a SHA-256 souborů.
`SHA256SUMS-*.txt` slouží k ověření stažených souborů.

Současná pipeline je **nepodepsaná a nenotarizovaná**. Nepotřebuje přístup
k podpisovým tajemstvím; právo zápisu do Releases má jen závěrečný job na tagu.
Automatický updater není zapnutý. Pro jeho zavedení jsou potřeba vlastní
podpisové klíče, ověřování podpisu aktualizace a otestovaný aktualizační kanál.
Instalátor neobsahuje backend ani PostgreSQL. Použijte server ze stejného
vydání a před upgradem serveru zálohujte DB podle [instalačního návodu](INSTALLATION.md).

## Ověření vydání

1. Na každém cíli ověřte instalaci, spuštění, připojení k serveru, přihlášení,
   otevření PDF a odinstalaci; uživatelská a serverová data musí zůstat zachována.
2. Zkontrolujte manifesty a úplnost čtyř platforem. Úspěšné sestavení není test
   instalace. Podepsání či notarizaci nikdy neodvozujte z existence DMG/EXE.
3. Dokud není ověření a případný podpis hotový, ponechte vydání jako draft.
   Zveřejnění je samostatný provozní krok.

Implementace vychází z [oficiálního postupu Tauri pro GitHub](https://v2.tauri.app/distribute/pipelines/github/)
a [distribučních formátů](https://v2.tauri.app/distribute/), ověřeno 18. 10. 2026.
Actions pro přenos artifacts jsou připnuté na commity tagů upload-artifact
v4.6.2 a download-artifact v4.3.0. Křížová instalace Windows/Linux ani běh nové
workflow na GitHubu nejsou doložené pouhým lokálním sestavením.

Lokální ověření 18. 10. 2026 (macOS Apple Silicon): `tauri build --debug`
sestavil nativní aplikaci. Interaktivní DMG úprava Finderu nedoběhla;
`CI=true pnpm --filter @open-faktura/desktop exec tauri bundle --debug --bundles dmg --ci`
vytvořilo DMG 6,62 MiB a `hdiutil verify` potvrdilo integritu obrazu.
Sběr instalátoru a generování SHA-256 manifestu proběhlo nad tímto skutečným
balíčkem. Jde o debug sestavení bez ověření instalace a přihlášení v tomto běhu,
nikoli o podepsané veřejné vydání.
