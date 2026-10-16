import { Link } from "react-router";
import { PageHeader } from "@/app/_components/PageHeader";
import { buttonVariants } from "@/components/ui/button";

/** Rendered for unknown routes, the App Router `not-found.tsx` equivalent. */
export default function NotFound() {
  return (
    <div>
      <PageHeader title="Page not found" sub="That tool does not exist." />
      <Link to="/status" className={buttonVariants({ variant: "outline" })}>
        Back to status
      </Link>
    </div>
  );
}
