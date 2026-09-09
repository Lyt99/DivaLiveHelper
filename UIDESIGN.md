# UIDESIGN.md — DIVA 直播助手 UI 设计规范

本文件固化项目的 UI 设计语言和组件规范。所有前端改动应遵循本文档。`AGENTS.md` 引用本文件作为 UI 实现的权威参考。

设计语言：**直播控制台**。像专业音频/直播工具，不像营销页面——紧凑、扁平、功能优先，用等宽数字和发丝分割线建立秩序。

---

## 1. 设计原则

| 原则 | 说明 |
|---|---|
| **控制台气质** | 页面是工具不是海报。禁止 hero 营销区、eyebrow 英文小标题、品牌色块卡片、侧栏管理后台布局。 |
| **工具外壳** | 一体化顶栏（品牌 + 标签导航 + 窗口控制）+ 底部常驻状态栏（信号灯 + 状态回显）。页面不放重复大标题，头部是功能工具栏 `.toolbar`。 |
| **扁平优先** | 不用渐变、投影 elevation。深度 = 1px 发丝线 + 表面色阶（`--bg` → `--raised`）。 |
| **无卡片** | 页面布局不用卡片面板（`.panel` 已移除）。分组靠发丝线 + 留白 + 标题层级；整页撑高、区块内部滚动。 |
| **克制用色** | Miku cyan 只做信号色：主按钮、选中态、队首、强调数字。pink 只给破坏性/点歌弹幕高亮。 |
| **等宽数字** | 编号、ID、计数、日志、星级一律 `var(--mono)` + `tabular-nums`。 |
| **日文用 JP 字形** | 歌名/作者等日文内容走 `--font-jp`，禁止用中文字形渲染日文。 |
| **桌面优先** | body `min-width: 860px`；唯一断点 `1000px`。 |
| **中文界面** | 用户可见文字均为简体中文；不用 emoji，图标用内联 SVG。 |

---

## 2. 色彩系统

### 2.1 品牌色

| 变量 | 深色值 | 浅色值 | 用途 |
|---|---|---|---|
| `--miku` | `#39C5BB` | `#12a298` | 信号色：主按钮、导航选中条、队首、统计数字、焦点 |
| `--miku-ink` | `#06302c` | `#05302c` | 主按钮文字（深青，不用白字） |
| `--miku-dim` | `rgba(57,197,187,.10)` | `rgba(18,162,152,.09)` | 队首背景、焦点环 |
| `--miku-mid` | `rgba(57,197,187,.34)` | `rgba(18,162,152,.30)` | hover/focus 边框、选区 |
| `--pink` | `#E12885` | `#c22274` | 破坏性 hover、点歌弹幕、失败提示 |
| `--pink-dim` / `--pink-mid` | 10% / 34% | 7% / 26% | 上述场景的背景与边框 |

### 2.2 表面色阶（6 级）

| 变量 | 深色值 | 浅色值 | 用途 |
|---|---|---|---|
| `--bg` | `#101315` | `#efede6` | 窗口底层（深色为冷炭黑，浅色系暖纸白） |
| `--bg-2` | `#15181c` | `#e7e5dc` | 标题栏、侧栏 |
| `--panel` | `#1a1e23` | `#faf9f5` | 面板 |
| `--panel-2` | `#21262c` | `#f0eee6` | 行 hover、输入框底色 |
| `--inset` | `#0c0f11` | `#e6e4db` | 凹槽：终端回显行、日志面板、分段控件底 |
| `--raised` | `#282e36` | `#ffffff` | 凸起：分段选中、toast、滚动条 |

### 2.3 文字与结构

| 变量 | 深色值 | 浅色值 | 用途 |
|---|---|---|---|
| `--text` | `#e4e8eb` | `#23262b` | 主文字 |
| `--muted` | `#8b929b` | `#6f757c` | 次要文字、标签 |
| `--faint` | `#5f666e` | `#a0a5aa` | 占位、序号、时间戳 |
| `--line` | 白 7% | 黑 10% | 发丝分隔线 |
| `--line-strong` | 白 15% | 黑 20% | 强调边框、虚线 |
| `--good` | `#3ec47c` | `#178a4c` | 在线/成功 |
| `--bad` | `#f0616d` | `#cf3a4a` | 离线/关闭 hover |
| `--warn` | `#d9a441` | `#9a7414` | 连接中（闪烁） |

