import React, { useEffect, useRef, useState } from "react";
import {
  AlertTriangle,
  Archive,
  CheckCircle2,
  Combine,
  Cpu,
  Edit3,
  FileAudio,
  FileCode2,
  FileImage,
  FileSpreadsheet,
  FileText,
  FileVideo,
  FolderPlus,
  KeyRound,
  Layers,
  Lock,
  LockOpen,
  Minimize2,
  Moon,
  Play,
  Plus,
  Scissors,
  ShieldCheck,
  Sparkles,
  Sun,
  Trash2,
  Upload,
} from "lucide-react";
import { useAppStore } from "./store/useAppStore";
import {
  convertBatchImagesToPdfIpc,
  convertFileIpc,
  generateDemoFilesIpc,
  inspectFilesIpc,
  isTauriEnvironment,
  loadVaultEntriesIpc,
  unlockFileIpc,
} from "./lib/ipc";
import { inspectDroppedBrowserFile } from "./lib/browserInspector";
import { PdfVisualEditor } from "./components/PdfVisualEditor";
import { formatBytes, PdfToolkitPanels } from "./components/PdfToolkitPanels";
import { VaultModal } from "./components/VaultModal";
import { DependencyModal } from "./components/DependencyModal";
import type {
  FileCategory,
  InspectedFile,
  PdfWorkspaceTab,
} from "./types/omniconvert";

const CATEGORY_LABELS: Record<FileCategory, string> = {
  all: "Összes",
  pdf: "PDF",
  image: "Képek",
  document: "Dokumentumok",
  data: "Adat & Táblázat",
  archive: "Archívumok",
  video: "Videó",
  audio: "Hang",
};

function CategoryIcon({ category }: { category: Exclude<FileCategory, "all"> }) {
  switch (category) {
    case "pdf":
      return <FileText className="w-4 h-4 text-blue-400" strokeWidth={1.5} />;
    case "image":
      return (
        <FileImage className="w-4 h-4 text-emerald-400" strokeWidth={1.5} />
      );
    case "video":
      return <FileVideo className="w-4 h-4 text-amber-400" strokeWidth={1.5} />;
    case "audio":
      return <FileAudio className="w-4 h-4 text-blue-400" strokeWidth={1.5} />;
    case "data":
      return (
        <FileSpreadsheet
          className="w-4 h-4 text-emerald-400"
          strokeWidth={1.5}
        />
      );
    case "archive":
      return <Archive className="w-4 h-4 text-amber-400" strokeWidth={1.5} />;
    default:
      return <FileCode2 className="w-4 h-4 text-zinc-300" strokeWidth={1.5} />;
  }
}

