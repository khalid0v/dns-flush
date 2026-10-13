import { useCallback, useEffect, useState } from "react";

/** Light/dark appearance. Defaults to the OS choice; the toggle only overrides it. */
export function useTheme() {
  const [dark, setDark] = useState<boolean>(() =>
    window.matchMedia("(prefers-color-scheme: dark)").matches,
  );

  useEffect(() => {
    document.documentElement.classList.toggle("dark", dark);
  }, [dark]);

  const toggle = useCallback(() => setDark((value) => !value), []);

  return { dark, toggle };
}
