import React from "react";
import {
  FileText,
  FolderPlus,
  KeyRound,
  Layers,
  Plus,
  ShieldCheck,
  Trash2,
} from "lucide-react";
import { useAppStore } from "./store/useAppStore";

export default function App() {
  const { files, activeCategory, setActiveCategory, clearFiles } = useAppStore();

  return (
    <div className="h-screen w-screen flex flex-col bg-zinc-950 text-zinc-100 select-none">
      {/* Top Swiss Utility Toolbar */}
      <header className="h-12 px-4 border-b border-zinc-800 bg-zinc-900/80 flex items-center justify-between shrink-0">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2 pr-3 border-r border-zinc-800">
            <Layers className="w-4 h-4 text-blue-500" strokeWidth={1.5} />
            <span className="font-semibold text-sm tracking-tight">
              OmniConvert
            </span>
            <span className="text-[11px] font-mono text-zinc-400 px-1.5 py-0.5 border border-zinc-800 bg-zinc-950 rounded">
              v1.0 OFFLINE
            </span>
          </div>

          <button
            id="btn-add-files"
            type="button"
            className="h-8 px-3 text-xs font-medium bg-blue-600 hover:bg-blue-500 text-white rounded flex items-center gap-1.5 transition-colors"
          >
            <Plus className="w-3.5 h-3.5" strokeWidth={1.5} />
            Fájlok hozzáadása
          </button>

          <button
            id="btn-add-folder"
            type="button"
            className="h-8 px-3 text-xs font-medium bg-zinc-800 hover:bg-zinc-700 text-zinc-200 border border-zinc-700 rounded flex items-center gap-1.5 transition-colors"
          >
            <FolderPlus className="w-3.5 h-3.5" strokeWidth={1.5} />
            Mappa hozzáadása
          </button>
        </div>

        <div className="flex items-center gap-2">
          <button
            id="btn-open-vault"
            type="button"
            className="h-8 px-3 text-xs font-medium bg-zinc-900 hover:bg-zinc-800 text-zinc-200 border border-zinc-800 rounded flex items-center gap-1.5 transition-colors"
          >
            <KeyRound className="w-3.5 h-3.5 text-amber-500" strokeWidth={1.5} />
            Saját Jelszótár (Auto-Unlock)
          </button>

          <button
            id="btn-clear-queue"
            type="button"
            onClick={clearFiles}
            disabled={files.length === 0}
            className="h-8 px-2.5 text-xs font-medium bg-zinc-900 hover:bg-zinc-800 disabled:opacity-40 text-zinc-400 hover:text-zinc-200 border border-zinc-800 rounded flex items-center gap-1.5 transition-colors"
          >
            <Trash2 className="w-3.5 h-3.5" strokeWidth={1.5} />
            Lista ürítése
          </button>
        </div>
      </header>

      {/* Category Filter Bar */}
      <nav className="h-10 px-4 border-b border-zinc-800 bg-zinc-950 flex items-center justify-between text-xs shrink-0">
        <div className="flex items-center gap-1">
          {(
            [
              ["all", "Összes"],
              ["pdf", "PDF"],
              ["image", "Képek"],
              ["document", "Dokumentumok"],
              ["data", "Adat & Táblázat"],
              ["archive", "Archívumok"],
              ["video", "Videó"],
              ["audio", "Hang"],
            ] as const
          ).map(([key, label]) => (
            <button
              key={key}
              type="button"
              onClick={() => setActiveCategory(key)}
              className={`px-2.5 py-1 rounded transition-colors ${
                activeCategory === key
                  ? "bg-zinc-800 text-zinc-100 border border-zinc-700 font-medium"
                  : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900"
              }`}
            >
              {label}
            </button>
          ))}
        </div>

        <div className="flex items-center gap-2 text-zinc-400 tabular-nums">
          <ShieldCheck className="w-3.5 h-3.5 text-emerald-500" strokeWidth={1.5} />
          <span>100% Offline Feldolgozás · {files.length} fájl a várólistán</span>
        </div>
      </nav>

      {/* Main Workspace */}
      <main className="flex-1 flex items-center justify-center p-6">
        <div className="w-full max-w-2xl border border-dashed border-zinc-800 bg-zinc-900/30 rounded p-12 flex flex-col items-center text-center gap-3">
          <FileText className="w-8 h-8 text-zinc-400" strokeWidth={1.5} />
          <h1 className="text-sm font-medium text-zinc-200">
            Húzza ide a fájlokat vagy mappákat az azonnali vizsgálathoz
          </h1>
          <p className="text-xs text-zinc-400 max-w-md">
            Automatikus Magic-Byte azonosítás, PDF Owner/User védelem-elemzés,
            helybeni magyar ékezetes PDF szövegszerkesztés és többszálú konvertálás.
          </p>
        </div>
      </main>
    </div>
  );
}
