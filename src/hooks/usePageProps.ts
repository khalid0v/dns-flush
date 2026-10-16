import { useAppStore } from "@/store/appStore";
import type { PageProps } from "@/lib/types";

/**
 * Derives the props every tool page needs from the global store, so pages
 * consume shared state through one stable contract instead of prop-drilling.
 */
export function usePageProps(): PageProps {
  const adapters = useAppStore((state) => state.adapters);
  const selectedIndex = useAppStore((state) => state.selectedIndex);
  const busy = useAppStore((state) => state.busy);
  const runAction = useAppStore((state) => state.runAction);
  const pushLog = useAppStore((state) => state.pushLog);

  const selected = adapters.find((adapter) => adapter.index === selectedIndex);

  return { adapters, selected, busy, runAction, pushLog };
}
