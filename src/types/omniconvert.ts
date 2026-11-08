export type FileCategory =
  | "all"
  | "pdf"
  | "image"
  | "video"
  | "audio"
  | "document"
  | "data"
  | "archive";

export type EncryptionStatus =
  | "open"
  | "owner_locked"
  | "user_locked"
  | "auto_unlocked"
  | "manually_unlocked";

export interface PdfMetadata {
  pageCount: number;
  hasTextLayer: boolean;
  pdfVersion: string;
  encryptionStatus: EncryptionStatus;
  permissionsSummary: string[];
  unlockedWithPassword?: string;
}

export interface ArchiveMetadata {
  itemCount: number;
  uncompressedSize: number;
  isEncrypted: boolean;
  encryptionStatus: EncryptionStatus;
}

export interface MediaMetadata {
  width?: number;
  height?: number;
  durationSecs?: number;
  codec?: string;
  bitrateKbps?: number;
  hasExif?: boolean;
  colorSpace?: string;
}

export interface DataMetadata {
  rowCount?: number;
  columnCount?: number;
  columns?: string[];
  sheetNames?: string[];
}

export interface InspectedFile {
  id: string;
  path: string;
  name: string;
  extension: string;
  detectedMime: string;
  detectedFormat: string;
  magicBytesHex: string;
  extensionMismatch: boolean;
  category: Exclude<FileCategory, "all">;
  sizeBytes: number;
  encryptionStatus: EncryptionStatus;
  statusBadgeText: string;
  availableTargets: string[];
  selectedTarget: string;
  pdfMeta?: PdfMetadata;
  archiveMeta?: ArchiveMetadata;
  mediaMeta?: MediaMetadata;
  dataMeta?: DataMetadata;
  conversionStatus: "idle" | "processing" | "done" | "error";
  progressPercent: number;
  outputPath?: string;
  outputSizeBytes?: number;
  savingsPercent?: number;
  errorMessage?: string;
  manualPassword?: string;
  /** Optional browser File object when dragged in WebView/fallback mode */
  rawFile?: File;
}

export type PdfWorkspaceTab =
  | "convert"
  | "visual_editor"
  | "unlock"
  | "merge"
  | "split"
  | "compress";

export interface ConversionOptions {
  outputDir: string;
  // Image options
  imageQuality: number; // 1 - 100
  resizeWidth?: number;
  resizeHeight?: number;
  keepAspectRatio: boolean;
  stripExif: boolean;
  mergeImagesToSinglePdf: boolean;
  // Video / Audio options
  videoResolution: "original" | "2160p" | "1080p" | "720p";
  videoCrf: number; // 18 - 32
  hardwareAcceleration: "auto" | "videotoolbox" | "nvenc" | "amf" | "qsv" | "cpu";
  audioBitrateKbps: number;
}

export type PdfCompressionProfile = "light" | "balanced" | "strong";

export type PdfSplitMode = "ranges" | "burst" | "every_n";

export interface PdfToolkitRequest {
  operation:
    | "merge"
    | "split"
    | "compress"
    | "rotate"
    | "delete_pages"
    | "pages_to_images";
  inputPaths: string[];
  outputDir: string;
  // Split options
  splitMode?: PdfSplitMode;
  pageRanges?: string; // e.g. "1-3, 5, 8-12"
  mergeRangesIntoSingleFile?: boolean;
  everyNPages?: number;
  // Compress options
  compressionProfile?: PdfCompressionProfile;
  // Rotate / Delete / Export options
  rotationDegrees?: 90 | 180 | 270;
  targetPages?: string; // e.g. "1, 4-6"
  imageFormat?: "png" | "jpg" | "webp";
}

export interface PdfToolkitResult {
  success: boolean;
  outputPaths: string[];
  originalSizeBytes: number;
  outputSizeBytes: number;
  savingsPercent: number;
  message: string;
}

export interface UnlockResult {
  fileId: string;
  inputPath: string;
  success: boolean;
  outputPath?: string;
  newStatus: EncryptionStatus;
  methodUsed: "owner_lock_strip" | "auto_vault" | "manual_password" | "already_open";
  matchedLabel?: string;
  message: string;
}

export interface VaultEntry {
  id: string;
  label: string;
  password: string;
  categoryHint: string; // e.g. "Banki kivonat", "Számla", "Céges ZIP"
  createdAt: string;
}

export interface DependencyStatus {
  name: "ffmpeg" | "pandoc" | "typst";
  installed: boolean;
  version?: string;
  binaryPath?: string;
  bundledOrSystem: "system" | "local_cache" | "missing";
  capabilities: string[];
}

export interface PdfTextBlock {
  id: string;
  pageIndex: number; // 0-indexed
  originalText: string;
  currentText: string;
  x: number; // PDF points from left
  y: number; // PDF points from top (canvas coordinate space)
  pdfY: number; // PDF bottom-left coordinate
  width: number;
  height: number;
  fontSize: number;
  fontFamily: "Inter" | "Roboto" | "JetBrainsMono" | "NotoSerif";
  colorHex: string;
  backgroundColorHex: string;
  isModified: boolean;
  isNew: boolean;
}

export interface PdfRedactionBox {
  id: string;
  pageIndex: number;
  x: number;
  y: number;
  width: number;
  height: number;
  fillColorHex: string;
}

export interface PdfSignatureItem {
  id: string;
  pageIndex: number;
  x: number;
  y: number;
  width: number;
  height: number;
  dataUrl: string;
}

export interface PdfEditorSnapshot {
  textBlocks: PdfTextBlock[];
  redactions: PdfRedactionBox[];
  signatures: PdfSignatureItem[];
  description: string;
}
