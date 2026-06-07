# Life-Log 产品规格说明

> 一个把"我有什么"和"我在做什么"持续记下来的个人系统。
> Mac 菜单栏 App，每小时主动问一次，支持语音输入，本地优先。

最后更新：2026-06-07（v2 — 加入任务管理）

---

## 1. 项目定位

### 1.1 一句话

> **把生活变成可查询的数据库。**

### 1.2 解决的问题

- **"我到底有什么东西？"** —— 物品散落各处，搬家、保修、转卖、找东西时全靠回忆
- **"我的时间到底花在哪了？"** —— 主观感觉"很忙"或"很闲"，但没有客观数据
- **"我去年这时候在干嘛？"** —— 没有持续记录，回忆只剩零碎片段
- **"我计划做的事 vs 我实际做了的事"** —— 任务列在那里，但有没有真做、花了多久、什么时候做的，事后无据可查

### 1.3 不解决的问题（明确划界）

- ❌ 不是项目管理（不做 Gantt、依赖关系、团队协作、多人共享）
- ❌ 不是日记 App（不强调长文写作、不做 Markdown 编辑器）
- ❌ 不是记账 App（物品可记价格，但不做收支平衡、预算、报表）
- ❌ 不是健康追踪（不接 HealthKit、不算卡路里）
- ❌ 不做社交（数据完全私有，不分享）

> **关于任务管理**：会做一个**轻量级**任务列表，重点不在于"管理任务"本身，而在于让任务和打卡产生联动（"我说在做 X" → 自动关联到任务 X，事后能看到这个任务总共花了多少时间）。不和 Things / Todoist / Notion 比功能数量。

### 1.4 核心设计原则

1. **记录成本必须低**。一次记录超过 10 秒，长期看一定崩。
2. **数据出口比入口重要**。回顾、搜索、可视化必须从第一版就有，否则记录变垃圾。
3. **本地优先**。数据是你的，离线能用，云只是同步通道。
4. **诚实承认局限**。Mac App 注定漏掉非电脑时段——明说，不假装覆盖全场景。
5. **AI 是助手不是主角**。LLM 帮你结构化、帮你总结，但所有原始输入永远保留，可以脱离 AI 单独存在。

---

## 2. 范围与阶段

### 2.1 第一阶段：Mac-only MVP（目标 2-3 周）

**Must have**：
- 菜单栏常驻图标 + 系统托盘菜单
- 每小时（可配置）弹出打卡窗口
- 文字 + 语音输入（本地 STT）
- LLM 结构化（默认 DeepSeek，可换）
- 物品库：增删改查、搜索、标签
- **任务管理**：增删改查、状态流转（pending / in_progress / done / cancelled）、优先级、标签
- **任务 × 打卡联动**：打卡时 LLM 自动尝试关联到已有任务；任务详情页能看到所有相关打卡和总耗时
- 简易回顾页：今天 / 本周时间轴 + 记录流 + 任务条
- 数据导出（CSV / JSON）
- 设置页（LLM key、STT 选择、勿扰时段、打卡间隔）

**Should have**：
- 全局快捷键（手动打卡 / 添加物品 / 添加任务）
- 拍照（调系统相机）/ 截图附加到记录
- AI 生成周报（含任务完成率统计）

**Won't have（本阶段不做）**：
- 手机端 App（但后端预留 HTTP 接口）
- 多设备同步（先靠 Dropbox 同步 SQLite 文件应付）
- 全文搜索（用 SQL LIKE 先顶着）
- 数据可视化的复杂图表（只做最简单的饼图 + 时间轴）
- 任务的重复规则、子任务、依赖关系、deadline 提醒（任务管理保持轻量，详见 §5.6）

### 2.2 第二阶段（视使用情况再定）

- 手机轻量补录入口（Android PWA 或快捷指令 + 本地 HTTP）
- AI 月报、年报
- 主动洞察（"你这周睡眠时间变少了"）
- 物品照片自动识别（拍照即录入）
- 与日历/邮件整合（自动推断"在开会"）

### 2.3 第三阶段（很远，可能不做）

- 真正的原生 Android App
- 多人共享物品库（家庭账本场景）
- 端到端加密的云同步

---

## 3. 技术栈

