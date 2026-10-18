import { describe, expect, test } from "bun:test";
import { springValue } from "../src/spring";

describe("springValue", () => {
  test("t=0 原样返回位移和速度", () => {
    const s = springValue(0.42, 391, -80, 0);
    expect(s.value).toBeCloseTo(391);
    expect(s.velocity).toBeCloseTo(-80);
  });

  test("从静止出发不过冲", () => {
    for (let i = 0; i <= 240; i++) {
      const s = springValue(0.42, 391, 0, (i / 240) * 1.4);
      expect(s.value).toBeGreaterThanOrEqual(-1e-6);
      expect(s.value).toBeLessThanOrEqual(391 + 1e-6);
    }
  });

  test("打断后位置和速度连续", () => {
    const mid = springValue(0.42, 391, 0, 0.14);
    const joined = springValue(0.32, mid.value, mid.velocity, 0);
    expect(joined.value).toBeCloseTo(mid.value);
    expect(joined.velocity).toBeCloseTo(mid.velocity);
    const step = springValue(0.32, mid.value, mid.velocity, 1 / 60);
    expect(Math.sign(step.value - mid.value)).toBe(Math.sign(mid.velocity));
  });
});
