// 轮盘几何：扇区路径与标签位置。都是纯函数，方便单测。
//
// 角度约定：0° = 12 点，顺时针；和 targetRotation / sectorCenter 保持一致。

import { sectorCenter } from "./wheelData";

export const WHEEL_VIEW = 100;
export const WHEEL_CENTER = 50;
export const WHEEL_OUTER = 46;
const LABEL_RADIUS = 33;

function round(value: number): number {
  return Number(value.toFixed(2));
}

function point(angle: number, radius: number): { x: number; y: number } {
  const rad = (angle * Math.PI) / 180;
  return {
    x: WHEEL_CENTER + radius * Math.sin(rad),
    y: WHEEL_CENTER - radius * Math.cos(rad),
  };
}

/** 第 index 个扇区的 SVG path（从圆心到外沿）。count = 1 时画整圆。 */
export function sectorPath(index: number, count: number, outer = WHEEL_OUTER): string {
  if (count <= 0) return "";
  if (count === 1) {
    const top = round(WHEEL_CENTER - outer);
    const bottom = round(WHEEL_CENTER + outer);
    return (
      "M " + WHEEL_CENTER + " " + top +
      " A " + outer + " " + outer + " 0 1 1 " + WHEEL_CENTER + " " + bottom +
      " A " + outer + " " + outer + " 0 1 1 " + WHEEL_CENTER + " " + top + " Z"
    );
  }

  const step = 360 / count;
  const start = point(index * step, outer);
  const end = point((index + 1) * step, outer);
  const largeArc = step > 180 ? 1 : 0;
  return (
    "M " + WHEEL_CENTER + " " + WHEEL_CENTER +
    " L " + round(start.x) + " " + round(start.y) +
    " A " + outer + " " + outer + " 0 " + largeArc + " 1 " + round(end.x) + " " + round(end.y) +
    " Z"
  );
}

/**
 * 标签放在扇区中线上；左半圈（90°–270°）转 180°，否则字是倒的。
 * 返回的角度是 SVG rotate() 的绝对角度。
 */
export function labelPlacement(index: number, count: number, radius = LABEL_RADIUS): {
  x: number;
  y: number;
  rotation: number;
  flipped: boolean;
} {
  const angle = sectorCenter(index, count);
  const position = point(angle, radius);
  const flipped = angle > 90 && angle < 270;
  return {
    x: round(position.x),
    y: round(position.y),
    rotation: round(flipped ? angle + 180 : angle),
    flipped,
  };
}
