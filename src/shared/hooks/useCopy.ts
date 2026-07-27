import { useCallback, useEffect, useRef, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

/** How long the "Copied" confirmation stays up. */
const CONFIRMATION_MS = 1600;

/**
 * Copies text and reports which value was copied last, so several copy
 * buttons on one screen can each show their own confirmation.
 */
export function useCopy() {
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => () => clearTimeout(timer.current), []);

  const copy = useCallback(async (key: string, value: string) => {
    await writeText(value);
    setCopiedKey(key);

    clearTimeout(timer.current);
    timer.current = setTimeout(() => setCopiedKey(null), CONFIRMATION_MS);
  }, []);

  return { copy, copiedKey };
}
