# diva-live-helper

B站直播点歌助手 - 自动读取直播间弹幕，解析点歌命令，在游戏中选择歌曲。

## 功能

1. **实时弹幕监听** - 连接B站直播间，实时接收弹幕消息
2. **点歌命令解析** - 识别 `点歌 <歌名>` 格式的弹幕
3. **歌曲搜索** - 在游戏数据库中搜索匹配的歌曲
4. **点歌队列** - 将找到的歌曲加入队列
5. **快捷键切歌** - 按下自定义快捷键，从队列取歌并切换游戏歌曲
6. **OBS 点歌队列组件** - 启动本地网页，OBS 浏览器源可实时显示当前点歌列表

## 安装

### 前置要求

- Python 3.14+
- [uv](https://github.com/astral-sh/uv) 包管理器
- Hatsune Miku Project DIVA Mega Mix Plus 游戏

### 安装步骤

1. 克隆项目
```bash
git clone <repository-url>
cd diva-live-helper
```

2. 安装依赖
```bash
uv sync
```

3. 配置
```bash
cp config.example.json config.json
# 编辑 config.json，填入直播间ID等配置
```

## 配置说明

编辑 `config.json` 文件：

```json
{
    "room_id": 12345678,          // B站直播间ID
    "hotkey": "ctrl+shift+n",     // 切歌快捷键
    "data_dir": "Data",           // 数据文件目录
    "mods_dir": "D:\\SteamLibrary\\steamapps\\common\\Hatsune Miku Project DIVA Mega Mix Plus\\mods",  // MOD文件夹路径
    "auto_play_next": false,      // 是否自动播放下一首
    "auto_play_interval": 300,    // 自动播放间隔（秒）
    "max_queue_size": 50,         // 最大队列长度
    "allow_duplicates": false,    // 是否允许重复点歌
    "obs_overlay_enabled": true,  // 是否启用 OBS 点歌队列组件
    "obs_overlay_host": "127.0.0.1", // OBS 组件监听地址
    "obs_overlay_port": 8765,      // OBS 组件监听端口
    "obs_overlay_title": "点歌队列", // OBS 组件标题
    "song_command_prefix": "点歌", // 点歌命令前缀
    "sessdata": ""                // B站SESSDATA（可选）
}
```

### 获取直播间ID

直播间ID在直播间URL中：`https://live.bilibili.com/12345678`，其中 `12345678` 就是直播间ID。

### 获取SESSDATA（可选）

1. 登录B站
2. 打开浏览器开发者工具（F12）
3. 在Application -> Cookies中找到 `SESSDATA`
4. 复制值填入配置文件

## 使用方法

1. 启动游戏 Hatsune Miku Project DIVA Mega Mix Plus
2. 运行程序
```bash
uv run python -m diva_live_helper.main
```

3. 在直播间发送弹幕点歌
```
点歌 Love is War
点歌 初音未来的消失
点歌 みくみくにしてあげる♪
```

4. 按下快捷键（默认 `Ctrl+Shift+N`）切换到队列中的下一首歌

## OBS 点歌队列组件

程序默认会启动一个本地网页组件，用于 OBS 的“浏览器”来源：

```text
http://127.0.0.1:8765/
```

使用方法：

1. 启动 `diva-live-helper` 后，确认控制台打印 `OBS点歌队列组件已启动`。
2. 在 OBS 中添加“浏览器”来源。
3. URL 填入控制台显示的地址，默认是 `http://127.0.0.1:8765/`。
4. 推荐宽度 `680`、高度按直播布局调整；网页背景透明，可直接叠在游戏画面上。

如果端口被占用，可以在 `config.json` 修改：

```json
{
    "obs_overlay_enabled": true,
    "obs_overlay_host": "127.0.0.1",
    "obs_overlay_port": 8766,
    "obs_overlay_title": "点歌队列"
}
```

出于安全考虑，组件只允许监听 `127.0.0.1` 或 `localhost`。不要把它暴露到局域网或公网；如果需要自定义展示，建议读取本机的 JSON 接口后自行转发。

队列数据接口为 `http://127.0.0.1:8765/api/queue`，方便需要自定义样式时复用。

## 数据文件

项目依赖以下数据文件（位于 `Data/` 目录）：

- `pv_db.txt` - 游戏歌曲数据库
- `mdata_pv_db.txt` - DLC歌曲数据库
- `AnotherSongName.json` - 歌曲别名数据库
- `HanziKanjiDict.txt` - 汉字到假名转换表

## MOD支持

程序启动时会自动扫描MOD文件夹，加载所有MOD中的歌曲数据库。每个MOD文件夹需要包含 `rom/mod_pv_db.txt` 文件，其中定义了歌曲的 `song_name` 和 `song_name_en`。

### MOD文件夹结构示例

```
mods/
├── My Song Pack/
│   └── rom/
│       └── mod_pv_db.txt
├── Another Pack/
│   └── rom/
│       └── mod_pv_db.txt
└── ...
```

### mod_pv_db.txt 格式示例

```
pv_4950.song_name=バッドシャーク
pv_4950.song_name_en=Bad Shark
pv_4951.song_name=誰だお前
pv_4951.song_name_en=Dare da Omae
```

## 公开中文曲名数据库

项目可以生成一个静态网页，用于公开浏览、筛选、下载和贡献中文曲名数据库：

```bash
uv run build-zh-site
```

生成结果位于 `docs/`，可直接作为 GitHub Pages 站点发布。页面支持按曲名、中文名、英文名、作者、MOD 来源、状态和证据类型筛选，并提供以下下载：

- `song_name_zh.json` — version 3 中文曲名数据库，包含译名和来源/证据信息
- `song_db.json` — 结构化歌曲数据库
- `song_name_zh.audit.tsv` — 表格形式审计数据

贡献译名时请优先提供可验证来源，例如 B站标题、网易云/QQ 音乐平台标题、萌娘百科或其他社区页面。不要提交无来源的硬翻译。

## 项目结构

```
diva-live-helper/
├── src/
│   └── diva_live_helper/
│       ├── __init__.py
│       ├── main.py              # 主入口
│       ├── config.py            # 配置管理
│       ├── danmaku.py           # B站弹幕处理
│       ├── song_search.py       # 歌曲搜索
│       ├── song_select.py       # 游戏内存修改
│       ├── queue.py             # 点歌队列
│       ├── hotkey.py            # 快捷键监听
│       └── tools/               # 数据库与站点生成工具
├── Data/                        # 数据文件
├── docs/                        # 生成的中文曲名数据库静态站点
├── config.example.json          # 配置示例
├── pyproject.toml               # 项目配置
└── README.md
```

## 开发

### 添加依赖

```bash
uv add <package-name>
```

### 运行测试

```bash
uv run pytest
```

## 注意事项

1. 游戏必须正在运行才能切换歌曲
2. 首次运行需要加载歌曲数据库，可能需要几秒钟
3. 快捷键监听需要管理员权限（Windows）
4. 如果遇到网络问题，请配置国内镜像源

## 许可证

MIT License
