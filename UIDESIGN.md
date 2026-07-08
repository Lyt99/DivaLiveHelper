# UIDESIGN.md — DIVA 直播助手 UI 设计规范

本文件固化项目的 UI 设计语言和组件规范。所有前端改动应遵循本文档。`AGENTS.md` 引用本文件作为 UI 实现的权威参考。

---

## 1. 设计原则

| 原则 | 说明 |
|---|---|
| **扁平优先** | 不使用渐变色、投影 elevation、斜面或光泽。深度通过 1px 边框 + 4 级表面色阶传达。 |
| **克制用色** | Miku cyan 是唯一主操作色；pink 保留给破坏性/次要 hover。难度色仅用于难度芯片。 |
| **单一过渡** | 全局 120ms `ease`；入场动画 200ms。不使用 cubic-bezier 或弹性动画。 |
| **桌面优先** | body `min-width: 860px`；唯一断点 `1000px`。不做移动端适配。 |
| **中文界面** | 所有用户可见文字使用简体中文；代码注释同样使用中文。 |
| **无 emoji** | 按钮和标签中不使用 emoji 或图标字体；图标用内联 SVG（`stroke-width: 1.5`）。 |

---

## 2. 色彩系统

### 2.1 品牌色

| 变量 | 深色值 | 浅色值 | 用途 |
|---|---|---|---|
| `--miku` | `#39C5BB` | `#2da8a0` | 主操作色：主按钮、焦点环、导航高亮、数字强调 |
| `--miku-dim` | `rgba(57,197,187,.12)` | `rgba(45,168,160,.08)` | hover 背景、品牌区、队列首位 |
| `--miku-mid` | `rgba(57,197,187,.28)` | `rgba(45,168,160,.18)` | hover/focus 边框、滚动条 |
| `--pink` | `#E12885` | `#c4226e` | 次要/破坏性：ghost 按钮 hover、失败提示 |
| `--pink-dim` | `rgba(225,40,133,.10)` | `rgba(196,34,110,.06)` | ghost hover 背景、失败提示背景 |
| `--pink-mid` | `rgba(225,40,133,.24)` | `rgba(196,34,110,.16)` | ghost hover 边框 |

### 2.2 表面色阶（4 级）

| 变量 | 深色值 | 浅色值 | 用途 |
|---|---|---|---|
| `--bg` | `#0d1117` | `#f6f8fa` | body 最底层 |
| `--bg-2` | `#161b22` | `#ffffff` | 标题栏、侧边栏 |
| `--panel` | `#1c2128` | `#ffffff` | 面板卡片 |
| `--panel-solid` | `#21262d` | `#ffffff` | 嵌套表面（输入框、卡片内行） |

### 2.3 文字与结构

| 变量 | 深色值 | 浅色值 | 用途 |
|---|---|---|---|
| `--text` | `#e6edf3` | `#1f2328` | 主文字 |
| `--muted` | `#7d8590` | `#656d76` | 标签、说明、占位 |
| `--line` | `rgba(255,255,255,.08)` | `rgba(31,35,40,.10)` | 全局 1px 分隔线 |
| `--good` | `#3fb950` | `#1a7f37` | 连接成功、调试通过 |
| `--bad` | `#f85149` | `#cf222e` | 连接失败、关闭按钮 hover |
| `--mono` | `"Cascadia Mono", Consolas, "Courier New", monospace` | 同 | ID、日志、星级数值 |

### 2.4 难度色（DIVA 游戏档位）

仅用于 `.difficulty-jump` 芯片，通过 `--difficulty-color` 局部变量注入：

| 档位 | 类名 | 色值 |
|---|---|---|
| 简单 | `.difficulty-easy` | `#009CC2` |
| 普通 | `.difficulty-normal` | `#1AB406` |
| 困难 | `.difficulty-hard` | `#D99800` |
| 极限 | `.difficulty-extreme` | `#DA011E` |
| EX极限 | `.difficulty-exextreme` | `#A90EEA` |

### 2.5 语义覆盖色（硬编码 rgba）

以下颜色直接写在选择器中，本质是 `--good` / `--bad` / `--pink` 的透明度变体：

| 色值 | 用途 |
|---|---|
| `rgba(63,185,80,.3)` | `.status-pill.ok` / `.debug-result.ok` 边框 |
| `rgba(63,185,80,.08)` | `.debug-result.ok span` 背景 |
| `rgba(248,81,73,.25)` | `.status-pill.bad` / `.debug-result.bad` 边框 |
| `rgba(248,81,73,.06)` | `.debug-result.bad span` 背景 |
| `rgba(225,40,133,.10)` | `.failure-toast` 背景 |
| `rgba(225,40,133,.30)` | `.failure-toast` 边框 |

