import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { platform } from "@tauri-apps/plugin-os";
import {
  checkAccessibilityPermission,
  requestAccessibilityPermission,
} from "tauri-plugin-macos-permissions-api";
import { toast } from "sonner";
import {
  openAccessibilitySettings,
  withPermissionTimeout,
} from "@/lib/permissions";

const AccessibilityPermissions: React.FC = () => {
  const { t } = useTranslation();
  const [hasAccessibility, setHasAccessibility] = useState(true);
  const isMacOS = platform() === "macos";

  useEffect(() => {
    if (!isMacOS) return;
    let cancelled = false;
    let checking = false;
    const check = async () => {
      if (checking) return;
      checking = true;
      try {
        const granted = await withPermissionTimeout(
          checkAccessibilityPermission(),
        );
        if (!cancelled) setHasAccessibility(granted);
      } catch (error) {
        console.warn("Failed to check accessibility permission:", error);
        if (!cancelled) setHasAccessibility(false);
      } finally {
        checking = false;
      }
    };
    void check();
    const onFocus = () => void check();
    const timer = setInterval(onFocus, 3000);
    window.addEventListener("focus", onFocus);
    return () => {
      cancelled = true;
      clearInterval(timer);
      window.removeEventListener("focus", onFocus);
    };
  }, [isMacOS]);

  const openSettings = async () => {
    try {
      await withPermissionTimeout(requestAccessibilityPermission());
      await openAccessibilitySettings();
    } catch {
      toast.error(t("onboarding.permissions.errors.requestFailed"));
    }
  };
  if (!isMacOS || hasAccessibility) return null;
  return (
    <div className="p-4 w-full rounded-lg border border-mid-gray">
      <div className="flex justify-between items-center gap-2">
        <p className="text-sm font-medium">
          {t("accessibility.permissionsDescription")}
        </p>
        <button
          onClick={openSettings}
          className="min-h-10 px-2 py-1 text-sm font-semibold border border-mid-gray/80 rounded hover:bg-logo-primary/10"
        >
          {t("accessibility.openSettings")}
        </button>
      </div>
    </div>
  );
};
export default AccessibilityPermissions;
