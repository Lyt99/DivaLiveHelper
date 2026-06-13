# diva-live-helper

B站直播点歌助手桌面版。应用使用 Rust + Tauri + React 构建，连接 B 站直播间弹幕，解析点歌请求，在 *Hatsune Miku Project DIVA Mega Mix Plus* 中写入游戏内存切歌，并提供 OBS 点歌队列覆盖层。

## 功能

- 实时连接 B 站直播间弹幕，支持 SESSDATA 和 WBI 签名取弹幕 token。
- 识别 `点歌 <歌名>` 弹幕，也可启用 OpenAI 兼容 LLM 解析自然语言点歌。
- 基于 `Data/song_db.json`、中文名、别名、作者和难度星级进行搜索。
- 在桌面 UI 中管理队列、配置、歌曲库、日志和离线点歌调试。
- 通过快捷键切到下一首，写入 `DivaMegaMix.exe` 内存。
- 启动本地 OBS 浏览器源页面，实时展示点歌队列。

## 前置要求

- Windows
- Node.js 20+
- Rust stable toolchain
- Hatsune Miku Project DIVA Mega Mix Plus
- 管理员权限（全局快捷键监听通常需要）

## 安装与运行

```bash
npm install
npm run tauri dev
```

构建发布包：

```bash
npm run tauri build
```

仅验证前端：

```bash
npm run build
```

仅验证 Rust 后端：

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

## 配置

复制 `config.example.json` 为 `config.json`，或在桌面应用的“设置”页面保存配置。`config.json` 包含敏感信息，默认不会提交到 Git。

关键字段：

- `room_id`：B 站直播间 ID。
- `hotkey`：切歌快捷键，例如 `ctrl+shift+n`。
- `data_dir`：歌曲数据目录，默认 `Data`。
- `mods_dir`：游戏 `mods/` 目录，每个 MOD 需要包含 `rom/mod_pv_db.txt`。
- `sessdata`：B 站 Cookie 中的 SESSDATA，可提高弹幕连接可靠性。
- `llm_enabled`：启用自然语言点歌解析；前缀点歌始终本地解析。
- `llm_api_key`：OpenAI 兼容接口 API Key；本地模型可留空。
- `llm_base_url` / `llm_model`：OpenAI 兼容服务地址与模型名。
- `obs_overlay_enabled` / `obs_overlay_host` / `obs_overlay_port`：OBS 覆盖层设置。

## 使用

1. 启动游戏 `DivaMegaMix.exe`。
2. 运行 `npm run tauri dev` 或启动已构建的桌面程序。
3. 在“设置”页填写直播间、MOD 路径、快捷键、LLM 等配置并保存。
4. 在“点歌”页连接直播间和游戏进程。
5. 观众发送弹幕：

```text
点歌 Love is War
点歌 初音未来的消失
点歌 みくみくにしてあげる♪ ex
```

6. 按快捷键或点击“切下一首”执行切歌。

## OBS 点歌队列覆盖层

启用后访问：

```text
http://127.0.0.1:8765/
```

队列 JSON 接口：

```text
http://127.0.0.1:8765/api/queue
```

覆盖层只允许监听 `127.0.0.1` 或 `localhost`，不要暴露到局域网或公网。

## 数据文件

`Data/` 是运行时数据目录，保留在仓库根目录：

- `song_db.json`：结构化歌曲数据库，通常由工具扫描基础 `pv_db.txt` 与 MOD 数据生成。
- `song_name_zh.json`：中文曲名数据库，version 3 `entries` 格式。
- `AnotherSongName.json`：旧别名映射。
- `HanziKanjiDict.txt`：汉字/汉字转写搜索辅助表。
- `pv_db.txt`：基础游戏歌曲数据库。

`docs/` 是已生成的公开中文曲名数据库静态站点，可用于 GitHub Pages 发布。

## 项目结构

```text
diva-live-helper/
├── src/                         # React / TypeScript 前端
│   ├── App.tsx
│   ├── main.tsx
│   ├── components/
│   ├── lib/
│   └── pages/
├── src-tauri/                   # Rust / Tauri 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   ├── icons/
│   └── src/
├── Data/                        # 歌曲数据库与搜索辅助数据
├── docs/                        # 中文曲名数据库静态站点
├── config.example.json          # 配置示例
├── package.json                 # 前端与 Tauri CLI 脚本
└── README.md
```

## 注意事项

- 仅支持 Windows；内存写入目标进程为 `DivaMegaMix.exe`。
- 游戏必须运行后才能连接游戏进程并切歌。
- 全局快捷键通常需要管理员权限。
- `config.json` 可能包含 SESSDATA 和 LLM API Key，不要提交。

## 许可证

MIT License
