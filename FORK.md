# Fork: ShardBrowser YT

Dieser Fork von [ProxyShard/ShardBrowser](https://github.com/ProxyShard/ShardBrowser) ergänzt Links der Form `shardbrowser://open/<profil-id>`. Ein Klick darauf öffnet das Profil im Launcher genau wie der Open-Button: sichtbar und ohne CDP. Läuft das Profil bereits, wird sein Fenster nach vorne geholt. Profile, die bei einem anderen Teammitglied offen sind, sperrt der Team-Server wie bei jedem Start.

Außerdem gibt es Links für den Channel-Organizer, jeweils mit einem Einmal-Code, den der Launcher beim Organizer einlöst:

- `shardbrowser://create/<code>` legt das Profil eines Kanals samt Proxy im Team an. Gegenüber Upstream ist dabei Noise für Canvas, WebGL, Audio, ClientRects und Fonts eingeschaltet (Upstream: alles aus, also gleiche Hashes für alle Profile eines Rechners). Die Vorlage stammt vom selben GPU-Hersteller wie die echte Grafikkarte, und die Speicher-Quota entspricht einer üblichen SSD statt der festen 10 GB der Bibliothek.
- `shardbrowser://delete/<code>` legt das Profil eines Kanals in den Team-Papierkorb (30 Tage wiederherstellbar) und löst es vom Kanal. Ein hier geöffnetes Profil wird nicht gelöscht. Der Proxy bleibt stehen, damit ein wiederhergestelltes Profil ihn noch hat.

- Änderungen: `src-tauri/src/open_link.rs`, `src-tauri/src/api.rs` (`create_for_link` und Helfer), wenige Zeilen in `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml` und `src-tauri/tauri.conf.json`.
- Zweig `youtube` mit den Änderungen. `main` bleibt unverändert.
- `yt-sync` übernimmt alle 6 Stunden den Stand von Upstream und startet `yt-build`. Bei einem Konflikt entsteht ein Issue.
- `yt-build` erzeugt das Windows-Setup und eine portable Version als Release `yt-v<version>-<n>`.

Der Browser-Kern wird unverändert vom Upstream-CDN geladen. Seine Lizenz verbietet Änderungen am Kern, die Lizenz des Launchers (MIT) erlaubt diesen Fork.