---

## 3. 主题机制

- **切换属性**：`<html data-theme="light">`（`document.documentElement.dataset.theme`）
- **默认主题**：`dark`（`:root` 即深色）
- **持久化**：`localStorage.theme`，值为 `"dark"` 或 `"light"`
- **切换组件**：`src/components/ThemeToggle.tsx`，位于侧边栏底部
- **无系统偏好检测**：首次运行始终使用深色主题
- **无 FOUC 预防**：初始渲染使用 `:root` 深色默认值，React hydrate 后切换

---

## 4. 字体排印

### 4.1 字体族

| 场景 | 字体族 |
|---|---|
| UI 文字 | `"Microsoft YaHei UI", "Microsoft YaHei", "Segoe UI", system-ui, sans-serif` |
| 等宽（ID/日志/星级） | `"Cascadia Mono", Consolas, "Courier New", monospace`（`var(--mono)`） |

### 4.2 字号阶

全部使用 px，不使用 rem/em。

| px | 代表用途 |
|---|---|
| 10 | 房间号标签 |
| 11 | 品牌副标题、版本号、调试提示、统计标签 |
| 12 | eyebrow、状态药丸、弹幕用户名、日志行、星级、失败提示请求人 |
| 13 | 窗口控制按钮、字段标签、toast、空状态、输入框 |
| 14 | 导航项、歌曲标题、调试结果、失败提示消息 |
| 15 | 品牌标题 |
| 16 | h2、房间号值、向导总计 |
| 20 | 统计数字 |
| 22 | 向导步骤标题 |
| 28 | h1 |
| 32 | 向导欢迎标题 |

### 4.3 字重阶

| 字重 | 用途 |
|---|---|
| 500 | 导航项、分段控件、向导标签 |
| 600 | 标题栏、h2、按钮、字段标签、调试结果 |
| 700 | h1、品牌、主按钮、徽章、统计数字 |
| 800 | 品牌头像、难度芯片文字 |

### 4.4 字间距

| 值 | 用途 |
|---|---|
| `-1px` | 向导大图标 |
| `-.01em` | h1 |
| `0` | h2 |
| `.02em` | 难度芯片标签 |
| `.04em` | eyebrow-meta、调试结果标签 |
| `.08em` | eyebrow、房间号标签（配合 uppercase） |

### 4.5 数字对齐

`font-variant-numeric: tabular-nums` 用于 `.song-index` 和 `.library-stats strong`，防止数字跳动。

---

## 5. 间距与布局

### 5.1 间距

项目不使用间距 token，全部使用 px gap 值：

| 场景 | 典型值 |
|---|---|
| 组件内 gap | `1px` / `2px` / `3px` / `4px` |
| 行内元素 gap | `6px` / `7px` / `8px` / `10px` |
| 卡片/面板内 gap | `12px` / `14px` / `16px` |
| 区块/页面 gap | `18px` / `20px` / `24px` |

### 5.2 关键尺寸

| 元素 | 值 |
|---|---|
| 标题栏高度 | `38px` |
| 侧边栏宽度 | `220px`（≤1000px 时 `180px`） |
| body 最小宽度 | `860px` |
| 内容区内边距 | `24px` |
| 面板内边距 | `20px`（hero 面板 `24px`） |
| 向导最大宽度 | `520px` |
| 悬浮窗最大宽度 | `520px`（`min(520px, calc(100vw - 24px))`） |

### 5.3 网格布局类

| 类名 | 列定义 | 用途 |
|---|---|---|
| `.control-grid` | `repeat(2, minmax(0,1fr))` | 队列页连接卡片 |
| `.grid-two` | `minmax(0,1.35fr) minmax(280px,.65fr)` | 队列 + 弹幕双栏 |
| `.settings-grid` / `.status-grid` | `repeat(2, minmax(0,1fr))` | 设置页 / 日志页 |
| `.library-stats` | `repeat(4, minmax(0,1fr))` | 设置页统计瓦片 |
| `.song-row` | `72px minmax(200px,1fr) 140px minmax(320px,.9fr)` | 曲库表格行 |

---

## 6. 圆角与阴影

### 6.1 圆角阶

| 值 | 用途 |
|---|---|
| `0` | 窗口控制按钮 |
| `3px` | 滚动条 |
| `4px` | 日志行、调试标签、行内代码 |
| `6px` | 状态药丸、输入框、弹幕项、难度芯片 |
| `8px` | **最常用** — 按钮、导航项、歌曲卡片、调试结果、分段控件、toast |
| `10px` | 大面板、品牌区、空状态、向导报告 |
| `14px` | 向导图标（仅此一处） |
| `50%` | 状态点、向导进度点 |

