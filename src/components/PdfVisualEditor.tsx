import React, { useEffect, useRef, useState } from "react";
import {
  ChevronLeft,
  ChevronRight,
  Download,
  Eraser,
  FileCheck2,
  PenTool,
  PlusSquare,
  Redo2,
  Replace,
  Search,
  Type,
  Undo2,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
import { useAppStore } from "../store/useAppStore";
import {
  createHungarianSamplePdfBytes,
  exportEditedPdfWithHungarianFonts,
  renderPdfPageAndExtractBlocks,
} from "../lib/pdfEditorEngine";
import {
  readBinaryFileBase64Ipc,
  writeBinaryFileBase64Ipc,
} from "../lib/ipc";
import type { PdfTextBlock } from "../types/omniconvert";

function uint8ToBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}

function base64ToUint8(base64: string): Uint8Array {
  const bin = atob(base64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) {
    out[i] = bin.charCodeAt(i);
  }
  return out;
}

export const PdfVisualEditor: React.FC = () => {
  const {
    files,
    selectedPdfFileId,
    setSelectedPdfFileId,
    editorPageIndex,
    editorPageCount,
    editorZoom,
    editorTextBlocks,
    editorRedactions,
    editorSignatures,
    editorUndoStack,
    editorRedoStack,
    initEditorState,
    setEditorPageIndex,
    setEditorZoom,
    updateTextBlock,
    addNewTextBlock,
    addRedactionBox,
    removeRedactionBox,
    addSignatureItem,
    removeSignatureItem,
    findAndReplaceText,
    undoEditor,
    redoEditor,
    setStatusBanner,
  } = useAppStore();

  const pdfFiles = files.filter((f) => f.category === "pdf");
  const activePdf =
    pdfFiles.find((f) => f.id === selectedPdfFileId) ?? pdfFiles[0] ?? null;

  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const sigCanvasRef = useRef<HTMLCanvasElement | null>(null);

  const [pdfBytes, setPdfBytes] = useState<Uint8Array | null>(null);
  const [pageSizePts, setPageSizePts] = useState<{ w: number; h: number }>({
    w: 595,
    h: 842,
  });
  const [activeBlockId, setActiveBlockId] = useState<string | null>(null);
  const [selectedFont, setSelectedFont] =
    useState<PdfTextBlock["fontFamily"]>("Inter");
  const [selectedFontSize, setSelectedFontSize] = useState<number>(11);
  const [findQuery, setFindQuery] = useState("");
  const [replaceQuery, setReplaceQuery] = useState("");
  const [matchCase, setMatchCase] = useState(false);
  const [isSignatureModalOpen, setIsSignatureModalOpen] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isDrawingSig, setIsDrawingSig] = useState(false);

  // Keyboard shortcuts for Undo / Redo (Ctrl/Cmd + Z, Ctrl/Cmd + Y)
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "z") {
        if (e.shiftKey) {
          e.preventDefault();
          redoEditor();
        } else {
          e.preventDefault();
          undoEditor();
        }
      } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "y") {
        e.preventDefault();
        redoEditor();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [undoEditor, redoEditor]);

  // Load PDF bytes from active PDF file (or generate Hungarian sample PDF)
  useEffect(() => {
    let cancelled = false;
    async function loadPdf() {
      try {
        if (activePdf?.rawFile) {
          const buf = await activePdf.rawFile.arrayBuffer();
          if (!cancelled) setPdfBytes(new Uint8Array(buf));
          return;
        }
        if (activePdf?.path) {
          try {
            const b64 = await readBinaryFileBase64Ipc(activePdf.path);
            if (!cancelled) {
              setPdfBytes(base64ToUint8(b64));
              return;
            }
          } catch {
            // Fall through to in-memory Hungarian sample PDF
          }
        }
        const sample = await createHungarianSamplePdfBytes();
        if (!cancelled) setPdfBytes(sample);
      } catch (err) {
        if (!cancelled) {
          setStatusBanner({
            type: "error",
            text: `PDF betöltési hiba: ${err instanceof Error ? err.message : String(err)}`,
          });
        }
      }
    }
    loadPdf();
    return () => {
      cancelled = true;
    };
  }, [activePdf?.id, activePdf?.path, activePdf?.rawFile, setStatusBanner]);

  // Render current page and extract text blocks when pdfBytes, page, or zoom changes
  useEffect(() => {
    let cancelled = false;
    async function renderPage() {
      if (!pdfBytes || !canvasRef.current) return;
      try {
        const result = await renderPdfPageAndExtractBlocks(
          pdfBytes,
          editorPageIndex,
          editorZoom,
          canvasRef.current
        );
        if (cancelled) return;
        setPageSizePts({ w: result.widthPts, h: result.heightPts });

        // Initialize text blocks for this page if not already loaded
        const existingForPage = editorTextBlocks.filter(
          (b) => b.pageIndex === editorPageIndex
        );
        if (existingForPage.length === 0 && result.extractedBlocks.length > 0) {
          initEditorState(result.extractedBlocks, result.pageCount);
        }
      } catch (err) {
        console.error("PDF render error:", err);
      }
    }
    renderPage();
    return () => {
      cancelled = true;
    };
  }, [pdfBytes, editorPageIndex, editorZoom]);

  const activeBlock =
    editorTextBlocks.find((b) => b.id === activeBlockId) ?? null;

  const handleSelectBlock = (block: PdfTextBlock) => {
    setActiveBlockId(block.id);
    setSelectedFont(block.fontFamily);
    setSelectedFontSize(block.fontSize);
  };

  const handleAddTextBox = () => {
    const newId = `new-text-${Date.now()}`;
    const newBlock: PdfTextBlock = {
      id: newId,
      pageIndex: editorPageIndex,
      originalText: "",
      currentText: "Új magyar szöveg (ő, ű, Ő, Ű)",
      x: 60,
      y: 120,
      pdfY: Math.max(40, pageSizePts.h - 134),
      width: 240,
      height: 22,
      fontSize: selectedFontSize,
      fontFamily: selectedFont,
      colorHex: "#111827",
      backgroundColorHex: "#FFFFFF",
      isModified: true,
      isNew: true,
    };
    addNewTextBlock(newBlock);
    setActiveBlockId(newId);
  };

  const handleAddWhiteout = () => {
    addRedactionBox({
      id: `redact-${Date.now()}`,
      pageIndex: editorPageIndex,
      x: 80,
      y: 200,
      width: 180,
      height: 28,
      fillColorHex: "#FFFFFF",
    });
    setStatusBanner({
      type: "info",
      text: "Kitakaró (Whiteout) téglalap beszúrva az aktuális oldalra.",
    });
  };

  const handleFindReplace = () => {
    if (!findQuery.trim()) return;
    const count = findAndReplaceText(findQuery, replaceQuery, matchCase);
    setStatusBanner({
      type: count > 0 ? "success" : "warning",
      text:
        count > 0
          ? `${count} szöveges előfordulás sikeresen lecserélve.`
          : `Nem található egyezés a(z) "${findQuery}" kifejezésre.`,
    });
  };

  const handleInsertDrawnSignature = () => {
    if (!sigCanvasRef.current) return;
    const dataUrl = sigCanvasRef.current.toDataURL("image/png");
    addSignatureItem({
      id: `sig-${Date.now()}`,
      pageIndex: editorPageIndex,
      x: 340,
      y: 620,
      width: 160,
      height: 64,
      dataUrl,
    });
    setIsSignatureModalOpen(false);
    setStatusBanner({
      type: "success",
      text: "Aláírás kép beszúrva a PDF oldalra.",
    });
  };

  const handleSaveEditedPdf = async () => {
    if (!pdfBytes) return;
    setIsSaving(true);
    try {
      const updatedBytes = await exportEditedPdfWithHungarianFonts(
        pdfBytes,
        editorTextBlocks,
        editorRedactions,
        editorSignatures
      );
      setPdfBytes(updatedBytes);

      const baseName = activePdf
        ? activePdf.name.replace(/\.pdf$/i, "")
        : "omniconvert_szerkesztett";
      const targetOutputPath = `./omniconvert-output/${baseName}_edited_utf8.pdf`;

      try {
        const b64 = uint8ToBase64(updatedBytes);
        await writeBinaryFileBase64Ipc(targetOutputPath, b64);
      } catch {
        // Also provide direct browser download fallback
      }

      const blob = new Blob([updatedBytes.buffer as ArrayBuffer], {
        type: "application/pdf",
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${baseName}_edited_utf8.pdf`;
      a.click();
      URL.revokeObjectURL(url);

      setStatusBanner({
        type: "success",
        text: `Módosított PDF elmentve beágyazott magyar UTF-8 (${selectedFont}) betűtípussal: ${baseName}_edited_utf8.pdf`,
      });
    } catch (err) {
      setStatusBanner({
        type: "error",
        text: `PDF mentési hiba: ${err instanceof Error ? err.message : String(err)}`,
      });
    } finally {
      setIsSaving(false);
    }
  };

  const pageBlocks = editorTextBlocks.filter(
    (b) => b.pageIndex === editorPageIndex
  );
  const pageRedactions = editorRedactions.filter(
    (r) => r.pageIndex === editorPageIndex
  );
  const pageSignatures = editorSignatures.filter(
    (s) => s.pageIndex === editorPageIndex
  );

  return (
    <div className="flex-1 flex flex-col min-h-0 bg-zinc-950">
      {/* Editor Secondary Toolbar */}
      <div className="h-11 px-4 border-b border-zinc-800 bg-zinc-900/60 flex items-center justify-between gap-2 shrink-0">
        <div className="flex items-center gap-2">
          {/* PDF Selector if multiple PDFs in queue */}
          {pdfFiles.length > 0 && (
            <select
              id="select-editor-pdf"
              value={activePdf?.id ?? ""}
              onChange={(e) => setSelectedPdfFileId(e.target.value)}
              className="h-7 px-2 text-xs bg-zinc-950 border border-zinc-800 rounded text-zinc-200 focus:outline-none focus:border-blue-600"
            >
              {pdfFiles.map((f) => (
                <option key={f.id} value={f.id}>
                  {f.name} ({f.pdfMeta?.pageCount ?? 1} oldal)
                </option>
              ))}
            </select>
          )}

          {/* Font family selector (embedded UTF-8 TTF fonts) */}
          <div className="flex items-center gap-1 pl-2 border-l border-zinc-800">
            <Type className="w-3.5 h-3.5 text-zinc-400" strokeWidth={1.5} />
            <select
              id="select-editor-font"
              value={activeBlock?.fontFamily ?? selectedFont}
              onChange={(e) => {
                const fam = e.target.value as PdfTextBlock["fontFamily"];
                setSelectedFont(fam);
                if (activeBlock) {
                  updateTextBlock(activeBlock.id, { fontFamily: fam });
                }
              }}
              className="h-7 px-2 text-xs bg-zinc-950 border border-zinc-800 rounded text-zinc-200"
            >
              <option value="Inter">Inter (UTF-8 Magyar TTF)</option>
              <option value="Roboto">Roboto (UTF-8 Magyar TTF)</option>
              <option value="JetBrainsMono">JetBrains Mono (UTF-8 TTF)</option>
              <option value="NotoSerif">Noto Serif (UTF-8 Magyar TTF)</option>
            </select>

            <input
              id="input-editor-fontsize"
              type="number"
              min={6}
              max={72}
              value={activeBlock?.fontSize ?? selectedFontSize}
              onChange={(e) => {
                const sz = Number(e.target.value) || 11;
                setSelectedFontSize(sz);
                if (activeBlock) {
                  updateTextBlock(activeBlock.id, { fontSize: sz });
                }
              }}
              className="w-14 h-7 px-2 text-xs bg-zinc-950 border border-zinc-800 rounded text-zinc-200 tabular-nums"
            />
          </div>

          {/* Insert Tools: New Textbox, Whiteout/Redaction, Signature */}
          <div className="flex items-center gap-1 pl-2 border-l border-zinc-800">
            <button
              id="btn-editor-add-text"
              type="button"
              onClick={handleAddTextBox}
              className="h-7 px-2.5 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-200 border border-zinc-700 rounded flex items-center gap-1.5"
            >
              <PlusSquare className="w-3.5 h-3.5 text-blue-400" strokeWidth={1.5} />
              Új szövegdoboz
            </button>

            <button
              id="btn-editor-add-whiteout"
              type="button"
              onClick={handleAddWhiteout}
              className="h-7 px-2.5 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-200 border border-zinc-700 rounded flex items-center gap-1.5"
            >
              <Eraser className="w-3.5 h-3.5 text-amber-400" strokeWidth={1.5} />
              Kitakarás (Whiteout)
            </button>

            <button
              id="btn-editor-add-signature"
              type="button"
              onClick={() => setIsSignatureModalOpen(true)}
              className="h-7 px-2.5 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-200 border border-zinc-700 rounded flex items-center gap-1.5"
            >
              <PenTool className="w-3.5 h-3.5 text-emerald-400" strokeWidth={1.5} />
              Aláírás beszúrása
            </button>
          </div>

          {/* Undo / Redo */}
          <div className="flex items-center gap-1 pl-2 border-l border-zinc-800">
            <button
              id="btn-editor-undo"
              type="button"
              onClick={undoEditor}
              disabled={editorUndoStack.length === 0}
              title="Visszavonás (Ctrl+Z)"
              className="h-7 px-2 text-xs bg-zinc-900 hover:bg-zinc-800 disabled:opacity-35 border border-zinc-800 rounded flex items-center gap-1 text-zinc-300 tabular-nums"
            >
              <Undo2 className="w-3.5 h-3.5" strokeWidth={1.5} />
              <span>{editorUndoStack.length}</span>
            </button>
            <button
              id="btn-editor-redo"
              type="button"
              onClick={redoEditor}
              disabled={editorRedoStack.length === 0}
              title="Újra (Ctrl+Y)"
              className="h-7 px-2 text-xs bg-zinc-900 hover:bg-zinc-800 disabled:opacity-35 border border-zinc-800 rounded flex items-center gap-1 text-zinc-300 tabular-nums"
            >
              <Redo2 className="w-3.5 h-3.5" strokeWidth={1.5} />
              <span>{editorRedoStack.length}</span>
            </button>
          </div>
        </div>

        {/* Save & Export PDF */}
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1 text-xs text-zinc-400 tabular-nums">
            <button
              type="button"
              onClick={() => setEditorZoom(Math.max(0.75, editorZoom - 0.15))}
              className="p-1 hover:bg-zinc-800 rounded"
            >
              <ZoomOut className="w-3.5 h-3.5" strokeWidth={1.5} />
            </button>
            <span className="w-11 text-center">
              {Math.round(editorZoom * 100)}%
            </span>
            <button
              type="button"
              onClick={() => setEditorZoom(Math.min(2.2, editorZoom + 0.15))}
              className="p-1 hover:bg-zinc-800 rounded"
            >
              <ZoomIn className="w-3.5 h-3.5" strokeWidth={1.5} />
            </button>
          </div>

          <button
            id="btn-editor-save-pdf"
            type="button"
            disabled={isSaving}
            onClick={handleSaveEditedPdf}
            className="h-7 px-3 text-xs font-medium bg-emerald-600 hover:bg-emerald-500 text-white rounded flex items-center gap-1.5 transition-colors"
          >
            <Download className="w-3.5 h-3.5" strokeWidth={1.5} />
            {isSaving ? "Mentés folyamatban..." : "PDF Mentése (UTF-8 ő/ű)"}
          </button>
        </div>
      </div>

      {/* Main Split Editor Body: Left Canvas + Right Inspector & Find/Replace */}
      <div className="flex-1 flex min-h-0">
        {/* Interactive PDF Page Canvas Area */}
        <div className="flex-1 overflow-auto bg-zinc-950 p-6 flex flex-col items-center">
          {/* Page navigation bar */}
          <div className="mb-3 px-3 py-1 bg-zinc-900 border border-zinc-800 rounded flex items-center gap-3 text-xs text-zinc-300 tabular-nums">
            <button
              type="button"
              disabled={editorPageIndex <= 0}
              onClick={() => setEditorPageIndex(Math.max(0, editorPageIndex - 1))}
              className="disabled:opacity-35 hover:text-white"
            >
              <ChevronLeft className="w-4 h-4" strokeWidth={1.5} />
            </button>
            <span>
              {editorPageIndex + 1} / {editorPageCount}. oldal
            </span>
            <button
              type="button"
              disabled={editorPageIndex + 1 >= editorPageCount}
              onClick={() =>
                setEditorPageIndex(
                  Math.min(editorPageCount - 1, editorPageIndex + 1)
                )
              }
              className="disabled:opacity-35 hover:text-white"
            >
              <ChevronRight className="w-4 h-4" strokeWidth={1.5} />
            </button>
          </div>

          {/* Relative Page Stage */}
          <div
            className="relative shadow-2xl border border-zinc-700 bg-white select-none"
            style={{
              width: Math.floor(pageSizePts.w * editorZoom),
              height: Math.floor(pageSizePts.h * editorZoom),
            }}
          >
            <canvas ref={canvasRef} className="block pointer-events-none" />

            {/* Whiteout / Redaction Overlays */}
            {pageRedactions.map((box) => (
              <div
                key={box.id}
                style={{
                  left: box.x * editorZoom,
                  top: box.y * editorZoom,
                  width: box.width * editorZoom,
                  height: box.height * editorZoom,
                  backgroundColor: box.fillColorHex,
                }}
                className="absolute border border-dashed border-amber-500/80 flex items-center justify-end pr-1 group"
              >
                <button
                  type="button"
                  onClick={() => removeRedactionBox(box.id)}
                  className="opacity-0 group-hover:opacity-100 text-[10px] px-1 bg-zinc-900 text-zinc-200 rounded"
                >
                  ×
                </button>
              </div>
            ))}

            {/* Signature Image Overlays */}
            {pageSignatures.map((sig) => (
              <div
                key={sig.id}
                style={{
                  left: sig.x * editorZoom,
                  top: sig.y * editorZoom,
                  width: sig.width * editorZoom,
                  height: sig.height * editorZoom,
                }}
                className="absolute border border-dashed border-emerald-500/80 group"
              >
                <img
                  src={sig.dataUrl}
                  alt="Aláírás"
                  className="w-full h-full object-contain"
                />
                <button
                  type="button"
                  onClick={() => removeSignatureItem(sig.id)}
                  className="opacity-0 group-hover:opacity-100 absolute -top-2 -right-2 text-[10px] px-1.5 py-0.5 bg-zinc-900 text-white rounded"
                >
                  ×
                </button>
              </div>
            ))}

            {/* Interactive In-Place Editable Text Blocks */}
            {pageBlocks.map((block) => {
              const isSelected = block.id === activeBlockId;
              return (
                <div
                  key={block.id}
                  onClick={() => handleSelectBlock(block)}
                  style={{
                    left: block.x * editorZoom,
                    top: block.y * editorZoom,
                    minWidth: Math.max(40, block.width * editorZoom),
                    minHeight: Math.max(16, block.height * editorZoom),
                  }}
                  className={`absolute transition-colors cursor-text ${
                    isSelected
                      ? "ring-2 ring-blue-600 bg-white z-20"
                      : block.isModified
                      ? "bg-white ring-1 ring-emerald-500/80 z-10"
                      : "hover:bg-blue-500/10 hover:ring-1 hover:ring-blue-500/50"
                  }`}
                >
                  {isSelected || block.isModified ? (
                    <input
                      type="text"
                      value={block.currentText}
                      onChange={(e) =>
                        updateTextBlock(
                          block.id,
                          { currentText: e.target.value },
                          true
                        )
                      }
                      style={{
                        fontSize: `${Math.max(10, block.fontSize * editorZoom * 0.92)}px`,
                        color: block.colorHex,
                      }}
                      className="w-full h-full px-1 bg-white text-zinc-900 focus:outline-none font-sans"
                    />
                  ) : (
                    <span className="sr-only">{block.currentText}</span>
                  )}
                </div>
              );
            })}
          </div>
        </div>

        {/* Right Side Panel: Global Find & Replace + Selected Block Inspector */}
        <aside className="w-80 border-l border-zinc-800 bg-zinc-900/40 p-4 flex flex-col gap-4 overflow-y-auto shrink-0">
          {/* UTF-8 Font Guarantee Card */}
          <div className="p-3 bg-zinc-900 border border-zinc-800 rounded">
            <div className="flex items-center gap-2 text-xs font-medium text-emerald-400 mb-1">
              <FileCheck2 className="w-4 h-4" strokeWidth={1.5} />
              <span>Magyar UTF-8 TTF Motor Aktív</span>
            </div>
            <p className="text-[11px] text-zinc-400 leading-relaxed">
              A <code className="text-zinc-200">@pdf-lib/fontkit</code> és a beágyazott{" "}
              <code className="text-zinc-200">public/fonts/*.ttf</code> készletek
              garantálják az <strong className="text-zinc-200">ő, ű, Ő, Ű</strong>{" "}
              karakterek nyomdai minőségű visszaírását.
            </p>
          </div>

          {/* Global Find & Replace */}
          <div className="p-3 bg-zinc-900 border border-zinc-800 rounded flex flex-col gap-2.5">
            <div className="flex items-center gap-1.5 text-xs font-medium text-zinc-200">
              <Search className="w-3.5 h-3.5 text-blue-400" strokeWidth={1.5} />
              <span>Globális Keresés és Csere</span>
            </div>
            <input
              id="input-pdf-find"
              type="text"
              placeholder="Keresendő szöveg..."
              value={findQuery}
              onChange={(e) => setFindQuery(e.target.value)}
              className="h-8 px-2.5 text-xs bg-zinc-950 border border-zinc-800 rounded text-zinc-100 focus:outline-none focus:border-blue-600"
            />
            <input
              id="input-pdf-replace"
              type="text"
              placeholder="Csere erre (pl. ő, ű ékezettel)..."
              value={replaceQuery}
              onChange={(e) => setReplaceQuery(e.target.value)}
              className="h-8 px-2.5 text-xs bg-zinc-950 border border-zinc-800 rounded text-zinc-100 focus:outline-none focus:border-blue-600"
            />
            <div className="flex items-center justify-between text-[11px] text-zinc-400">
              <label className="flex items-center gap-1.5 cursor-pointer">
                <input
                  type="checkbox"
                  checked={matchCase}
                  onChange={(e) => setMatchCase(e.target.checked)}
                  className="rounded border-zinc-700"
                />
                <span>Kis- és nagybetű érzékeny</span>
              </label>
            </div>
            <button
              id="btn-pdf-replace-all"
              type="button"
              onClick={handleFindReplace}
              className="h-8 px-3 text-xs font-medium bg-blue-600 hover:bg-blue-500 text-white rounded flex items-center justify-center gap-1.5 transition-colors"
            >
              <Replace className="w-3.5 h-3.5" strokeWidth={1.5} />
              Összes cseréje a PDF-ben
            </button>
          </div>

          {/* Active Text Block Inspector */}
          <div className="p-3 bg-zinc-900 border border-zinc-800 rounded flex flex-col gap-2.5">
            <div className="text-xs font-medium text-zinc-200">
              Kijelölt Szövegblokk Szerkesztése
            </div>
            {activeBlock ? (
              <>
                <label className="text-[11px] text-zinc-400">
                  Szövegtartalom (In-Place):
                </label>
                <textarea
                  rows={3}
                  value={activeBlock.currentText}
                  onChange={(e) =>
                    updateTextBlock(activeBlock.id, {
                      currentText: e.target.value,
                    })
                  }
                  className="p-2 text-xs bg-zinc-950 border border-zinc-800 rounded text-zinc-100 focus:outline-none focus:border-blue-600"
                />
                {/* Quick Hungarian Accent Strip */}
                <div className="flex flex-wrap gap-1">
                  {["ő", "ű", "Ő", "Ű", "á", "é", "í", "ó", "ö", "ú", "ü"].map(
                    (ch) => (
                      <button
                        key={ch}
                        type="button"
                        onClick={() =>
                          updateTextBlock(activeBlock.id, {
                            currentText: activeBlock.currentText + ch,
                          })
                        }
                        className="px-2 py-0.5 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded border border-zinc-700 font-mono"
                      >
                        {ch}
                      </button>
                    )
                  )}
                </div>
                <div className="grid grid-cols-2 gap-2 pt-1 text-[11px] text-zinc-400 tabular-nums">
                  <div>
                    <span>X pozíció (pt):</span>
                    <input
                      type="number"
                      value={Math.round(activeBlock.x)}
                      onChange={(e) =>
                        updateTextBlock(activeBlock.id, {
                          x: Number(e.target.value),
                        })
                      }
                      className="w-full mt-1 h-7 px-2 bg-zinc-950 border border-zinc-800 rounded text-zinc-200"
                    />
                  </div>
                  <div>
                    <span>Y pozíció (pt):</span>
                    <input
                      type="number"
                      value={Math.round(activeBlock.y)}
                      onChange={(e) =>
                        updateTextBlock(activeBlock.id, {
                          y: Number(e.target.value),
                        })
                      }
                      className="w-full mt-1 h-7 px-2 bg-zinc-950 border border-zinc-800 rounded text-zinc-200"
                    />
                  </div>
                </div>
              </>
            ) : (
              <p className="text-[11px] text-zinc-400">
                Kattintson bármelyik szövegsorra a bal oldali PDF vásznon vagy az
                alábbi listában a közvetlen helybeni átíráshoz.
              </p>
            )}
          </div>

          {/* All Extracted Text Blocks on Current Page */}
          <div className="flex-1 flex flex-col min-h-0">
            <div className="text-xs font-medium text-zinc-300 mb-2 tabular-nums">
              Oldalon lévő szövegsorok ({pageBlocks.length})
            </div>
            <div className="flex-1 overflow-y-auto space-y-1 pr-1">
              {pageBlocks.map((b) => (
                <button
                  key={b.id}
                  type="button"
                  onClick={() => handleSelectBlock(b)}
                  className={`w-full text-left px-2.5 py-1.5 rounded text-xs border transition-colors truncate ${
                    b.id === activeBlockId
                      ? "bg-blue-600/20 border-blue-600 text-zinc-100"
                      : b.isModified
                      ? "bg-emerald-950/30 border-emerald-700/60 text-emerald-200"
                      : "bg-zinc-900 border-zinc-800 text-zinc-300 hover:bg-zinc-800"
                  }`}
                >
                  {b.currentText || "(Üres szövegdoboz)"}
                </button>
              ))}
            </div>
          </div>
        </aside>
      </div>

      {/* Signature Pad Modal */}
      {isSignatureModalOpen && (
        <div className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4">
          <div className="w-full max-w-md bg-zinc-900 border border-zinc-800 rounded p-4 flex flex-col gap-3">
            <div className="flex items-center justify-between border-b border-zinc-800 pb-2">
              <span className="text-xs font-semibold text-zinc-100">
                Aláírás Rajzolása és Beszúrása
              </span>
              <button
                type="button"
                onClick={() => setIsSignatureModalOpen(false)}
                className="text-xs text-zinc-400 hover:text-zinc-200"
              >
                Bezárás
              </button>
            </div>
            <canvas
              ref={sigCanvasRef}
              width={380}
              height={140}
              onMouseDown={(e) => {
                setIsDrawingSig(true);
                const ctx = sigCanvasRef.current?.getContext("2d");
                if (ctx && sigCanvasRef.current) {
                  const rect = sigCanvasRef.current.getBoundingClientRect();
                  ctx.strokeStyle = "#1d4ed8";
                  ctx.lineWidth = 2.2;
                  ctx.lineCap = "round";
                  ctx.beginPath();
                  ctx.moveTo(e.clientX - rect.left, e.clientY - rect.top);
                }
              }}
              onMouseMove={(e) => {
                if (!isDrawingSig || !sigCanvasRef.current) return;
                const ctx = sigCanvasRef.current.getContext("2d");
                const rect = sigCanvasRef.current.getBoundingClientRect();
                if (ctx) {
                  ctx.lineTo(e.clientX - rect.left, e.clientY - rect.top);
                  ctx.stroke();
                }
              }}
              onMouseUp={() => setIsDrawingSig(false)}
              className="bg-white border border-zinc-700 rounded cursor-crosshair mx-auto"
            />
            <div className="flex items-center justify-between pt-1">
              <button
                type="button"
                onClick={() => {
                  const ctx = sigCanvasRef.current?.getContext("2d");
                  if (ctx && sigCanvasRef.current) {
                    ctx.clearRect(
                      0,
                      0,
                      sigCanvasRef.current.width,
                      sigCanvasRef.current.height
                    );
                  }
                }}
                className="h-8 px-3 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded"
              >
                Törlés
              </button>
              <button
                type="button"
                onClick={handleInsertDrawnSignature}
                className="h-8 px-4 text-xs font-medium bg-blue-600 hover:bg-blue-500 text-white rounded"
              >
                Aláírás beszúrása az oldalra
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
