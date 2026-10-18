import type { Status } from "./bridge";

export function runLabel(status: Status | undefined): string {
  if (status === "running") return "暂停";
  if (status === "paused") return "继续";
  return "开始";
}
