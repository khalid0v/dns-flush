import type { ReactNode } from "react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";

interface ConfirmActionProps {
  /** Short title naming what will happen, e.g. "Reset network components?" */
  title: string;
  /** Current state, planned change, and impact. Shown before confirming. */
  description: string;
  /** Verb for the confirm button, e.g. "Reset". */
  actionLabel: string;
  destructive?: boolean;
  disabled?: boolean;
  onConfirm: () => void;
  children: ReactNode;
}

/**
 * Confirmation dialog for every state-changing repair: shows the current
 * state and planned change, then asks. Flush/scan stay direct actions.
 */
export function ConfirmAction({
  title,
  description,
  actionLabel,
  destructive,
  disabled,
  onConfirm,
  children,
}: ConfirmActionProps) {
  return (
    <AlertDialog>
      <AlertDialogTrigger
        render={
          <Button variant={destructive ? "destructive" : "default"} disabled={disabled} />
        }
      >
        {children}
      </AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription>{description}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            variant={destructive ? "destructive" : "default"}
            onClick={onConfirm}
          >
            {actionLabel}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
