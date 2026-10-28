# Fork: ShardBrowser YT

Dieser Fork von [ProxyShard/ShardBrowser](https://github.com/ProxyShard/ShardBrowser) ergänzt Links der Form `shardbrowser://open/<profil-id>`. Ein Klick darauf öffnet das Profil im Launcher genau wie der Open-Button: sichtbar und ohne CDP. Läuft das Profil bereits, wird sein Fenster nach vorne geholt. Profile, die bei einem anderen Teammitglied offen sind, sperrt der Team-Server wie bei jedem Start.

- Änderungen: `src-tauri/src/open_link.rs`, wenige Zeilen in `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml` und `src-tauri/tauri.conf.json`.
- Zweig `youtube` mit den Änderungen. `main` bleibt unverändert.
- `yt-sync` übernimmt alle 6 Stunden den Stand von Upstream und startet `yt-build`. Bei einem Konflikt entsteht ein Issue.
- `yt-build` erzeugt das Windows-Setup und eine portable Version als Release `yt-v<version>-<n>`.

Der Browser-Kern wird unverändert vom Upstream-CDN geladen. Seine Lizenz verbietet Änderungen am Kern, die Lizenz des Launchers (MIT) erlaubt diesen Fork.
