import { labelPlacement, sectorPath, WHEEL_CENTER, WHEEL_OUTER, WHEEL_VIEW } from "./wheelGeometry";
import { shortLabel, type WheelEntry, type WheelPalette } from "./wheelData";

/** 轮盘边长；SVG 是矢量，改这个值不会糊。 */
export const WHEEL_SIZE = 440;

/**
 * 静止的轮盘：只用 SVG 画扇区与短标签。
 *
 * 旋转不在这里做 —— 页面上的圆心指针自己转，扇区保持不动。
 */
export function WheelDisc({
  entries,
  palette,
}: {
  entries: readonly WheelEntry[];
  palette: WheelPalette;
}) {
  return (
    <div className="wheel-disc" style={{ width: WHEEL_SIZE, height: WHEEL_SIZE }}>
      <svg
        viewBox={"0 0 " + WHEEL_VIEW + " " + WHEEL_VIEW}
        role="img"
        aria-label={"候选轮盘，共 " + entries.length + " 项"}
      >
        {entries.map((entry, index) => (
          <path
            key={entry.id}
            d={sectorPath(index, entries.length)}
            fill={palette.background[index % palette.background.length]}
            stroke={palette.border}
            strokeWidth={0.5}
          />
        ))}

        {entries.map((entry, index) => {
          const label = labelPlacement(index, entries.length);
          return (
            <text
              key={entry.id}
              x={label.x}
              y={label.y}
              transform={"rotate(" + label.rotation + " " + label.x + " " + label.y + ")"}
              textAnchor="middle"
              dominantBaseline="central"
              fill={palette.text}
              fontSize={3.8}
              fontWeight={600}
              fontFamily="sans-serif"
            >
              {entry.short ?? shortLabel(entry.label)}
            </text>
          );
        })}

        <circle
          cx={WHEEL_CENTER}
          cy={WHEEL_CENTER}
          r={WHEEL_OUTER}
          fill="none"
          stroke={palette.border}
          strokeWidth={0.8}
        />
      </svg>
    </div>
  );
}