### 2.4 难度色（DIVA 游戏档位）

仅用于 `.difficulty-jump` 芯片，经 `--difficulty-color` 注入：

| 档位 | 类名 | 色值 |
|---|---|---|
| 简单 | `.difficulty-easy` | `#009CC2` |
| 普通 | `.difficulty-normal` | `#1AB406` |
| 困难 | `.difficulty-hard` | `#D99800` |
| 极限 | `.difficulty-extreme` | `#DA011E` |
| EX极限 | `.difficulty-exextreme` | `#A90EEA` |

ok/bad 的透明变体一律用 `color-mix(in srgb, var(--good) 35%, transparent)` 形式，不写死 rgba。

---

## 3. 主题机制

- 切换属性：`<html data-theme="light">`；默认深色；持久化 `localStorage.theme`。
- 切换组件 `src/components/ThemeToggle.tsx`：状态栏右侧的 `.icon-button`（太阳/月亮 SVG 图标），不是文字按钮。
- 无系统偏好检测，首启始终深色。

---

## 4. 字体排印

### 4.1 字体族（三组）

| 变量 | 字栈 | 用途 |
|---|---|---|
| `--font-ui` | Segoe UI Variable → Segoe UI → **Microsoft YaHei UI** → Yu Gothic UI → system-ui | 全局 UI（中文优先） |
| `--font-jp` | **Yu Gothic UI** → Yu Gothic → Meiryo UI → Hiragana Sans → Noto Sans CJK JP → Microsoft YaHei UI | 日文内容：`.track-main strong`、`.song-row strong`、`.song-sub`、`.song-author`、`.debug-result strong`、悬浮窗 `.ov-name` |
| `--mono` | Cascadia Mono → Consolas → Courier New → Microsoft YaHei UI | 编号/ID/计数/日志/星级/回显行；CJK 回退雅黑 |

规则：**日文汉字必须用 JP 字形渲染**（桜・恋・戦 等字形与中文字形不同），新增显示歌名的组件时必须套用 `--font-jp`。

### 4.2 字号阶（px）

| px | 用途 |
|---|---|
| 10 | 导航序号、面板编号、表头、版本号、芯片标签 |
| 11 | 副标题、统计标签、track meta、歌曲 ID |
| 12 | 信号灯、辅助说明、日志、回显行、芯片星级、小号按钮 |
| 13 | 全局基准：按钮、输入框、正文、导航项 |
| 14 | 歌名、h2 以外的强调 |
| 15 | h2 |
| 18 | 控制台标题、向导步骤标题 |
| 20 | 页面 h1、统计数字 |
| 30 | 向导欢迎标题 |

### 4.3 字重

400 正文 / 500 导航 / 600 按钮与标题 / 650 h2 / 700 h1 与主按钮与统计数字。

---

## 5. 间距与关键尺寸

| 元素 | 值 |
|---|---|
| 顶栏 `.topbar` | `44px` |
| 状态栏 `.statusbar` | `26px` |
| 内容区 padding | `22px 26px` |
| 页面 gap | `16px` |
| 按钮高 | `30px`（`.button-sm` `25px`） |
| 输入框高 | `32px` |
| 网格 | `.workspace`（点歌台分栏）= `1.4fr / .6fr`；`.settings-grid` = 两列（`gap: 26px 32px`）；`.song-row` = ID / 曲名 / 作者 / MOD 来源 / 难度五列 |
| 整页撑高 | `.queue-page` / `.library-page` / `.logs-page`：`height: 100%` + flex 列，滚动区 `flex: 1; min-height: 0` |

---

## 6. 圆角与阴影

| 圆角 | 用途 |
|---|---|
| `4-5px` | 难度芯片、日志行、代码片 |
| `6px` | **基准**：按钮、输入框、导航项、track 行、信号灯相关 |
| `8px` | 面板、分段控件、toast |
| `10px/50%` | 开关、状态圆点 |

