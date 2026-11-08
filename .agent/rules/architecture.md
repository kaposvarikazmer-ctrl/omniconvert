# OmniConvert – Architektúra, Típusbiztonsági és "Anti-AI" UI Irányelvek

Ez a dokumentum rögzíti az **OmniConvert** (Univerzális Offline Fájlkonvertáló, PDF Eszköztár, Vizuális PDF Szövegszerkesztő és Dokumentum-Feloldó Desktop App) kötelező érvényű mérnöki és dizájn szabályait.

---

## 1. Technológiai Stack és Projektstruktúra

- **Alaprendszer:** Tauri v2 (`src-tauri/`) + React 19 + TypeScript + Vite + Tailwind CSS (`src/`).
- **Állapotkezelés:** Zustand (`src/store/useAppStore.ts`) típusbiztos szeletekkel (fájl-várólista, kategória szűrés, PDF szerkesztő Undo/Redo verem, Auto-Unlock Saját Jelszótár, Dependency Manager állapot).
- **Rust Backend (`src-tauri/src/`):**
  - `inspector.rs`: `FileInspector` – dupla azonosítás (`infer` magic bytes + `mime_guess` kiterjesztés), PDF védelem-detektálás (`Open`, `OwnerLocked`, `UserLocked`), archívum- és média-metaadatok, `Smart Routing Matrix`.
  - `converters/`: `ConversionPipeline` trait implementációk:
    - `image_conv.rs`: Natív Rust képfeldolgozás (`image`, `webp`, `resvg`, `kamadak-exif`, `ico`, `tiff`, `qoi`, `bmp`, `tga`, `pnm`, `gif`, valamint Képek -> PDF összefűzés).
    - `data_conv.rs`: Táblázatok és strukturált adatfájlok (`calamine`, `rust_xlsxwriter`, `csv`, `serde_json`, `serde_yaml`, `toml`, `quick-xml`, SQL `INSERT` generátor, PDF táblázat).
    - `archive_conv.rs`: Archívumok átcsomagolása (`zip`, `sevenz-rust`, `tar`, `flate2`, `bzip2`, `xz2`).
    - `media_doc_conv.rs`: Dokumentumok, E-könyvek, Videók és Hangfájlok (natív Rust feldolgozók + FFmpeg / Pandoc / Typst sidecar/lokális bináris integráció hardveres gyorsítással).
  - `pdf_toolkit.rs`: `PdfToolkit` (`lopdf`, `image`) – Merge (összefűzés), Split/Extract/Burst (darabolás), 3 profilú Compress (stream Flate újratömörítés + XObject képoptimalizálás `%` kijelzéssel), oldalak forgatása és törlése.
  - `unlocker.rs`: `FileUnlocker` – PDF Owner Lock azonnali eltávolítása jelszó nélkül, Auto-Unlock Saját Jelszótár (`Vault`), végleges jelszómentesítés (PDF, ZIP, 7Z).
  - `deps.rs`: `DependencyManager` – FFmpeg, Pandoc és Typst binárisok ellenőrzése és automatikus lokális letöltése.

---

## 2. Rust és TypeScript Típusbiztonsági Szabályok

1. **Szigorú Típusmegfeleltetés (1:1 Serde <-> TypeScript):**
   - Minden Tauri IPC struktúrának (`#[derive(Serialize, Deserialize, Clone, Debug)]` és `#[serde(rename_all = "camelCase")]`) pontos TypeScript megfelelővel kell rendelkeznie a `src/types/omniconvert.ts` fájlban.
   - Tilos az `any` típus használata TypeScriptben (`strict: true`).
2. **Hibakezelés (`Result<T, String>` / `AppError`):**
   - A Rust backendben tilos a `.unwrap()` használata felhasználói fájlokon vagy I/O műveleteken; minden hibát kontextusos, magyar nyelvű hibaüzenettel kell visszaadni.
3. **Párhuzamosítás (`rayon` & `tokio`):**
   - A CPU-intenzív konverziók és tömeges fájlvizsgálatok `rayon::prelude::*` és `tokio::task::spawn_blocking` segítségével futnak, hogy a UI főszála soha ne akadjon meg.
4. **Magyar Ékezetes (UTF-8) PDF Betűtípus Követelmény:**
   - A `PdfVisualEditor` modulban (`pdf-lib` + `@pdf-lib/fontkit`) kötelező a `public/fonts/` mappában tárolt teljes Unicode `.ttf` betűtípusok (*Inter*, *Roboto*, *JetBrains Mono*, *Noto Serif*) beágyazása, biztosítva az `ő`, `ű`, `Ő`, `Ű` karakterek hibátlan megjelenítését.

---

## 3. Szigorú "Anti-AI" UI/UX Irányelvek (Swiss Utility Design)

1. **TILTOTT ELEMEK:**
   - **Tilos** a lila-ciánkék-rózsaszín neon színátmenetek (`bg-gradient-to-*`) használata.
   - **Tilos** a lebegő, elmosódott színes gömbök (`blur-3xl` dekorációs blobok) és a túlzott glassmorphism alkalmazása.
   - **Tilos** az emojik használata a kódban, gombokban, státuszokban vagy címekben. Kizárólag `lucide-react` ikonok (`strokeWidth={1.5}`) használhatók.
   - **Tilos** a marketinges szövegezés vagy hero szekció — az alkalmazás induláskor azonnal a munkaterületet jeleníti meg.
2. **Elvárt Esztétika (Swiss Utility / Raycast, Linear & Apple Preview):**
   - **Színpaletta:** Neutrális `Zinc` tónusok (`border-zinc-200` világos módban, `border-zinc-800` sötét módban), tűéles `1px solid` elválasztókkal.
   - **Funkcionális színek:**
     - Elsődleges művelet / Kijelölés: `#2563EB` (`blue-600`)
     - Kész / Feloldott / Nyitott állapot: `#10B981` (`emerald-500`)
     - Zárolt / Védett fájl figyelmeztetés: `#F59E0B` (`amber-500`)
     - Hibaállapot: `#EF4444` (`red-500`)
   - **Tipográfia:** `Inter`, `-apple-system`, `Segoe UI` alapértelmezett rendszer-betűtípus; minden fájlméret, oldalszám, időtartam és százalékos érték kötelezően `tabular-nums` osztályt kap.

---

## 4. Build és Ellenőrzési Parancsok

Minden fejlesztési fázis végén kötelezően futtatandó ellenőrzések:

```bash
# 1. Rust backend típus- és fordítás-ellenőrzés
cargo check --manifest-path src-tauri/Cargo.toml

# 2. Frontend TypeScript + Vite produkciós build ellenőrzés
npm run build

# 3. Rust egységtesztek futtatása
cargo test --manifest-path src-tauri/Cargo.toml
```
