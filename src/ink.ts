function channel(c: number): number {
  const x = c / 255;
  return x <= 0.04045 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4;
}

function parseColor(color: string): [number, number, number] | null {
  const hex = color.trim().match(/^#([0-9a-f]{6})$/i);
  if (hex) {
    const n = parseInt(hex[1], 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  }
  const rgb = color.match(/rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)/i);
  if (rgb) return [Number(rgb[1]), Number(rgb[2]), Number(rgb[3])];
  return null;
}

/** 在阶段填充色上挑深字或白字，取对比更高的一侧。 */
export function contrastingInk(color: string): string {
  const rgb = parseColor(color);
  if (!rgb) return "#1d1d1f";
  const L = 0.2126 * channel(rgb[0]) + 0.7152 * channel(rgb[1]) + 0.0722 * channel(rgb[2]);
  const dark = 0.012;
  const againstDark = (Math.max(L, dark) + 0.05) / (Math.min(L, dark) + 0.05);
  const againstWhite = (Math.max(L, 1) + 0.05) / (Math.min(L, 1) + 0.05);
  return againstDark >= againstWhite ? "#1d1d1f" : "#ffffff";
}
