---
type: solution
title: 活体计时应用的采集相位竞速：IIFE 缺 async 全灭 + 自然翻相位串相，须相位归一+断言硬化
date: 2026-08-15
category: workflow-issues
module: test-automation
problem_type: tooling_pitfall
component: test_scripts
severity: medium
symptoms:
  - "进度帧采集首跑全部步骤 FAIL/空返回（IIFE 未声明 async 却内含 await）"
  - "同一状态的前后两张截图进度条位置不一致（接管间隙计时器自然推进，两帧不同相位）"
  - "采集「工作中段」帧拿到的 done% 与采集时快照对不上"
root_cause: async_missing_and_phase_race
resolution_type: playbook
related_components:
  - "scripts/v11-progress.ps1"
  - "scripts/cdp-seq.mjs"
tags: [capture, phase-race, async-iife, cdp, timer, screenshot, assertion]
---

# 活体计时应用的采集相位竞速

## Problem

番茄钟是**运行中持续变化的 UI**（计时数字、进度条每 tick 前进）。对这类「活体状态」采集
验收帧时有两个坑：脚本的 async 语义坑、采集时机与自然计时的竞速坑。v11 进度条三态采集
首跑同时踩中两个，全灭。

## Root Cause 与解法

### 1. cdp-seq eval 的 IIFE 需要 async（当体内有 await 时）
「CDP eval 一律 IIFE」是既有纪律（R27 顶层 const 重声明事故），但 IIFE 里要
`await window.__TAURI__.core.invoke(...)`（如设置计时相位）时，**必须写 async IIFE**
`(async()=>{...})()`——同步 IIFE 返回 Promise 对象，cdp-seq 拿到的是序列化的空壳，
后续步骤连锁全灭。v12-menus/v13-menus 的 Tray-Eval 即按此写法（async IIFE 为默认）。

### 2. 相位竞速：接管瞬间计时器自然翻相位
目标「工作中段 36.7%」帧，从脚本接管到截图的间隙里计时器自然推进/翻段——前后帧
相位不同、与快照数值对不上，两态串相。**修法（v11-progress.ps1 定稿）**：
- **相位归一**：采集前把计时器设置到**确定性的已知相位**（如 set 后固定等待到
  目标段中点），不要在任意相位上「碰巧截」；
- **断言硬化**：每张时敏帧采集后**断言**帧内数值与 snapshot 一致（bar 36.7% ↔
  snapshot done 36.7%），不匹配判 FAIL 而非静默出图——v11 实测暂停两次采样 20.1%
  冻结即用此断言守住。

## Prevention

- 任何与时间相关的 UI 帧采集（计时数字/进度条/倒计时态）：先冻结（暂停）或归一相位，
  禁止「运行中随手截」；
- 时敏帧采集脚本内置「帧值 ↔ snapshot 值」逐 tick 断言，把竞速从「肉眼发现串相」
  变成「脚本当场 FAIL」；
- cdp-seq eval 模板统一 async IIFE（同步场景无副作用，异步场景不再翻车）。

## 证据

scripts/v11-progress.ps1（相位归一+断言硬化后三态全 PASS，bar↔snapshot 逐 tick 一致：
36.7%↔36.7%、26.6%↔26.6%、暂停 20.1% 两次采样冻结）；翻车过程见 PROGRESS v11 终验
「翻车/让步如实记」第 ① 条。
