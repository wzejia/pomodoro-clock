/**
 * 休息到休息的平滑弹簧（WWDC23 Animate with springs，bounce 固定为 0）。
 *
 * mass = 1
 * stiffness = (2π / duration)^2
 * damping = (1 - bounce) * 4π / duration
 * bounce = 0 ⇒ ζ = damping / (2 √(stiffness · mass)) = 1（临界阻尼，不过冲）
 *
 * y 是相对目标的位移。临界阻尼：
 * ω0 = 2π / duration
 * y(t) = (y0 + (v0 + ω0 y0) t) e^{-ω0 t}
 *
 * value / velocity 都相对目标，朝着 0 收敛。velocity 是 dy/dt，打断时原样当作新目标的初速度。
 */
export function springValue(
  durationSec: number,
  y0: number,
  v0: number,
  tSec: number,
): { value: number; velocity: number } {
  const bounce = 0;
  const mass = 1;
  const stiffness = (2 * Math.PI / durationSec) ** 2;
  const damping = (1 - bounce) * ((4 * Math.PI) / durationSec);
  const omega = Math.sqrt(stiffness / mass);
  const decay = Math.exp(-(damping / (2 * mass)) * tSec);
  const value = (y0 + (v0 + omega * y0) * tSec) * decay;
  const velocity = (v0 - omega * (v0 + omega * y0) * tSec) * decay;
  return { value, velocity };
}
