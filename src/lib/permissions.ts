import { invoke } from "@tauri-apps/api/core";

// An IPC failure must not leave the permission screen waiting forever.
export async function withPermissionTimeout<T>(
  operation: Promise<T>,
): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      operation,
      new Promise<never>((_, reject) => {
        timer = setTimeout(
          () => reject(new Error("Permission check timed out")),
          5000,
        );
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}

export function openAccessibilitySettings(): Promise<void> {
  return invoke("open_accessibility_settings");
}
