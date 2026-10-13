import { useState } from "react";
import { PageHeader } from "@/components/app/PageHeader";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import type { PageProps } from "@/lib/types";

export function FlushPage({ busy, runAction }: PageProps) {
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
