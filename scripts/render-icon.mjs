#!/usr/bin/env bun
// SVG → PNG 光栅化（复用 WebView2 CDP :9223，浏览器级 WS 建新 target，不污染 app 页面）
// 用法: bun scripts/render-icon.mjs <svgPath> <size> <outPng>
import { readFileSync, writeFileSync } from "fs";
import { resolve } from "path";

const [, , svgPath, sizeArg, outPng] = process.argv;
const size = Number(sizeArg);
if (!svgPath || !size || !outPng) { console.error("usage: render-icon.mjs <svg> <size> <out.png>"); process.exit(1); }
for (const k of ["HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy", "ALL_PROXY", "all_proxy"]) delete process.env[k];

const ver = await fetch("http://127.0.0.1:9223/json/version").then((r) => r.json());
const ws = new WebSocket(ver.webSocketDebuggerUrl);
let id = 0;
const pending = new Map();
const send = (method, params = {}, sessionId) =>
  new Promise((resolve, reject) => {
    const mid = ++id;
    pending.set(mid, { resolve, reject });
    ws.send(JSON.stringify({ id: mid, method, params, ...(sessionId ? { sessionId } : {}) }));
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

const { targetId } = await send("Target.createTarget", { url: "about:blank" });
const { sessionId } = await send("Target.attachToTarget", { targetId, flatten: true });
const s = (m, p = {}) => send(m, p, sessionId);

await s("Page.enable");
await s("Runtime.enable");
await s("Emulation.setDeviceMetricsOverride", { width: size, height: size, deviceScaleFactor: 1, mobile: false });
const svg = readFileSync(resolve(svgPath), "utf8");
const html = `<!doctype html><html><head><style>html,body{margin:0;padding:0;background:transparent}svg{display:block;width:${size}px;height:${size}px}</style></head><body>${svg}</body></html>`;
await s("Runtime.evaluate", { expression: `document.open();document.write(${JSON.stringify(html)});document.close();"ok"`, awaitPromise: true });
await new Promise((r) => setTimeout(r, 400));
const shot = await s("Page.captureScreenshot", { format: "png" });
writeFileSync(outPng, Buffer.from(shot.data, "base64"));
console.log(`rendered ${outPng} (${size}x${size})`);

await send("Target.closeTarget", { targetId }).catch(() => {});
ws.close();
process.exit(0);
