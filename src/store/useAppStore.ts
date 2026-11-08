import { create } from "zustand";
import type {
  ConversionOptions,
  DependencyStatus,
  FileCategory,
  InspectedFile,
  PdfEditorSnapshot,
  PdfRedactionBox,
  PdfSignatureItem,
  PdfTextBlock,
  PdfWorkspaceTab,
  VaultEntry,
} from "../types/omniconvert";

interface AppState {
  theme: "dark" | "light";
  files: InspectedFile[];
  activeCategory: FileCategory;
  activePdfTab: PdfWorkspaceTab;
  selectedPdfFileId: string | null;
  vaultEntries: VaultEntry[];
  isVaultModalOpen: boolean;
  isDependencyModalOpen: boolean;
  dependencies: DependencyStatus[];
  conversionOptions: ConversionOptions;
  statusBanner: { type: "info" | "success" | "warning" | "error"; text: string } | null;

  // Visual PDF Editor state with full Undo/Redo stack
  editorPageIndex: number;
  editorPageCount: number;
  editorZoom: number;
  editorTextBlocks: PdfTextBlock[];
  editorRedactions: PdfRedactionBox[];
  editorSignatures: PdfSignatureItem[];
  editorUndoStack: PdfEditorSnapshot[];
  editorRedoStack: PdfEditorSnapshot[];

  // Actions
  toggleTheme: () => void;
  setActiveCategory: (cat: FileCategory) => void;
  setActivePdfTab: (tab: PdfWorkspaceTab) => void;
  setSelectedPdfFileId: (id: string | null) => void;
  addOrUpdateFiles: (incoming: InspectedFile[]) => void;
  updateFileTarget: (id: string, target: string) => void;
  ApplyCategoryTarget: (category: Exclude<FileCategory, "all">, target: string) => void;
  updateFileManualPassword: (id: string, password: string) => void;
  updateFileState: (id: string, patch: Partial<InspectedFile>) => void;
  reorderFiles: (fromIndex: number, toIndex: number) => void;
  removeFile: (id: string) => void;
  clearFiles: () => void;
  setVaultEntries: (entries: VaultEntry[]) => void;
  setVaultModalOpen: (open: boolean) => void;
  setDependencyModalOpen: (open: boolean) => void;
  setDependencies: (deps: DependencyStatus[]) => void;
  updateConversionOptions: (patch: Partial<ConversionOptions>) => void;
  setStatusBanner: (
    banner: { type: "info" | "success" | "warning" | "error"; text: string } | null
  ) => void;

  // PDF Visual Editor actions
  initEditorState: (blocks: PdfTextBlock[], pageCount: number) => void;
  setEditorPageIndex: (idx: number) => void;
  setEditorZoom: (zoom: number) => void;
  pushEditorSnapshot: (description: string) => void;
  updateTextBlock: (id: string, patch: Partial<PdfTextBlock>, recordUndo?: boolean) => void;
  addNewTextBlock: (block: PdfTextBlock) => void;
  addRedactionBox: (box: PdfRedactionBox) => void;
  removeRedactionBox: (id: string) => void;
  addSignatureItem: (sig: PdfSignatureItem) => void;
  removeSignatureItem: (id: string) => void;
  findAndReplaceText: (search: string, replaceWith: string, matchCase: boolean) => number;
  undoEditor: () => void;
  redoEditor: () => void;
}

const DEFAULT_VAULT: VaultEntry[] = [
  {
    id: "vault-default-1",
    label: "Céges Számla / Adószám Kód",
    password: "12345678",
    categoryHint: "Számla PDF / NAV",
    createdAt: "2026-11-01",
  },
  {
    id: "vault-default-2",
    label: "Banki Kivonat Alapjelszó",
    password: "omniconvert2026",
    categoryHint: "Banki PDF / ZIP",
    createdAt: "2026-11-01",
  },
];