| 层 | 选型 | 理由 |
|---|---|---|
| 桌面框架 | **Tauri 2** | 包小（10-20MB），菜单栏/通知/全局快捷键支持完整，Rust 后端性能好 |
| 前端 | **Svelte 5 + TypeScript + Vite** | 轻量、模板语法直觉、跟 Tauri 配合好 |
| 样式 | **Tailwind CSS** | 写 UI 快，菜单栏小窗适合 utility class |
| Rust 后端 | rusqlite, tokio, reqwest, serde, tray-icon, notify-rust | 标准组合 |
| 数据库 | **SQLite**（rusqlite） | 单文件，可放 Dropbox，Datasette 可直接打开 |
| LLM | **抽象层 + 多 provider**，默认 DeepSeek | 见 §6 |
| STT | **抽象层 + 多 provider**，默认本地 faster-whisper 或 SenseVoice | 见 §7 |
| 打包 | tauri-cli + 公证 | Mac App Store 暂不上 |

**版本基线**：
- Tauri 2.x
- Node 20+
- Rust 1.75+
- SQLite 3.40+

---

## 4. 数据模型

### 4.1 总体思路

**万物皆 event**。物品、打卡、随手笔记，底层是同一张 `events` 表，靠 `type` 字段区分。原因：
- 未来想加新类型（吃了什么、读完一本书、看了一部电影）不用改架构
- 时间轴回顾只需要一个查询
- LLM 提取的结构化字段统一存 JSON，schema 演进无痛

**任务是唯一的例外**——独立成 `tasks` 表，因为：
- 任务有自己的生命周期（pending → in_progress → done），不只是"发生在某个时刻的事"
- 任务会被多次打卡关联（通过 `task_events` 关联表）
- 任务的字段查询频繁（"所有 pending 任务"、"按优先级排序"）放 JSON 里性能差
- 任务的"创建"、"开始"、"完成" 三个时刻**也是 event**，会同步落到 events 表，确保时间轴看得到

### 4.2 表定义

