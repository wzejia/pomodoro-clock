#!/usr/bin/env bun
// v7 验收用单会话 CDP 序列执行器（WebView2 :9223）
//   bun cdp-seq.mjs state
//   bun cdp-seq.mjs jitter <x> <y> <jx> <jy>     — down→(60ms)→move(jitter)→(60ms)→up，全程 ~150ms
//   bun cdp-seq.mjs down <x> <y> / move <x> <y> / up <x> <y>
//   bun cdp-seq.mjs downmove <x> <y> <tx> <ty>   — down 后立即 move 到 (tx,ty)（触发拖拽）
//   bun cdp-seq.mjs leave                        — 向 #pill 与 #panel-head 派发 mouseleave（合成回归）
//   bun cdp-seq.mjs esc                          — 派发 Escape keydown
//   bun cdp-seq.mjs eval '<js>'
//   bun cdp-seq.mjs shot <path>                  — Page.captureScreenshot（离屏渲染，不受遮挡影响）
import { writeFileSync } from "fs";

const [, , cmd, ...rest] = process.argv;
for (const k of ["HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy", "ALL_PROXY", "all_proxy"]) delete process.env[k];

const list = await fetch("http://127.0.0.1:9223/json/list").then((r) => r.json());
// v11 双窗口：默认锁主窗（localhost 且非托盘菜单页）；CDP_MATCH=tray-menu 锁托盘菜单页
const match = process.env.CDP_MATCH || "";
const page = list.find((t) =>
  t.type === "page" &&
  (match ? t.url.includes(match) : t.url.includes("localhost") && !t.url.includes("tray-menu")));
if (!page) { console.error("no page target"); process.exit(1); }

const ws = new WebSocket(page.webSocketDebuggerUrl);
let id = 0;
const pending = new Map();
const send = (method, params = {}) =>
  new Promise((resolve, reject) => {
    const mid = ++id;
    pending.set(mid, { resolve, reject });
    ws.send(JSON.stringify({ id: mid, method, params }));
  });
ws.onmessage = (e) => {
  const msg = JSON.parse(e.data);
  if (msg.id && pending.has(msg.id)) {
    const { resolve, reject } = pending.get(msg.id);
    pending.delete(msg.id);
    msg.error ? reject(new Error(JSON.stringify(msg.error))) : resolve(msg.result);
  }
};
await new Promise((r) => (ws.onopen = r));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const mouse = (type, x, y, extra = {}) =>
  send("Input.dispatchMouseEvent", { type, x, y, button: "left", clickCount: 1, ...extra });
const evaluate = async (expr) => {
  const r = await send("Runtime.evaluate", { expression: expr, awaitPromise: true, returnByValue: true });
  if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails));
  return r.result?.value;
};

if (cmd === "state") {
  console.log(await evaluate(`JSON.stringify({cls: document.getElementById('app').className, drawer: document.getElementById('app').classList.contains('drawer-open'), menu: document.getElementById('ctx-menu').classList.contains('open'), dbg: window.__dbg})`));
} else if (cmd === "jitter") {
  const [x, y, jx, jy] = rest.map(Number);
  await mouse("mouseMoved", x, y, { button: "none", clickCount: 0 });
  await mouse("mousePressed", x, y);
  await sleep(60);
  await mouse("mouseMoved", x + jx, y + jy);
  await sleep(60);
  await mouse("mouseReleased", x + jx, y + jy);
  console.log("jittered");
} else if (cmd === "down") {
  const [x, y] = rest.map(Number);
  await mouse("mouseMoved", x, y, { button: "none", clickCount: 0 });
  await mouse("mousePressed", x, y);
  console.log("down");
} else if (cmd === "move") {
  const [x, y] = rest.map(Number);
  await mouse("mouseMoved", x, y);
  console.log("moved");
} else if (cmd === "up") {
  const [x, y] = rest.map(Number);
  await mouse("mouseReleased", x, y);
  console.log("up");
} else if (cmd === "downmove") {
  const [x, y, tx, ty] = rest.map(Number);
  await mouse("mouseMoved", x, y, { button: "none", clickCount: 0 });
  await mouse("mousePressed", x, y);
  await sleep(50);
  await mouse("mouseMoved", tx, ty);
  console.log("downmoved");
} else if (cmd === "leave") {
  await evaluate(`document.getElementById('pill').dispatchEvent(new MouseEvent('mouseleave', {bubbles: false})); document.getElementById('panel-head').dispatchEvent(new MouseEvent('mouseleave', {bubbles: false})); 'left'`);
  console.log("leave-dispatched");
} else if (cmd === "esc") {
  await evaluate(`document.dispatchEvent(new KeyboardEvent('keydown', {key: 'Escape', bubbles: true})); 'esc'`);
  console.log("esc-dispatched");
} else if (cmd === "eval") {
  console.log(JSON.stringify(await evaluate(rest.join(" "))));
} else if (cmd === "shot") {
  const r = await send("Page.captureScreenshot", { format: "png" });
  writeFileSync(rest[0], Buffer.from(r.data, "base64"));
  console.log("shot-saved");
} else {
  console.error("usage: cdp-seq.mjs state|jitter|down|move|up|downmove|leave|esc|eval|shot");
  process.exit(1);
}
ws.close();
process.exit(0);
