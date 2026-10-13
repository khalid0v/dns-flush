import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { ActivityPanel } from "@/components/app/ActivityPanel";
import { Header } from "@/components/app/Header";
import { Sidebar } from "@/components/app/Sidebar";
import { Card, CardContent } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { AutoPage } from "@/features/auto/AutoPage";
import { DnsPage } from "@/features/dns/DnsPage";
import { FlushPage } from "@/features/flush/FlushPage";
import { IpPage } from "@/features/ip/IpPage";
import { RenewPage } from "@/features/renew/RenewPage";
import { ResetPage } from "@/features/reset/ResetPage";
import { ScanPage } from "@/features/scan/ScanPage";
import { StatusPage } from "@/features/status/StatusPage";
import { useTheme } from "@/hooks/useTheme";
import { preferredAdapter } from "@/lib/adapters";
import { api } from "@/lib/api";
import type {
  AdapterInfo,
  LogKind,
  LogLine,
  PageKey,
  PublicIpPayload,
} from "@/lib/types";

const MAX_LINES = 500;
const TOAST_MS = 6000;

export default function App() {
  const [page, setPage] = useState<PageKey>("status");
  const [adapters, setAdapters] = useState<AdapterInfo[]>([]);
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [lines, setLines] = useState<LogLine[]>([]);
  const [busy, setBusy] = useState(false);
  const [publicIp, setPublicIp] = useState<PublicIpPayload | null>(null);
  const [publicIpLoading, setPublicIpLoading] = useState(true);
  const [monitorOn, setMonitorOn] = useState(true);
  const [toast, setToast] = useState<string | null>(null);
  const [activityVisible, setActivityVisible] = useState(true);
  const { dark, toggle: toggleTheme } = useTheme();

  const pushLog = useCallback((kind: LogKind, text: string) => {
    setLines((prev) => [...prev.slice(-MAX_LINES + 1), { kind, text }]);
  }, []);

  const runAction = useCallback(
    async (fn: () => Promise<unknown>) => {
      setBusy(true);
      try {
        await fn();
      } catch (error) {
        pushLog("err", error instanceof Error ? error.message : String(error));
      } finally {
        setBusy(false);
      }
    },
    [pushLog],
  );

  /** Replace the adapter list, keeping the selection or auto-selecting the live one. */
  const applyAdapters = useCallback((list: AdapterInfo[]) => {
    setAdapters(list);
    setSelectedIndex((current) => {
      if (current !== null && list.some((adapter) => adapter.index === current)) {
        return current;
      }
      return list[preferredAdapter(list)]?.index ?? null;
    });
  }, []);

  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];
    let cancelled = false;
    void (async () => {
      unlisteners.push(
        await listen<LogLine>("log", (event) => pushLog(event.payload.kind, event.payload.text)),
      );
      unlisteners.push(
        await listen<AdapterInfo[]>("adapters", (event) => applyAdapters(event.payload)),
      );
      unlisteners.push(
        await listen<PublicIpPayload>("public-ip", (event) => {
          setPublicIp(event.payload);
          setPublicIpLoading(false);
        }),
      );
      unlisteners.push(
        await listen<{ text: string }>("toast", (event) => setToast(event.payload.text)),
      );
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
  }, [applyAdapters, pushLog]);

  useEffect(() => {
    if (toast === null) return;
    const timer = setTimeout(() => setToast(null), TOAST_MS);
    return () => clearTimeout(timer);
  }, [toast]);

  const toggleMonitor = useCallback(
    (on: boolean) => {
      setMonitorOn(on);
      api.setMonitorEnabled(on).catch((error: unknown) => pushLog("err", String(error)));
    },
    [pushLog],
  );

  const refreshAll = useCallback(() => {
    void runAction(async () => {
      try {
        applyAdapters(await api.listAdapters());
      } catch (error) {
        pushLog("err", String(error));
      }
      setPublicIpLoading(true);
      try {
        setPublicIp({ ok: true, value: await api.getPublicIp() });
      } catch (error) {
        setPublicIp({ ok: false, value: String(error) });
      } finally {
        setPublicIpLoading(false);
      }
      try {
        await api.refreshMonitor();
      } catch (error) {
        pushLog("err", String(error));
      }
    });
  }, [applyAdapters, pushLog, runAction]);

  const selected = adapters.find((adapter) => adapter.index === selectedIndex);
  const pageProps = { adapters, selected, busy, runAction, pushLog };

  return (
    <div className="flex h-screen overflow-hidden bg-background text-foreground">
      <Sidebar page={page} onNavigate={setPage} />
      <div className="flex min-w-0 flex-1 flex-col">
        <Header
          busy={busy}
          adapters={adapters}
          selectedIndex={selectedIndex}
          onSelectIndex={setSelectedIndex}
          onRefresh={refreshAll}
          dark={dark}
          onToggleTheme={toggleTheme}
          activityVisible={activityVisible}
          onToggleActivity={() => setActivityVisible((value) => !value)}
        />
        <ScrollArea className="min-h-0 flex-1">
          <main className="mx-auto w-full max-w-4xl px-6 py-6">
            {page === "status" && (
              <StatusPage
                {...pageProps}
                publicIp={publicIp}
                publicIpLoading={publicIpLoading}
                monitorOn={monitorOn}
                onToggleMonitor={toggleMonitor}
              />
            )}
            {page === "scan" && <ScanPage {...pageProps} />}
            {page === "flush" && <FlushPage {...pageProps} />}
            {page === "renew" && <RenewPage {...pageProps} />}
            {page === "reset" && <ResetPage {...pageProps} />}
            {page === "dns" && <DnsPage {...pageProps} />}
            {page === "ip" && <IpPage {...pageProps} />}
            {page === "auto" && <AutoPage {...pageProps} />}
          </main>
        </ScrollArea>
        {activityVisible && (
          <ActivityPanel
            lines={lines}
            onClear={() => setLines([])}
            onHide={() => setActivityVisible(false)}
          />
        )}
      </div>
      {toast !== null && (
        <div className="fixed right-4 bottom-4 z-50 max-w-sm">
          <Card className="border-primary">
            <CardContent className="pt-4 text-sm">{toast}</CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}
