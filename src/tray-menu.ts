/* v11 托盘自绘菜单页（弃用原生 muda 菜单）：无边框小窗贴托盘图标弹出，复用 ctx-menu 苹果风。
   页面随窗口创建即加载（窗口 hidden），每次打开靠 tray-menu-opened 事件现查动态项——
   Rust 侧 tick 不再写托盘文案，文案/勾选态全部以此刻真相为准。 */

const tauri = (window as any).__TAURI__;
const invoke = tauri.core.invoke;
const listen: (event: string, cb: (e: { payload: any }) => void) => void = tauri.event.listen;
const emit: (event: string, payload?: unknown) => Promise<void> = tauri.event.emit;

const $ = <T extends HTMLElement = HTMLElement>(id: string) =>
  document.getElementById(id) as T;

function applyTheme(theme: string) {
  document.documentElement.dataset.theme = theme;
}
function applyMaterial(material: string) {
  document.documentElement.dataset.material = material;
}

/* 有意隐藏（点了某项动作/已处理关闭）与「真失焦」区分：blur 事件据此决定是否自动关 */
let hidingIntentionally = false;

/* 动态文案/勾选（每次打开现查）：开始/暂停随计时状态，自启勾跟随真实注册态 */
async function refreshState() {
  const s = (await invoke("timer_snapshot")) as { status: string };
  $("tm-toggle").textContent =
    s.status === "running" ? "暂停" : s.status === "paused" ? "继续" : "开始";
  const on = (await invoke("get_autostart")) as boolean;
  const el = $("tm-autostart");
  el.classList.toggle("checked", on);
  el.setAttribute("aria-checked", String(on));
}

async function dismiss() {
  hidingIntentionally = true;
  await invoke("tray_menu_hide");
}

/* 四项动作（统一「动作→focus_main→hide」次序，quit 除外）；
   错误兜底：任何 invoke 失败 console.error + 仍尝试 hide（不许页面卡死） */
$("tm-toggle").onclick = async () => {
  try {
    await invoke("timer_toggle");
    await invoke("focus_main");
  } catch (err) {
    console.error("[tray-menu] toggle 失败", err);
  }
  await dismiss().catch((err) => console.error("[tray-menu] hide 失败", err));
};
$("tm-settings").onclick = async () => {
  try {
    await invoke("focus_main");
    await dismiss();
    await emit("open-settings");
  } catch (err) {
    console.error("[tray-menu] settings 失败", err);
    await invoke("tray_menu_hide").catch(() => {});
  }
};
$("tm-autostart").onclick = async () => {
  try {
    const cur = $("tm-autostart").getAttribute("aria-checked") === "true";
    const now = (await invoke("set_autostart", { enabled: !cur })) as boolean;
    $("tm-autostart").classList.toggle("checked", now);
    $("tm-autostart").setAttribute("aria-checked", String(now));
    await invoke("focus_main");
  } catch (err) {
    console.error("[tray-menu] autostart 失败", err);
  }
  await dismiss().catch((err) => console.error("[tray-menu] hide 失败", err));
};
$("tm-quit").onclick = () => {
  invoke("quit_app").catch((err: unknown) => console.error("[tray-menu] quit 失败", err));
};

/* 打开时刷新（兜底启动竞态）：动态项现查 + 主题/材质重新对表。
   先 blur 掉残留焦点：本页常驻不 reload，上次弹出/自动化点击留下的 activeElement 会跨
   hide/show 存活，下次弹出时以「无操作自带焦点」形态出现（round-40 P2-1 环的来源之一）。 */
listen("tray-menu-opened", () => {
  hidingIntentionally = false;
  (document.activeElement as HTMLElement | null)?.blur?.();
  refreshState().catch((err) => console.error("[tray-menu] refresh 失败", err));
  emit("query-ui-style").catch(() => {});
});

/* 主窗广播的主题/材质对表（托盘页跟随应用换肤） */
listen("ui-style", (e) => {
  applyTheme(e.payload.theme);
  applyMaterial(e.payload.material);
});

/* 真失焦（点桌面/别窗）→ 自动关；Esc 同语义（不经 hidingIntentionally——
   主面板若展开且未聚焦会被整收，见 main.ts 的 tray-menu-closed 处理） */
listen("tray-menu-blurred", () => {
  if (hidingIntentionally) return;
  invoke("tray_menu_hide").catch((err: unknown) => console.error("[tray-menu] blur hide 失败", err));
});
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    invoke("tray_menu_hide").catch((err: unknown) => console.error("[tray-menu] esc hide 失败", err));
  }
});

/* 加载即对表主题/材质；量菜单实际尺寸回告窗口自适配（免硬编码） */
emit("query-ui-style").catch(() => {});
const r = $("tray-menu").getBoundingClientRect();
invoke("tray_menu_fit", { width: Math.ceil(r.width), height: Math.ceil(r.height) }).catch(
  (err: unknown) => console.error("[tray-menu] fit 失败", err),
);
