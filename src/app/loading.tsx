import { Skeleton } from "@/components/ui/skeleton";

/** Route-segment fallback shown while a lazily loaded page arrives. */
export default function Loading() {
  return (
    <div aria-busy className="flex flex-col gap-4">
      <div className="space-y-2">
        <Skeleton className="h-7 w-56" />
        <Skeleton className="h-4 w-80" />
      </div>
      <div className="grid gap-4 sm:grid-cols-2">
        <Skeleton className="h-28 w-full" />
        <Skeleton className="h-28 w-full" />
      </div>
    </div>
  );
}
