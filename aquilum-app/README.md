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

```powershell
npm install
npm run release
```

Собирает нативное приложение с фичей `installed` (автообновление, автозапуск) и упаковывает его в установщик по `native/tauri.conf.json` через CLI Tauri. Установщик ставится туда же, куда ставилась версия на Tauri, и обновляет её. Подпись — тот же ключ minisign, `latest.json` в прежнем формате.

Финальные файлы: `../.artifacts/releases/<version>/`.

Кэши:
- `../.artifacts/cargo-native/` — разработка и тесты;
- `../.artifacts/native-bundle/` — сборка релиза.

Прежняя версия на Tauri лежит в `../tauri-version/` и нужна только для сверки возможностей.
