---
type: solution
title: 用户并行使用期的采集纪律：备份-恢复+哈希比对+idle 门控+config 现读（零 clobber 五条）
date: 2026-08-15
category: workflow-issues
module: test-automation
problem_type: convention
component: test_scripts
severity: medium
applies_when:
  - "app 处于日常真实使用中（配置/计时/统计是真数据），自动化验证与其并行"
  - "本机还有并行 AI 会话可能干预运行中的 app（外部 set_config/重启计时）"
symptoms:
  - "采集验证跑完后用户的真实设置/统计数据被覆盖或污染"
  - "采集开工时 DOM 态与文档记录的「用户配置」不一致（快照过时或被外部改写）"
root_cause: concurrent_user_state
resolution_type: playbook
related_components:
  - "scripts/v11-tray.ps1"
  - "scripts/v13-menus.ps1"
  - "src-tauri/target/debug/config.json"
tags: [capture, user-data, backup-restore, hash-verify, idle-gate, config-snapshot-drift, concurrent-sessions]
---

# 用户并行使用期的采集纪律（v11 成型、v13 演进，五条）

## Context

领导从 2026-08-14 晚起日常真实使用本 app（自启开关、材质、短休时长、自录快捷键、跑番茄、
攒真实统计），而视觉评审/功能验证的采集管线与使用并行。此外本机存在并行 AI 会话，曾观察到
对运行中 app 的外部干预（set_config、重启计时、改 config）。此纪律的目标：**采集验证零
clobber 用户状态，且不被外部干预骗到**。

## Guidance（五条）

1. **改前备份、验后恢复+哈希比对**：脚本凡动 config.json/stats.json（含注入历史数据验翻页
   这类），先备份，验完恢复，恢复后比对哈希与备份一致才算数（v11 全程零 clobber 实证；
   v10 注入统计后按此恢复真实 stats.json）。
2. **idle 门控**：计时器运行期间只跑非破坏性步骤；quit/capture 等破坏性或重操作等 idle
   监控触发后再跑——不打断用户正在跑的番茄。
3. **只翻 DOM 不落盘**：主题/材质切换采集走 `documentElement.dataset` 直改 + ui-style
   真实广播链路，**不经 set_material/set_config 落盘**；结束还原。
4. **还原目标=开工现读 config.json**：不要信 PROGRESS/BLOCKED 里记录的「用户配置」快照
   ——config 会被用户或并行会话改（v13 实证：会话内被外部两改 classic→liquid→classic，
   而文档快照还停在更早的 classic）。开工第一件事 `Get-Content config.json` 记下持久值，
   终态按它对齐（含托盘 ui-style 广播同步）。
5. **光标/hover 卫生**：真输入注入的落点、残留 hover 按坑速查表清理（清 hover 须发真实
   移动消息，详档 transparent-webview2-automation-pitfalls.md §5）。

## Why This Matters

- 一次 clobber 用户真实数据（统计/配置），损失的是对整条自动化管线的信任；这五条在 v11
  领导全程使用期零事故跑通。
- 「文档快照」本身会漂移（CLAUDE.md/BLOCKED 记录的配置过时、implementation-notes 声称
  已维护实为不存在）——纪律第 4 条把「信快照」改成「信现读」，是同类问题的通用解。

## When to Apply

每次采集/验证脚本开工时过一遍五条；新写采集管线把这五条写进脚本头注释（v13-menus.ps1
即按此格式）。

## 证据

v11：DEV_LOG「领导并行亲验纪律成型」条（备份-恢复+stats 哈希比对零 clobber）；v13：
三次采集开工态互异+config 两改的处置记录（PROGRESS v13 翻车记 ④、BLOCKED #12）。
