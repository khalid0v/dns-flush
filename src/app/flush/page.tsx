import { useState } from "react";
import { PageHeader } from "@/app/_components/PageHeader";
import { Button } from "@/components/ui/button";
import { usePageProps } from "@/hooks/usePageProps";
import { api } from "@/services/api";

export default function FlushPage() {
  const { busy, runAction } = usePageProps();
  const [done, setDone] = useState(false);

  return (
    <div>
      <PageHeader
        title="Flush DNS cache"
        sub="Clear the Windows DNS resolver cache. Safe to run any time."
      />
      <Button
        disabled={busy}
        onClick={() => {
          setDone(false);
          void runAction(async () => {
            await api.flushDns();
            setDone(true);
          });
        }}
      >
        Flush DNS cache
      </Button>
      {done && !busy && (
        <p className="mt-3 text-sm text-muted-foreground">
          Finished — see the activity log for details.
        </p>
      )}
    </div>
  );
}