```sql
-- 事件主表
CREATE TABLE events (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  ts            INTEGER NOT NULL,        -- 事件发生时间戳（毫秒，UTC）
  type          TEXT    NOT NULL,        -- 'checkin' | 'item' | 'note'
  raw_text      TEXT,                    -- 用户原始文字输入（可能来自 STT 转写）
  raw_voice_path TEXT,                   -- 原始录音文件相对路径
  structured    TEXT,                    -- LLM 提取的 JSON
  tags          TEXT,                    -- JSON 数组: ["工作","戴森"]
  media         TEXT,                    -- JSON 数组: ["photos/2026/06/07/abc.jpg"]
  source        TEXT,                    -- 'manual' | 'scheduled' | 'shortcut' | 'http_api'
  llm_provider  TEXT,                    -- 哪个 provider 处理的（追溯用）
  llm_model     TEXT,
  stt_provider  TEXT,
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL,
  deleted_at    INTEGER                  -- 软删除时间戳，NULL = 未删除
);

CREATE INDEX idx_events_ts        ON events(ts);
CREATE INDEX idx_events_type      ON events(type);
CREATE INDEX idx_events_deleted   ON events(deleted_at);
CREATE INDEX idx_events_type_ts   ON events(type, ts);

-- 标签字典
CREATE TABLE tags (
  name        TEXT PRIMARY KEY,
  kind        TEXT,                      -- 'activity' | 'item_category' | 'project' | 'person' | 'place' | 'mood' | 'other'
  use_count   INTEGER DEFAULT 0,
  is_primary  INTEGER DEFAULT 0,         -- 是否升格为"主标签"（LLM 推荐后人工确认）
  color       TEXT,                      -- 时间轴显示色（可选）
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL
);

CREATE INDEX idx_tags_kind ON tags(kind);

-- 打卡调度记录（用来回答"有几次我没打"）
CREATE TABLE checkin_schedule (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  scheduled_at INTEGER NOT NULL,         -- 计划弹出时间
  fired_at     INTEGER,                  -- 实际弹出时间
  responded_at INTEGER,                  -- 用户响应时间
  event_id     INTEGER,                  -- 关联到 events.id（如有响应）
  status       TEXT NOT NULL,            -- 'pending' | 'fired' | 'responded' | 'skipped' | 'expired'
  FOREIGN KEY (event_id) REFERENCES events(id)
);

CREATE INDEX idx_schedule_status ON checkin_schedule(status);
CREATE INDEX idx_schedule_scheduled ON checkin_schedule(scheduled_at);

-- 配置（K/V）
CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT                             -- 复杂值存 JSON 字符串
);

-- AI 报告缓存（避免重复生成）
CREATE TABLE reports (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  kind         TEXT NOT NULL,            -- 'week' | 'month' | 'year' | 'custom'
  range_start  INTEGER NOT NULL,
  range_end    INTEGER NOT NULL,
  content_md   TEXT NOT NULL,            -- 报告正文 Markdown
  llm_provider TEXT,
  llm_model    TEXT,
  created_at   INTEGER NOT NULL,
  pinned       INTEGER DEFAULT 0
);

CREATE INDEX idx_reports_range ON reports(range_start, range_end);

-- 任务表（独立于 events，有自己的生命周期）
CREATE TABLE tasks (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  title         TEXT    NOT NULL,
  description   TEXT,
  status        TEXT    NOT NULL,       -- 'pending' | 'in_progress' | 'done' | 'cancelled' | 'archived'
  priority      INTEGER DEFAULT 3,      -- 1 (最高) - 5 (最低)
  tags          TEXT,                   -- JSON 数组
  project       TEXT,                   -- 归属项目（可选；和 checkin.structured.project 共享同一套字符串）
  due_at        INTEGER,                -- 截止时间戳（可选；本阶段不做提醒）
  estimate_min  INTEGER,                -- 预估时长，分钟（可选）
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL,
  started_at    INTEGER,                -- 第一次进入 in_progress 的时间
  done_at       INTEGER,                -- 完成时间
  cancelled_at  INTEGER,
  archived_at   INTEGER,
  deleted_at    INTEGER,                -- 软删除
  source        TEXT,                   -- 'manual' | 'llm_extracted' | 'http_api'
  parent_id     INTEGER,                -- 子任务的父任务 id（可选；本阶段 UI 不主推子任务）
  FOREIGN KEY (parent_id) REFERENCES tasks(id)
);

CREATE INDEX idx_tasks_status   ON tasks(status);
CREATE INDEX idx_tasks_due      ON tasks(due_at);
CREATE INDEX idx_tasks_parent   ON tasks(parent_id);
CREATE INDEX idx_tasks_priority ON tasks(priority);

-- 任务 × 事件 关联（一个任务可以关联多次打卡 / 笔记 / 完成事件）
CREATE TABLE task_events (
  task_id   INTEGER NOT NULL,
  event_id  INTEGER NOT NULL,
  relation  TEXT    NOT NULL,           -- 'work_on' | 'created_by' | 'completed_by' | 'mention'
  created_at INTEGER NOT NULL,
  PRIMARY KEY (task_id, event_id, relation),
  FOREIGN KEY (task_id)  REFERENCES tasks(id),
  FOREIGN KEY (event_id) REFERENCES events(id)
);

CREATE INDEX idx_task_events_event ON task_events(event_id);
CREATE INDEX idx_task_events_relation ON task_events(relation);
```

### 4.3 `structured` 字段按 type 的 schema

**type = 'checkin'**
```json
{
  "activity": "写代码",
  "project": "life-log",
  "mood": "专注",
  "energy": 8,
  "location": "家",
  "with_whom": [],
  "linked_task_ids": [42],
  "task_action": "work_on"
}
```
所有字段都可选（LLM 提取不到就不放）。`mood` 枚举：`专注 / 平稳 / 兴奋 / 疲惫 / 焦虑 / 低落 / 开心 / 无聊 / 其他`。`energy` 是 1-10 整数。

`linked_task_ids` 和 `task_action` 由 LLM 在打卡时尝试关联到 `tasks` 表中的 pending/in_progress 任务，并同步写入 `task_events` 关联表。`task_action` 枚举：`work_on`（正在做）/ `completed_by`（这次打卡导致任务完成）/ `mention`（只是提了一句）。用户在打卡预览页可改/可取消关联。