阴影只有两处：焦点环 `0 0 0 2px var(--miku-dim)` 和在线信号点 `0 0 5px var(--good)`。选中/队首的左侧色条一律用 `box-shadow: inset 2px 0 0 <色>`，不影响布局。

---

## 7. 动效

- 全局过渡 `120ms ease`（background / border-color / color / filter）。
- 入场 `@keyframes rise-in`（160ms，`translateY(-5px)` → 0）：toast、失败提示。
- 连接中信号 `@keyframes signal-blink`（900ms 透明度呼吸）。
- 悬浮窗 `ov-fade-in`（280ms，按行错开 30ms）。
- 禁止位移 hover、弹性缓动。

---

## 8. 组件规范

### 8.1 按钮族

基础：`height: 30px; padding: 0 14px; border-radius: 6px; font-size: 13px; font-weight: 600`。小号加 `.button-sm`（25px / 12px）。

| 类名 | 样式 | hover |
|---|---|---|
| `.primary-button` | `--miku` 底 + `--miku-ink` 深青字（700） | `brightness(1.1)` |
| `.secondary-button` | `--panel-2` 底 + 发丝边框 | 边框/文字 → miku |
| `.ghost-button` | 透明 + 发丝边框 + muted 字 | 边框/文字/背景 → pink 系 |
| `.icon-button` | 28px 方形图标按钮 | `--panel-2` 底 |

行内次要操作（移除等）用 `.ghost-button.button-sm`，默认 `opacity: 0`，行 hover / focus-visible 才显示。

### 8.2 分区（无卡片）

页面内容的分组不使用带边框/底色的盒子，统一用「mono 编号 + 标题发丝线 + 留白」：

```html
<div class="config-panel">
  <h2><span class="panel-index">01</span>基础设置</h2>
  ...字段...
</div>
```

`.config-panel > h2` 下方是 1px 发丝线；`.panel-index` 是 mono 10px faint 编号，`.panel-count` 是 mono 11px 计数。数据区（重建报告 `.report-panel`）用顶部发丝线与上文分隔。

### 8.3 信号灯 `.signal`

连接状态统一用信号灯，不用药丸：

```html
<span class="signal ok"><i />直播间 <span class="mono">#21452505</span></span>
```

6px 圆点：`.ok` 绿光 / `.bad` 红 / `.busy` 琥珀色闪烁；默认灰。

### 8.4 窗口外壳

```
┌ .topbar ───────────────────────────────────────────┐
│ ♪ 品牌 · 01–04 标签导航（底部 2px miku 指示条）· 窗口控制 │  44px，可拖拽
├ .content ──────────────────────────────────────────┤
│ 页面（.toolbar 功能性头部 + 面板）                       │
├ .statusbar ────────────────────────────────────────┤
│ ● 直播间 ● 游戏        › 状态回显 · vX.Y.Z · 主题开关 │  26px，常驻
└─────────────────────────────────────────────────────┘
```

- 顶栏导航 `.topbar-tab`：mono 序号 `.tab-index` + 标签；选中 = 文字转 `--text` + 底部 2px `--miku` 边。
- 状态栏左侧是 `.signal` 信号灯，右侧 `.statusbar-echo`（mono，`›` 前缀伪元素）显示最近一条事件消息。
- 全局状态由 `src/lib/status.ts` 管理：`reportStatus()` 发消息，`setConnectionState()` 同步信号灯，`refreshDanmakuState()` 在弹幕操作后刷新状态。游戏进程由主窗口统一每 2 秒检测，页面只订阅共享结果，不提供手动连接按钮；禁止再建页面级 message bar。

### 8.5 页面工具栏 `.toolbar`

每页的功能性头部（搜索、连接操作、保存按钮），无标题文字。右侧动作用 `.toolbar-actions`（`margin-left: auto`），右侧元信息用 `.toolbar-meta`。连接操作组：

```html
<div class="console-group">
  <span class="console-key">直播间</span>
  <span class="console-value">21452505</span>   <!-- mono 值芯片 -->
  <button class="secondary-button button-sm">连接</button>
</div>
<i class="console-sep" />
```

### 8.6 点歌台工作区 `.workspace`

队列页**不用面板卡片**。整页撑满内容区高度，左右两栏（队列 / 弹幕）用一根发丝竖线分隔，栏内各自独立滚动：

