; ============================================================================
; 番茄钟 NSIS 安装器钩子
; ----------------------------------------------------------------------------
; 目的：修掉异机安装时的两类失败（实证见 docs/solutions/ 对应条目）：
;   1) "Error opening file for writing: ...\pomodoro-clock.exe"
;      -> 旧版进程仍在运行，exe 被独占锁；NSIS 的 File 命令写主程序时失败。
;   2) "Error creating: ...\uninstall.exe" + "Error creating shortcut: ..."
;      -> 目标位置存在旧版残留且不可覆盖。NSIS 的 File 命令先删后写所以主程序
;         没问题，但 WriteUninstaller / CreateShortcut 不覆盖已有目标，于是静默失败。
;
; 策略（NSIS_HOOK_PREINSTALL，装前）：
;   A. 检测旧版进程是否在跑，在跑就用中文提示并结束它（早于官方宏，
;      避免用户看到官方宏的英文提示）。
;   B. 主动删除安装目录下的旧 uninstall.exe 与开始菜单/桌面旧快捷方式，
;      让后续 WriteUninstaller / CreateShortcut 有干净的写入目标。
;   C. 若旧文件删不掉（仍被占用），用中文 MessageBox 说明原因，让用户决定继续还是中止。
;
; 实现约束（踩过的坑，勿改）：
;   [1] 本文件被 tauri 在 installer.nsi 第 31 行 include，而 additional 插件目录
;       (!addplugindir) 在第 92 行才声明。=> 凡是「在 include 期就要编译」的
;       代码都不能直接调 nsis_tauri_utils 插件，否则报 "Plugin not found"。
;       故本文件内所有插件调用必须放在 **宏体** 里（宏体在 !insertmacro 处
;       才展开，那时插件目录已就绪）。
;       注意：Function 体是在 include 期编译的，同样不能用插件 —— 所以清理逻辑
;       必须写成宏，不能写成 Function。
;   [2] 宏参数名不能叫 $path / $label（与 NSIS 内置 ${FileExists} 的参数名冲突）。
;   [3] 不能用 ${__LINE__} 生成跳转标签：嵌套宏展开后会出现带小数点的非法标签名。
;   [4] 同一宏被调用多次时，宏体内的标签必须唯一，否则报 "label already declared"。
;       故每个调用点用不同的 !define 标签名前缀。
;   [5] hooks.nsh 编码必须是 UTF-8 with BOM（NSIS Unicode 模式下无 BOM 会按 ANSI
;       解析，中文变乱码并直接编译失败）。
; ============================================================================

; Var 声明放在顶层（include 期编译）：Var 本身不涉及插件调用，安全。
Var POMO_FILE
Var POMO_LABEL
Var POMO_EXE

!macro PomoCleanStale label_ok label_cancel
  ; 入参：$POMO_FILE=路径，$POMO_LABEL=中文描述（由调用方 StrCpy 赋值）
  ${If} ${FileExists} "$POMO_FILE"
    ; 只读位先清掉：NSIS 的 Delete 不会自动处理只读文件，
    ; 而 CreateShortcut 遇到已存在的只读目标会直接失败。
    System::Call 'kernel32::SetFileAttributesW(w "$POMO_FILE" i 0x80)'

    Delete "$POMO_FILE"
    ${IfNot} ${FileExists} "$POMO_FILE"
      DetailPrint "已清理旧版残留：$POMO_LABEL"
      Goto ${label_ok}
    ${EndIf}

    ; 还在 —— 试试能不能打开（打不开 = 被独占锁）
    FileOpen $0 "$POMO_FILE" r
    ${If} $0 == ""
      ; 被独占。尝试结束番茄钟进程后重试一次。
      nsis_tauri_utils::KillProcessCurrentUser "$POMO_EXE"
      Pop $1
      Sleep 800
      FileOpen $0 "$POMO_FILE" r
    ${EndIf}

    ${If} $0 == ""
      ; 仍然被独占：给中文提示，让用户决定继续还是中止
      ${IfNot} ${Silent}
        MessageBox MB_OKCANCEL|MB_ICONEXCLAMATION \
          "无法清理旧版残留文件：$\n$POMO_LABEL$\n$\n该文件正被其他程序占用（通常是番茄钟仍在后台运行，或正被安全软件锁定）。$\n$\n请先完全退出番茄钟（托盘图标 → 右键 → 退出），并确认安全软件没有正在扫描，然后重新运行本安装程序。$\n$\n是否跳过该文件继续安装？" \
          /SD IDOK IDOK ${label_ok} IDCANCEL ${label_cancel}
      ${EndIf}
      DetailPrint "旧版残留 $POMO_LABEL 被占用，无法清理，继续安装"
      Goto ${label_ok}
    ${EndIf}

    ; 能打开但删不掉 -> 权限不足
    FileClose $0
    DetailPrint "警告：旧版残留 $POMO_LABEL 无法删除（权限不足），将尝试直接覆盖"
  ${EndIf}
  Goto ${label_ok}