**type = 'item'**
```json
{
  "name": "戴森 V12",
  "brand": "Dyson",
  "category_suggestion": "清洁电器",
  "price": 3000,
  "currency": "CNY",
  "location": "客厅",
  "bought_at": 1717689600000,
  "warranty_until": null,
  "condition": "全新",
  "notes": ""
}
```

**type = 'note'**
```json
{
  "title": "...",
  "body": "..."
}
```

### 4.3.1 任务的 LLM 提取输出

当用户输入像「明天前把洗碗机装完」「调研一下 SwiftUI 这周内出个结论」这类话时（无论从打卡入口、添加任务入口、还是物品/笔记入口），LLM 用一个统一 schema 提取任务字段，直接写入 `tasks` 表：

```json
{
  "title": "安装洗碗机",
  "priority": 2,
  "tags": ["家务"],
  "project": null,
  "due_at": 1717948800000,
  "estimate_min": 60,
  "status": "pending"
}
```

打卡的 `task_action: "completed_by"` 触发时，LLM 把对应任务的 `status` 改为 `done` 并填 `done_at`。

### 4.4 文件目录布局

App 的数据目录（macOS 默认 `~/Library/Application Support/com.life-log/`）：

```
data/
├─ life-log.db            # SQLite 主库
├─ life-log.db-shm        # SQLite WAL
├─ life-log.db-wal
├─ voices/
│   └─ YYYY/MM/DD/<ulid>.m4a
├─ photos/
│   └─ YYYY/MM/DD/<ulid>.jpg
├─ exports/               # 导出文件
└─ backups/               # 自动每日 zip 备份，保留 30 天
```

可选：用户在设置里把 `data/` 目录指到 Dropbox/iCloud 文件夹下，实现单机多设备同步（**注意：同一时刻只能一个进程运行，否则 SQLite 会冲突**）。

---

## 5. 交互设计

### 5.1 菜单栏图标状态

| 图标 | 含义 |
|---|---|
| 🟢 绿 | 一切正常 |
| 🟠 橙 | 有未响应的打卡 |
| 🔵 蓝 | 正在录音 |
| 🟡 黄 | LLM/STT 处理中 |
| 🔴 红 | 错误（API 失败、磁盘满等） |

点击图标弹出菜单：
```
─────────────────────────
  🎤 现在打卡         ⌘⇧L
  📦 添加物品         ⌘⇧I
  ✅ 添加任务         ⌘⇧T
  📝 随手记           ⌘⇧N
─────────────────────────
  📊 回顾
  📦 物品库
  ✅ 任务
─────────────────────────
  ⚙️  设置
  ⏸  暂停打卡 1 小时
  📤 导出数据
─────────────────────────
  退出
```

### 5.2 每小时打卡

**触发**：
- 默认 9:00 - 22:00 之间，每小时一次
- 实际弹出时间在 `:50 ± 5 min` 区间内随机（避免整点撞会议/吃饭）
- 勿扰时段（默认 23:00 - 8:00）不弹
- "暂停打卡 1 小时" 临时跳过

**弹窗**（360 × 240，屏幕右下角）：
```
┌──────────────────────────────────┐
│ 14:53  你这小时在干嘛？     ×    │
│                                  │
│ ┌──────────────────────────────┐ │
│ │ （文字输入，自动 focus）       │ │
│ │                              │ │
│ └──────────────────────────────┘ │
│                                  │
│ 🎤 按住 Space 录音               │
│                                  │
│ [稍后再说]  [跳过]   [✓ 保存]    │
└──────────────────────────────────┘
```

- **按住 Space**（或点 🎤 按钮）录音，松开停止
- 录音最长 60s，超过自动停止
- 录音结束 → 本地 STT 转文字 → 自动填入文字框（可改）
- 点 "保存" → 后台调 LLM 结构化 → 入库 → 弹窗消失
- 点 "稍后再说" → 5 分钟后再弹一次（最多 3 次）
- 点 "跳过" → 标记 skipped，不再弹
- 关闭窗口（按 Esc 或 ×）= 跳过
- 处理失败也保存（raw_text + raw_voice_path 不会丢）

**任务关联（LLM 后台执行，不阻塞保存）**：

保存后 LLM 异步比对当前 pending/in_progress 任务列表。如果识别出关联，菜单栏图标短暂闪一下并显示一个非模态小提示：

