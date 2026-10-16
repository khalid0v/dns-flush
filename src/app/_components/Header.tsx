import { Menu, Moon, PanelBottom, RotateCw, Sun } from "lucide-react";
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
import { cn } from "@/lib/utils";
import { useAppStore } from "@/store/appStore";

interface HeaderProps {
  dark: boolean;
  onToggleTheme: () => void;
  activityVisible: boolean;
  onToggleActivity: () => void;
  onOpenSidebar: () => void;
}

export function Header({
  dark,
  onToggleTheme,
  activityVisible,
  onToggleActivity,
  onOpenSidebar,
}: HeaderProps) {
  const busy = useAppStore((state) => state.busy);
  const adapters = useAppStore((state) => state.adapters);
  const selectedIndex = useAppStore((state) => state.selectedIndex);
  const selectAdapter = useAppStore((state) => state.selectAdapter);
  const refreshAll = useAppStore((state) => state.refreshAll);

  return (
    <header className="flex flex-wrap items-center gap-x-3 gap-y-2 border-b bg-sidebar px-3 py-2.5 sm:px-4">
      <Button
        variant="ghost"
        size="icon"
        className="md:hidden"
        title="Open navigation"
        aria-label="Open navigation"
        onClick={onOpenSidebar}
      >
        <Menu className="size-5" />
      </Button>

      <Badge variant={busy ? "destructive" : "secondary"} className="gap-1.5">
        <span
          className={cn(
            "size-1.5 rounded-full",
            busy ? "animate-pulse bg-amber-400" : "bg-green-500",
          )}
        />
        {busy ? "Working…" : "Ready"}
      </Badge>

      <span className="hidden text-sm text-muted-foreground sm:inline">
        Network adapter
      </span>
      <Select
        value={selectedIndex === null ? "" : String(selectedIndex)}
        onValueChange={(value) => selectAdapter(Number(value))}
        disabled={adapters.length === 0}
      >
        <SelectTrigger className="w-full sm:w-72 sm:flex-1">
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

      <Button
        variant="outline"
        size="sm"
        disabled={busy}
        title="Refresh status"
        onClick={refreshAll}
      >
        <RotateCw className="size-4" />
        <span className="hidden sm:inline">Refresh status</span>
      </Button>

      <div className="ml-auto flex items-center gap-2">
        <Button
          variant="ghost"
          size="sm"
          title={activityVisible ? "Hide activity" : "Show activity"}
          aria-label={activityVisible ? "Hide activity" : "Show activity"}
          onClick={onToggleActivity}
        >
          <PanelBottom className="size-4" />
          <span className="hidden sm:inline">
            {activityVisible ? "Hide activity" : "Show activity"}
          </span>
        </Button>
        <Button
          variant="ghost"
          size="sm"
          title={dark ? "Light mode" : "Dark mode"}
          aria-label={dark ? "Switch to light mode" : "Switch to dark mode"}
          onClick={onToggleTheme}
        >
          {dark ? <Sun className="size-4" /> : <Moon className="size-4" />}
          <span className="hidden sm:inline">
            {dark ? "Light mode" : "Dark mode"}
          </span>
        </Button>
      </div>
    </header>
  );
}
