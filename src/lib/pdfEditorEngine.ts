import { PDFDocument, rgb } from "pdf-lib";
import fontkit from "@pdf-lib/fontkit";
import * as pdfjsLib from "pdfjs-dist";
import type {
  PdfRedactionBox,
  PdfSignatureItem,
  PdfTextBlock,
} from "../types/omniconvert";

// Configure local offline PDF.js worker
pdfjsLib.GlobalWorkerOptions.workerSrc = "/pdf.worker.min.mjs";

const FONT_URLS: Record<PdfTextBlock["fontFamily"], string> = {
  Inter: "/fonts/Inter-Regular.ttf",
  Roboto: "/fonts/Roboto-Regular.ttf",
  JetBrainsMono: "/fonts/JetBrainsMono-Regular.ttf",
  NotoSerif: "/fonts/NotoSerif-Regular.ttf",
};

const fontBytesCache: Partial<Record<PdfTextBlock["fontFamily"], ArrayBuffer>> =
  {};

async function loadFontBytes(
  family: PdfTextBlock["fontFamily"]
): Promise<ArrayBuffer> {
  if (fontBytesCache[family]) {
    return fontBytesCache[family]!;
  }
  const url = FONT_URLS[family] || FONT_URLS.Inter;
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`Nem sikerült betölteni a(z) ${family} UTF-8 betűtípust.`);
  }
  const buf = await res.arrayBuffer();
  fontBytesCache[family] = buf;
  return buf;
}

function hexToRgbColor(hex: string) {
  const clean = hex.replace("#", "");
  const num = parseInt(clean, 16);
  if (Number.isNaN(num) || clean.length !== 6) {
    return rgb(0.07, 0.07, 0.08);
  }
  const r = ((num >> 16) & 255) / 255;
  const g = ((num >> 8) & 255) / 255;
  const b = (num & 255) / 255;
  return rgb(r, g, b);
}

/**
 * Renders a specific PDF page onto an HTML5 Canvas and extracts all interactive text blocks
 * with exact PDF and viewport coordinates (x, y, width, height, fontSize, content).
 */
export async function renderPdfPageAndExtractBlocks(
  pdfBytes: Uint8Array,
  pageIndex: number,
  scale: number,
  canvas: HTMLCanvasElement
): Promise<{
  pageCount: number;
  widthPts: number;
  heightPts: number;
  extractedBlocks: PdfTextBlock[];
}> {
  const loadingTask = pdfjsLib.getDocument({
    data: pdfBytes.slice(),
    isEvalSupported: false,
  });
  const pdfDoc = await loadingTask.promise;
  const pageCount = pdfDoc.numPages;
  const safePageNum = Math.min(Math.max(1, pageIndex + 1), pageCount);
  const page = await pdfDoc.getPage(safePageNum);

  const unscaledViewport = page.getViewport({ scale: 1.0 });
  const viewport = page.getViewport({ scale });

  canvas.width = Math.floor(viewport.width);
  canvas.height = Math.floor(viewport.height);

  const ctx = canvas.getContext("2d");
  if (ctx) {
    ctx.fillStyle = "#ffffff";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    await page.render({
      canvasContext: ctx,
      viewport,
    }).promise;
  }

  const textContent = await page.getTextContent();
  const extractedBlocks: PdfTextBlock[] = [];

  textContent.items.forEach((item, idx) => {
    if ("str" in item && item.str.trim().length > 0) {
      // transform is [scaleX, skewY, skewX, scaleY, tx, ty] in PDF bottom-left coordinates
      const tx = Number(item.transform[4] ?? 40);
      const ty = Number(item.transform[5] ?? 700);
      const fontSizePts = Math.max(
        8,
        Math.round(Math.hypot(Number(item.transform[0]), Number(item.transform[1])))
      );
      const widthPts = Math.max(24, Number(item.width) || item.str.length * (fontSizePts * 0.55));
      const heightPts = Math.max(fontSizePts * 1.25, Number(item.height) || fontSizePts * 1.25);

      // Convert PDF bottom-left (tx, ty) to top-left canvas coordinate space (unscaled points)
      const topY = Math.max(0, unscaledViewport.height - ty - fontSizePts);

      extractedBlocks.push({
        id: `p${pageIndex}-block-${idx}`,
        pageIndex,
        originalText: item.str,
        currentText: item.str,
        x: Math.round(tx * 10) / 10,
        y: Math.round(topY * 10) / 10,
        pdfY: Math.round(ty * 10) / 10,
        width: Math.round(widthPts * 10) / 10,
        height: Math.round(heightPts * 10) / 10,
        fontSize: fontSizePts,
        fontFamily: "Inter",
        colorHex: "#111827",
        backgroundColorHex: "#FFFFFF",
        isModified: false,
        isNew: false,
      });
    }
  });

  return {
    pageCount,
    widthPts: unscaledViewport.width,
    heightPts: unscaledViewport.height,
    extractedBlocks,
  };
}

