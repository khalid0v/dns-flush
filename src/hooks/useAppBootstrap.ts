import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { api } from "@/lib/api";
import type { AdapterInfo, LogLine, PublicIpPayload } from "@/lib/types";
import { useAppStore } from "@/store/appStore";

/**
 * Subscribes to backend events and performs the initial load, once per app.
 * Mirrors the old App mount effect: listeners are registered before the first
 * fetches so no early event is missed. A subscription that resolves after
 * teardown immediately unsubscribes, so StrictMode's double mount in dev
 * cannot leave duplicate listeners behind.
 */
export function useAppBootstrap() {
  useEffect(() => {
    const { pushLog, applyAdapters, setPublicIp, setPublicIpLoading, showToast } =
      useAppStore.getState();

    const unlisteners: UnlistenFn[] = [];
    let cancelled = false;

    const subscribe = async <T>(channel: string, handler: (payload: T) => void) => {
      const unlisten = await listen<T>(channel, (event) => handler(event.payload));
      if (cancelled) {
        unlisten();
        return;
      }
      unlisteners.push(unlisten);
    };

    void (async () => {
      try {
        await subscribe<LogLine>("log", (payload) => pushLog(payload.kind, payload.text));
        await subscribe<AdapterInfo[]>("adapters", (payload) => applyAdapters(payload));
        await subscribe<PublicIpPayload>("public-ip", (payload) => {
          setPublicIp(payload);
          setPublicIpLoading(false);
        });
        await subscribe<{ text: string }>("toast", (payload) => showToast(payload.text));
      } catch (error) {
        pushLog("err", `Event subscription failed: ${String(error)}`);
      }
      if (cancelled) return;

      try {
        const list = await api.listAdapters();
        if (!cancelled) {
          applyAdapters(list);
          pushLog("info", `${list.length} adapter(s) found.`);
        }
      } catch (error) {
        if (!cancelled) pushLog("err", String(error));
      }

      try {
        const ip = await api.getPublicIp();
        if (!cancelled) setPublicIp({ ok: true, value: ip });
      } catch (error) {
        if (!cancelled) setPublicIp({ ok: false, value: String(error) });
      } finally {
        if (!cancelled) setPublicIpLoading(false);
      }

      try {
        await api.setMonitorEnabled(true);
        await api.refreshMonitor();
      } catch (error) {
        if (!cancelled) pushLog("err", String(error));
      }
    })();

    return () => {
      cancelled = true;
      for (const unlisten of unlisteners) unlisten();
    };
  }, []);
}