export const useAppStore = create<AppState>((set, get) => ({
  theme: "dark",
  files: [],
  activeCategory: "all",
  activePdfTab: "convert",
  selectedPdfFileId: null,
  vaultEntries: DEFAULT_VAULT,
  isVaultModalOpen: false,
  isDependencyModalOpen: false,
  dependencies: [],
  conversionOptions: {
    outputDir: "./omniconvert-output",
    imageQuality: 90,
    keepAspectRatio: true,
    stripExif: true,
    mergeImagesToSinglePdf: false,
    videoResolution: "original",
    videoCrf: 23,
    hardwareAcceleration: "auto",
    audioBitrateKbps: 192,
  },
  statusBanner: null,

  editorPageIndex: 0,
  editorPageCount: 1,
  editorZoom: 1.2,
  editorTextBlocks: [],
  editorRedactions: [],
  editorSignatures: [],
  editorUndoStack: [],
  editorRedoStack: [],

  toggleTheme: () =>
    set((state) => {
      const next = state.theme === "dark" ? "light" : "dark";
      if (typeof document !== "undefined") {
        document.documentElement.classList.toggle("dark", next === "dark");
      }
      return { theme: next };
    }),

  setActiveCategory: (cat) => set({ activeCategory: cat }),
  setActivePdfTab: (tab) => set({ activePdfTab: tab }),
  setSelectedPdfFileId: (id) => set({ selectedPdfFileId: id }),

  addOrUpdateFiles: (incoming) =>
    set((state) => {
      const map = new Map(state.files.map((f) => [f.path, f]));
      for (const item of incoming) {
        map.set(item.path, item);
      }
      const merged = Array.from(map.values());
      const firstPdf = merged.find((f) => f.category === "pdf");
      return {
        files: merged,
        selectedPdfFileId: state.selectedPdfFileId ?? firstPdf?.id ?? null,
      };
    }),

  updateFileTarget: (id, target) =>
    set((state) => ({
      files: state.files.map((f) =>
        f.id === id ? { ...f, selectedTarget: target } : f
      ),
    })),

  ApplyCategoryTarget: (category, target) =>
    set((state) => ({
      files: state.files.map((f) =>
        f.category === category && f.availableTargets.includes(target)
          ? { ...f, selectedTarget: target }
          : f
      ),
    })),

  updateFileManualPassword: (id, password) =>
    set((state) => ({
      files: state.files.map((f) =>
        f.id === id ? { ...f, manualPassword: password } : f
      ),
    })),

  updateFileState: (id, patch) =>
    set((state) => ({
      files: state.files.map((f) => (f.id === id ? { ...f, ...patch } : f)),
    })),

  reorderFiles: (fromIndex, toIndex) =>
    set((state) => {
      const copy = [...state.files];
      const [moved] = copy.splice(fromIndex, 1);
      if (moved) {
        copy.splice(toIndex, 0, moved);
      }
      return { files: copy };
    }),

  removeFile: (id) =>
    set((state) => {
      const remaining = state.files.filter((f) => f.id !== id);
      const nextPdf = remaining.find((f) => f.category === "pdf");
      return {
        files: remaining,
        selectedPdfFileId:
          state.selectedPdfFileId === id
            ? nextPdf?.id ?? null
            : state.selectedPdfFileId,
      };
    }),

  clearFiles: () =>
    set({
      files: [],
      selectedPdfFileId: null,
      statusBanner: null,
    }),

  setVaultEntries: (entries) => set({ vaultEntries: entries }),
  setVaultModalOpen: (open) => set({ isVaultModalOpen: open }),
  setDependencyModalOpen: (open) => set({ isDependencyModalOpen: open }),
  setDependencies: (deps) => set({ dependencies: deps }),
  updateConversionOptions: (patch) =>
    set((state) => ({
      conversionOptions: { ...state.conversionOptions, ...patch },
    })),
  setStatusBanner: (banner) => set({ statusBanner: banner }),

  initEditorState: (blocks, pageCount) =>
    set({
      editorTextBlocks: blocks,
      editorPageCount: Math.max(1, pageCount),
      editorPageIndex: 0,
      editorRedactions: [],
      editorSignatures: [],
      editorUndoStack: [],
      editorRedoStack: [],
    }),

  setEditorPageIndex: (idx) => set({ editorPageIndex: idx }),
  setEditorZoom: (zoom) => set({ editorZoom: zoom }),

  pushEditorSnapshot: (description) => {
    const state = get();
    const snapshot: PdfEditorSnapshot = {
      textBlocks: state.editorTextBlocks.map((b) => ({ ...b })),
      redactions: state.editorRedactions.map((r) => ({ ...r })),
      signatures: state.editorSignatures.map((s) => ({ ...s })),
      description,
    };
    set({
      editorUndoStack: [...state.editorUndoStack.slice(-29), snapshot],
      editorRedoStack: [],
    });
  },

  updateTextBlock: (id, patch, recordUndo = true) => {
    if (recordUndo) {
      get().pushEditorSnapshot("Szöveg módosítása");
    }
    set((state) => ({
      editorTextBlocks: state.editorTextBlocks.map((b) =>
        b.id === id
          ? {
              ...b,
              ...patch,
              isModified:
                (patch.currentText ?? b.currentText) !== b.originalText ||
                b.isNew,
            }
          : b
      ),
    }));
  },

  addNewTextBlock: (block) => {
    get().pushEditorSnapshot("Új szövegdoboz beszúrása");
    set((state) => ({
      editorTextBlocks: [...state.editorTextBlocks, block],
    }));
  },

  addRedactionBox: (box) => {
    get().pushEditorSnapshot("Kitakarás (Whiteout) hozzáadása");
    set((state) => ({
      editorRedactions: [...state.editorRedactions, box],
    }));
  },

  removeRedactionBox: (id) => {
    get().pushEditorSnapshot("Kitakarás törlése");
    set((state) => ({
      editorRedactions: state.editorRedactions.filter((r) => r.id !== id),
    }));
  },

  addSignatureItem: (sig) => {
    get().pushEditorSnapshot("Aláírás beszúrása");
    set((state) => ({
      editorSignatures: [...state.editorSignatures, sig],
    }));
  },

  removeSignatureItem: (id) => {
    get().pushEditorSnapshot("Aláírás törlése");
    set((state) => ({
      editorSignatures: state.editorSignatures.filter((s) => s.id !== id),
    }));
  },

  findAndReplaceText: (search, replaceWith, matchCase) => {
    if (!search) return 0;
    const state = get();
    let count = 0;
    const escaped = search.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const regex = new RegExp(escaped, matchCase ? "g" : "gi");

    const updated = state.editorTextBlocks.map((block) => {
      if (regex.test(block.currentText)) {
        regex.lastIndex = 0;
        const matches = block.currentText.match(regex);
        count += matches ? matches.length : 0;
        const nextText = block.currentText.replace(regex, replaceWith);
        return {
          ...block,
          currentText: nextText,
          isModified: nextText !== block.originalText || block.isNew,
        };
      }
      return block;
    });

    if (count > 0) {
      get().pushEditorSnapshot(`Keresés és csere (${count} találat)`);
      set({ editorTextBlocks: updated });
    }
    return count;
  },

  undoEditor: () => {
    const state = get();
    if (state.editorUndoStack.length === 0) return;
    const previous = state.editorUndoStack[state.editorUndoStack.length - 1];
    const currentSnapshot: PdfEditorSnapshot = {
      textBlocks: state.editorTextBlocks.map((b) => ({ ...b })),
      redactions: state.editorRedactions.map((r) => ({ ...r })),
      signatures: state.editorSignatures.map((s) => ({ ...s })),
      description: "Undo pont",
    };
    set({
      editorTextBlocks: previous.textBlocks,
      editorRedactions: previous.redactions,
      editorSignatures: previous.signatures,
      editorUndoStack: state.editorUndoStack.slice(0, -1),
      editorRedoStack: [...state.editorRedoStack, currentSnapshot],
    });
  },

  redoEditor: () => {
    const state = get();
    if (state.editorRedoStack.length === 0) return;
    const next = state.editorRedoStack[state.editorRedoStack.length - 1];
    const currentSnapshot: PdfEditorSnapshot = {
      textBlocks: state.editorTextBlocks.map((b) => ({ ...b })),
      redactions: state.editorRedactions.map((r) => ({ ...r })),
      signatures: state.editorSignatures.map((s) => ({ ...s })),
      description: "Redo pont",
    };
    set({
      editorTextBlocks: next.textBlocks,
      editorRedactions: next.redactions,
      editorSignatures: next.signatures,
      editorRedoStack: state.editorRedoStack.slice(0, -1),
      editorUndoStack: [...state.editorUndoStack, currentSnapshot],
    });
  },
}));