/**
 * Writes modified in-place text blocks, new text boxes, whiteout/redaction rectangles,
 * and signature images back into the PDF using `pdf-lib` and `@pdf-lib/fontkit`
 * with full Hungarian UTF-8 (`ő`, `ű`, `Ő`, `Ű`) TrueType font embedding.
 */
export async function exportEditedPdfWithHungarianFonts(
  originalPdfBytes: Uint8Array,
  textBlocks: PdfTextBlock[],
  redactions: PdfRedactionBox[],
  signatures: PdfSignatureItem[]
): Promise<Uint8Array> {
  const pdfDoc = await PDFDocument.load(originalPdfBytes, {
    ignoreEncryption: true,
  });
  pdfDoc.registerFontkit(fontkit);

  // Embed requested UTF-8 TrueType fonts
  const usedFamilies = new Set<PdfTextBlock["fontFamily"]>(["Inter"]);
  for (const b of textBlocks) {
    if (b.isModified || b.isNew) {
      usedFamilies.add(b.fontFamily);
    }
  }

  const embeddedFonts: Partial<
    Record<PdfTextBlock["fontFamily"], Awaited<ReturnType<typeof pdfDoc.embedFont>>>
  > = {};

  for (const fam of usedFamilies) {
    const fontBuf = await loadFontBytes(fam);
    embeddedFonts[fam] = await pdfDoc.embedFont(fontBuf, { subset: true });
  }

  const pages = pdfDoc.getPages();

  // 1. Apply Whiteout / Redaction boxes
  for (const box of redactions) {
    const page = pages[box.pageIndex];
    if (!page) continue;
    const { height: pageH } = page.getSize();
    const pdfBottomY = pageH - box.y - box.height;
    page.drawRectangle({
      x: box.x,
      y: pdfBottomY,
      width: box.width,
      height: box.height,
      color: hexToRgbColor(box.fillColorHex || "#FFFFFF"),
    });
  }

  // 2. Apply Modified In-Place Text Blocks & New Text Boxes
  for (const block of textBlocks) {
    if (!block.isModified && !block.isNew) continue;
    const page = pages[block.pageIndex];
    if (!page) continue;
    const { height: pageH } = page.getSize();
    const font = embeddedFonts[block.fontFamily] || embeddedFonts.Inter!;

    const baselineY =
      block.pdfY > 0
        ? block.pdfY
        : Math.max(12, pageH - block.y - block.fontSize);

    // If modifying existing PDF text in-place, cover original text area with background color
    if (!block.isNew) {
      const coverWidth = Math.max(
        block.width + 6,
        font.widthOfTextAtSize(block.currentText || " ", block.fontSize) + 6
      );
      page.drawRectangle({
        x: Math.max(0, block.x - 2),
        y: baselineY - block.fontSize * 0.25,
        width: coverWidth,
        height: block.fontSize * 1.35,
        color: hexToRgbColor(block.backgroundColorHex || "#FFFFFF"),
      });
    }

    if (block.currentText.trim().length > 0) {
      page.drawText(block.currentText, {
        x: block.x,
        y: baselineY,
        size: block.fontSize,
        font,
        color: hexToRgbColor(block.colorHex || "#111827"),
      });
    }
  }

  // 3. Apply Signature Images (PNG / JPEG data URLs)
  for (const sig of signatures) {
    const page = pages[sig.pageIndex];
    if (!page) continue;
    const { height: pageH } = page.getSize();
    const pdfBottomY = pageH - sig.y - sig.height;

    const base64Clean = sig.dataUrl.replace(/^data:image\/\w+;base64,/, "");
    const rawBin = atob(base64Clean);
    const imgBytes = new Uint8Array(rawBin.length);
    for (let i = 0; i < rawBin.length; i++) {
      imgBytes[i] = rawBin.charCodeAt(i);
    }

    const embeddedImg = sig.dataUrl.startsWith("data:image/jpeg")
      ? await pdfDoc.embedJpg(imgBytes)
      : await pdfDoc.embedPng(imgBytes);

    page.drawImage(embeddedImg, {
      x: sig.x,
      y: pdfBottomY,
      width: sig.width,
      height: sig.height,
    });
  }

  return pdfDoc.save();
}

