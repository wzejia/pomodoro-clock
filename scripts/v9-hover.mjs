#!/usr/bin/env bun
// v9：单 WS 会话 hover/press 定时帧采集（WebView2 :9223）
//   bun v9-hover.mjs hover <prefix> <x> <y>   — 纯悬停：+40/+120/+700ms 三帧，末帧后撤走复位
//   bun v9-hover.mjs press <prefix> <x> <y>   — 按压：down 后 +60/+350ms 两帧，移开后再 up（不触发点击，不改计时状态）
import { writeFileSync } from "fs";

const [, , mode, prefix, sx, sy] = process.argv;
for (const k of ["HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy", "ALL_PROXY", "all_proxy"]) delete process.env[k];
const x = Number(sx), y = Number(sy);

const list = await fetch("http://127.0.0.1:9223/json/list").then((r) => r.json());
const page = list.find((t) => t.type === "page" && t.url.includes("localhost"));
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
const shot = async (name) => {
  const r = await send("Page.captureScreenshot", { format: "png" });
  writeFileSync(name, Buffer.from(r.data, "base64"));
  console.log("saved", name);
};
const moved = (mx, my) => send("Input.dispatchMouseEvent", { type: "mouseMoved", x: mx, y: my, button: "none", clickCount: 0 });

// 先挪到上方 70px 中性点，让既有 hover 态归零、动画沉淀
await moved(x, y - 70);
await sleep(350);
await moved(x, y);

if (mode === "hover") {
  await sleep(40); await shot(`${prefix}-mid1.png`);
  await sleep(80); await shot(`${prefix}-mid2.png`);   // 累计 +120ms
  await sleep(580); await shot(`${prefix}-end.png`);   // 累计 +700ms（动画已结束）
  await moved(x, y - 70); await sleep(300);            // 撤走，复位 hover
} else if (mode === "press") {
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x, y, button: "left", clickCount: 1 });
  await sleep(60); await shot(`${prefix}-press1.png`);
  await sleep(290); await shot(`${prefix}-press2.png`);
  await moved(x, y - 70);                              // 移开再松手：不触发 click，计时状态不变
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x, y: y - 70, button: "left", clickCount: 1 });
  await sleep(300);
} else {
  console.error("usage: v9-hover.mjs hover|press <prefix> <x> <y>");
  process.exit(1);
}
ws.close();
process.exit(0);
