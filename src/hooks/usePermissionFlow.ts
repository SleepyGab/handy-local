import { useCallback, useEffect, useRef, useState } from "react";
import { platform } from "@tauri-apps/plugin-os";
import {
  checkAccessibilityPermission,
  checkMicrophonePermission,
  requestAccessibilityPermission,
  requestMicrophonePermission,
} from "tauri-plugin-macos-permissions-api";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { useSettingsStore } from "@/stores/settingsStore";
import {
  openAccessibilitySettings,
  withPermissionTimeout,
} from "@/lib/permissions";

type PermissionStatus = "checking" | "needed" | "waiting" | "granted";
type PermissionKind = "accessibility" | "microphone";
type Permissions = Record<PermissionKind, PermissionStatus>;

export function usePermissionFlow(onComplete: () => void, preview: boolean) {
  const { t } = useTranslation();
  const [os] = useState(() => platform());
  const isMacOS = os === "macos";
  const isWindows = os === "windows";
  const [permissions, setPermissions] = useState<Permissions>({
    accessibility: isMacOS ? "checking" : "granted",
    microphone: isMacOS || isWindows ? "checking" : "granted",
  });
  const [needsRecovery, setNeedsRecovery] = useState(false);
  const [ready, setReady] = useState(false);
  const completeRef = useRef(onComplete);
  completeRef.current = onComplete;
  const checkRef = useRef<() => Promise<void>>(async () => {});
  const mounted = useRef(false);
  const refreshAudioDevices = useSettingsStore(
    (state) => state.refreshAudioDevices,
  );
  const refreshOutputDevices = useSettingsStore(
    (state) => state.refreshOutputDevices,
  );

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  useEffect(() => {
    if (preview) {
      setPermissions({
        accessibility: isMacOS ? "needed" : "granted",
        microphone: isMacOS || isWindows ? "needed" : "granted",
      });
      return;
    }
    if (!isMacOS && !isWindows) {
      completeRef.current();
      return;
    }

    let cancelled = false;
    let checking = false;
    let completed = false;
    let initialized = false;
    let errors = 0;
    let completionTimer: ReturnType<typeof setTimeout> | undefined;
    const check = async () => {
      // Slow checks must not accumulate overlapping polls or complete twice.
      if (cancelled || checking || completed) return;
      checking = true;
      try {
        const [accessibility, microphone] = await withPermissionTimeout(
          isMacOS
            ? Promise.all([
                checkAccessibilityPermission(),
                checkMicrophonePermission(),
              ])
            : commands
                .getWindowsMicrophonePermissionStatus()
                .then((status) => [
                  true,
                  !status.supported || status.overall_access !== "denied",
                ]),
        );
        if (cancelled) return;
        errors = 0;
        setPermissions((prev) => ({
          accessibility: accessibility
            ? "granted"
            : prev.accessibility === "waiting"
              ? "waiting"
              : "needed",
          microphone: microphone
            ? "granted"
            : prev.microphone === "waiting"
              ? "waiting"
              : "needed",
        }));
        if (isMacOS && accessibility && !initialized) {
          initialized = true;
          // Keep side effects out of React state updaters (StrictMode can replay them).
          const results = await withPermissionTimeout(
            Promise.all([
              commands.initializeEnigo(),
              commands.initializeShortcuts(),
            ]),
          );
          if (cancelled) return;
          if (results.some((result) => result.status === "error")) {
            initialized = false;
            throw new Error("Input initialization failed");
          }
        }
        if (accessibility && microphone) {
          completed = true;
          // Device enumeration failure must not trap an already-authorized user.
          await withPermissionTimeout(
            Promise.all([refreshAudioDevices(), refreshOutputDevices()]),
          ).catch((error) =>
            console.warn("Failed to refresh audio devices:", error),
          );
          if (!cancelled) {
            setReady(true);
            completionTimer = setTimeout(() => completeRef.current(), 300);
          }
        }
      } catch (error) {
        if (cancelled) return;
        initialized = false;
        console.warn("Failed to check permissions:", error);
        errors += 1;
        setNeedsRecovery(true);
        setPermissions((prev) => ({
          accessibility:
            prev.accessibility === "granted" ? "granted" : "needed",
          microphone: prev.microphone === "granted" ? "granted" : "needed",
        }));
        if (errors === 3)
          toast.error(t("onboarding.permissions.errors.checkFailed"));
      } finally {
        checking = false;
      }
    };
    checkRef.current = check;
    void check();
    // Also detect grants made outside our own request button.
    const polling = setInterval(() => void check(), 1000);
    const onFocus = () => void check();
    const onVisibility = () => {
      if (!document.hidden) void check();
    };
    window.addEventListener("focus", onFocus);
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      cancelled = true;
      clearInterval(polling);
      clearTimeout(completionTimer);
      window.removeEventListener("focus", onFocus);
      document.removeEventListener("visibilitychange", onVisibility);
      checkRef.current = async () => {};
    };
  }, [
    isMacOS,
    isWindows,
    preview,
    refreshAudioDevices,
    refreshOutputDevices,
    t,
  ]);

  // A negative permission response is valid, not an exception. Bound the waiting
  // UI as well as IPC calls, while continuing to notice a later grant.
  useEffect(() => {
    if (
      permissions.accessibility !== "waiting" &&
      permissions.microphone !== "waiting"
    )
      return;
    const timer = setTimeout(() => {
      setPermissions((prev) => ({
        accessibility:
          prev.accessibility === "waiting" ? "needed" : prev.accessibility,
        microphone: prev.microphone === "waiting" ? "needed" : prev.microphone,
      }));
      setNeedsRecovery(true);
    }, 15000);
    return () => clearTimeout(timer);
  }, [permissions.accessibility, permissions.microphone]);

  const request = useCallback(
    async (kind: PermissionKind) => {
      if (preview) return;
      setPermissions((prev) => ({ ...prev, [kind]: "waiting" }));
      try {
        await withPermissionTimeout(
          (async () => {
            if (kind === "accessibility") {
              await requestAccessibilityPermission();
              await openAccessibilitySettings();
            } else if (isWindows) {
              const result = await commands.openMicrophonePrivacySettings();
              if (result.status === "error") throw new Error(result.error);
            } else {
              await requestMicrophonePermission();
            }
          })(),
        );
        if (mounted.current) await checkRef.current();
      } catch (error) {
        if (!mounted.current) return;
        console.warn("Failed to request permission:", error);
        setPermissions((prev) => ({
          ...prev,
          [kind]: prev[kind] === "granted" ? "granted" : "needed",
        }));
        setNeedsRecovery(true);
        toast.error(t("onboarding.permissions.errors.requestFailed"));
      }
    },
    [isWindows, preview, t],
  );

  return {
    isMacOS,
    isWindows,
    permissions,
    needsRecovery,
    ready,
    recheck: () => {
      if (!preview) void checkRef.current();
    },
    request,
  };
}
