import { invoke } from "@tauri-apps/api/core";
import type {
  ConversionOptions,
  DependencyStatus,
  InspectedFile,
  PdfToolkitRequest,
  PdfToolkitResult,
  UnlockResult,
  VaultEntry,
} from "../types/omniconvert";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export function isTauriEnvironment(): boolean {
  return typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
}

async function callBackend<T>(
  command: string,
  args: Record<string, unknown> = {}
): Promise<T> {
  if (isTauriEnvironment()) {
    return invoke<T>(command, args);
  }
  const response = await fetch(`http://127.0.0.1:1422/api/ipc/${command}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(args),
  });
  if (!response.ok) {
    const errText = await response.text();
    throw new Error(errText || `Backend IPC error (${response.status})`);
  }
  return response.json() as Promise<T>;
}

export async function inspectFilesIpc(
  paths: string[],
  vaultPasswords: VaultEntry[] = []
): Promise<InspectedFile[]> {
  return callBackend<InspectedFile[]>("inspect_files", {
    paths,
    vaultPasswords,
  });
}

export async function convertFileIpc(
  fileId: string,
  inputPath: string,
  targetFormat: string,
  options: ConversionOptions
): Promise<{
  outputPath: string;
  outputSizeBytes: number;
  savingsPercent: number;
}> {
  return callBackend("convert_single_file", {
    fileId,
    inputPath,
    targetFormat,
    options,
  });
}

export async function convertBatchImagesToPdfIpc(
  inputPaths: string[],
  outputDir: string,
  outputFileName: string
): Promise<PdfToolkitResult> {
  return callBackend("merge_images_to_pdf", {
    inputPaths,
    outputDir,
    outputFileName,
  });
}

export async function runPdfToolkitIpc(
  request: PdfToolkitRequest
): Promise<PdfToolkitResult> {
  return callBackend<PdfToolkitResult>("run_pdf_toolkit", { request });
}

export async function unlockFileIpc(
  fileId: string,
  inputPath: string,
  outputDir: string,
  manualPassword?: string,
  vaultEntries: VaultEntry[] = []
): Promise<UnlockResult> {
  return callBackend<UnlockResult>("unlock_protected_file", {
    fileId,
    inputPath,
    outputDir,
    manualPassword,
    vaultEntries,
  });
}

export async function loadVaultEntriesIpc(): Promise<VaultEntry[]> {
  return callBackend<VaultEntry[]>("load_vault_entries", {});
}

export async function saveVaultEntriesIpc(
  entries: VaultEntry[]
): Promise<VaultEntry[]> {
  return callBackend<VaultEntry[]>("save_vault_entries", { entries });
}

export async function checkDependenciesIpc(): Promise<DependencyStatus[]> {
  return callBackend<DependencyStatus[]>("check_dependencies", {});
}

export async function installDependencyIpc(
  depName: "ffmpeg" | "pandoc" | "typst"
): Promise<DependencyStatus> {
  return callBackend<DependencyStatus>("install_dependency", { depName });
}

export async function readBinaryFileBase64Ipc(path: string): Promise<string> {
  return callBackend<string>("read_file_base64", { path });
}

export async function writeBinaryFileBase64Ipc(
  outputPath: string,
  base64Data: string
): Promise<{ outputPath: string; sizeBytes: number }> {
  return callBackend("write_file_base64", { outputPath, base64Data });
}

export async function generateDemoFilesIpc(): Promise<string[]> {
  return callBackend<string[]>("generate_demo_samples", {});
}
