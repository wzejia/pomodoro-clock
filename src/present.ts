/** 卡片是否已经可以打开设置：停在面板尺寸，或展开弹簧过了感知时长。 */
export function isCardPresented(input: {
  expanded: boolean;
  animating: boolean;
  height: number;
  panelHeight: number;
  elapsedSec: number;
  durationSec: number;
  expanding: boolean;
}): boolean {
  if (input.expanded && !input.animating && input.height >= input.panelHeight - 1) return true;
  return input.animating && input.expanding && input.elapsedSec >= input.durationSec;
}