```
┌─────────────────────────────────────┐
│ 这次打卡可能关联到：                 │
│   ✅ #42  安装洗碗机                 │
│      [确认关联] [改为已完成] [忽略]  │
└─────────────────────────────────────┘
```

5 秒不操作默认确认关联（写入 task_events，relation='work_on'）。这是为了不打断你正在做的事——LLM 大概率猜对，错了也只是个 task_events 行，回顾时可以删。

### 5.3 物品库

**列表页**：
- 顶部：搜索框 + 标签筛选 + 新增按钮
- 网格视图：每件物品一张卡片，缩略图 + 名称 + 标签
- 列表视图：表格，可按时间/名称/价格排序
- 默认按"最近添加"排

**详情页**：
- 大图、所有字段、标签、备注
- 编辑按钮、删除按钮（软删除，回收站可恢复）
- "在哪买的" / "什么时候买的" / "现在哪" 一目了然

**添加流程**：
1. 全局快捷键 ⌘⇧I 或菜单栏点 "添加物品"
2. 弹一个稍大的窗口（500 × 400）
3. 文字框（"我刚买了个戴森…"）+ 拍照按钮 + 语音按钮
4. 用户输入后，LLM 提取字段
5. 跳到预览页：左边原文，右边自动填好的表单（用户可改）
6. 确认入库

### 5.4 回顾页（独立窗口，900 × 600）

三个 tab：

**Tab 1 - 时间轴**
- 横向滚动，时间从左到右
- 每条 checkin 是一个色块（颜色按主标签）
- **任务条**：每个已完成/进行中任务画一条横向条，从 `started_at` 到 `done_at`（或当前时间），点击展开看关联的所有打卡
- 鼠标悬停看详情
- 上方按钮：今天 / 本周 / 本月 / 自定义
- 切换：[活动视图 | 任务视图 | 叠加视图]
- 下方饼图：本时段活动占比 / 任务耗时占比

**Tab 2 - 记录流**
- 竖向 feed，按时间倒序
- 每条记录显示：时间、类型图标、原文、结构化字段
- 关联到任务的 checkin 显示一个小徽章 `→ #42 安装洗碗机`，可点击跳到任务详情
- 有录音的可点 ▶ 播放
- 有照片的显示缩略图
- 可编辑、可删除

**Tab 3 - 报告**
- 列表显示已生成的报告（周报、月报）
- 点 "生成新报告" → 选时间范围 → 调 LLM
- 报告内容是 Markdown 渲染（标题、列表、引用）
- 报告里 LLM 输入包含本时段的 events + tasks 完成情况
- 可固定（置顶）、可导出

### 5.5 任务管理

**核心理念**：任务管理是为打卡服务的，不是为了取代你脑子里的 GTD 系统。所以**不做**：循环规则、子任务深层嵌套、看板视图、依赖关系、提醒推送、健身房式打分。**做**：列表 + 状态 + 优先级 + 打卡联动。

**任务列表（独立窗口或回顾页里的一个 tab，700 × 500）**：

```
┌────────────────────────────────────────────────────────┐
│ 任务   [🔍 搜索]  [+ 新增 ⌘N]                          │
├────────────────────────────────────────────────────────┤
│ [全部] [今天] [本周] [无期限]   [#工作] [#家务] [#学习] │
├────────────────────────────────────────────────────────┤
│ ▼ 进行中 (2)                                            │
│   🔥 ① 调研 Tauri 2 menubar 用法    #life-log  3h▶     │
│   ② 安装洗碗机                       #家务     1h▶     │
│ ▼ 待办 (5)                                              │
│   ① 写打卡弹窗的前端                 #life-log  ~2h    │
│   ② 把秋装从衣柜搬到收纳箱           #家务      ~30m   │
│   ...                                                  │
│ ▼ 已完成（最近 7 天，4）                                │
│   ✓ 把洗碗机搬下楼                  6/5  耗时 25m      │
│   ...                                                  │
└────────────────────────────────────────────────────────┘
```

