import { Suspense, useEffect, useState } from "react";
import { Outlet } from "react-router";
import { ActivityPanel } from "./_components/ActivityPanel";
import { Header } from "./_components/Header";
import { Sidebar, SidebarDrawer } from "./_components/Sidebar";
import { Toast } from "./_components/Toast";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useAppBootstrap } from "@/hooks/useAppBootstrap";
import { useTheme } from "@/hooks/useTheme";
import Loading from "./loading";

/** Root layout: the persistent shell that wraps every tool page. */
export default function AppLayout() {
  useAppBootstrap();
  const { dark, toggle: toggleTheme } = useTheme();
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [activityVisible, setActivityVisible] = useState(true);

  useEffect(() => {
    const onResize = () => {
      if (window.innerWidth >= 768) setSidebarOpen(false);
    };
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  return (
    <div className="flex h-dvh overflow-hidden bg-background text-foreground">
      <Sidebar />
      <SidebarDrawer open={sidebarOpen} onOpenChange={setSidebarOpen} />
      <div className="flex min-w-0 flex-1 flex-col">
        <Header
          dark={dark}
          onToggleTheme={toggleTheme}
          activityVisible={activityVisible}
          onToggleActivity={() => setActivityVisible((value) => !value)}
          onOpenSidebar={() => setSidebarOpen(true)}
        />
        <ScrollArea className="min-h-0 flex-1">
          <main className="mx-auto w-full max-w-4xl px-4 py-5 sm:px-6 sm:py-6">
            <Suspense fallback={<Loading />}>
              <Outlet />
            </Suspense>
          </main>
        </ScrollArea>
        {activityVisible && <ActivityPanel onHide={() => setActivityVisible(false)} />}
      </div>
      <Toast />
    </div>
  );
}
