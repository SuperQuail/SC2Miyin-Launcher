/** 候选共用的语气图标。只为演示，不进生产 —— 生产那套在 react/ui/Toast.tsx。 */
import type { LabTone } from "./types";

const PATHS: Record<LabTone, string[]> = {
  success: ["M20 6 9 17l-5-5"],
  warning: ["M12 9v5", "M12 17.5v.5", "M10.3 3.9 2.6 17a2 2 0 0 0 1.7 3h15.4a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z"],
  error: ["M18 6 6 18", "M6 6l12 12"],
  info: ["M12 11v6", "M12 7.5v.5", "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Z"],
};

export function ToneIcon({ tone, className }: { tone: LabTone; className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true">
      {PATHS[tone].map((d) => (
        <path key={d} d={d} />
      ))}
    </svg>
  );
}
