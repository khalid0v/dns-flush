import { Trash2, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { LogKind } from "@/types";
import { cn } from "@/components/utils";
import { useAppStore } from "@/store/appStore";

const KIND_CLASS: Record<LogKind, string> = {
  head: "font-semibold text-primary",
  info: "text-foreground",
  ok: "text-green-600 dark:text-green-400",
  warn: "text-amber-600 dark:text-amber-400",
  err: "text-red-600 dark:text-red-400",
};

interface ActivityPanelProps {
  onHide: () => void;
}

export function ActivityPanel({ onHide }: ActivityPanelProps) {
  const lines = useAppStore((state) => state.lines);
  const clearLog = useAppStore((state) => state.clearLog);

  return (
    <section className="border-t bg-sidebar">
      <div className="flex items-center gap-2 px-3 pt-2 sm:px-4">
        <p className="text-sm font-semibold">Activity</p>
        <div className="ml-auto flex items-center gap-1">
          <Button variant="ghost" size="sm" onClick={clearLog}>
            <Trash2 className="size-4" />
            <span className="hidden sm:inline">Clear activity</span>
          </Button>
          <Button variant="ghost" size="sm" onClick={onHide}>
            <X className="size-4" />
            <span className="hidden sm:inline">Hide</span>
          </Button>
        </div>
      </div>
      <ScrollArea className="h-24 px-3 pb-3 sm:h-32 sm:px-4">
        <div className="flex flex-col gap-0.5 font-mono text-xs">
          {lines.length === 0 ? (
            <p className="text-muted-foreground">No activity yet.</p>
          ) : (
            lines.map((line, at) => (
              // Lines are append-only; the position is a stable key.
              <p key={at} className={cn("break-words", KIND_CLASS[line.kind])}>
                {line.text}
              </p>
            ))
          )}
        </div>
      </ScrollArea>
    </section>
  );
}
