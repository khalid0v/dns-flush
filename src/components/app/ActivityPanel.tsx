import { Trash2, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { LogKind } from "@/lib/types";
import { cn } from "@/lib/utils";

export interface ActivityLine {
  kind: LogKind;
  text: string;
}

const KIND_CLASS: Record<LogKind, string> = {
  head: "font-semibold text-primary",
  info: "text-foreground",
  ok: "text-green-600 dark:text-green-400",
  warn: "text-amber-600 dark:text-amber-400",
  err: "text-red-600 dark:text-red-400",
};

interface ActivityPanelProps {
  lines: ActivityLine[];
  onClear: () => void;
  onHide: () => void;
}

export function ActivityPanel({ lines, onClear, onHide }: ActivityPanelProps) {
  return (
    <section className="border-t bg-sidebar">
      <div className="flex items-center gap-2 px-4 pt-2">
        <p className="text-sm font-semibold">Activity</p>
        <div className="ml-auto flex items-center gap-1">
          <Button variant="ghost" size="sm" onClick={onClear}>
            <Trash2 className="size-4" />
            Clear activity
          </Button>
          <Button variant="ghost" size="sm" onClick={onHide}>
            <X className="size-4" />
            Hide
          </Button>
        </div>
      </div>
      <ScrollArea className="h-32 px-4 pb-3">
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
