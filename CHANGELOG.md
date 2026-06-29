# 更新日志

本项目从 `v0.0.1`（最新基线提交打包版本）之后的改动记录在这里。

## Unreleased

### 新增

- 新增队列悬浮窗：主窗口可打开独立 Tauri overlay 窗口显示点歌队列，支持拖拽、关闭、置顶切换，并通过 `queue-updated` 事件与轮询保持同步。
- 新增点歌失败提示：主点歌页与队列悬浮窗都会显示最近失败的点歌请求；LLM 判断“不是点歌意图”的弹幕不会计为失败。
- 新增弹幕表情清理：点歌搜索前会剥离 B站弹幕中的 `[喝彩]`、`[doge]`、`[2233娘]` 等方括号表情，避免干扰歌名或作者搜索。
- 新增曲库难度直跳按钮：歌曲库中每首歌会按实际存在的难度显示彩色按钮，点击即可直接切换到对应曲目与难度。
- 新增应用图标：设计并生成 `src-tauri/icons/icon.ico`、`icon.png`、`icon.svg`，图标包含音乐音符、弹幕/队列线与 Miku cyan / B站 pink 配色。
- 新增图标生成脚本：`scripts/generate_icon.py` 可重复生成 SVG、PNG 与多尺寸 Windows ICO。
- 新增 Rust 单元测试覆盖：配置、队列、歌曲搜索、歌曲库解析、弹幕协议、OBS overlay 渲染、游戏切歌辅助函数等模块。

### 修复

- 修复 `exextreme` 等偏好难度缺失时，搜索层已经回退到实际档位但入队仍保留原偏好档位的问题；队列现在携带搜索层实际选中的 `difficulty_tier`。
- 修复悬浮窗打开后白屏/卡死：overlay 窗口不再依赖缺失的 `#/overlay` 路由，改用 Tauri 窗口 label 识别并跳过主窗口 first-run 初始化。
- 修复队列删除/清空后悬浮窗不同步：删除、清空、切歌等队列变更都会发出 `queue-updated` 快照。
- 修复 overlay 小窗被主窗口全局样式污染的问题，覆盖 `html/body/#root` 的最小宽度与背景色，保持透明小窗显示。
- 修复调试点歌路径与生产弹幕路径不一致的问题，调试路径同样会剥离弹幕表情。

### 改进

- First-run Wizard 支持跳过直播间 ID，并在重建歌曲库前先保存当前配置，便于使用 MOD 目录。
- 数据目录解析更贴合 Tauri 打包后的资源布局，优先从可执行文件旁与资源目录查找 `Data`。
- 曲库难度按钮使用 DIVA 难度色：简单 `#009CC2`、普通 `#1AB406`、困难 `#D99800`、极限 `#DA011E`、EX极限 `#A90EEA`，并保留星级数值显示。
- 左下角署名文案现在显示程序版本号，例如 `v0.1.0 Powered by Milkchan`。
- LLM 点歌提示词更新，要求忽略弹幕方括号表情且不将其纳入歌名或作者名。
- 开发者文档更新，记录当前数据文件、难度选择、悬浮窗、中文名加载与内存写入约束。

### 验证

- `cargo test --manifest-path src-tauri/Cargo.toml`：126 个 Rust 单元测试通过。
- `cargo check --manifest-path src-tauri/Cargo.toml`：通过。
- `npm run build`：通过。
- `src-tauri/icons/icon.ico` 包含 `256, 128, 64, 48, 32, 16` 六个 Windows 图标尺寸。
