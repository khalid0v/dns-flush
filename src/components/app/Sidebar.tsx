import {
  Activity,
  Droplets,
  Globe,
  Network,
  Radar,
  RefreshCw,
  Sparkles,
  TriangleAlert,
} from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import type { PageKey } from "@/lib/types";
import { cn } from "@/lib/utils";

interface NavItem {
  key: PageKey;
  label: string;
  icon: typeof Activity;
}

const OVERVIEW: NavItem[] = [
  { key: "status", label: "Network status", icon: Activity },
  { key: "scan", label: "Find devices", icon: Radar },
];

const REPAIRS: NavItem[] = [
  { key: "flush", label: "Flush DNS cache", icon: Droplets },
  { key: "renew", label: "Renew connection", icon: RefreshCw },
  { key: "reset", label: "Reset network", icon: TriangleAlert },
  { key: "dns", label: "Choose DNS servers", icon: Globe },
  { key: "ip", label: "Set IP address", icon: Network },
  { key: "auto", label: "Automatic setup", icon: Sparkles },
];

interface SidebarProps {
  page: PageKey;
  onNavigate: (page: PageKey) => void;
}

function Group({ title, items, page, onNavigate }: {
  title: string;
  items: NavItem[];
  page: PageKey;
  onNavigate: (page: PageKey) => void;
}) {
  return (
    <div>
      <p className="px-3 pb-1 text-xs font-medium text-muted-foreground">{title}</p>
      <nav className="flex flex-col gap-0.5">
        {items.map((item) => {
          const Icon = item.icon;
          const active = page === item.key;
          return (
            <Button
              key={item.key}
              variant={active ? "secondary" : "ghost"}
              className={cn("justify-start gap-2.5", !active && "text-muted-foreground")}
              onClick={() => onNavigate(item.key)}
            >
              <Icon className="size-4 shrink-0" />
              <span className="truncate">{item.label}</span>
            </Button>
          );
        })}
      </nav>
    </div>
  );
}

export function Sidebar({ page, onNavigate }: SidebarProps) {
  return (
    <aside className="flex w-56 shrink-0 flex-col border-r bg-sidebar">
      <div className="px-4 pt-4 pb-2">
        <p className="text-lg font-semibold tracking-tight">trubo</p>
        <p className="text-xs text-muted-foreground">Windows network toolkit</p>
      </div>
      <div className="flex flex-1 flex-col gap-4 overflow-y-auto px-2 py-2">
        <Group title="Overview" items={OVERVIEW} page={page} onNavigate={onNavigate} />
        <Group title="Repair" items={REPAIRS} page={page} onNavigate={onNavigate} />
      </div>
      <div className="px-4 py-3">
        <Separator className="mb-2" />
        <p className="text-xs text-muted-foreground">
          dev by ahmed khalid |{" "}
          <button
            type="button"
            className="text-primary underline-offset-4 hover:underline"
            onClick={() => void openUrl("https://github.com/khalid0v")}
          >
            github.com/khalid0v
          </button>
        </p>
      </div>
    </aside>
  );
}
