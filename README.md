# OmniConvert — Univerzális Offline Fájlkonvertáló, PDF Eszköztár, Vizuális PDF Szövegszerkesztő és Dokumentum-Feloldó

Az **OmniConvert** egy 100%-ban offline működő, hirdetésmentes, villámgyors kereszplatformos asztali alkalmazás (**macOS** `.dmg`/`.app` és **Windows** `.exe`/`.msi`), amely **Tauri v2**, **Rust** (`tokio` + `rayon`) és **React 19 + TypeScript + Tailwind CSS** alapokon egyesíti az univerzális fájlkonvertálást, a natív PDF-eszköztárat, a helybeni magyar ékezetes (`ő`, `ű`) PDF-szövegszerkesztőt és a 3-szintű jelszó-/korlátozás-eltávolítót.

---

## Főbb Modulok és Funkciók

### 1. Smart File Inspector (`src-tauri/src/inspector.rs`)
- **Dupla azonosítás (Magic Bytes + Kiterjesztés):** Az `infer` és `mime_guess` crate-ek segítségével beolvassa a fájl első bájtjait (magic header), így a hibásan elnevezett vagy kiterjesztés nélküli fájlokat is pontosan felismeri.
- **Azonnali PDF Védelem- és Metaadat-analízis:**
  - `Nyitott (Open)`: Nincs titkosítás, azonnal szerkeszthető.
  - `Másolásvédett (Owner Lock)`: Megnyitható, de a másolás/nyomtatás/szerkesztés zárolva van (`/Encrypt` dictionary) — **jelszó bekérése nélkül azonnal feloldható!**
  - `Jelszóval titkosított (User Lock)` / `Auto-Unlocked`: Megnyitási jelszót kér, vagy a **Saját Jelszótárból (Auto-Unlock Keyring)** a háttérben automatikusan feloldódik.
- **Smart Routing Matrix & Vegyes Fájlcsomag Csoportosítás:** Kizárólag a fizikailag és logikailag érvényes célformátumokat kínálja fel, vegyes fájlcsomagoknál kategóriánkénti egykattintásos célformátum-választással.

### 2. Általános Konvertáló Mátrix (`src-tauri/src/converters/`)
- **Képek és Grafikák (`image`, `webp`, `ravif`, `resvg`, `kamadak-exif`):**
  - `PNG`, `JPG/JPEG`, `WEBP`, `AVIF`, `GIF`, `TIFF`, `BMP`, `ICO`, `TGA`, `PNM`, `QOI`, `DDS`, `SVG` (vektorból raszterizálás), valamint **Képek -> Egyetlen PDF-be fűzve**.
  - Minőség (1–100%), aránytartó átméretezés, EXIF és GPS metaadatok eltávolítása.
- **Adatfájlok és Táblázatok (`calamine`, `rust_xlsxwriter`, `csv`, `serde_json`, `serde_yaml`, `toml`, `quick-xml`):**
  - `XLSX`, `XLS`, `ODS`, `CSV`, `TSV`, `JSON`, `YAML`, `TOML`, `XML`, `SQL INSERT` script, valamint **formázott PDF táblázat** generálás.
- **Archívumok (`zip`, `sevenz-rust`, `tar`, `flate2`, `bzip2`, `lzma-rs`):**
  - Átcsomagolás `ZIP`, `7Z`, `TAR`, `TAR.GZ`, `TAR.BZ2`, `TAR.XZ` formátumok között.
- **Videók, Hangfájlok és Dokumentumok (`media_doc_conv.rs` + `deps.rs`):**
  - Videó (`MP4`, `MKV`, `MOV`, `AVI`, `WEBM`, `GIF`, `WEBP`) hardveres gyorsítással (`VideoToolbox`, `NVENC`, `AMF`, `QSV`), felbontásváltással (`4K`, `1080p`, `720p`) és közvetlen hangsáv-kinyeréssel (`MP3`, `WAV`, `FLAC`, `AAC`, `OPUS`).
  - Dokumentumok (`PDF`, `DOCX`, `ODT`, `RTF`, `TXT`, `MD`, `HTML`, `EPUB`, `TEX`, `RST`).

### 3. Natív PDF Eszköztár (`src-tauri/src/pdf_toolkit.rs`)
- **PDF Összefűzés (Merge):** Több PDF és képfájl egyesítése egyetlen PDF dokumentumba sorrend-átrendezéssel.
- **PDF Darabolás (Split & Extract):** Tartomány-alapú kinyerés (`1-3, 5, 8-12` egybe vagy külön), `Burst` (minden oldal külön PDF-be), vagy darabolás `X` oldalanként.
- **PDF Tömörítés (Compress):** 3 profil (`Light`, `Balanced`, `Strong`) – Flate stream újratömörítés és beágyazott `XObject` képek DPI-skálázása pontos `%` méretcsökkenés kijelzéssel.
- **PDF Oldalkezelés:** Oldalak forgatása (`90°`/`180°`/`270°`), megadott oldalak törlése, oldalak kimentése nagy felbontású képként (`PNG`/`JPG`/`WEBP`).

### 4. Jelszóeltávolító és Korlátozás-Feloldó (`src-tauri/src/unlocker.rs`)
1. **PDF Owner Lock Stripper:** Jelszó bekérése nélkül azonnal eltávolítja a másolási, nyomtatási és szerkesztési korlátozásokat.
2. **Auto-Unlock Saját Jelszótár (`Vault`):** A felhasználó saját elmentett jelszavait (pl. banki kivonatokhoz, számlákhoz tartozó adószámok/kódok) a háttérben automatikusan végigpróbálja minden védett PDF, ZIP és 7Z fájlon.
3. **Végleges Jelszómentesítés (Permanent Decrypt & Save):** A feloldott fájlokat teljesen titkosítatlan formában menti el a kimeneti mappába.

### 5. Beépített Vizuális PDF Szövegszerkesztő (`src/components/PdfVisualEditor.tsx`)
- **Helybeni szövegátírás (In-Place Text Editing):** `pdfjs-dist` alapú vászon renderelés és interaktív szövegblokk-kinyerés (`x, y, width, height, fontSize`).
- **Teljes Magyar Ékezetes (UTF-8) Betűtípus Támogatás:** A `public/fonts/` mappába csomagolt Unicode `.ttf` betűtípusok (*Inter*, *Roboto*, *JetBrains Mono*, *Noto Serif*) és a `@pdf-lib/fontkit` garantálják az **`ő`, `ű`, `Ő`, `Ű`** karakterek nyomdai minőségű mentését.
- **Extra Eszközök:** Új szövegdobozok, Kitakarás (`Whiteout/Redaction`), Aláírás rajzolása és beszúrása, Globális Keresés és Csere (`Find & Replace`), valamint teljes `Undo/Redo` (`Ctrl/Cmd + Z`) történet.

---

## Fejlesztés és Build Parancsok

```bash
# Függőségek telepítése
npm install

# Rust backend ellenőrzése és tesztelése
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml

# Frontend produkciós build
npm run build

# Tauri v2 asztali alkalmazás indítása / csomagolása
npm run tauri dev
npm run tauri build
```