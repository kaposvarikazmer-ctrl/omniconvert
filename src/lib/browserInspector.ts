import type {
  EncryptionStatus,
  FileCategory,
  InspectedFile,
  VaultEntry,
} from "../types/omniconvert";

const ROUTING_MATRIX: Record<Exclude<FileCategory, "all">, string[]> = {
  pdf: [
    "pdf-unlocked",
    "pdf-compressed",
    "png",
    "jpg",
    "webp",
    "txt",
    "md",
    "html",
    "docx",
  ],
  image: [
    "png",
    "jpg",
    "webp",
    "avif",
    "pdf",
    "ico",
    "tiff",
    "bmp",
    "gif",
    "qoi",
    "tga",
    "pnm",
  ],
  video: [
    "mp4",
    "mkv",
    "mov",
    "webm",
    "avi",
    "gif",
    "webp",
    "mp3",
    "wav",
    "flac",
    "aac",
    "opus",
  ],
  audio: ["mp3", "wav", "flac", "aac", "ogg", "opus", "m4a", "aiff", "ac3"],
  document: [
    "pdf",
    "docx",
    "odt",
    "md",
    "html",
    "txt",
    "rtf",
    "epub",
    "tex",
    "rst",
  ],
  data: [
    "xlsx",
    "csv",
    "tsv",
    "json",
    "yaml",
    "toml",
    "xml",
    "sql",
    "pdf",
    "html",
    "md",
  ],
  archive: ["zip", "7z", "tar", "tar.gz", "tar.bz2", "tar.xz"],
};

export function categorizeFormatClient(fmt: string): Exclude<FileCategory, "all"> {
  switch (fmt.toLowerCase()) {
    case "pdf":
      return "pdf";
    case "png":
    case "jpg":
    case "jpeg":
    case "webp":
    case "avif":
    case "gif":
    case "tiff":
    case "tif":
    case "bmp":
    case "ico":
    case "tga":
    case "pnm":
    case "qoi":
    case "dds":
    case "svg":
    case "heic":
    case "heif":
      return "image";
    case "mp4":
    case "mkv":
    case "mov":
    case "avi":
    case "webm":
    case "flv":
    case "wmv":
    case "mpeg":
    case "m4v":
    case "3gp":
    case "ts":
    case "ogv":
      return "video";
    case "mp3":
    case "wav":
    case "flac":
    case "aac":
    case "ogg":
    case "opus":
    case "m4a":
    case "wma":
    case "aiff":
    case "alac":
    case "ac3":
      return "audio";
    case "xlsx":
    case "xls":
    case "ods":
    case "csv":
    case "tsv":
    case "json":
    case "yaml":
    case "yml":
    case "toml":
    case "xml":
    case "parquet":
    case "sql":
      return "data";
    case "zip":
    case "7z":
    case "tar":
    case "tar.gz":
    case "tgz":
    case "tar.bz2":
    case "tar.xz":
      return "archive";
    default:
      return "document";
  }
}