### 6.2 阴影

项目几乎不用 `box-shadow`。仅两处：

| 选择器 | 值 | 用途 |
|---|---|---|
| `input:focus, select:focus` | `0 0 0 2px var(--miku-dim)` | 焦点环 |
| `.live-dot` | `0 0 6px var(--good)` | 直播状态发光点 |

**不使用 elevation 阴影。** 深度通过 `--line`（1px 边框）+ 4 级表面色阶传达。

---

## 7. 动效

### 7.1 过渡

| 时长 | 缓动 | 用途 |
|---|---|---|
| `120ms` | `ease` | 全局标准过渡：background、border-color、color、box-shadow、filter |
| `200ms` | `ease` | 向导进度点（background + transform）、toast 入场 |
| `280ms` | `ease` | 悬浮窗列表行入场 |

不使用 cubic-bezier、弹性或回弹动画。

### 7.2 关键帧

```css
@keyframes toast-in          { from { opacity: 0; transform: translateY(-8px) } to { opacity: 1; transform: translateY(0) } }
@keyframes failure-toast-in  { from { opacity: 0; transform: translateY(-4px) } to { opacity: 1; transform: translateY(0) } }
@keyframes ov-fade-in        { from { opacity: 0 } to { opacity: 1 } }
```

### 7.3 hover 约定

| 元素 | hover 效果 |
|---|---|
| 按钮（primary） | `filter: brightness(1.1)` |
| 难度芯片 | `filter: brightness(1.12)` |
| ghost 按钮 | 边框/文字/背景切到 pink 系 |
| 卡片/行 | 边框切到 `--miku-mid` 或背景切到 `--miku-dim` |
| 导航项 | 文字切 `--text` + 背景 `--miku-dim` |

**hover 不使用位移（translateY）。**

---

## 8. 组件规范

### 8.1 按钮族

所有按钮共享基础规则：

```css
padding: 8px 16px;
border-radius: 8px;
border: 1px solid var(--line);
font-weight: 600;
font-size: 13px;
white-space: nowrap;
transition: background 120ms ease, border-color 120ms ease, color 120ms ease;
```

| 类名 | 背景 | 文字 | hover |
|---|---|---|---|
| `.primary-button` | `var(--miku)` | `#fff` | `brightness(1.1)` |
| `.secondary-button` | `var(--panel-solid)` | `var(--text)` | 边框/文字 → miku |
| `.ghost-button` | 透明 | `var(--muted)` | 边框/文字/背景 → pink 系 |
| `.theme-toggle` | 同 secondary | 同 secondary | 同 secondary |

**禁用态**：`opacity: .4; cursor: not-allowed`（全局 `button:disabled`）。

### 8.2 面板

```css
.panel       { border: 1px solid var(--line); border-radius: 10px; background: var(--panel); padding: 20px; }
.hero-panel  { 同 panel，padding: 24px + flex space-between }
.table-panel { padding: 0; overflow: hidden; }
```

### 8.3 状态药丸

```html
<span class="status-pill ok">在线</span>
```

- `::before` 7px 圆点，颜色 = `currentColor`
- `.ok` → `var(--good)`
- `.bad` → `var(--bad)`

### 8.4 难度芯片

```html
<button class="difficulty-jump difficulty-extreme" onClick={...}>
  <span>极限</span>
  <strong>9.5</strong>
</button>
```

- 纯色背景（`var(--difficulty-color)`），白字
- 边框 `color-mix(70% 难度色, 30% 黑)`
- `border-radius: 6px; padding: 4px 8px`
- hover: `brightness(1.12)`，不位移
- focus-visible: 2px outline（难度色 55% + 白 45%）
- disabled: `cursor: wait`（配合全局 opacity）

### 8.5 输入框

```css
input, select {
  width: 100%;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--panel-solid);
  color: var(--text);
  padding: 8px 12px;
}
input:focus, select:focus {
  border-color: var(--miku);
  box-shadow: 0 0 0 2px var(--miku-dim);
}
input[type='checkbox'] {
  width: 18px; height: 18px;
  accent-color: var(--miku);
}
```

### 8.6 分段控件

```html
<div class="segmented-control">
  <button class="segmented-btn active">极限</button>
  <button class="segmented-btn">困难</button>
</div>
```

