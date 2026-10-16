import { createHashRouter } from "react-router";
import AppLayout from "./layout";
import ErrorPage from "./error";
import Home from "./page";
import NotFound from "./not-found";
import {
  AutoPage,
  DnsPage,
  FlushPage,
  IpPage,
  RenewPage,
  ResetPage,
  ScanPage,
  StatusPage,
} from "./_pages";

/**
 * App Router-style route tree: `layout` wraps every `page`. Uses a hash router
 * because Tauri serves the app over a custom asset protocol, where path-based
 * URLs do not survive a reload.
 */
export const router = createHashRouter([
  {
    path: "/",
    element: <AppLayout />,
    errorElement: <ErrorPage />,
    children: [
      { index: true, element: <Home /> },
      { path: "status", element: <StatusPage /> },
      { path: "scan", element: <ScanPage /> },
      { path: "flush", element: <FlushPage /> },
      { path: "renew", element: <RenewPage /> },
      { path: "reset", element: <ResetPage /> },
      { path: "dns", element: <DnsPage /> },
      { path: "ip", element: <IpPage /> },
      { path: "auto", element: <AutoPage /> },
      { path: "*", element: <NotFound /> },
    ],
  },
]);
