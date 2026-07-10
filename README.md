# diva-live-helper

> B站直播点歌助手桌面版 — 监听 Bilibili 直播间弹幕，解析点歌请求，在 *Hatsune Miku Project DIVA Mega Mix Plus* 中自动切歌。
>
> A desktop tool that listens to Bilibili live danmaku, parses song requests, and switches songs in *Hatsune Miku Project DIVA Mega Mix Plus* via memory writes. Built with Rust + Tauri + React.

## 免责声明

本项目是**非官方粉丝自制工具**，与 SEGA、Crypton Future Media、bilibili 无任何关联。

- 本工具通过写入游戏进程内存来切换歌曲，属于对游戏的修改行为。使用本工具的风险由使用者自行承担，作者不对因使用本工具导致的任何损失（包括但不限于游戏损坏、存档丢失、账号封禁）负责。
- *Hatsune Miku Project DIVA* 是 SEGA 的商标，初音未来是 Crypton Future Media 的商标。
- 本工具仅供个人学习和娱乐使用，请遵守相关游戏和平台的使用条款。

## 功能

- **弹幕监听**：实时连接 B 站直播间弹幕，支持 SESSDATA Cookie 和 WBI 签名获取弹幕 token。
- **点歌解析**：识别 `点歌 <歌名>` 前缀弹幕进行本地解析；可启用 OpenAI 兼容 LLM 解析自然语言点歌请求。
- **多策略搜索**：基于原名、中文名、汉字→汉字转写、别名索引、英文名进行多轮模糊匹配，支持难度星级过滤与回退。
- **队列管理**：桌面 UI 管理点歌队列、配置、歌曲库、日志，支持自动切歌、重复策略、队列容量限制。
- **游戏切歌**：通过快捷键或 UI 操作，写入 `DivaMegaMix.exe` 内存完成切歌，支持 MOD 歌曲。
- **OBS 覆盖层**：内置本地 HTTP 服务器，提供 OBS 浏览器源页面，实时展示点歌队列。
- **首次引导**：首次启动时通过向导完成直播间、快捷键、MOD 路径、LLM、难度偏好等配置。

## 前置要求

- **Windows**（内存写入仅支持 Windows 平台）
- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) stable 工具链
- *Hatsune Miku Project DIVA Mega Mix Plus*（Steam 版）
- 管理员权限（全局快捷键监听通常需要）

## 快速开始

### 从源码构建

```bash
# 安装前端依赖
npm install

# 开发模式运行
npm run tauri dev

# 构建发布包（生成 .exe / .msi）
npm run tauri build
```

### 验证

```bash
# 前端类型检查 + Vite 构建
npm run build

# Rust 后端检查
cargo check --manifest-path src-tauri/Cargo.toml

# Rust 单元测试
cargo test --manifest-path src-tauri/Cargo.toml
```

### 使用流程

1. 启动游戏 `DivaMegaMix.exe`。
2. 运行 `npm run tauri dev` 或启动已构建的桌面程序。
3. 首次启动会进入引导向导，按步骤填写直播间 ID、MOD 路径、快捷键、LLM 配置等。
4. 在"点歌"页连接直播间和游戏进程。
5. 观众在直播间发送弹幕：

   ```text
   点歌 Love is War
   点歌 初音未来的消失
   点歌 みくみくにしてあげる♪
   ```

6. 按快捷键或点击"切下一首"执行切歌。

## 配置

复制 `config.example.json` 为 `config.json`，或在桌面应用的"设置"页面保存配置。`config.json` 可能包含 SESSDATA 和 LLM API Key，已在 `.gitignore` 中排除，不会提交到 Git。

关键字段：

| 字段 | 说明 | 默认值 |
|---|---|---|
| `room_id` | B 站直播间 ID | — |
| `hotkey` | 切歌全局快捷键，如 `ctrl+shift+n` | — |
| `data_dir` | 歌曲数据目录 | `Data` |
| `mods_dir` | 游戏 `mods/` 目录，每个 MOD 需含 `rom/mod_pv_db.txt` | — |
| `sessdata` | B 站 Cookie 中的 SESSDATA，提高弹幕连接可靠性 | — |
| `song_command_prefix` | 弹幕点歌前缀 | `点歌` |
| `llm_enabled` | 启用自然语言点歌解析（前缀点歌始终本地解析） | `false` |
| `llm_api_key` | OpenAI 兼容接口 API Key（本地模型可留空） | — |
| `llm_base_url` | OpenAI 兼容服务地址 | `https://api.deepseek.com` |
| `llm_model` | 模型名称 | `deepseek-chat` |
| `llm_max_tokens` | 响应 token 上限 | `150` |
| `obs_overlay_enabled` | 启用 OBS 覆盖层 | `true` |
| `obs_overlay_host` | 覆盖层监听地址（仅允许 `127.0.0.1` / `localhost`） | `127.0.0.1` |
| `obs_overlay_port` | 覆盖层监听端口 | `8765` |
| `auto_play_next` | 启用自动切歌 | `false` |
| `auto_play_interval` | 自动切歌间隔（秒） | `300` |
| `max_queue_size` | 队列容量上限 | `50` |
| `allow_duplicates` | 允许重复点歌 | `false` |
| `default_search_difficulty` | 默认搜索难度档位 | `extreme` |
| `difficulty_tolerance` | 星级过滤容差（±） | `0.5` |
| `difficulty_fallback` | 无匹配时的回退方向：`easier` 或 `harder` | `easier` |
| `fetch_chinese_names` | 从外部来源抓取中文名 | `false` |
| `http_proxy` | LLM 客户端代理 | — |

## OBS 点歌队列覆盖层

启用后，在 OBS 中添加浏览器源，URL 填写：

```text
http://127.0.0.1:8765/
```

