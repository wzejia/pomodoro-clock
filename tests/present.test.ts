import { describe, expect, test } from "bun:test";
import { isCardPresented } from "../src/present";

const panel = {
  expanded: true,
  animating: false,
  height: 467,
  panelHeight: 467,
  elapsedSec: 0,
  durationSec: 0.42,
  expanding: false,
};

describe("isCardPresented", () => {
  test("已经停在面板尺寸", () => {
    expect(isCardPresented(panel)).toBe(true);
  });

  test("展开未到感知时长", () => {
    expect(isCardPresented({
      ...panel,
      animating: true,
      height: 300,
      elapsedSec: 0.2,
      expanding: true,
    })).toBe(false);
  });

  test("展开过了感知时长就可以进设置", () => {
    expect(isCardPresented({
      ...panel,
      animating: true,
      height: 463,
      elapsedSec: 0.42,
      expanding: true,
    })).toBe(true);
  });

  test("收起过程不算已经呈现", () => {
    expect(isCardPresented({
      ...panel,
      animating: true,
      height: 200,
      elapsedSec: 0.5,
      expanding: false,
    })).toBe(false);
  });
});
