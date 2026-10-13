import { Moon, PanelBottom, RotateCw, Sun } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { adapterTitle } from "@/lib/adapters";
import type { AdapterInfo } from "@/lib/types";
import { cn } from "@/lib/utils";

interface HeaderProps {
  busy: boolean;
  adapters: AdapterInfo[];
  selectedIndex: number | null;
  onSelectIndex: (index: number) => void;
  onRefresh: () => void;
  dark: boolean;
  onToggleTheme: () => void;
  activityVisible: boolean;
  onToggleActivity: () => void;
}

export function Header({
  busy,
  adapters,
  selectedIndex,
  onSelectIndex,
  onRefresh,
  dark,
  onToggleTheme,
  activityVisible,
  onToggleActivity,
}: HeaderProps) {
  return (
    <header className="flex flex-wrap items-center gap-x-3 gap-y-2 border-b bg-sidebar px-4 py-2.5">
      <Badge variant={busy ? "destructive" : "secondary"} className="gap-1.5">
        <span
          className={cn(
            "size-1.5 rounded-full",
            busy ? "animate-pulse bg-amber-400" : "bg-green-500",
          )}
        />
        {busy ? "Working…" : "Ready"}
      </Badge>

      <span className="text-sm text-muted-foreground">Network adapter</span>
      <Select
        value={selectedIndex === null ? "" : String(selectedIndex)}
        onValueChange={(value) => onSelectIndex(Number(value))}
        disabled={adapters.length === 0}
      >
        <SelectTrigger className="w-72 max-w-full flex-1">
          <SelectValue placeholder="No adapter found" />
        </SelectTrigger>
        <SelectContent>
          {adapters.map((adapter) => (
            <SelectItem key={adapter.index} value={String(adapter.index)}>
              {adapterTitle(adapter)}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      <Button variant="outline" size="sm" disabled={busy} onClick={onRefresh}>
        <RotateCw className="size-4" />
        Refresh status
      </Button>

      <div className="ml-auto flex items-center gap-2">
        <Button variant="ghost" size="sm" onClick={onToggleActivity}>
          <PanelBottom className="size-4" />
          {activityVisible ? "Hide activity" : "Show activity"}
        </Button>
        <Button variant="ghost" size="sm" onClick={onToggleTheme}>
          {dark ? <Sun className="size-4" /> : <Moon className="size-4" />}
          {dark ? "Light mode" : "Dark mode"}
        </Button>
      </div>
    </header>
  );
}
