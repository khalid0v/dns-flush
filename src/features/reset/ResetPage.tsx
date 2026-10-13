import { ConfirmAction } from "@/components/app/ConfirmAction";
import { PageHeader } from "@/components/app/PageHeader";
import { api } from "@/lib/api";
import type { PageProps } from "@/lib/types";

export function ResetPage({ busy, runAction }: PageProps) {
  return (
    <div>
      <PageHeader
        title="Reset network components"
        sub="This may interrupt your connection. Restart the PC afterward."
      />
      <p className="mb-4 text-sm text-amber-600 dark:text-amber-400">
        This changes system network state: it resets Winsock and the TCP/IP
        stack back to installation defaults.
      </p>
      <ConfirmAction
        title="Reset network components?"
        description="Runs netsh winsock reset and netsh int ip reset, then requires a restart to finish. Any active connections will drop."
        actionLabel="Reset"
        destructive
        disabled={busy}
        onConfirm={() => void runAction(() => api.resetStack())}
      >
        Reset network
      </ConfirmAction>
    </div>
  );
}