- 默认按状态分组：进行中、待办、已完成（最近 7 天）
- 优先级 ① ② ③ ④ ⑤ 数字徽章；① ② 高亮（红/橙）
- 每条任务右侧显示**已用时**（来自所有 `relation='work_on'` 的 checkin 累加，每条按 60 分钟估）或**预估时长**
- 点 ▶ 按钮 = "现在开始做这个" → 状态变 in_progress + 立即弹打卡窗口，文字框预填 "在做 [任务名]"
- 右键菜单：编辑 / 标记完成 / 取消 / 归档 / 删除
- 拖拽改优先级（同状态分组内）

**任务详情页（独立小窗 或 列表右侧抽屉）**：

```
┌──────────────────────────────────────────┐
│  ✅ 安装洗碗机                            │
│  优先级 ②   #家务                         │
│  创建于 6/5 14:30   开始于 6/7 10:00      │
│                                          │
│  描述：                                   │
│  ┌──────────────────────────────────────┐│
│  │ 拆包装、接水管、装到橱柜里。           ││
│  └──────────────────────────────────────┘│
│                                          │
│  相关打卡（3 次，累计 ~2h 30m）：         │
│  • 6/7 10:53  "在装洗碗机，先拆包装"     │
│  • 6/7 11:51  "接水管，有点漏"           │
│  • 6/7 12:48  "好了，装上柜了"  ✓ 完成   │
│                                          │
│  [编辑] [标记完成] [归档]                 │
└──────────────────────────────────────────┘
```

**添加任务**：
- 全局快捷键 ⌘⇧T 或菜单栏点 "添加任务"
- 弹一个小窗（400 × 300）
- 文字框：自然语言，比如"明天前把洗碗机装完，优先级高"
- LLM 提取 → 预览：标题、优先级、标签、due_at、estimate_min
- 也可点 "切换为表单模式" 手动填字段
- 也支持语音

**状态流转规则**：
- `pending` → `in_progress`：手动开始 或 LLM 在 checkin 里识别到 `task_action: work_on` 触发
- `in_progress` → `done`：手动完成 或 LLM 识别到 `task_action: completed_by` 触发
- `done` → `archived`：完成超过 30 天自动归档（也可手动）
- 任何状态 → `cancelled`：手动
- `archived` / `cancelled` 不再显示在主列表

### 5.6 设置页

```
通用
├─ 启动时自动运行           [ ✓ ]
├─ 菜单栏图标显示样式       [简约 ▼]
└─ 主题                    [跟随系统 ▼]

打卡
├─ 打卡时段                09:00 — 22:00
├─ 间隔                    [60 分钟 ▼]
├─ 勿扰时段                23:00 — 08:00
├─ 最多稍后次数            [3 ▼]
├─ 全局快捷键              ⌘⇧L
└─ 自动关联任务            [✓] 默认 5 秒后自动确认 LLM 推荐的关联

任务
├─ 添加任务快捷键          ⌘⇧T
├─ 自动归档                [✓] 完成 30 天后归档
├─ 已用时估算              [按 60 分钟/打卡 ▼]
└─ 默认优先级              [③ ▼]

LLM
├─ Provider                [DeepSeek ▼]
│   预设：Claude / DeepSeek / Kimi / 通义 / SiliconFlow / OpenRouter / Ollama / 自定义
├─ Base URL                https://api.deepseek.com/v1
├─ API Key                 ••••••••
├─ Model                   deepseek-chat
└─ [测试连接]

STT
├─ Provider                [本地 SenseVoice ▼]
│   预设：faster-whisper / SenseVoice / OpenAI / SiliconFlow
├─ 模型路径                ~/.../models/sensevoice-small
└─ [测试录音]

数据
├─ 数据目录                ~/Library/Application Support/com.life-log/data
├─ [更改…] [打开目录]
├─ 自动备份                [✓] 每日，保留 30 天
└─ [导出全部为 JSON] [导出物品为 CSV]

关于
└─ Life-Log v0.1.0 · 数据完全本地 · MIT License
```

---

## 6. LLM 抽象层

