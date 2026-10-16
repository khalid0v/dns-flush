import { Navigate } from "react-router";

/** Index route: send the user to the default tool. */
export default function Home() {
  return <Navigate to="/status" replace />;
}