- flex 容器，`border-radius: 8px; overflow: hidden`
- `.active`：`background: var(--miku); color: #fff`
- 非 active hover：`background: var(--miku-dim)`

### 8.7 Toast

| 类型 | 类名 | 位置 | 自动消失 |
|---|---|---|---|
| 保存成功 | `.toast-bubble` | `fixed; top: 52px; right: 20px` | 2.2s |
| 点歌失败 | `.failure-toast`（队列）/ `.ov-failure-toast`（悬浮窗） | 行内堆叠，最多 3 条 | 4s |

入场动画 200ms，`translateY(-8px)` → `0`。

### 8.8 空状态

```html
<div class="empty-state">队列是空的</div>
<div class="empty-state small">弹幕连接后会显示在这里</div>
```

`min-height: 160px`（`.small` 为 `80px`），虚线边框，居中 muted 文字。

### 8.9 eyebrow 标签

```html
<p class="eyebrow">Live Stage</p>
```

12px / 700 / uppercase / `letter-spacing: .08em` / `var(--miku)`。

---

## 9. 页面布局规范

### 9.1 标准页面结构

```html
<section class="page">
  <header class="page-header [split]">
    <div class="page-header-main"><h1>...</h1><p class="muted">...</p></div>
    <div class="header-actions">...</div>
  </header>
  <!-- panels -->
</section>
```

### 9.2 面板内标题

```html
<div class="panel">
  <div class="panel-header [compact]">
    <h2>队列</h2>
    <span class="queue-count-badge">3</span>
  </div>
  <!-- content -->
</div>
```

### 9.3 曲库表格

使用 CSS Grid（非 `<table>`）：

```html
<div class="panel table-panel">
  <div class="song-table">
    <div class="song-row">
      <span class="mono">001</span>
      <div><strong>千本桜</strong><span>original · Senbonzakura</span></div>
      <span>ryo</span>
      <div class="library-difficulty-buttons">...</div>
    </div>
  </div>
</div>
```

---

## 10. 悬浮窗规范

悬浮窗（`/overlay`）是独立 Tauri 窗口，**不加载全局 `index.css`**。所有样式通过内联 `<style>` 注入，类名以 `ov-` 前缀隔离。

### 10.1 容器

```css
.ov-overlay {
  width: min(520px, calc(100vw - 24px));
  margin: 12px;
  border-radius: 12px;
  background: rgba(0, 0, 0, .55);
  border: 1px solid rgba(255, 255, 255, .08);
  backdrop-filter: blur(8px);
  box-shadow: 0 8px 28px rgba(0, 0, 0, .35);
}
```

### 10.2 关键差异

| 特性 | 主窗口 | 悬浮窗 |
|---|---|---|
| 样式来源 | `src/index.css` | 内联 `<style>`（`ov-` 前缀） |
| 背景 | `var(--bg)` 不透明 | `rgba(0,0,0,.55)` + `backdrop-filter: blur(8px)` |
| 拖拽 | `-webkit-app-region: drag` | `getCurrentWindow().startDragging()` |
| 窗口控制 | 系统样式 | 22px 无边框药丸 |
| 最小宽度 | `860px` | 无（透明 body override） |

### 10.3 列表行入场

每行使用 `ov-fade-in`（280ms），按索引错开 `animation-delay: ${index * 30}ms`。

---

## 11. 响应式

唯一断点：`@media (max-width: 1000px)`。

| 变化 | 规则 |
|---|---|
| 双栏网格 | `grid-template-columns: 1fr`（单列堆叠） |
| 页面标题 | `flex-direction: column`（标题和操作垂直排列） |
| 侧边栏 | `220px` → `180px` |
| 曲库行 | `72px 1fr 140px .9fr` → `60px 1fr`（后续列折到第二列） |

---

## 12. 可访问性约定

| 元素 | aria 属性 |
|---|---|
| 失败提示容器 | `aria-live="assertive"` |
| 弹幕列表 | `aria-live="polite"` |
| 房间号显示 | `aria-label="直播间房间号 N"` |
| toast 气泡 | `role="status"` |
| 失败提示条 | `role="alert"` |
| 窗口控制按钮 | `aria-label="最小化/最大化/关闭"` |
| 导航图标 | `aria-hidden="true"` |

---

## 13. 已知不一致

| 问题 | 说明 |
|---|---|
| 向导按钮类名 | Wizard 使用 `.btn-primary` 等未定义类，应改为 `.primary-button` 等 |
| `.eyebrow-tag` / `.eyebrow::before` | 已 `display: none`，为废弃/预留 |
| `.live-dot` | 已定义但未使用 |