- `.queue-side`：右边框发丝线 + `padding-right: 22px`；`.feed-side`：`padding-left: 22px`。
- 栏头 `.side-head`：13px 小标题 + 右侧 `.panel-count` 计数，下方发丝线。
- 滚动区 `.queue-scroll` / `.feed`：`flex: 1; min-height: 0; overflow-y: auto`。
- ≤1000px 折为单列，取消内部滚动。

### 8.7 NEXT 焦点区

队首不放在 tracklist 里，单独用 `.next-up` 展示并**固定在队列滚动区上方**。纯排版聚焦，禁止底色卡片：mono NEXT 标签（前置 6px miku 方块）+ JP 20px 标题 + mono meta，底部发丝线分隔。

### 8.8 队列 tracklist

队首进 `.next-up`（见 8.7），其余项在 tracklist 中从 `02` 开始编号。**发丝线分隔行，不用行底色卡片**：

```html
<ol class="tracklist">
  <li class="track">
    <span class="track-index">02</span>
    <div class="track-main">
      <strong>千本桜</strong>
      <span class="track-meta">#001 · 点歌人 · 8.0★</span>
    </div>
    <button class="ghost-button button-sm track-remove">移除</button>
  </li>
</ol>
```

- 序号 mono 右对齐；meta 行 mono 11px；移除按钮 hover 行才显示。

### 8.9 弹幕流 `.feed`

分割线列表（非卡片）：用户名 miku 12px 内联 + 内容 13px 内联。点歌弹幕 `.request`：pink-dim 底 + inset 2px 粉丝信号条。

### 8.10 难度芯片

结构同旧版（`.difficulty-jump` 双行：标签 + 星 SVG + mono 数值），尺寸收紧：`min-width: 58px; padding: 3px 7px; border-radius: 5px`；标签 10px / 星级 12px。样式为色晕芯片：档位色 13% 透明底 + 38% 发丝线描边 + 档位色文字（暗色主题经 `color-mix` 提亮 28% 白，浅色主题用原色，见 `--difficulty-ink`）；hover 实心档位色 + 白字。

### 8.11 表单

- 输入框：`--panel-2` 底，focus 时 miku 边框 + 2px miku-dim 环。
- 开关 `.switch`：34×20 药丸，替代原生 checkbox。checked 时 miku 底白点。所有布尔设置必须用它。
- 分段控件：`--inset` 凹槽底 + 2px padding；`.active` 用 `--raised` 凸起 + inset 发丝描边，不用 miku 填充。
- 辅助说明统一 `.hint`（12px muted 纯文字），禁止左边框高亮提示框。

### 8.12 统计行 `.stat-row`

一个无盒子的统计条：上下发丝线 + `.stat` 之间竖线分隔，mono 20px miku 数字 + 11px muted 标签。禁止每个数字一个盒子。

### 8.13 Toast 与失败提示

- `.toast-bubble`：右上，`--raised` 底 + 左侧 2px miku 条，`rise-in` 入场，2.2s 消失。
- `.failure-toast`：pink-dim 底 + inset 2px 粉条，行内堆叠最多 3 条，4s 消失。

### 8.14 空状态

虚线框（`--line-strong`）+ faint 12px 文字，无背景色。`.small` 72px 高。

---

## 9. 页面规范

### 9.1 页面头部 = 工具栏

导航标签已标明当前页面，**不再放 h1 大标题**。每页头部是功能工具栏 `.toolbar`，内容由页面功能决定：

| 页面 | 工具栏内容 |
|---|---|
| 点歌台 | 直播间连接操作 / 游戏进程自动检测信号灯 `.console-group` + 右侧「打开悬浮窗」「切下一首」 |
| 歌曲库 | 搜索框 + 来源筛选下拉框 `.source-select` + 右侧 `.toolbar-meta` 计数 |
| 设置 | `.hint` 自动保存说明 + 右侧保存状态信号灯；输入框失焦、开关或单选改变即保存，不提供手动保存按钮 |
| 日志 | 状态 `.console-group` ×2：游戏进程信号灯及自动检测说明，弹幕信号灯及连接操作 |
| 关于 | 无工具栏：身份区（`.about-name` 字标）+ 三个 `.about-section` 分区 |