### 6.1 Trait 定义

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;

    /// 普通对话
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String>;

    /// 结构化输出，schema 是 JSON Schema
    async fn chat_structured(
        &self,
        messages: Vec<ChatMessage>,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value>;

    /// 健康检查（测试连接按钮调用）
    async fn ping(&self) -> Result<()>;
}
```

### 6.2 内置 provider

| Provider | 模块 | 结构化输出方式 |
|---|---|---|
| Anthropic（Claude） | `anthropic.rs` | tool_use |
| OpenAI 兼容（DeepSeek/Kimi/通义/SiliconFlow/OpenRouter/自定义） | `openai_compat.rs` | `response_format: {type: "json_object"}` + schema 注入到 prompt |
| Ollama 本地 | `ollama.rs` | `format: "json"` + schema 注入到 prompt |

### 6.3 预设清单（设置页下拉）

```rust
const PRESETS: &[(&str, &str, &str)] = &[
    ("Claude Haiku",     "https://api.anthropic.com",         "claude-haiku-4-5-20251001"),
    ("DeepSeek",         "https://api.deepseek.com/v1",       "deepseek-chat"),
    ("Kimi (Moonshot)",  "https://api.moonshot.cn/v1",        "moonshot-v1-8k"),
    ("通义千问",          "https://dashscope.aliyuncs.com/compatible-mode/v1", "qwen-turbo"),
    ("SiliconFlow",      "https://api.siliconflow.cn/v1",     "Qwen/Qwen2.5-7B-Instruct"),
    ("OpenRouter",       "https://openrouter.ai/api/v1",      "deepseek/deepseek-chat"),
    ("Ollama (本地)",     "http://localhost:11434/v1",         "qwen2.5:7b"),
];
```

### 6.4 错误处理与重试

- 网络错误 → 指数退避，重试 3 次
- 结构化输出格式不合法 → 重试一次，强化 prompt
- 仍失败 → 保存原始 raw_text，event 入库但 structured 为 null，菜单栏图标变红，错误通知
- 关键：**永远不因为 LLM 失败而丢用户输入**

---

## 7. STT 抽象层

### 7.1 Trait

```rust
#[async_trait]
pub trait SttProvider: Send + Sync {
    fn name(&self) -> &str;
    fn is_local(&self) -> bool;

    /// 转写一段音频文件
    async fn transcribe(&self, audio_path: &Path, lang: Option<&str>) -> Result<String>;

