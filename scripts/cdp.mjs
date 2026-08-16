#!/usr/bin/env bun
// 极简 CDP 客户端（WebView2 :9223）
//   bun scripts/cdp.mjs eval '<js表达式>'
//   bun scripts/cdp.mjs click <x> <y>
//   bun scripts/cdp.mjs drag <x1> <y1> <x2> <y2>
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

const mouse = (type, x, y, extra = {}) =>
  send("Input.dispatchMouseEvent", { type, x, y, button: "left", clickCount: 1, ...extra });

if (cmd === "eval") {
  const result = await send("Runtime.evaluate", {
    expression: rest.join(" "), awaitPromise: true, returnByValue: true,
  });
  if (result.exceptionDetails) console.log("EXC: " + JSON.stringify(result.exceptionDetails.exception?.description ?? result.exceptionDetails.text));
  else console.log(JSON.stringify(result.result?.value, null, 1));
} else if (cmd === "click") {
  const [x, y] = rest.map(Number);
  await mouse("mouseMoved", x, y, { button: "none", clickCount: 0 });
  await mouse("mousePressed", x, y);
  await new Promise((r) => setTimeout(r, 80));
  await mouse("mouseReleased", x, y);
  console.log("clicked");
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
} else if (cmd === "drag") {
  const [x1, y1, x2, y2] = rest.map(Number);
  await mouse("mouseMoved", x1, y1, { button: "none", clickCount: 0 });
  await mouse("mousePressed", x1, y1);
  const steps = 20;
  for (let i = 1; i <= steps; i++) {
    await mouse("mouseMoved", x1 + ((x2 - x1) * i) / steps, y1 + ((y2 - y1) * i) / steps);
    await new Promise((r) => setTimeout(r, 20));
  }
  await mouse("mouseReleased", x2, y2);
  console.log("dragged");
} else {
  console.error("usage: cdp.mjs eval|click|drag ...");
  process.exit(1);
}
ws.close();
process.exit(0);
