#!/usr/bin/env bun
// CDP screencast 逐帧取证：展开动画期间每帧存 PNG + 记录 viewport 尺寸（验单调）
// 用法: bun scripts/capture-screencast.mjs <outDir> [expand|collapse]
import { mkdirSync, writeFileSync } from "fs";

const [, , outDir, mode = "expand"] = process.argv;
for (const k of ["HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy", "ALL_PROXY", "all_proxy"]) delete process.env[k];
mkdirSync(outDir, { recursive: true });

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

const frames = []; // {ts, w, h, file}
ws.onmessage = (e) => {
  const msg = JSON.parse(e.data);
  if (msg.id && pending.has(msg.id)) {
    const { resolve, reject } = pending.get(msg.id);
    pending.delete(msg.id);
    msg.error ? reject(new Error(JSON.stringify(msg.error))) : resolve(msg.result);
  } else if (msg.method === "Page.screencastFrame") {
    const { data, metadata, sessionId } = msg.params;
    const file = `${outDir}/f${String(frames.length).padStart(2, "0")}.png`;
    writeFileSync(file, Buffer.from(data, "base64"));
    frames.push({ ts: metadata.timestamp, w: metadata.deviceWidth, h: metadata.deviceHeight, file });
    send("Page.screencastFrameAck", { sessionId }).catch(() => {});
  }
};
await new Promise((r) => (ws.onopen = r));

await send("Page.enable");
await send("Page.startScreencast", { format: "png", everyNthFrame: 1 });
await new Promise((r) => setTimeout(r, 300)); // 先收一帧稳态

const t0 = Date.now();
const expr = mode === "expand"
  ? 'document.getElementById("app").classList.contains("expanded") || (() => { const p = document.getElementById("pill"); const r = p.getBoundingClientRect(); p.dispatchEvent(new MouseEvent("mousedown",{clientX:100,clientY:38,screenX:500,screenY:300,bubbles:true})); p.dispatchEvent(new MouseEvent("mouseup",{clientX:100,clientY:38,screenX:500,screenY:300,bubbles:true})); })(), "go"'
  : 'document.getElementById("btn-collapse").click(), "go"';
await send("Runtime.evaluate", { expression: expr, awaitPromise: true, returnByValue: true });
await new Promise((r) => setTimeout(r, 1200));
await send("Page.stopScreencast");

// 输出尺寸序列（相对触发的毫秒、viewport 宽x高）
const lines = frames.map((f, i) => `${String(Math.round((f.ts - frames[0].ts) * 1000)).padStart(4)}ms  ${f.w}x${f.h}  ${f.file.split(/[\\/]/).pop()}`);
console.log(lines.join("\n"));
ws.close();
process.exit(0);