/**
 * Generates a realistic multi-page Hungarian invoice/contract PDF in memory using `pdf-lib`
 * + `@pdf-lib/fontkit` so the Visual Editor works immediately even on newly created or demo files.
 */
export async function createHungarianSamplePdfBytes(): Promise<Uint8Array> {
  const pdfDoc = await PDFDocument.create();
  pdfDoc.registerFontkit(fontkit);
  const interBytes = await loadFontBytes("Inter");
  const font = await pdfDoc.embedFont(interBytes);

  const page1 = pdfDoc.addPage([595, 842]);
  page1.drawText("OMNICONVERT PÉNZÜGYI SZÁMLA ÉS TELJESÍTÉSI IGAZOLÁS", {
    x: 45,
    y: 785,
    size: 14,
    font,
    color: rgb(0.14, 0.38, 0.92),
  });
  page1.drawText("Számlaszám: OMNI-2026-00482 · Kiállítás dátuma: 2026. november 08.", {
    x: 45,
    y: 758,
    size: 10,
    font,
    color: rgb(0.25, 0.25, 0.28),
  });
  page1.drawText("Szállító: OmniConvert Rendszermérnöki Kft. (1054 Budapest, Fővám tér 12.)", {
    x: 45,
    y: 725,
    size: 11,
    font,
    color: rgb(0.1, 0.1, 0.12),
  });
  page1.drawText("Vevő: Kaposvári Kázmér Fejlesztői Stúdió (Adószám: 12345678-2-41)", {
    x: 45,
    y: 703,
    size: 11,
    font,
    color: rgb(0.1, 0.1, 0.12),
  });
  page1.drawText("1. Tétel: Cross-Platform Rust & Tauri v2 Desktop licenc — 480 000 Ft", {
    x: 45,
    y: 660,
    size: 11,
    font,
    color: rgb(0.08, 0.08, 0.1),
  });
  page1.drawText("2. Tétel: Magyar ékezetes (ő, ű, Ő, Ű) PDF betűkészlet modul — 120 000 Ft", {
    x: 45,
    y: 636,
    size: 11,
    font,
    color: rgb(0.08, 0.08, 0.1),
  });
  page1.drawText("Mindösszesen fizetendő: 600 000 Ft (ÁFA mentes belső elszámolás)", {
    x: 45,
    y: 595,
    size: 12,
    font,
    color: rgb(0.05, 0.55, 0.35),
  });
  page1.drawText("Megjegyzés: Kattintson bármelyik szövegsorra a helybeni (In-Place) átíráshoz!", {
    x: 45,
    y: 550,
    size: 10,
    font,
    color: rgb(0.4, 0.4, 0.45),
  });

  return pdfDoc.save();
}
