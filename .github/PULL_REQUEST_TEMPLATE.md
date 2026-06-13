## 变更内容

- [ ] 新增/修正中文曲名
- [ ] 更新 `Data/song_name_zh.json`
- [ ] 同步更新 `docs/data/` 中公开站点使用的数据副本（如适用）

## 证据要求

请列出每个新增或修改译名的来源链接。中文曲名必须来自现有社区或平台证据，或属于允许保留原题显示的 self-display 类型；不要提交无证据硬翻译。

| 原曲名 | 中文名 | 证据链接 | 证据类型 |
| --- | --- | --- | --- |
|  |  |  |  |

## 验证

```bash
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
```
