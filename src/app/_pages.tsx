import { lazy } from "react";

/* One lazy chunk per route, matching Next's per-route code splitting. */
export const StatusPage = lazy(() => import("./status/page"));
export const ScanPage = lazy(() => import("./scan/page"));
export const FlushPage = lazy(() => import("./flush/page"));
export const RenewPage = lazy(() => import("./renew/page"));
export const ResetPage = lazy(() => import("./reset/page"));
export const DnsPage = lazy(() => import("./dns/page"));
export const IpPage = lazy(() => import("./ip/page"));
export const AutoPage = lazy(() => import("./auto/page"));
