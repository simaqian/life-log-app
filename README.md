# Life-Log

> 把生活变成可查询的数据库。

一个 Mac 菜单栏 App：
- **每小时** 弹一次「你在干嘛？」，支持文字 + 语音输入
- **物品库** 记录你拥有的全部物品（拍照、标签、搜索）
- **任务管理** 轻量列表，任务与打卡自动关联，看得到每个任务真实耗时
- **AI 结构化** 你随便说，DeepSeek/Claude/本地模型帮你打标签
- **回顾** 时间轴 + 记录流 + AI 周报
- **本地优先** 数据完全在你的机器上

## 状态

🚧 **MVP 骨架阶段**（v0.1.0）

已搭建：
- ✅ Tauri 2 + Svelte 5 + Rust 工程结构
- ✅ 菜单栏 tray + 全局快捷键骨架
- ✅ SQLite 数据库 + 7 张表 migrations（events / tags / tasks / task_events / checkin_schedule / settings / reports）
- ✅ LLM 抽象层：Claude / DeepSeek / Kimi / 通义 / SiliconFlow / OpenRouter / Ollama
- ✅ STT 抽象层：本地 whisper / SenseVoice / 云端
- ✅ 打卡占位窗口（文字保存 → DB 落库）

待做：
- ⏳ 每小时定时器（scheduler.rs）
- ⏳ 语音录音 + STT 转写
- ⏳ LLM 异步结构化
- ⏳ 物品库 / 任务管理 / 回顾页
- ⏳ 设置页（绑 LLM key / STT 模型路径）
- ⏳ 全局快捷键真正注册

完整规划见 [docs/SPEC.md](docs/SPEC.md)；技术决策见 [docs/DECISIONS.md](docs/DECISIONS.md)。

## 环境要求

- **macOS 11+**
- **Node 20+**（实测 22.x OK）
- **Rust 1.75+**（实测 1.96 OK）
- **Xcode Command Line Tools**（`xcode-select --install`）

## 跑起来

```bash
cd life-log
npm install        # 装前端依赖（首次 ~1 分钟）
npm run tauri:dev  # 同时起 Vite + Rust，首次 ~3-5 分钟编译 Tauri 依赖
```

跑起来后：

1. **菜单栏出现一个黑色小图标**（开发期是个圆点占位图）—— 点它弹菜单
2. **同时主窗口会自动打开**（开发期占位，显示后端 ping 状态和最近 events）
3. **菜单里点 "🎤 现在打卡"** → 右下角弹出 360×280 打卡窗口
4. 输入文字按 ⌘↩ 保存；Esc 跳过；× 关闭

数据存在 `~/Library/Application Support/com.crd.life-log/data/life-log.db`。

直接用 `sqlite3` 或 [Datasette](https://datasette.io/) 打开查看：

```bash
sqlite3 ~/Library/Application\ Support/com.crd.life-log/data/life-log.db
> .tables
> SELECT * FROM events;
```

## 项目结构

```
life-log/
├─ docs/                      规格 & 决策文档
│   ├─ SPEC.md
│   └─ DECISIONS.md
├─ src/                       Svelte 前端
│   ├─ App.svelte             根组件 + hash 路由
│   ├─ main.ts                入口
│   ├─ app.css                Tailwind
│   ├─ routes/
│   │   ├─ Home.svelte        主窗口（占位）
│   │   └─ Checkin.svelte     打卡弹窗
│   └─ lib/
│       └─ api.ts             前端 → Rust 的胶水
├─ src-tauri/                 Rust 后端
│   ├─ Cargo.toml
│   ├─ tauri.conf.json
│   ├─ build.rs
│   ├─ icons/                 应用图标 + tray 图标（占位）
│   ├─ capabilities/          Tauri 2 权限声明
│   └─ src/
│       ├─ main.rs            入口（调 lib::run）
│       ├─ lib.rs             tray + setup + state 注入
│       ├─ db.rs              SQLite + migrations
│       ├─ commands.rs        Tauri command（前端 invoke 的目标）
│       ├─ llm/               LLM 抽象层
│       │   ├─ mod.rs
│       │   ├─ openai_compat.rs  (DeepSeek/Kimi/通义/SiliconFlow/OpenRouter)
│       │   ├─ anthropic.rs
│       │   └─ ollama.rs
│       └─ stt/               STT 抽象层
│           ├─ mod.rs
│           ├─ whisper_local.rs
│           ├─ sensevoice.rs
│           └─ cloud.rs
├─ index.html
├─ package.json
├─ tsconfig.json
├─ vite.config.ts
├─ tailwind.config.js
└─ postcss.config.js
```

## 故障排查

**菜单栏图标不出现** → 检查 `src-tauri/icons/tray.png` 是否存在（占位图是黑色小圆点，可能看不清，特别是 macOS 状态栏深色背景下；后续会换成真正的图标）

**编译报错 `tauri-build`** → 检查 Rust 版本 ≥ 1.75：`rustc --version`

**前端 1420 端口占用** → 关掉别的 Vite 进程：`lsof -i:1420`

**数据库锁** → 关掉所有 `life-log` 进程；确保只有一个实例在跑（Dropbox 同步场景下尤其注意）

## License

MIT