export async function inspectDroppedBrowserFile(
  file: File,
  indexHint: number,
  vaultEntries: VaultEntry[]
): Promise<InspectedFile> {
  const slice = file.slice(0, Math.min(file.size, 65536));
  const buffer = await slice.arrayBuffer();
  const bytes = new Uint8Array(buffer);

  const magicBytesHex = Array.from(bytes.slice(0, 8))
    .map((b) => b.toString(16).toUpperCase().padStart(2, "0"))
    .join(" ");

  const extParts = file.name.split(".");
  const ext = extParts.length > 1 ? extParts[extParts.length - 1].toLowerCase() : "";

  let detectedFormat = ext || "txt";
  let detectedMime = file.type || "application/octet-stream";

  // Check magic bytes signatures
  if (
    bytes[0] === 0x25 &&
    bytes[1] === 0x50 &&
    bytes[2] === 0x44 &&
    bytes[3] === 0x46
  ) {
    detectedFormat = "pdf";
    detectedMime = "application/pdf";
  } else if (
    bytes[0] === 0x89 &&
    bytes[1] === 0x50 &&
    bytes[2] === 0x4e &&
    bytes[3] === 0x47
  ) {
    detectedFormat = "png";
    detectedMime = "image/png";
  } else if (bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) {
    detectedFormat = "jpg";
    detectedMime = "image/jpeg";
  } else if (
    bytes[0] === 0x52 &&
    bytes[1] === 0x49 &&
    bytes[2] === 0x46 &&
    bytes[3] === 0x46 &&
    bytes[8] === 0x57 &&
    bytes[9] === 0x45 &&
    bytes[10] === 0x42 &&
    bytes[11] === 0x50
  ) {
    detectedFormat = "webp";
    detectedMime = "image/webp";
  } else if (
    bytes[0] === 0x37 &&
    bytes[1] === 0x7a &&
    bytes[2] === 0xbc &&
    bytes[3] === 0xaf
  ) {
    detectedFormat = "7z";
    detectedMime = "application/x-7z-compressed";
  } else if (bytes[0] === 0x50 && bytes[1] === 0x4b && bytes[2] === 0x03 && bytes[3] === 0x04) {
    if (!["docx", "xlsx", "odt", "ods", "epub"].includes(ext)) {
      detectedFormat = "zip";
      detectedMime = "application/zip";
    }
  }

  const normalizedExt = ext === "jpeg" ? "jpg" : ext;
  const extensionMismatch =
    Boolean(normalizedExt) &&
    normalizedExt !== detectedFormat &&
    !(detectedFormat === "zip" && ["docx", "xlsx", "odt", "ods", "epub"].includes(normalizedExt));

  const category = categorizeFormatClient(detectedFormat);
  const availableTargets = ROUTING_MATRIX[category].filter(
    (t) => t !== detectedFormat
  );

  let encryptionStatus: EncryptionStatus = "open";
  let statusBadgeText = "Nyitott · Feldolgozásra kész";
  let pdfMeta: InspectedFile["pdfMeta"] = undefined;

  if (category === "pdf") {
    const rawLatin1 = new TextDecoder("latin1").decode(bytes);
    const pageMatches = rawLatin1.match(/\/Type\s*\/Page\b/g);
    const pageCount = Math.max(1, pageMatches ? pageMatches.length : 1);
    const hasEncrypt = rawLatin1.includes("/Encrypt");
    const isOwnerLock =
      rawLatin1.includes("%OMNI-OWNER-LOCKED") ||
      (hasEncrypt && !rawLatin1.includes("%OMNI-USER-PW:"));

    if (!hasEncrypt) {
      encryptionStatus = "open";
      statusBadgeText = `Nyitott · ${pageCount} oldal`;
    } else if (isOwnerLock) {
      encryptionStatus = "owner_locked";
      statusBadgeText = "Másolásvédett (Owner Lock) – Azonnal feloldható";
    } else {
      const matched = vaultEntries.find((v) =>
        rawLatin1.includes(`%OMNI-USER-PW:${v.password}`)
      );
      if (matched) {
        encryptionStatus = "auto_unlocked";
        statusBadgeText = `Automatikusan feloldva a Jelszótárból (${matched.label})`;
      } else {
        encryptionStatus = "user_locked";
        statusBadgeText = "Jelszóval titkosított (User Lock) – Jelszó szükséges";
      }
    }

    pdfMeta = {
      pageCount,
      hasTextLayer: rawLatin1.includes("BT") && rawLatin1.includes("ET"),
      pdfVersion: "1.7",
      encryptionStatus,
      permissionsSummary:
        encryptionStatus === "owner_locked"
          ? ["Másolás / Nyomtatás zárolva", "Jelszó nélkül azonnal feloldható"]
          : ["Normál hozzáférés"],
    };
  }

  return {
    id: `dropped-${indexHint}-${Date.now()}`,
    path: file.name,
    name: file.name,
    extension: ext,
    detectedMime,
    detectedFormat,
    magicBytesHex,
    extensionMismatch,
    category,
    sizeBytes: file.size,
    encryptionStatus,
    statusBadgeText,
    availableTargets,
    selectedTarget: availableTargets[0] ?? "pdf",
    pdfMeta,
    conversionStatus: "idle",
    progressPercent: 0,
    rawFile: file,
  };
}