!macroend

!macro NSIS_HOOK_PREINSTALL
  DetailPrint "番茄钟安装器：正在检查旧版状态……"

  ; 0) 记下主程序名（供下方宏运行时使用）
  StrCpy $POMO_EXE "${MAINBINARYNAME}.exe"

  ; 1) 旧版进程是否在跑？在跑就用中文提示并结束它。
  ;    放在最前面是为了让用户看到中文提示，而不是官方宏的英文提示；
  ;    结束后官方 CheckIfAppIsRunning 宏将找不到进程，自然跳过。
  nsis_tauri_utils::FindProcessCurrentUser "$POMO_EXE"
  Pop $0
  ${If} $0 == 0
    ${IfNot} ${Silent}
      MessageBox MB_OKCANCEL|MB_ICONEXCLAMATION \
        "检测到番茄钟正在运行。$\n$\n继续安装需要先结束它（否则安装程序无法覆盖正在运行的程序文件）。$\n$\n是否结束番茄钟并继续安装？" \
        /SD IDOK IDOK PomoProcOk IDCANCEL PomoProcCancel
      PomoProcOk:
        DetailPrint "用户同意结束正在运行的番茄钟"
        Goto PomoProcKill
      PomoProcCancel:
        DetailPrint "用户取消安装（番茄钟仍在运行）"
        Abort
    ${Else}
      DetailPrint "静默安装：检测到番茄钟正在运行，将结束它"
      PomoProcKill:
    ${EndIf}
    nsis_tauri_utils::KillProcessCurrentUser "$POMO_EXE"
    Pop $1
    Sleep 800
  ${EndIf}

  ; 2) 清理旧卸载器 —— WriteUninstaller 不先删后写，残留会直接导致这一步失败
  StrCpy $POMO_FILE "$INSTDIR\uninstall.exe"
  StrCpy $POMO_LABEL "旧卸载器 uninstall.exe"
  !insertmacro PomoCleanStale PomoUnstOk PomoUnstCancel
  PomoUnstOk:
  PomoUnstCancel:

  ; 3) 清理开始菜单旧快捷方式 —— CreateShortcut 同样不覆盖已有目标
  StrCpy $POMO_FILE "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  StrCpy $POMO_LABEL "开始菜单快捷方式"
  !insertmacro PomoCleanStale PomoSmOk PomoSmCancel
  PomoSmOk:
  PomoSmCancel:

  ; 4) 清理桌面旧快捷方式
  StrCpy $POMO_FILE "$DESKTOP\${PRODUCTNAME}.lnk"
  StrCpy $POMO_LABEL "桌面快捷方式"
  !insertmacro PomoCleanStale PomoDkOk PomoDkCancel
  PomoDkOk:
  PomoDkCancel:

  DetailPrint "番茄钟安装器：旧版状态检查完成。"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; 装后自检：主程序必须存在且非空。
  ; 依据项目既有教训（docs/solutions/workflow-issues/disk-full-nsis-temp-false-positive.md）：
  ; 0 字节 exe 会让后续所有资源级验证产生假阳性，必须以文件长度作为前置闸门。
  ${If} ${FileExists} "$INSTDIR\${MAINBINARYNAME}.exe"
    FileOpen $0 "$INSTDIR\${MAINBINARYNAME}.exe" r
    ${If} $0 != ""
      FileSeek $0 0 END
      Pop $1
      FileClose $0
      ${If} $1 < 1
        ${IfNot} ${Silent}
          MessageBox MB_OK|MB_ICONEXCLAMATION \
            "安装未成功：主程序 pomodoro-clock.exe 为 0 字节。$\n$\n常见原因是系统盘（临时目录所在盘）空间不足，或被安全软件拦截。$\n请清理磁盘空间或临时关闭安全软件后重新安装。"
        ${EndIf}
        DetailPrint "安装校验失败：主程序为 0 字节"
      ${Else}
        DetailPrint "安装校验通过：主程序大小 $1 字节"
      ${EndIf}
    ${Else}
      ${IfNot} ${Silent}
        MessageBox MB_OK|MB_ICONEXCLAMATION \
          "安装未成功：无法读取主程序 pomodoro-clock.exe。$\n$\n请检查安全软件是否拦截了本次安装，然后重新安装。"
      ${EndIf}
      DetailPrint "安装校验失败：主程序不可读"
    ${EndIf}
  ${Else}
    ${IfNot} ${Silent}
      MessageBox MB_OK|MB_ICONEXCLAMATION \
        "安装未成功：未找到主程序 pomodoro-clock.exe。$\n$\n请检查安全软件是否隔离了本次安装，然后重新安装。"
    ${EndIf}
    DetailPrint "安装校验失败：主程序缺失"
  ${EndIf}
!macroend
