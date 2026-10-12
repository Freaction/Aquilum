# Aquilum

Нативное приложение: Rust, Masonry, winit и vello_cpu, без WebView. Код окна — `native/`, общее ядро (файлы, поиск, история, MCP) — `core/`.

## Разработка

```powershell
native\run.bat
```

Собирает release и запускает окно. Если окно уже открыто, оно закрывается штатно, с сохранением, и запускается новая сборка.

Пересборка на лету при правке кода: `native\scripts\watch.ps1`.

## Проверка

```powershell
cargo test -p aquilum-native
cargo test -p aquilum-core
```

## Релиз

Пакеты собирает workflow `Release` в публичном репозитории (запуск руками). Сборка с фичей `installed` (автообновление, автозапуск), упаковка:
- [cargo-packager](https://github.com/crabnebula-dev/cargo-packager) по `[package.metadata.packager]` в `native/Cargo.toml`: установщик NSIS, `.app` и DMG, AppImage;
- [nfpm](https://nfpm.goreleaser.com) по `native/packaging/nfpm.yaml`: `.deb` и `.rpm` с явным списком зависимостей.

Установщик ставится туда же, куда ставилась версия на Tauri, с тем же ключом удаления в реестре, и обновляет её; имена пакетов deb и rpm (`aquilum`) прежние. Подпись — тот же ключ minisign, `latest.json` в прежнем формате. Перед выгрузкой каждый пакет ставится и запускается на чистой машине.

Локально установщик Windows без подписи:

```powershell
cargo install cargo-packager --version 0.11.8 --locked
cargo build --release -p aquilum-native --features installed
copy ..\.artifacts\cargo-native\release\aquilum-native.exe ..\.artifacts\cargo-native\release\aquilum-app.exe
cargo packager -p aquilum-native --release --formats nsis --out-dir ..\.artifacts\packages
```

Версия задаётся в `package.json` и переносится в `Cargo.toml` командой `npm run version:sync`.

Кэш сборки: `../.artifacts/cargo-native/`.

Прежняя версия на Tauri лежит в `../tauri-version/` и нужна только для сверки возможностей.