队列 JSON 接口（可供其他工具消费）：

```text
http://127.0.0.1:8765/api/queue
```

覆盖层只允许监听 `127.0.0.1` 或 `localhost`，不要暴露到局域网或公网。

## 数据文件

`Data/` 是运行时数据目录：

| 文件 | 说明 |
|---|---|
| `base_song_db.json` | 编译期基础歌曲库源文件，通过 `include_str!` 嵌入二进制。包含本体与 DLC 曲目，`source` 标记为 `base` 或 `dlc`。仅在"重建歌曲库"时用于合并生成 `song_db.json`。 |
| `song_db.json` | 结构化运行时歌曲数据库，由基础库与 MOD 数据合并生成。 |
| `song_name_zh.json` | 中文曲名数据库（version 3 `entries` 格式）。嵌入式基础版本编译进二进制，磁盘上的同名文件可覆盖。 |
| `AnotherSongName.json` | 旧版别名映射 `{ alias: canonical_name }`。 |
| `HanziKanjiDict.txt` | 汉字→汉字转写搜索辅助表。 |
| `pv_db.txt` / `mdata_pv_db.txt` | 旧版本体与 DLC 歌曲数据库，保留用于参考或重新生成 `base_song_db.json`。 |

`docs/` 是已生成的公开中文曲名数据库静态站点，可用于 GitHub Pages 发布。

## 项目结构

```text
diva-live-helper/
├── src/                         # React / TypeScript 前端
│   ├── App.tsx                  # 路由与事件监听
│   ├── main.tsx                 # React 入口
│   ├── types.ts                 # 前端 API / 事件类型
│   ├── lib/tauri.ts             # invoke() 封装
│   ├── components/              # 布局、主题切换等共享组件
│   └── pages/                   # 队列、设置、日志、歌曲库、向导、覆盖层
├── src-tauri/                   # Rust / Tauri 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json          # Tauri v2 配置
│   ├── capabilities/            # Tauri 权限配置
│   ├── icons/
│   └── src/
│       ├── lib.rs               # Builder、状态、命令注册
│       ├── commands.rs          # 暴露给前端的 Tauri 命令
│       ├── config.rs            # 配置模型、加载/保存/校验
│       ├── danmaku.rs           # B 站弹幕 WebSocket 客户端
│       ├── song_db.rs           # 歌曲 DB 模型与解析
│       ├── song_search.rs       # 多策略歌曲搜索
│       ├── song_select.rs       # 游戏内存写入切歌
│       ├── queue.rs             # 歌曲队列与历史
│       ├── hotkey.rs            # 全局快捷键
│       ├── llm_intent.rs        # OpenAI 兼容意图解析
│       ├── obs_overlay.rs       # 本地 OBS 浏览器源服务器
│       └── db_tool.rs           # 歌曲 DB 重建工具
├── Data/                        # 歌曲数据库与搜索辅助数据
├── docs/                        # 中文曲名数据库静态站点
├── config.example.json          # 配置示例
├── package.json                 # 前端与 Tauri CLI 脚本
└── LICENSE
```

## 架构概览

```
弹幕 → 前缀匹配 / LLM 解析 → 多策略搜索 → 队列 → 快捷键/UI → 内存写入切歌
                                                    ↓
                                              OBS 覆盖层同步
```

1. 前端通过 `start_danmaku(roomId)` 启动弹幕监听。
2. Rust 后端解析真实房间 ID，获取 WBI 签名的弹幕 token，连接 WebSocket。
3. 前缀匹配（`点歌 <歌名>`）→ 本地解析；否则若启用 LLM → 调用 OpenAI 兼容接口提取点歌意图。
4. `SongSearcher` 执行多轮搜索（原名 → 中文名 → 汉字转写 → 别名 → 英文名），可选星级过滤与回退。
5. 首个结果加入 `SongQueue`，默认拒绝重复。
6. 快捷键或 UI 操作出队，调用 `SongSelector::change_song(pv_id, difficulty_tier)` 写入游戏内存。

## 开发

```bash
# 安装依赖
npm install

# 开发模式（热重载）
npm run tauri dev

# 类型检查
npm run build

# Rust 检查
cargo check --manifest-path src-tauri/Cargo.toml
```

前端页面使用 React + Zustand 状态管理，UI 设计规范见 [`UIDESIGN.md`](./UIDESIGN.md)。

## 贡献

欢迎提交 Issue 和 Pull Request。

- 提交信息使用英文祈使句，不加 `feat:` / `fix:` 等前缀。
- 用户可见字符串和代码注释使用中文。
- 不要提交 `config.json`（可能包含 SESSDATA 和 API Key）。
- 前端改动需遵循 [`UIDESIGN.md`](./UIDESIGN.md) 设计规范。

## 致谢

- [DivaDanmuSelecter](https://github.com/hiki8man/DivaDanmuSelecter) — 基于 Pymem 和 Blivedm 实现的歌姬计划弹幕点歌姬，本项目的灵感来源与前身（本项目最初即为其 Python 重写，后迁移至 Rust + Tauri）。
- [Select Song with PVID](https://gamebanana.com/tools/21051) — GameBanana 上的 Project DIVA Mega Mix+ PVID 选歌工具；本项目 `song_select.rs` 的游戏内存写入偏移量参考自此工具的逆向成果，`HanziKanjiDict.txt` 所用的日文汉字到中文汉字映射表亦来源于此。
- [Tauri](https://tauri.app/) — 跨平台桌面应用框架
- [React](https://react.dev/) — 前端 UI 库
- [DeepSeek](https://www.deepseek.com/) — 默认 LLM 服务
- B 站弹幕 WebSocket 协议的社区逆向文档

## 许可证

[MIT License](./LICENSE)