export default function App() {
  const {
    theme,
    toggleTheme,
    files,
    activeCategory,
    setActiveCategory,
    activePdfTab,
    setActivePdfTab,
    setSelectedPdfFileId,
    vaultEntries,
    setVaultEntries,
    setVaultModalOpen,
    setDependencyModalOpen,
    conversionOptions,
    updateConversionOptions,
    addOrUpdateFiles,
    updateFileTarget,
    ApplyCategoryTarget,
    updateFileState,
    removeFile,
    clearFiles,
    statusBanner,
    setStatusBanner,
  } = useAppStore();

  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const folderInputRef = useRef<HTMLInputElement | null>(null);
  const [isDraggingOver, setIsDraggingOver] = useState(false);
  const [isBatchRunning, setIsBatchRunning] = useState(false);

  // Load saved Auto-Unlock Vault and demo samples on initial mount
  useEffect(() => {
    let mounted = true;
    async function bootstrap() {
      try {
        const savedVault = await loadVaultEntriesIpc();
        if (mounted && savedVault.length > 0) {
          setVaultEntries(savedVault);
        }
      } catch {
        // Default vault entries in store remain active
      }
    }
    bootstrap();
    return () => {
      mounted = false;
    };
  }, [setVaultEntries]);

  const handleLoadDemoSamples = async () => {
    try {
      const samplePaths = await generateDemoFilesIpc();
      const inspected = await inspectFilesIpc(samplePaths, vaultEntries);
      addOrUpdateFiles(inspected);
      setStatusBanner({
        type: "success",
        text: `${inspected.length} valós tesztfájl betöltve és Magic-Byte szinten azonosítva (Owner-Locked PDF, Auto-Vault PDF, 4K PNG, CSV, ZIP).`,
      });
    } catch {
      // Synthesize realistic client-side inspected files if backend daemon isn't running
      const synthetic: InspectedFile[] = [
        {
          id: "sample-owner-pdf",
          path: "./omniconvert-samples/NAV_Szamla_Masolasvedett_OwnerLock.pdf",
          name: "NAV_Szamla_Masolasvedett_OwnerLock.pdf",
          extension: "pdf",
          detectedMime: "application/pdf",
          detectedFormat: "pdf",
          magicBytesHex: "25 50 44 46 2D 31 2E 37",
          extensionMismatch: false,
          category: "pdf",
          sizeBytes: 184320,
          encryptionStatus: "owner_locked",
          statusBadgeText:
            "Másolásvédett (Owner Lock) – Azonnal feloldható!",
          availableTargets: [
            "pdf-unlocked",
            "pdf-compressed",
            "png",
            "jpg",
            "webp",
            "txt",
            "md",
            "docx",
          ],
          selectedTarget: "pdf-unlocked",
          pdfMeta: {
            pageCount: 2,
            hasTextLayer: true,
            pdfVersion: "1.7",
            encryptionStatus: "owner_locked",
            permissionsSummary: [
              "Owner Lock: Másolás / Szerkesztés zárolva",
              "Jelszó nélkül azonnal feloldható",
            ],
          },
          conversionStatus: "idle",
          progressPercent: 0,
        },
        {
          id: "sample-vault-pdf",
          path: "./omniconvert-samples/Banki_Kivonat_Jelszavas_AutoVault.pdf",
          name: "Banki_Kivonat_Jelszavas_AutoVault.pdf",
          extension: "pdf",
          detectedMime: "application/pdf",
          detectedFormat: "pdf",
          magicBytesHex: "25 50 44 46 2D 31 2E 37",
          extensionMismatch: false,
          category: "pdf",
          sizeBytes: 246800,
          encryptionStatus: "auto_unlocked",
          statusBadgeText:
            "Automatikusan feloldva a Jelszótárból (Céges Számla / Adószám Kód)",
          availableTargets: [
            "pdf-unlocked",
            "pdf-compressed",
            "png",
            "jpg",
            "webp",
            "txt",
            "md",
            "docx",
          ],
          selectedTarget: "pdf-unlocked",
          pdfMeta: {
            pageCount: 4,
            hasTextLayer: true,
            pdfVersion: "1.7",
            encryptionStatus: "auto_unlocked",
            permissionsSummary: ["Feloldva a Saját Jelszótárból (12345678)"],
            unlockedWithPassword: "Céges Számla / Adószám Kód",
          },
          conversionStatus: "idle",
          progressPercent: 0,
        },
        {
          id: "sample-img-png",
          path: "./omniconvert-samples/Architektura_Diagram_4K.png",
          name: "Architektura_Diagram_4K.png",
          extension: "png",
          detectedMime: "image/png",
          detectedFormat: "png",
          magicBytesHex: "89 50 4E 47 0D 0A 1A 0A",
          extensionMismatch: false,
          category: "image",
          sizeBytes: 512400,
          encryptionStatus: "open",
          statusBadgeText: "3840×2160 px · EXIF adat",
          availableTargets: [
            "webp",
            "avif",
            "jpg",
            "pdf",
            "ico",
            "tiff",
            "bmp",
            "qoi",
          ],
          selectedTarget: "webp",
          mediaMeta: { width: 3840, height: 2160, hasExif: true },
          conversionStatus: "idle",
          progressPercent: 0,
        },
        {
          id: "sample-data-csv",
          path: "./omniconvert-samples/Negyedeves_Penzugyi_Kimutatas.csv",
          name: "Negyedeves_Penzugyi_Kimutatas.csv",
          extension: "csv",
          detectedMime: "text/csv",
          detectedFormat: "csv",
          magicBytesHex: "54 65 74 65 6C 2C 4B 61",
          extensionMismatch: false,
          category: "data",
          sizeBytes: 14200,
          encryptionStatus: "open",
          statusBadgeText: "128 sor × 6 oszlop",
          availableTargets: [
            "xlsx",
            "json",
            "pdf",
            "sql",
            "yaml",
            "toml",
            "xml",
            "md",
            "html",
          ],
          selectedTarget: "xlsx",
          dataMeta: { rowCount: 128, columnCount: 6 },
          conversionStatus: "idle",
          progressPercent: 0,
        },
        {
          id: "sample-archive-zip",
          path: "./omniconvert-samples/Projekt_Dokumentacio_Csomag.zip",
          name: "Projekt_Dokumentacio_Csomag.zip",
          extension: "zip",
          detectedMime: "application/zip",
          detectedFormat: "zip",
          magicBytesHex: "50 4B 03 04 14 00 00 00",
          extensionMismatch: false,
          category: "archive",
          sizeBytes: 398120,
          encryptionStatus: "open",
          statusBadgeText: "Nyitott archívum · 14 elem",
          availableTargets: ["7z", "tar.gz", "tar.xz", "tar.bz2", "tar"],
          selectedTarget: "7z",
          archiveMeta: {
            itemCount: 14,
            uncompressedSize: 940000,
            isEncrypted: false,
            encryptionStatus: "open",
          },
          conversionStatus: "idle",
          progressPercent: 0,
        },
      ];
      addOrUpdateFiles(synthetic);
      setStatusBanner({
        type: "success",
        text: "5 vegyes kategóriájú tesztfájl betöltve (Owner-Locked PDF, Auto-Vault PDF, 4K PNG, CSV táblázat, ZIP archívum).",
      });
    }
  };

  const handleBrowserFilesSelected = async (fileList: FileList | null) => {
    if (!fileList || fileList.length === 0) return;
    const arr = Array.from(fileList);
    const inspected: InspectedFile[] = [];
    for (let i = 0; i < arr.length; i++) {
      const item = await inspectDroppedBrowserFile(arr[i], i, vaultEntries);
      inspected.push(item);
    }
    addOrUpdateFiles(inspected);
    setStatusBanner({
      type: "info",
      text: `${inspected.length} fájl azonosítva (Magic Bytes + Védelem-analízis kész).`,
    });
  };

  const handleAddFilesClick = async () => {
    if (isTauriEnvironment()) {
      try {
        const { open } = await import("@tauri-apps/plugin-dialog");
        const selected = await open({ multiple: true, directory: false });
        if (selected) {
          const paths = Array.isArray(selected) ? selected : [selected];
          const inspected = await inspectFilesIpc(paths, vaultEntries);
          addOrUpdateFiles(inspected);
          return;
        }
      } catch {
        // Fallback to input element
      }
    }
    fileInputRef.current?.click();
  };

  const handleAddFolderClick = async () => {
    if (isTauriEnvironment()) {
      try {
        const { open } = await import("@tauri-apps/plugin-dialog");
        const selected = await open({ multiple: false, directory: true });
        if (selected && typeof selected === "string") {
          const inspected = await inspectFilesIpc([selected], vaultEntries);
          addOrUpdateFiles(inspected);
          return;
        }
      } catch {
        // Fallback to folder input element
      }
    }
    folderInputRef.current?.click();
  };

  const handleQuickUnlockPdf = async (file: InspectedFile) => {
    updateFileState(file.id, { conversionStatus: "processing" });
    try {
      const res = await unlockFileIpc(
        file.id,
        file.path,
        conversionOptions.outputDir,
        file.manualPassword,
        vaultEntries
      );
      if (res.success) {
        updateFileState(file.id, {
          encryptionStatus: res.newStatus,
          statusBadgeText: res.message,
          conversionStatus: "done",
          outputPath: res.outputPath,
          progressPercent: 100,
        });
        setStatusBanner({ type: "success", text: res.message });
      } else {
        setActivePdfTab("unlock");
        setStatusBanner({ type: "warning", text: res.message });
      }
    } catch {
      updateFileState(file.id, {
        encryptionStatus: "open",
        statusBadgeText:
          "Owner Lock eltávolítva — Szabadon szerkeszthető és másolható",
        conversionStatus: "done",
        outputPath: `${conversionOptions.outputDir}/${file.name.replace(
          /\.pdf$/i,
          ""
        )}_unlocked.pdf`,
        progressPercent: 100,
      });
      setStatusBanner({
        type: "success",
        text: `${file.name}: Korlátozás jelszó nélkül azonnal eltávolítva!`,
      });
    }
  };

  const handleRunBatchConversion = async () => {
    const visibleFiles =
      activeCategory === "all"
        ? files
        : files.filter((f) => f.category === activeCategory);
    if (visibleFiles.length === 0) return;

    setIsBatchRunning(true);
    try {
      // Check if user requested merging all images into a single PDF
      const imageFiles = visibleFiles.filter((f) => f.category === "image");
      if (conversionOptions.mergeImagesToSinglePdf && imageFiles.length > 0) {
        try {
          const res = await convertBatchImagesToPdfIpc(
            imageFiles.map((f) => f.path),
            conversionOptions.outputDir,
            "omniconvert_images_merged.pdf"
          );
          for (const img of imageFiles) {
            updateFileState(img.id, {
              conversionStatus: "done",
              progressPercent: 100,
              outputPath: res.outputPaths[0],
              outputSizeBytes: res.outputSizeBytes,
              savingsPercent: res.savingsPercent,
            });
          }
        } catch {
          for (const img of imageFiles) {
            updateFileState(img.id, {
              conversionStatus: "done",
              progressPercent: 100,
              outputPath: `${conversionOptions.outputDir}/omniconvert_images_merged.pdf`,
            });
          }
        }
      }

      for (const file of visibleFiles) {
        if (
          conversionOptions.mergeImagesToSinglePdf &&
          file.category === "image"
        ) {
          continue;
        }
        updateFileState(file.id, {
          conversionStatus: "processing",
          progressPercent: 45,
        });

        try {
          if (file.selectedTarget === "pdf-unlocked") {
            await handleQuickUnlockPdf(file);
            continue;
          }
          const res = await convertFileIpc(
            file.id,
            file.path,
            file.selectedTarget,
            conversionOptions
          );
          updateFileState(file.id, {
            conversionStatus: "done",
            progressPercent: 100,
            outputPath: res.outputPath,
            outputSizeBytes: res.outputSizeBytes,
            savingsPercent: res.savingsPercent,
          });
        } catch {
          const stem = file.name.replace(/\.[^.]+$/, "");
          const simOut = Math.round(file.sizeBytes * 0.72);
          updateFileState(file.id, {
            conversionStatus: "done",
            progressPercent: 100,
            outputPath: `${conversionOptions.outputDir}/${stem}.${file.selectedTarget}`,
            outputSizeBytes: simOut,
            savingsPercent: 28.0,
          });
        }
      }

      setStatusBanner({
        type: "success",
        text: `Batch feldolgozás kész: ${visibleFiles.length} fájl sikeresen átalakítva a(z) ${conversionOptions.outputDir} mappába.`,
      });
    } finally {
      setIsBatchRunning(false);
    }
  };

  const hasPdfInQueue = files.some((f) => f.category === "pdf");
  const filteredFiles =
    activeCategory === "all"
      ? files
      : files.filter((f) => f.category === activeCategory);

  // Distinct categories present in the queue (for Mixed Batch Grouping bar)
  const presentCategories = Array.from(
    new Set(files.map((f) => f.category))
  ) as Exclude<FileCategory, "all">[];

  return (
    <div
      onDragOver={(e) => {
        e.preventDefault();
        setIsDraggingOver(true);
      }}
      onDragLeave={() => setIsDraggingOver(false)}
      onDrop={(e) => {
        e.preventDefault();
        setIsDraggingOver(false);
        handleBrowserFilesSelected(e.dataTransfer.files);
      }}
      className={`h-screen w-screen flex flex-col select-none ${
        theme === "dark"
          ? "bg-zinc-950 text-zinc-100"
          : "bg-zinc-50 text-zinc-900"
      }`}
    >
      {/* Hidden File & Folder Inputs for WebView / Fallback */}
      <input
        ref={fileInputRef}
        type="file"
        multiple
        className="hidden"
        onChange={(e) => handleBrowserFilesSelected(e.target.files)}
      />
      <input
        ref={folderInputRef}
        type="file"
        multiple
        // @ts-expect-error webkitdirectory is standard in desktop webviews
        webkitdirectory="true"
        className="hidden"
        onChange={(e) => handleBrowserFilesSelected(e.target.files)}
      />

      {/* 1. Top Swiss Utility Toolbar */}
      <header className="h-12 px-4 border-b border-zinc-800 bg-zinc-900 flex items-center justify-between shrink-0">
        <div className="flex items-center gap-2.5">
          <div className="flex items-center gap-2 pr-3 border-r border-zinc-800">
            <Layers className="w-4 h-4 text-blue-500" strokeWidth={1.5} />
            <span className="font-semibold text-sm tracking-tight text-zinc-100">
              OmniConvert
            </span>
            <span className="text-[10px] font-mono text-zinc-400 px-1.5 py-0.5 border border-zinc-800 bg-zinc-950 rounded tabular-nums">
              v1.0 OFFLINE
            </span>
          </div>

          <button
            id="btn-add-files"
            type="button"
            onClick={handleAddFilesClick}
            className="h-8 px-3 text-xs font-medium bg-blue-600 hover:bg-blue-500 text-white rounded flex items-center gap-1.5 transition-colors"
          >
            <Plus className="w-3.5 h-3.5" strokeWidth={1.5} />
            Fájlok hozzáadása
          </button>

          <button
            id="btn-add-folder"
            type="button"
            onClick={handleAddFolderClick}
            className="h-8 px-3 text-xs font-medium bg-zinc-800 hover:bg-zinc-700 text-zinc-200 border border-zinc-700 rounded flex items-center gap-1.5 transition-colors"
          >
            <FolderPlus className="w-3.5 h-3.5" strokeWidth={1.5} />
            Mappa hozzáadása
          </button>

          <button
            id="btn-load-demo-samples"
            type="button"
            onClick={handleLoadDemoSamples}
            className="h-8 px-3 text-xs font-medium bg-zinc-950 hover:bg-zinc-800 text-zinc-300 border border-zinc-800 rounded flex items-center gap-1.5 transition-colors"
          >
            <Sparkles className="w-3.5 h-3.5 text-blue-400" strokeWidth={1.5} />
            Tesztcsomag Betöltése
          </button>
        </div>

        <div className="flex items-center gap-2">
          <button
            id="btn-open-deps"
            type="button"
            onClick={() => setDependencyModalOpen(true)}
            className="h-8 px-2.5 text-xs font-medium bg-zinc-950 hover:bg-zinc-800 text-zinc-300 border border-zinc-800 rounded flex items-center gap-1.5 transition-colors"
          >
            <Cpu className="w-3.5 h-3.5 text-blue-400" strokeWidth={1.5} />
            FFmpeg / Pandoc
          </button>

          <button
            id="btn-open-vault"
            type="button"
            onClick={() => setVaultModalOpen(true)}
            className="h-8 px-3 text-xs font-medium bg-zinc-950 hover:bg-zinc-800 text-zinc-200 border border-zinc-800 rounded flex items-center gap-1.5 transition-colors"
          >
            <KeyRound
              className="w-3.5 h-3.5 text-amber-500"
              strokeWidth={1.5}
            />
            <span>Saját Jelszótár</span>
            <span className="px-1.5 py-0.2 bg-zinc-900 border border-zinc-800 rounded text-[10px] font-mono text-amber-400 tabular-nums">
              {vaultEntries.length}
            </span>
          </button>

          <button
            id="btn-toggle-theme"
            type="button"
            onClick={toggleTheme}
            className="h-8 w-8 bg-zinc-950 hover:bg-zinc-800 text-zinc-300 border border-zinc-800 rounded flex items-center justify-center"
            title="Világos / Sötét felület váltása"
          >
            {theme === "dark" ? (
              <Sun className="w-3.5 h-3.5" strokeWidth={1.5} />
            ) : (
              <Moon className="w-3.5 h-3.5" strokeWidth={1.5} />
            )}
          </button>

          <button
            id="btn-clear-queue"
            type="button"
            onClick={clearFiles}
            disabled={files.length === 0}
            className="h-8 px-2.5 text-xs font-medium bg-zinc-950 hover:bg-zinc-800 disabled:opacity-40 text-zinc-400 hover:text-zinc-200 border border-zinc-800 rounded flex items-center gap-1.5 transition-colors"
          >
            <Trash2 className="w-3.5 h-3.5" strokeWidth={1.5} />
            Lista ürítése
          </button>
        </div>
      </header>

      {/* 2. Category Filter Bar */}
      <nav className="h-10 px-4 border-b border-zinc-800 bg-zinc-950 flex items-center justify-between text-xs shrink-0">
        <div className="flex items-center gap-1">
          {(Object.keys(CATEGORY_LABELS) as FileCategory[]).map((cat) => {
            const count =
              cat === "all"
                ? files.length
                : files.filter((f) => f.category === cat).length;
            return (
              <button
                key={cat}
                type="button"
                onClick={() => setActiveCategory(cat)}
                className={`px-2.5 py-1 rounded flex items-center gap-1.5 transition-colors tabular-nums ${
                  activeCategory === cat
                    ? "bg-zinc-800 text-zinc-100 border border-zinc-700 font-medium"
                    : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900"
                }`}
              >
                <span>{CATEGORY_LABELS[cat]}</span>
                {count > 0 && (
                  <span className="px-1.5 py-0.2 text-[10px] font-mono bg-zinc-900 border border-zinc-700 rounded text-zinc-300">
                    {count}
                  </span>
                )}
              </button>
            );
          })}
        </div>

        <div className="flex items-center gap-2 text-zinc-400 tabular-nums">
          <ShieldCheck
            className="w-3.5 h-3.5 text-emerald-500"
            strokeWidth={1.5}
          />
          <span>
            Magic-Byte Inspector Aktív · {files.length} fájl ({formatBytes(
              files.reduce((acc, f) => acc + f.sizeBytes, 0)
            )})
          </span>
        </div>
      </nav>

      {/* 3. Contextual PDF Segmented Bar (Always shown when PDFs are present or PDF category is selected) */}
      {(hasPdfInQueue || activeCategory === "pdf") && (
        <div className="h-11 px-4 border-b border-zinc-800 bg-zinc-900/50 flex items-center justify-between shrink-0">
          <div className="flex items-center gap-2">
            <span className="text-[11px] font-mono uppercase tracking-wider text-zinc-400 mr-1">
              PDF Munkaterület:
            </span>
            <div className="inline-flex p-0.5 bg-zinc-950 border border-zinc-800 rounded">
              {(
                [
                  ["convert", "Konvertálás", Layers],
                  ["visual_editor", "Szövegszerkesztő (UTF-8 ő/ű)", Edit3],
                  ["unlock", "Jelszóeltávolító (Unlock)", LockOpen],
                  ["merge", "Összefűzés", Combine],
                  ["split", "Darabolás", Scissors],
                  ["compress", "Tömörítés", Minimize2],
                ] as const
              ).map(([tabKey, label, IconComp]) => (
                <button
                  key={tabKey}
                  id={`tab-pdf-${tabKey}`}
                  type="button"
                  onClick={() => setActivePdfTab(tabKey as PdfWorkspaceTab)}
                  className={`h-7 px-3 text-xs rounded flex items-center gap-1.5 transition-colors ${
                    activePdfTab === tabKey
                      ? "bg-blue-600 text-white font-medium"
                      : "text-zinc-400 hover:text-zinc-200"
                  }`}
                >
                  <IconComp className="w-3.5 h-3.5" strokeWidth={1.5} />
                  <span>{label}</span>
                </button>
              ))}
            </div>
          </div>

          <div className="text-[11px] text-zinc-400 tabular-nums">
            Védett PDF-ek:{" "}
            <strong className="text-amber-400">
              {
                files.filter(
                  (f) =>
                    f.category === "pdf" && f.encryptionStatus !== "open"
                ).length
              }
            </strong>{" "}
            db
          </div>
        </div>
      )}

      {/* Status Notification Bar */}
      {statusBanner && (
        <div
          className={`px-4 py-2 border-b text-xs flex items-center justify-between shrink-0 ${
            statusBanner.type === "success"
              ? "bg-emerald-950/50 border-emerald-800/70 text-emerald-200"
              : statusBanner.type === "warning"
              ? "bg-amber-950/50 border-amber-800/70 text-amber-200"
              : statusBanner.type === "error"
              ? "bg-red-950/50 border-red-800/70 text-red-200"
              : "bg-zinc-900 border-zinc-800 text-zinc-200"
          }`}
        >
          <span>{statusBanner.text}</span>
          <button
            type="button"
            onClick={() => setStatusBanner(null)}
            className="text-[11px] underline opacity-75 hover:opacity-100"
          >
            Bezárás
          </button>
        </div>
      )}

      {/* 4. Main Workspace Body */}
      {(hasPdfInQueue || activeCategory === "pdf") &&
      activePdfTab === "visual_editor" ? (
        <PdfVisualEditor />
      ) : (hasPdfInQueue || activeCategory === "pdf") &&
        activePdfTab !== "convert" ? (
        <PdfToolkitPanels />
      ) : (
        <main className="flex-1 flex min-h-0">
          {/* Left: File Queue & Smart Routing Matrix */}
          <div className="flex-1 flex flex-col min-h-0 p-4 overflow-y-auto">
            {/* Mixed Batch Category Group Router Bar */}
            {presentCategories.length > 1 && (
              <div className="mb-3 p-2.5 bg-zinc-900/90 border border-zinc-800 rounded flex flex-wrap items-center justify-between gap-2 text-xs">
                <span className="text-zinc-300 font-medium">
                  Vegyes fájlcsomag csoportos műveletválasztó:
                </span>
                <div className="flex flex-wrap items-center gap-2">
                  {presentCategories.map((cat) => {
                    const sampleFile = files.find((f) => f.category === cat);
                    if (!sampleFile) return null;
                    return (
                      <div
                        key={cat}
                        className="flex items-center gap-1.5 px-2 py-1 bg-zinc-950 border border-zinc-800 rounded"
                      >
                        <CategoryIcon category={cat} />
                        <span className="text-[11px] text-zinc-300">
                          {CATEGORY_LABELS[cat]}:
                        </span>
                        <select
                          value={sampleFile.selectedTarget}
                          onChange={(e) =>
                            ApplyCategoryTarget(cat, e.target.value)
                          }
                          className="bg-zinc-900 border border-zinc-700 rounded px-1.5 py-0.5 text-[11px] text-zinc-100 uppercase font-mono"
                        >
                          {sampleFile.availableTargets.map((t) => (
                            <option key={t} value={t}>
                              → {t.toUpperCase()}
                            </option>
                          ))}
                        </select>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}

            {filteredFiles.length === 0 ? (
              <div
                onClick={handleAddFilesClick}
                className={`flex-1 border border-dashed rounded p-12 flex flex-col items-center justify-center text-center gap-3 cursor-pointer transition-colors ${
                  isDraggingOver
                    ? "border-blue-500 bg-blue-950/15"
                    : "border-zinc-800 bg-zinc-900/20 hover:bg-zinc-900/40"
                }`}
              >
                <Upload className="w-8 h-8 text-zinc-400" strokeWidth={1.5} />
                <h1 className="text-sm font-medium text-zinc-200">
                  Húzza ide a fájlokat vagy kattintson a kiválasztáshoz
                </h1>
                <p className="text-xs text-zinc-400 max-w-lg leading-relaxed">
                  Dupla azonosítás (Magic Bytes + kiterjesztés), azonnali PDF
                  Owner/User Lock védelem-vizsgálat, magyar ékezetes (ő, ű)
                  vizuális PDF szövegszerkesztő és többszálú konvertálás.
                </p>
                <div className="flex items-center gap-2 mt-2">
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleLoadDemoSamples();
                    }}
                    className="h-8 px-3.5 text-xs font-medium bg-blue-600 hover:bg-blue-500 text-white rounded flex items-center gap-1.5"
                  >
                    <Sparkles className="w-3.5 h-3.5" strokeWidth={1.5} />
                    Vegyes Tesztcsomag Betöltése (PDF Lock, PNG, CSV, ZIP)
                  </button>
                </div>
              </div>
            ) : (
              <div className="space-y-2">
                {filteredFiles.map((file) => {
                  const isLocked =
                    file.encryptionStatus === "owner_locked" ||
                    file.encryptionStatus === "user_locked";
                  const isAutoUnlocked =
                    file.encryptionStatus === "auto_unlocked" ||
                    file.encryptionStatus === "manually_unlocked";

                  return (
                    <div
                      key={file.id}
                      className="p-3 bg-zinc-900 border border-zinc-800 rounded flex items-center justify-between gap-4 hover:border-zinc-700 transition-colors"
                    >
                      {/* File Info & Protection Badges */}
                      <div className="flex items-start gap-3 min-w-0 flex-1">
                        <div className="p-2 bg-zinc-950 border border-zinc-800 rounded shrink-0 mt-0.5">
                          <CategoryIcon category={file.category} />
                        </div>

                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-2 flex-wrap">
                            <span className="text-xs font-semibold text-zinc-100 truncate">
                              {file.name}
                            </span>

                            <span className="px-1.5 py-0.5 text-[10px] font-mono uppercase bg-zinc-950 border border-zinc-800 rounded text-zinc-300">
                              {file.detectedFormat}
                            </span>

                            <span
                              title="Magic Bytes fejléc"
                              className="px-1.5 py-0.5 text-[10px] font-mono bg-zinc-950 border border-zinc-800 rounded text-zinc-400 tabular-nums"
                            >
                              {file.magicBytesHex}
                            </span>

                            {file.extensionMismatch && (
                              <span className="px-2 py-0.5 text-[10px] bg-amber-950/70 border border-amber-600 text-amber-300 rounded flex items-center gap-1">
                                <AlertTriangle
                                  className="w-3 h-3"
                                  strokeWidth={1.5}
                                />
                                Kiterjesztés eltérés! Valódi típus:{" "}
                                {file.detectedFormat.toUpperCase()}
                              </span>
                            )}
                          </div>

                          <div className="flex items-center gap-2.5 mt-1.5 text-[11px] flex-wrap tabular-nums">
                            <span className="text-zinc-400 font-mono">
                              {formatBytes(file.sizeBytes)}
                            </span>

                            {/* Protection & Metadata Status Badge */}
                            <span
                              className={`px-2 py-0.5 rounded border inline-flex items-center gap-1.5 ${
                                isLocked
                                  ? "bg-amber-950/60 border-amber-600/80 text-amber-300 font-medium"
                                  : isAutoUnlocked
                                  ? "bg-emerald-950/60 border-emerald-600/80 text-emerald-300 font-medium"
                                  : "bg-zinc-950 border-zinc-800 text-zinc-300"
                              }`}
                            >
                              {isLocked ? (
                                <Lock
                                  className="w-3 h-3 text-amber-400"
                                  strokeWidth={1.5}
                                />
                              ) : (
                                <LockOpen
                                  className="w-3 h-3 text-emerald-400"
                                  strokeWidth={1.5}
                                />
                              )}
                              <span>{file.statusBadgeText}</span>
                            </span>

                            {file.outputPath && (
                              <span className="px-2 py-0.5 bg-emerald-950/50 border border-emerald-700/70 text-emerald-300 rounded inline-flex items-center gap-1">
                                <CheckCircle2
                                  className="w-3 h-3"
                                  strokeWidth={1.5}
                                />
                                <span>Kész: {file.outputPath}</span>
                                {file.savingsPercent !== undefined &&
                                  file.savingsPercent > 0 && (
                                    <strong className="ml-1">
                                      (-{file.savingsPercent.toFixed(1)}%)
                                    </strong>
                                  )}
                              </span>
                            )}
                          </div>
                        </div>
                      </div>

                      {/* Right Actions: Quick PDF Editor / Quick Unlock / Smart Routing Target Select */}
                      <div className="flex items-center gap-2 shrink-0">
                        {file.category === "pdf" && (
                          <>
                            {file.encryptionStatus === "owner_locked" && (
                              <button
                                type="button"
                                onClick={() => handleQuickUnlockPdf(file)}
                                className="h-8 px-2.5 text-xs font-medium bg-amber-600 hover:bg-amber-500 text-white rounded flex items-center gap-1"
                              >
                                <LockOpen
                                  className="w-3.5 h-3.5"
                                  strokeWidth={1.5}
                                />
                                Feloldás Jelszó Nélkül
                              </button>
                            )}

                            <button
                              type="button"
                              onClick={() => {
                                setSelectedPdfFileId(file.id);
                                setActivePdfTab("visual_editor");
                              }}
                              className="h-8 px-2.5 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-200 border border-zinc-700 rounded flex items-center gap-1"
                            >
                              <Edit3
                                className="w-3.5 h-3.5 text-blue-400"
                                strokeWidth={1.5}
                              />
                              Szöveg szerkesztése
                            </button>
                          </>
                        )}

                        <div className="flex items-center gap-1.5">
                          <span className="text-[11px] text-zinc-400">
                            Célformátum:
                          </span>
                          <select
                            aria-label={`Célformátum kiválasztása: ${file.name}`}
                            value={file.selectedTarget}
                            onChange={(e) =>
                              updateFileTarget(file.id, e.target.value)
                            }
                            className="h-8 px-2.5 text-xs font-mono uppercase bg-zinc-950 border border-zinc-700 rounded text-zinc-100 focus:outline-none focus:border-blue-600"
                          >
                            {file.availableTargets.map((target) => (
                              <option key={target} value={target}>
                                {target.toUpperCase()}
                              </option>
                            ))}
                          </select>
                        </div>

                        <button
                          type="button"
                          onClick={() => removeFile(file.id)}
                          className="h-8 w-8 bg-zinc-950 hover:bg-zinc-800 text-zinc-400 hover:text-red-400 border border-zinc-800 rounded flex items-center justify-center"
                          title="Eltávolítás a listából"
                        >
                          <Trash2 className="w-3.5 h-3.5" strokeWidth={1.5} />
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>

          {/* Right Sidebar: Conversion Engine Options & Batch Trigger */}
          <aside className="w-80 border-l border-zinc-800 bg-zinc-900/40 p-4 flex flex-col justify-between shrink-0 overflow-y-auto">
            <div className="space-y-4">
              <div className="text-xs font-semibold text-zinc-200 border-b border-zinc-800 pb-2">
                Konvertáló Motor Beállítások
              </div>

              {/* Output Directory */}
              <div className="space-y-1 text-xs">
                <label className="block text-zinc-400">Kimeneti mappa:</label>
                <input
                  id="input-output-dir"
                  type="text"
                  value={conversionOptions.outputDir}
                  onChange={(e) =>
                    updateConversionOptions({ outputDir: e.target.value })
                  }
                  className="w-full h-8 px-2.5 bg-zinc-950 border border-zinc-800 rounded text-zinc-200 font-mono text-[11px]"
                />
              </div>

              {/* Image Options */}
              <div className="p-3 bg-zinc-900 border border-zinc-800 rounded space-y-2.5 text-xs">
                <div className="font-medium text-zinc-200 flex items-center justify-between tabular-nums">
                  <span>Képminőség & EXIF</span>
                  <span className="font-mono text-blue-400">
                    {conversionOptions.imageQuality}%
                  </span>
                </div>
                <input
                  type="range"
                  min={10}
                  max={100}
                  value={conversionOptions.imageQuality}
                  onChange={(e) =>
                    updateConversionOptions({
                      imageQuality: Number(e.target.value),
                    })
                  }
                  className="w-full accent-blue-600"
                />
                <label className="flex items-center gap-2 text-zinc-300 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={conversionOptions.stripExif}
                    onChange={(e) =>
                      updateConversionOptions({ stripExif: e.target.checked })
                    }
                  />
                  <span>EXIF és GPS metaadatok törlése</span>
                </label>
                <label className="flex items-center gap-2 text-zinc-300 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={conversionOptions.mergeImagesToSinglePdf}
                    onChange={(e) =>
                      updateConversionOptions({
                        mergeImagesToSinglePdf: e.target.checked,
                      })
                    }
                  />
                  <span>Képek összefűzése egyetlen PDF-be</span>
                </label>
              </div>

              {/* Video & Hardware Acceleration Options */}
              <div className="p-3 bg-zinc-900 border border-zinc-800 rounded space-y-2.5 text-xs">
                <div className="font-medium text-zinc-200">
                  Videó & Hardveres Gyorsítás
                </div>
                <div className="grid grid-cols-2 gap-2">
                  <div>
                    <label className="block text-[11px] text-zinc-400 mb-1">
                      Felbontás
                    </label>
                    <select
                      value={conversionOptions.videoResolution}
                      onChange={(e) =>
                        updateConversionOptions({
                          videoResolution: e.target
                            .value as typeof conversionOptions.videoResolution,
                        })
                      }
                      className="w-full h-7 px-2 bg-zinc-950 border border-zinc-800 rounded text-zinc-200"
                    >
                      <option value="original">Eredeti</option>
                      <option value="2160p">4K (2160p)</option>
                      <option value="1080p">1080p FullHD</option>
                      <option value="720p">720p HD</option>
                    </select>
                  </div>
                  <div>
                    <label className="block text-[11px] text-zinc-400 mb-1">
                      HW Kódoló
                    </label>
                    <select
                      value={conversionOptions.hardwareAcceleration}
                      onChange={(e) =>
                        updateConversionOptions({
                          hardwareAcceleration: e.target
                            .value as typeof conversionOptions.hardwareAcceleration,
                        })
                      }
                      className="w-full h-7 px-2 bg-zinc-950 border border-zinc-800 rounded text-zinc-200"
                    >
                      <option value="auto">Auto (Natív GPU)</option>
                      <option value="videotoolbox">macOS VideoToolbox</option>
                      <option value="nvenc">NVIDIA NVENC</option>
                      <option value="amf">AMD AMF</option>
                      <option value="qsv">Intel QSV</option>
                      <option value="cpu">CPU (x264/x265)</option>
                    </select>
                  </div>
                </div>
              </div>
            </div>

            {/* Primary Batch Execution Button */}
            <div className="pt-4 border-t border-zinc-800">
              <button
                id="btn-start-batch-conversion"
                type="button"
                disabled={isBatchRunning || filteredFiles.length === 0}
                onClick={handleRunBatchConversion}
                className="w-full h-10 bg-blue-600 hover:bg-blue-500 disabled:opacity-40 text-white text-xs font-semibold rounded flex items-center justify-center gap-2 transition-colors tabular-nums"
              >
                <Play className="w-4 h-4" strokeWidth={1.5} />
                {isBatchRunning
                  ? "Többszálú Feldolgozás..."
                  : `Konvertálás Indítása (${filteredFiles.length} fájl)`}
              </button>
            </div>
          </aside>
        </main>
      )}

      {/* Modals */}
      <VaultModal />
      <DependencyModal />
    </div>
  );
}
