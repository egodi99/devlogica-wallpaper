# DevLogica Wallpaper

Sfondi animati DevLogica per **Windows** e **macOS**, con supporto multimonitor.
Gli sfondi sono shader che girano interamente sulla scheda grafica, quindi la CPU resta praticamente a zero.
L'app non ha finestre: vive nella **barra di sistema** (Windows) o nella **barra dei menu** (Mac).

## Funzioni

- **Tre modalità multimonitor**
  - *Stesso sfondo su ogni schermo*
  - *Uno sfondo diverso per schermo*
  - *Un unico sfondo esteso su tutti*: ogni schermo disegna la sua porzione della stessa scena, e le porzioni combaciano al pixel.
- **Sfondi inclusi:** Lame di luce, Barre diagonali, Geometrie, Reticolo.
- **Sfondi personalizzati:** basta un file `.wgsl` in una cartella, senza ricompilare (vedi sotto).
- **Fluidità regolabile:** 15 / 24 / 30 / 60 fps.
- **Risoluzione di rendering regolabile:** Automatica, Piena o Ridotta (vedi *Memoria*).
- **Logo** attivabile o disattivabile.
- **Pausa manuale.**
- **Pausa automatica:**
  - su Windows con giochi, presentazioni e app a schermo intero;
  - su Mac quando lo sfondo è coperto.
- **Avvio all'accesso.**
- **Schermi collegati o scollegati** vengono rilevati da soli.
- **Riavvio di Explorer** (Windows) gestito automaticamente.

## Requisiti per compilare

| | |
|---|---|
| Tutti | Rust stabile, da https://rustup.rs |
| Windows | *Visual Studio Build Tools* con il carico di lavoro "Sviluppo di applicazioni desktop con C++" |
| macOS | Xcode Command Line Tools (`xcode-select --install`) |

## Provare al volo

```bash
cargo run --release
```

L'icona compare nella barra di sistema o nella barra dei menu. Si chiude da lì, con **Esci**.

## Creare l'app da distribuire

**Windows** (PowerShell, nella cartella del progetto):

```powershell
.\packaging\windows\build.ps1
```

Il risultato è `dist\DevLogica Wallpaper.exe`: un unico file senza installazione, basta copiarlo sui PC.

**macOS:**

```bash
./packaging/macos/build-app.sh             # per l'architettura del Mac su cui compili
./packaging/macos/build-app.sh universal   # Apple Silicon + Intel nella stessa app
```

Il risultato è `dist/DevLogica Wallpaper.app`. Ha una firma *ad-hoc*, che non richiede un account Apple Developer.
Per distribuirla, comprimila in uno zip.

## Primo avvio senza firma

**Windows:** SmartScreen mostra "Windows ha protetto il PC". Clicca **Ulteriori informazioni** e poi **Esegui comunque**. Succede solo la prima volta.

**macOS:** al primo avvio il sistema blocca l'app. Hai due strade:

- vai in **Impostazioni di Sistema → Privacy e sicurezza**, scorri fino al messaggio sull'app e clicca **Apri comunque**;
- oppure, da Terminale:
  ```bash
  xattr -dr com.apple.quarantine "/Applications/DevLogica Wallpaper.app"
  ```

Il blocco compare solo se il file è stato scaricato da un browser o ricevuto via AirDrop. Copiandolo da una cartella condivisa in rete spesso non compare.

## Sfondi personalizzati

Dal menu scegli **Apri cartella sfondi personalizzati**, copia lì un file `.wgsl` e poi scegli **Ricarica sfondi**.
In `esempi/onde.wgsl` trovi un esempio commentato da cui partire.

Uno sfondo deve definire due funzioni:

```wgsl
// nome: Nome mostrato nel menu
fn logo_layout() -> vec3f { ... }        // centro x, centro y, larghezza del logo (pixel della tela)
fn scene(p: vec2f) -> vec3f { ... }      // colore del pixel p (y verso il basso)
```

