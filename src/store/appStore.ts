import { create } from "zustand";
import { preferredAdapter } from "@/lib/adapters";
import { api } from "@/lib/api";
import type { AdapterInfo, LogKind, LogLine, PublicIpPayload } from "@/lib/types";

const MAX_LINES = 500;

interface AppState {
  /* Adapters */
  adapters: AdapterInfo[];
  selectedIndex: number | null;
  applyAdapters: (list: AdapterInfo[]) => void;
  selectAdapter: (index: number) => void;

  /* Activity log */
  lines: LogLine[];
  pushLog: (kind: LogKind, text: string) => void;
  clearLog: () => void;

  /* Long-running action coordination */
  busy: boolean;
  runAction: (fn: () => Promise<unknown>) => Promise<void>;

  /* Public IP */
  publicIp: PublicIpPayload | null;
  publicIpLoading: boolean;
  setPublicIp: (payload: PublicIpPayload) => void;
  setPublicIpLoading: (loading: boolean) => void;

  /* Live monitor */
  monitorOn: boolean;
  setMonitorEnabled: (on: boolean) => void;

  /* Toast */
  toast: string | null;
  showToast: (text: string) => void;
  dismissToast: () => void;

  /* Orchestration */
  refreshAll: () => void;
}

export const useAppStore = create<AppState>()((set, get) => ({
  adapters: [],
  selectedIndex: null,

  /** Replace the adapter list, keeping the selection or auto-selecting the live one. */
  applyAdapters: (list) =>
    set((state) => {
      const current = state.selectedIndex;
      if (current !== null && list.some((adapter) => adapter.index === current)) {
        return { adapters: list };
      }
      return {
        adapters: list,
        selectedIndex: list[preferredAdapter(list)]?.index ?? null,
      };
    }),

  selectAdapter: (index) => set({ selectedIndex: index }),

  lines: [],

  pushLog: (kind, text) =>
    set((state) => ({
      lines: [...state.lines.slice(-MAX_LINES + 1), { kind, text }],
    })),

  clearLog: () => set({ lines: [] }),

  busy: false,

  runAction: async (fn) => {
    set({ busy: true });
    try {
      await fn();
    } catch (error) {
      get().pushLog("err", error instanceof Error ? error.message : String(error));
    } finally {
      set({ busy: false });
    }
  },

  publicIp: null,
  publicIpLoading: true,
  setPublicIp: (payload) => set({ publicIp: payload }),
  setPublicIpLoading: (loading) => set({ publicIpLoading: loading }),

  monitorOn: true,

  setMonitorEnabled: (on) => {
    set({ monitorOn: on });
    void api.setMonitorEnabled(on).catch((error: unknown) => {
      get().pushLog("err", String(error));
    });
  },

  toast: null,
  showToast: (text) => set({ toast: text }),
  dismissToast: () => set({ toast: null }),

  refreshAll: () =>
    void get().runAction(async () => {
      try {
        get().applyAdapters(await api.listAdapters());
      } catch (error) {
        get().pushLog("err", String(error));
      }
      set({ publicIpLoading: true });
      try {
        set({ publicIp: { ok: true, value: await api.getPublicIp() } });
      } catch (error) {
        set({ publicIp: { ok: false, value: String(error) } });
      } finally {
        set({ publicIpLoading: false });
      }
      try {
        await api.refreshMonitor();
      } catch (error) {
        get().pushLog("err", String(error));
      }
    }),
}));
