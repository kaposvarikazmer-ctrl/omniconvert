# AGENT.md – OmniConvert Fejlesztői Kézikönyv és Szabályrendszer

Az **OmniConvert** egy 100%-ban offline működő, hirdetésmentes, kereszplatformos (macOS & Windows) asztali alkalmazás univerzális fájlkonvertálásra, natív PDF műveletekre, vizuális PDF-szövegszerkesztésre és jelszó-/korlátozás-eltávolításra.

Részletes architektúra-szabályok: [`.agent/rules/architecture.md`](.agent/rules/architecture.md).

---

## Gyors Áttekintés

- ** Keretrendszer:** Tauri v2 (Rust 2021 Edition backend + React 19 / TypeScript 5 / Vite 6 / Tailwind CSS frontend).
- ** Fő modulok:**
  1. `FileInspector`: Magic-byte (`infer`) + kiterjesztés (`mime_guess`) alapú fájltípus-azonosítás, PDF titkosítás-vizsgálat (`Open`, `OwnerLocked`, `UserLocked`), Smart Routing Matrix.
  2. `ConversionPipeline`: Képek, Dokumentumok, Adatfájlok/Táblázatok, Archívumok, Videók és Hangfájlok többszálú (`rayon` + `tokio`) konvertálása.
  3. `PdfToolkit`: PDF Összefűzés (Merge), Darabolás (Split/Burst), 3-szintű Tömörítés (Compress `%` megtakarítással), Forgatás/Oldaltörlés/Képként mentés.
  4. `FileUnlocker`: PDF Owner Lock azonnali eltávolítása jelszó nélkül, Auto-Unlock Saját Jelszótár (`Vault`), végleges jelszómentesítés (PDF, ZIP, 7Z).
  5. `PdfVisualEditor`: `pdfjs-dist` + `pdf-lib` + `@pdf-lib/fontkit` alapú helybeni szövegszerkesztő beágyazott magyar UTF-8 TTF betűtípusokkal (`public/fonts/`), keresés és csere, kitakarás (whiteout), aláírás beszúrása és Undo/Redo támogatással.
  6. `DependencyManager`: FFmpeg, Pandoc és Typst binárisok automatikus ellenőrzése és lokális letöltése.

---

## Kötelező Ellenőrzési Lépések Minden Fázis Végén

1. `cargo check --manifest-path src-tauri/Cargo.toml`
2. `npm run build`
3. Atomi Git commit az adott fázis leírásával.

---

## "Anti-AI" Swiss Utility UI Checklist

- [x] Csak `Zinc` / `Slate` neutrális színek + `1px solid` keretek (`border-zinc-200` / `border-zinc-800`)
- [x] Kizárólag funkcionális kiemelőszínek: `#2563EB` (kék), `#10B981` (smaragdzöld), `#F59E0B` (borostyánsárga)
- [x] Zéró színátmenet (gradient), zéró blurred blob, zéró emoji
- [x] `lucide-react` ikonok `1.5px` vonalvastagsággal
- [x] Kötelező `tabular-nums` minden számértéknél, fájlméretnél és százaléknál
