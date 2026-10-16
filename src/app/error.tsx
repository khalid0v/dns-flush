import { useNavigate, useRouteError } from "react-router";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

/** Route-level error boundary, the App Router `error.tsx` equivalent. */
export default function ErrorPage() {
  const error = useRouteError();
  const navigate = useNavigate();

  const message =
    error instanceof Error ? error.message : typeof error === "string" ? error : "Unknown error";

  return (
    <Card className="border-destructive/40">
      <CardHeader>
        <CardTitle>Something went wrong</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <p className="text-sm text-muted-foreground">
          This page failed to render. The details below help diagnose the issue.
        </p>
        <pre className="overflow-auto rounded-md bg-muted p-3 text-xs">{message}</pre>
        <Button variant="outline" onClick={() => void navigate("/status")}>
          Back to status
        </Button>
      </CardContent>
    </Card>
  );
}
