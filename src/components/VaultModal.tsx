import React, { useState } from "react";
import { KeyRound, LockOpen, Plus, Shield, Trash2, X } from "lucide-react";
import { useAppStore } from "../store/useAppStore";
import { saveVaultEntriesIpc } from "../lib/ipc";
import type { VaultEntry } from "../types/omniconvert";

export const VaultModal: React.FC = () => {
  const {
    isVaultModalOpen,
    setVaultModalOpen,
    vaultEntries,
    setVaultEntries,
    setStatusBanner,
  } = useAppStore();

  const [label, setLabel] = useState("");
  const [password, setPassword] = useState("");
  const [categoryHint, setCategoryHint] = useState("Számla / Banki kivonat");

  if (!isVaultModalOpen) return null;

  const handleAddEntry = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!label.trim() || !password.trim()) return;

    const newEntry: VaultEntry = {
      id: `vault-${Date.now()}`,
      label: label.trim(),
      password: password.trim(),
      categoryHint: categoryHint.trim() || "Általános",
      createdAt: new Date().toISOString().slice(0, 10),
    };

    const next = [...vaultEntries, newEntry];
    setVaultEntries(next);
    setLabel("");
    setPassword("");

    try {
      await saveVaultEntriesIpc(next);
    } catch {
      // Local state already updated
    }
    setStatusBanner({
      type: "success",
      text: `"${newEntry.label}" hozzáadva az Auto-Unlock Saját Jelszótárhoz.`,
    });
  };

  const handleRemoveEntry = async (id: string) => {
    const next = vaultEntries.filter((e) => e.id !== id);
    setVaultEntries(next);
    try {
      await saveVaultEntriesIpc(next);
    } catch {
      // Local state updated
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/75 flex items-center justify-center p-4">
      <div className="w-full max-w-xl bg-zinc-900 border border-zinc-800 rounded shadow-2xl flex flex-col max-h-[85vh]">
        {/* Header */}
        <div className="px-4 py-3 border-b border-zinc-800 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <KeyRound className="w-4 h-4 text-amber-500" strokeWidth={1.5} />
            <h2 className="text-sm font-semibold text-zinc-100">
              Automatikus Saját Jelszótár (Auto-Unlock Keyring)
            </h2>
          </div>
          <button
            type="button"
            onClick={() => setVaultModalOpen(false)}
            className="p-1 text-zinc-400 hover:text-zinc-200 rounded"
          >
            <X className="w-4 h-4" strokeWidth={1.5} />
          </button>
        </div>

        {/* Explanation Banner */}
        <div className="px-4 py-2.5 bg-zinc-950/70 border-b border-zinc-800 flex items-start gap-2.5 text-xs text-zinc-400">
          <Shield
            className="w-4 h-4 text-emerald-500 shrink-0 mt-0.5"
            strokeWidth={1.5}
          />
          <p className="leading-relaxed">
            Amikor megnyitási jelszóval védett <strong className="text-zinc-200">PDF, ZIP vagy 7Z</strong>{" "}
            fájlt húz be, az OmniConvert a háttérben azonnal végigpróbálja az itt
            elmentett saját jelszavait (pl. adószámok, banki kivonat kódok), és
            találat esetén <strong className="text-emerald-400">jelszó bekérése nélkül</strong>{" "}
            feloldja a dokumentumot.
          </p>
        </div>

        {/* Add New Password Form */}
        <form
          onSubmit={handleAddEntry}
          className="p-4 border-b border-zinc-800 grid grid-cols-12 gap-2 items-end"
        >
          <div className="col-span-4">
            <label className="block text-[11px] text-zinc-400 mb-1">
              Megnevezés / Címke
            </label>
            <input
              id="input-vault-label"
              type="text"
              placeholder="pl. Telekom számla kód"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              className="w-full h-8 px-2.5 text-xs bg-zinc-950 border border-zinc-800 rounded text-zinc-100 focus:outline-none focus:border-blue-600"
            />
          </div>
          <div className="col-span-4">
            <label className="block text-[11px] text-zinc-400 mb-1">
              Jelszó / Adószám / PIN
            </label>
            <input
              id="input-vault-password"
              type="text"
              placeholder="Jelszó..."
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="w-full h-8 px-2.5 text-xs font-mono bg-zinc-950 border border-zinc-800 rounded text-zinc-100 focus:outline-none focus:border-blue-600"
            />
          </div>
          <div className="col-span-4 flex gap-1.5">
            <div className="flex-1">
              <label className="block text-[11px] text-zinc-400 mb-1">
                Kategória
              </label>
              <input
                type="text"
                value={categoryHint}
                onChange={(e) => setCategoryHint(e.target.value)}
                className="w-full h-8 px-2 text-xs bg-zinc-950 border border-zinc-800 rounded text-zinc-300 focus:outline-none focus:border-blue-600"
              />
            </div>
            <button
              id="btn-vault-add"
              type="submit"
              className="h-8 px-3 bg-blue-600 hover:bg-blue-500 text-white text-xs font-medium rounded flex items-center gap-1 shrink-0"
            >
              <Plus className="w-3.5 h-3.5" strokeWidth={1.5} />
              Mentés
            </button>
          </div>
        </form>

        {/* Saved Entries List */}
        <div className="flex-1 overflow-y-auto p-4 space-y-2">
          <div className="text-xs font-medium text-zinc-400 mb-1 tabular-nums">
            Tárolt automatikus feloldó kulcsok ({vaultEntries.length})
          </div>
          {vaultEntries.map((entry) => (
            <div
              key={entry.id}
              className="px-3 py-2 bg-zinc-950 border border-zinc-800 rounded flex items-center justify-between text-xs"
            >
              <div className="flex items-center gap-3">
                <LockOpen
                  className="w-3.5 h-3.5 text-emerald-500 shrink-0"
                  strokeWidth={1.5}
                />
                <div>
                  <div className="font-medium text-zinc-200">{entry.label}</div>
                  <div className="text-[11px] text-zinc-400">
                    {entry.categoryHint} · Hozzáadva: {entry.createdAt}
                  </div>
                </div>
              </div>
              <div className="flex items-center gap-3">
                <code className="px-2 py-0.5 bg-zinc-900 border border-zinc-800 rounded font-mono text-zinc-300">
                  {entry.password}
                </code>
                <button
                  type="button"
                  onClick={() => handleRemoveEntry(entry.id)}
                  className="p-1 text-zinc-400 hover:text-red-400 rounded"
                >
                  <Trash2 className="w-3.5 h-3.5" strokeWidth={1.5} />
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
