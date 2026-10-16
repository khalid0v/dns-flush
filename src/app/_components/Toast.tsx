import { useEffect } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { useAppStore } from "@/store/appStore";

const TOAST_MS = 6000;

/** Transient notice pushed by the backend, auto-dismissed after a few seconds. */
export function Toast() {
  const toast = useAppStore((state) => state.toast);
  const dismissToast = useAppStore((state) => state.dismissToast);

  useEffect(() => {
    if (toast === null) return;
    const timer = setTimeout(dismissToast, TOAST_MS);
    return () => clearTimeout(timer);
  }, [toast, dismissToast]);

  if (toast === null) return null;

  return (
    <div className="fixed right-4 bottom-4 z-50 max-w-sm">
      <Card className="border-primary">
        <CardContent className="pt-4 text-sm">{toast}</CardContent>
      </Card>
    </div>
  );
}