    async fn ping(&self) -> Result<()>;
}
```

### 7.2 内置 provider

| Provider | 模块 | 备注 |
|---|---|---|
| 本地 faster-whisper | `whisper_local.rs` | 调 whisper.cpp 二进制 |
| 本地 SenseVoice | `sensevoice.rs` | 中文最准，调 funasr-onnx |
| OpenAI Whisper API | `openai_whisper.rs` | $0.006/min |
| SiliconFlow | `siliconflow_stt.rs` | 国内便宜替代 |

### 7.3 首次启动 STT 模型下载

- 默认推荐 SenseVoice Small（~250MB）
- 第一次启动时引导下载：进度条 + 取消按钮
- 下到 `~/Library/Application Support/com.life-log/models/sensevoice-small/`
- 可在设置里删除/重下

---

## 8. HTTP 本地接口（为手机端预留）

后端启动一个 `127.0.0.1:39152` 的 HTTP server，**只绑 localhost**，加 token 鉴权（settings 里生成一次性 token，复制到手机端配置）。

```
POST /api/v1/events           # 新增事件
GET  /api/v1/events?since=…   # 查询
GET  /api/v1/items
POST /api/v1/upload           # 上传音频/图片
POST /api/v1/checkin          # 主动触发一次打卡（手机收到通知点开调这个）
GET  /api/v1/tasks?status=…   # 查询任务列表
POST /api/v1/tasks            # 新建任务
PATCH /api/v1/tasks/:id       # 改状态/字段
POST /api/v1/tasks/:id/link   # 手动关联到一个 event
```

第一阶段只实现这些端点，UI 不暴露，等以后做手机端再启用。

---

## 9. 错误与边界场景

| 场景 | 处理 |
|---|---|
| 录音权限被拒 | 弹引导，跳到系统设置 |
| LLM 网络失败 | 保留原文，提示稍后重试，菜单栏变红 |
| STT 模型未下载 | 引导下载；下载完成前不能用语音 |
| 磁盘空间不足 | 备份失败时通知，主写入也失败时菜单栏变红 |
| 数据库损坏 | 启动时检测，自动回退到最近一次备份 |
| 用户改系统时间 | 用毫秒时间戳记录，回看时按 ts 排序，不依赖系统当前时间 |
| Mac 休眠中错过打卡 | 醒来时若漏掉的打卡 ≤ 30min，补弹一次；否则跳过并记录 expired |
| 多次"稍后" | 上限 3 次，超过自动跳过 |
| 一次录音同时按了多个 STT | 排队，按顺序处理 |
| LLM 返回非法 JSON | 重试一次；仍失败按"结构化失败"处理 |
| LLM 把打卡错关联到任务 | 5 秒确认窗口内可改/可忽略；事后任务详情页里可删 task_events 行 |
| LLM 把同一句话识别成多个任务 | 预览页让用户勾选哪些真的要建 |
| 任务关联的 event 被删 | task_events 行 ON DELETE 级联清掉；任务本身不受影响 |
| 任务积压到上百条 | 列表只默认显示 pending+in_progress；done/cancelled/archived 折叠且分页 |

---

## 10. 隐私与安全

- 所有数据默认存本地，不上传任何云
- LLM API key 用 macOS Keychain 存
- LLM 调用时只把当条 raw_text 发出去，**不发历史数据**
- STT 默认本地，云端 STT 调用前用户须勾选确认
- HTTP 接口只绑 127.0.0.1，token 鉴权
- 录音/照片文件存在本地，可一键删除全部

---

## 11. 度量指标（自己评估这个 App 是否成功）

3 个月后看：
- **打卡响应率**：每天有多少次按时打卡 / 应弹次数。> 50% 算成功
- **物品录入数**：录到 50 件 = 真在用；< 20 件 = 失败
- **回顾页打开次数**：每周 > 2 次 = 数据真有价值；从不打开 = 沦为数据坟场
- **AI 报告生成数**：> 4 份 = 形成习惯
- **任务完成率**：done / (done + cancelled + 创建超 30 天仍 pending)。> 60% = 任务管理对你有用；< 30% = 你在用它当"愿望清单"而非"待办清单"
- **打卡 × 任务关联率**：有 linked_task 的 checkin / 全部 checkin。> 25% = 联动产生价值；< 10% = 任务和打卡其实是两个世界，要么重新设计要么砍掉任务

若 3 个月后任何一项远低于目标，认真考虑停掉相应模块而非继续投入。

---

## 12. 已知风险

1. **每小时打卡的疲劳**：参考学术界 ESM 研究，2 周后响应率会跌到 40%。对策：默认间隔可调到 2h、4h；勿扰时段；"暂停 1 小时"按钮；不强制必答
2. **物品录入的半途而废**：80% 的人录到 20 件就放弃。对策：第一周给一个"批量录入向导"，30 分钟集中录主要物品；不追求 100% 覆盖
3. **AI 结构化失误**：LLM 偶尔会把"我在写代码做生活记录" 理解成两件事。对策：所有结构化结果可手动改；保留原文永远不丢
4. **数据格式锁定**：自己造的数据库，离开这个 App 数据就死了。对策：JSON/CSV 导出从第一版就有；schema 文档化（这份文档）；将来万一弃用，导出后 grep 也能用
5. **任务管理功能爆炸**：用户用着用着会说"加个看板视图吧"、"加个 deadline 提醒吧"、"加个甘特图吧"、"加个标签 emoji 吧"——每条单看合理，加完就成了"又一个半残的 Todoist"。对策：明确锚点是"任务为打卡服务"，任何新功能必须先答出"这能让打卡数据更有价值吗？" §1.3 划界、§5.5 的 "**不做** 列表"是底线，违反就拒绝
6. **任务管理变成第二个收件箱**：盲目添加任务、不做、不归档，半年后列表 200 条 pending，反而焦虑。对策：60 天未动的 pending 自动提示"是否归档"；度量 §11 的"任务完成率"指标定期看；不做就承认不做

---

## 13. 开放问题（后续讨论）

- [ ] 物品要不要支持"位置变更历史"（从客厅搬到卧室）？
- [ ] 时间轴上要不要叠加日历事件？
- [ ] AI 报告要不要支持自定义 prompt 模板？
- [ ] 是否做 CLI 工具（不开 App 也能记录）？
- [ ] 同步问题终极方案：自建 sync server vs CRDT vs 始终 Dropbox？
