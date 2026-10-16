import {
  Activity,
  Droplets,
  ExternalLink,
  Globe,
  Network,
  Radar,
  RefreshCw,
  Sparkles,
  TriangleAlert,
} from "lucide-react";
import { NavLink } from "react-router";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Button, buttonVariants } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import type { PageKey } from "@/lib/types";
import { cn } from "@/lib/utils";

interface NavItem {
  key: PageKey;
  label: string;
  icon: typeof Activity;
}

interface NavGroup {
  title: string;
  items: NavItem[];
}

const GROUPS: NavGroup[] = [
  {
    title: "Overview",
    items: [
      { key: "status", label: "Network status", icon: Activity },
      { key: "scan", label: "Find devices", icon: Radar },
    ],
  },
  {
    title: "Repair",
    items: [
      { key: "flush", label: "Flush DNS cache", icon: Droplets },
      { key: "renew", label: "Renew connection", icon: RefreshCw },
      { key: "reset", label: "Reset network", icon: TriangleAlert },
      { key: "dns", label: "Choose DNS servers", icon: Globe },
      { key: "ip", label: "Set IP address", icon: Network },
      { key: "auto", label: "Automatic setup", icon: Sparkles },
    ],
  },
];

/**
 * The navigation list. `compact` renders an icon-only rail with accessible
 * names; the expanded form shows group headings and labels.
 */
function NavList({
  compact,
  onNavigate,
}: {
  compact: boolean;
  onNavigate?: () => void;
}) {
  return (
    <div className="flex flex-col gap-4">
      {GROUPS.map((group) => (
        <div key={group.title}>
          {!compact && (
            <p className="px-3 pb-1 text-xs font-medium text-muted-foreground">
              {group.title}
            </p>
          )}
          <nav className="flex flex-col gap-0.5">
            {group.items.map((item) => {
              const Icon = item.icon;
              return (
                <NavLink
                  key={item.key}
                  to={`/${item.key}`}
                  title={compact ? item.label : undefined}
                  aria-label={item.label}
                  onClick={onNavigate}
                  className={({ isActive }) =>
                    cn(
                      buttonVariants({
                        variant: isActive ? "secondary" : "ghost",
                        size: compact ? "icon" : "default",
                      }),
                      "relative",
                      compact ? "mx-auto" : "justify-start gap-2.5",
                      !isActive && "text-muted-foreground",
                      isActive &&
                        !compact &&
                        "before:absolute before:inset-y-1.5 before:left-0 before:w-0.5 before:rounded-full before:bg-primary",
                    )
                  }
                >
                  <Icon className="size-4 shrink-0" />
                  {!compact && <span className="truncate">{item.label}</span>}
                </NavLink>
              );
            })}
          </nav>
        </div>
      ))}
    </div>
  );
}

function Brand({ compact }: { compact: boolean }) {
  if (compact) {
    return (
      <div className="flex justify-center px-2 pt-4 pb-2">
        <span
          aria-hidden
          className="grid size-8 place-items-center rounded-lg bg-primary text-sm font-semibold text-primary-foreground"
        >
          T
        </span>
      </div>
    );
  }
  return (
    <div className="px-4 pt-4 pb-2">
      <p className="text-lg font-semibold tracking-tight">trubo</p>
      <p className="text-xs text-muted-foreground">Windows network toolkit</p>
    </div>
  );
}

function Credit({ compact }: { compact: boolean }) {
  const open = () => void openUrl("https://github.com/khalid0v");
  if (compact) {
    return (
      <div className="flex justify-center px-2 py-3">
        <Button
          variant="ghost"
          size="icon"
          title="github.com/khalid0v"
          aria-label="Open the author's GitHub profile"
          onClick={open}
        >
          <ExternalLink className="size-4" />
        </Button>
      </div>
    );
  }
  return (
    <div className="px-4 py-3">
      <Separator className="mb-2" />
      <p className="text-xs text-muted-foreground">
        dev by ahmed khalid |{" "}
        <button
          type="button"
          className="text-primary underline-offset-4 hover:underline"
          onClick={open}
        >
          github.com/khalid0v
        </button>
      </p>
    </div>
  );
}

/**
 * The persistent sidebar: a labelled column on large windows and an icon rail
 * on medium ones. Below the `md` breakpoint the navigation moves into
 * `SidebarDrawer` instead.
 */
export function Sidebar() {
  return (
    <>
      <aside className="hidden w-14 shrink-0 flex-col border-r bg-sidebar md:flex lg:hidden">
        <Brand compact />
        <div className="flex flex-1 flex-col overflow-y-auto px-2 py-2">
          <NavList compact />
        </div>
        <Credit compact />
      </aside>
      <aside className="hidden w-56 shrink-0 flex-col border-r bg-sidebar lg:flex">
        <Brand compact={false} />
        <div className="flex flex-1 flex-col overflow-y-auto px-2 py-2">
          <NavList compact={false} />
        </div>
        <Credit compact={false} />
      </aside>
    </>
  );
}

interface SidebarDrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/** Off-canvas navigation for narrow windows. */
export function SidebarDrawer({ open, onOpenChange }: SidebarDrawerProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="inset-y-0 left-0 flex h-dvh w-64 max-w-[80vw] flex-col border-r duration-200 data-open:animate-in data-open:slide-in-from-left data-closed:animate-out data-closed:slide-out-to-left">
        <DialogTitle className="sr-only">Main navigation</DialogTitle>
        <Brand compact={false} />
        <div className="flex flex-1 flex-col overflow-y-auto px-2 py-2">
          <NavList compact={false} onNavigate={() => onOpenChange(false)} />
        </div>
        <Credit compact={false} />
      </DialogContent>
    </Dialog>
  );
}