### 9.2 曲库表格

`page library-page`（整页撑高）> `.song-table-wrap`（内部滚动）> `.song-table` > `.song-row`。**不使用** `<table>`，也没有外框卡片。首行 `.song-row.song-head`：mono 10px faint 标签，吸顶 `sticky; top: 0`，背景直接用页面底色 `--bg`。数据行发丝线分隔：`.song-id`（mono faint）/ `strong` + `.song-sub`（JP 字体）/ `.song-author` / `.song-source` / 难度芯片组。来源列用 12px muted，长名称省略并通过 `title` 展示全文；窄窗口沿用两列堆叠。

### 9.3 日志

`page logs-page`（整页撑高）：工具栏放两组 `.console-group`（信号灯 + 名称 + 操作按钮），下方 `.log-stream` 是发丝线隔开的 mono 12px 文本流，内部滚动；`LogLine` 把时间戳拆成 `.log-time`（faint）弱化。

### 9.4 向导

进度 = mono 分数标签（`02 / 07`）+ 2px 轨道条 `.wizard-progress-track/.wizard-progress-fill`，不用圆点。欢迎步：`.wizard-kicker`（mono miku）+ 30px h1 + `.wizard-rule`（36×3 miku 短横线）。按钮一律 `.primary-button` / `.secondary-button` / `.ghost-button`。

目录步骤进入后自动发现 Steam 游戏及现有 `mods/`，用 `.hint` 显示查找状态、结果和手动回退说明，游戏路径允许换行。已有路径及手动修改优先，离开步骤后忽略迟到结果；初始配置读取完成前禁用“开始配置”，避免覆盖用户输入。

---

## 10. 叠加层规范（悬浮窗 + OBS）

两处叠加层共享同一套设计：悬浮窗（`/overlay`，独立 Tauri 窗口，内联 `<style>` + `ov-` 前缀，不加载 `index.css`）与 OBS 浏览器源（`obs_overlay.rs::render_overlay` 内嵌页面，轮询 `/api/queue`）。改动时必须两边同步。

### 10.1 行结构

每行三列：`序号/NEXT + 曲名 + 星级`

- **序号**：mono、`tabular-nums`、右对齐、35% 白；队首用 `NEXT` 标签（miku 色、加粗、letter-spacing）。
- **曲名**：JP 字栈（Yu Gothic 优先），悬浮窗 15px / OBS 19px，600 字重，超长省略。
- **星级**：mono、700、`toFixed(1)` + `★`，**按 `difficulty_tier` 着色**。叠加在半透明黑 + 游戏画面上，使用提亮版档位色：`easy #25b8e6` / `normal #38d21f` / `hard #f0b41d` / `extreme #ff3b57` / `exextreme #c55aff`。tier 值必须白名单校验后再拼进 class。
- 队首行：`rgba(57,197,187,.13)` 底 + inset 2px miku 左条。

### 10.2 容器

`rgba(0,0,0,.55)` + `backdrop-filter: blur(8px)`，圆角 5px（悬浮窗）/ 10px（OBS），标题栏计数数字用 miku 色。失败提示用 pink `rgba(225,40,133,.16/.4)`，关闭按钮 hover 用 pink。

---

## 11. 响应式

唯一断点 `@media (max-width: 1000px)`：

- `.grid-two` / `.settings-grid` / `.status-grid` 折为单列
- 顶栏标签收紧（隐藏 `.tab-index` 序号）
- `.statusbar-echo` 最大宽度收窄到 260px
- 曲库行折为两列，`.song-head` 隐藏

---

## 12. 可访问性约定

| 元素 | 约定 |
|---|---|
| 弹幕列表 / 回显行 | `aria-live="polite"` |
| 失败提示 | `role="alert"` + `aria-live="assertive"` |
| toast | `role="status"` |
| 窗口控制 / 主题切换 | `aria-label` + `title` |
| 装饰 SVG | `aria-hidden="true"` |
| 曲库表头 | `aria-hidden="true"`（仅视觉对齐用） |
| 按钮 focus | `:focus-visible` 2px `--miku-mid` outline |
