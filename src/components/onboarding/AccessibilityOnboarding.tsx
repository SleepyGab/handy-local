import { useTranslation } from "react-i18next";
import { relaunch } from "@tauri-apps/plugin-process";
import { toast } from "sonner";
import { usePermissionFlow } from "@/hooks/usePermissionFlow";
import HandyTextLogo from "../icons/HandyTextLogo";
import { Keyboard, Mic, Check, Loader2 } from "lucide-react";

interface AccessibilityOnboardingProps {
  onComplete: () => void;
  preview?: boolean;
}

const AccessibilityOnboarding: React.FC<AccessibilityOnboardingProps> = ({
  onComplete,
  preview = false,
}) => {
  const { t } = useTranslation();
  const {
    isMacOS,
    isWindows,
    permissions,
    needsRecovery,
    ready,
    request,
    recheck,
  } = usePermissionFlow(onComplete, preview);
  const showMicrophonePermission = isMacOS || isWindows;
  const showAccessibilityPermission = isMacOS;
  const allGranted = ready;
  const isChecking =
    permissions.accessibility === "checking" ||
    permissions.microphone === "checking";
  const handleGrantAccessibility = () => void request("accessibility");
  const handleGrantMicrophone = () => void request("microphone");
  const restart = async () => {
    if (preview) return;
    try {
      await relaunch();
    } catch {
      toast.error(t("onboarding.permissions.errors.restartFailed"));
    }
  };

  // Still checking platform/initial permissions
  if (isChecking) {
    return (
      <div className="h-screen w-full flex items-center justify-center">
        <Loader2 className="w-8 h-8 animate-spin text-text/50" />
      </div>
    );
  }

  // All permissions granted - show success briefly
  if (allGranted) {
    return (
      <div className="h-screen w-full flex flex-col items-center justify-center gap-4">
        <div className="p-4 rounded-full bg-emerald-500/20">
          <Check className="w-12 h-12 text-emerald-400" />
        </div>
        <p className="text-lg font-medium text-text">
          {t("onboarding.permissions.allGranted")}
        </p>
      </div>
    );
  }

  // Show permissions request screen
  return (
    <div className="min-h-screen w-full flex flex-col p-6 gap-6 items-center justify-center">
      <div className="flex flex-col items-center gap-2">
        <HandyTextLogo width={200} />
      </div>

      <div className="max-w-md w-full flex flex-col items-center gap-4">
        <div className="text-center mb-2">
          <h2 className="text-xl font-semibold text-text mb-2">
            {t("onboarding.permissions.title")}
          </h2>
          <p className="text-text/70">
            {t("onboarding.permissions.description")}
          </p>
        </div>

        {/* Microphone Permission Card */}
        {showMicrophonePermission && (
          <div className="w-full p-4 rounded-lg bg-white/5 border border-mid-gray/20">
            <div className="flex items-center gap-4">
              <div className="p-3 rounded-full bg-logo-primary/20 shrink-0">
                <Mic className="w-6 h-6 text-logo-primary" />
              </div>
              <div className="flex-1 min-w-0">
                <h3 className="font-medium text-text">
                  {t("onboarding.permissions.microphone.title")}
                </h3>
                <p className="text-sm text-text/60 mb-3">
                  {t("onboarding.permissions.microphone.description")}
                </p>
                {permissions.microphone === "granted" ? (
                  <div className="flex items-center gap-2 text-emerald-400 text-sm">
                    <Check className="w-4 h-4" />
                    {t("onboarding.permissions.granted")}
                  </div>
                ) : permissions.microphone === "waiting" ? (
                  <div className="flex items-center gap-2 text-text/50 text-sm">
                    <Loader2 className="w-4 h-4 animate-spin" />
                    {t("onboarding.permissions.waiting")}
                  </div>
                ) : (
                  <button
                    onClick={handleGrantMicrophone}
                    className="px-4 py-2 rounded-lg bg-logo-primary hover:bg-logo-primary/90 text-white text-sm font-medium transition-colors"
                  >
                    {isWindows
                      ? t("accessibility.openSettings")
                      : t("onboarding.permissions.grant")}
                  </button>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Accessibility Permission Card */}
        {showAccessibilityPermission && (
          <div className="w-full p-4 rounded-lg bg-white/5 border border-mid-gray/20">
            <div className="flex items-center gap-4">
              <div className="p-3 rounded-full bg-logo-primary/20 shrink-0">
                <Keyboard className="w-6 h-6 text-logo-primary" />
              </div>
              <div className="flex-1 min-w-0">
                <h3 className="font-medium text-text">
                  {t("onboarding.permissions.accessibility.title")}
                </h3>
                <p className="text-sm text-text/60 mb-3">
                  {t("onboarding.permissions.accessibility.description")}
                </p>
                {permissions.accessibility === "granted" ? (
                  <div className="flex items-center gap-2 text-emerald-400 text-sm">
                    <Check className="w-4 h-4" />
                    {t("onboarding.permissions.granted")}
                  </div>
                ) : permissions.accessibility === "waiting" ? (
                  <div className="flex items-center gap-2 text-text/50 text-sm">
                    <Loader2 className="w-4 h-4 animate-spin" />
                    {t("onboarding.permissions.waiting")}
                  </div>
                ) : (
                  <button
                    onClick={handleGrantAccessibility}
                    className="px-4 py-2 rounded-lg bg-logo-primary hover:bg-logo-primary/90 text-white text-sm font-medium transition-colors"
                  >
                    {t("onboarding.permissions.grant")}
                  </button>
                )}
              </div>
            </div>
          </div>
        )}
        {needsRecovery && (
          <p role="status" className="text-sm text-text/70">
            {t(
              permissions.accessibility !== "granted"
                ? "onboarding.permissions.recovery.accessibility"
                : permissions.microphone !== "granted"
                  ? "onboarding.permissions.recovery.microphone"
                  : "onboarding.permissions.recovery.initialization",
            )}
          </p>
        )}
        <div className="flex flex-wrap justify-center gap-3">
          <button
            onClick={recheck}
            className="px-3 py-2 rounded-lg border border-mid-gray/30 text-sm hover:bg-white/5"
          >
            {t("onboarding.permissions.recheck")}
          </button>
          {isMacOS && permissions.accessibility === "waiting" && (
            <button
              onClick={handleGrantAccessibility}
              className="px-3 py-2 rounded-lg border border-mid-gray/30 text-sm hover:bg-white/5"
            >
              {t("accessibility.openSettings")}
            </button>
          )}
          {needsRecovery && (
            <button
              onClick={restart}
              className="px-3 py-2 rounded-lg border border-mid-gray/30 text-sm hover:bg-white/5"
            >
              {t("onboarding.permissions.restart")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
};

export default AccessibilityOnboarding;