Variabili e funzioni disponibili:
- `u.canvas`: dimensione della tela;
- `u.time`: secondi;
- `u.since`: secondi dall'ultimo cambio di sfondo, utile per le animazioni di apertura;
- tutte le funzioni di `shaders/prelude.wgsl`.

Se uno shader contiene errori, l'app non si blocca: mostra un fondo nero e scrive l'errore nel log.

Cartella degli sfondi personalizzati:
- **Windows:** `%APPDATA%\DevLogica Wallpaper\sfondi`
- **macOS:** `~/Library/Application Support/DevLogica Wallpaper/sfondi`

## Immagini statiche

Si possono esportare immagini statiche, utili come sfondo normale o come anteprima:

```bash
devlogica-wallpaper --snapshot lame 3840 2160 20 lame-4k.png
devlogica-wallpaper --snapshot barre 2880 1800 20 barre.png --no-logo
```

Con `--tile x y w h` si disegna solo una porzione della tela, come fa uno schermo in modalità estesa.

## Struttura del progetto

```
src/main.rs            avvio, istanza unica, opzione --snapshot
src/app.rs             finestre per schermo, modalità, ritmo dei fotogrammi
src/render.rs          wgpu: GPU condivisa, shader, logo, disegno
src/tray.rs            menu della barra di sistema
src/config.rs          impostazioni (config.json) e log
src/wallpapers.rs      catalogo sfondi inclusi e personalizzati
src/platform/          Windows (WorkerW/Progman) e macOS (livello desktop)
shaders/               preludio comune e sfondi inclusi
assets/                logo e icone
packaging/             script di build
```

Impostazioni e log (`config.json`, `log.txt`) si trovano nella cartella superiore a `sfondi`.

## Note tecniche e punti da verificare

- **Windows:** la finestra viene agganciata dietro le icone con il metodo usato anche da Lively Wallpaper. È previsto anche il caso di **Windows 11 24H2**, in cui la struttura delle finestre del desktop è cambiata. Il log indica quale metodo è stato usato.
- **Schermi con scale diverse** (es. MacBook Retina + monitor esterno): nella modalità estesa la scena viene calcolata in pixel fisici. Tra due schermi con densità molto diverse il passaggio può risultare leggermente sfalsato.
- **Consumi:** la GPU integrata viene preferita a quella dedicata. A 30 fps uno sfondo occupa la GPU per una frazione di millisecondo a fotogramma. Se serve, la fluidità a 15 fps dimezza ancora il lavoro.

## Memoria

Gli accorgimenti per tenere bassa la memoria:

- **Un solo backend grafico per sistema:** DirectX 12 su Windows, Metal su Mac. Vulkan e OpenGL non vengono né compilati né caricati.
- **Due buffer video per schermo invece di tre.** A 30 fps bastano.
- **Risoluzione di rendering** (menu → *Risoluzione*). Lo sfondo viene disegnato a una risoluzione più bassa e il sistema lo ingrandisce allo schermo. Su effetti morbidi come questi la differenza quasi non si vede.

| Memoria video per schermo | 1080p | 1440p | 4K |
|---|---|---|---|
| Automatica (max 1440 righe) | ~17 MB | ~30 MB | ~30 MB |
| Piena | ~17 MB | ~30 MB | ~66 MB |
| Ridotta (metà) | ~4 MB | ~7 MB | ~17 MB |

- **Nessun livello di validazione o debug** nelle build di rilascio, e blocchi di memoria GPU piccoli.
- **Logo** caricato a 1400 px, più che sufficiente anche su schermi 4K.
- **Solo gli shader degli sfondi in uso** restano in memoria.

Per misurare i consumi reali, usa sempre una build di rilascio (`cargo build --release`). La build di debug è molto più pesante.
Su Windows, Gestione attività mostra la memoria della GPU in una colonna separata: nella scheda *Dettagli* aggiungi le colonne relative alla memoria GPU dedicata e condivisa.
