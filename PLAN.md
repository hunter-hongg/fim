# Fim — 模态编辑器设计

## 项目概述

Rust 编写的模态编辑器（类 Vim），终端 TUI，插件支持 Lua（嵌入）+ Python（RPC）。

## 架构

```
┌─────────────────────────────────────────────────┐
│                    fim (binary)                  │
├──────────┬──────────┬───────────────────────────┤
│   Core   │    UI    │      Plugin System        │
│          │ (ratatui)│                           │
│ ┌──────┐ │ ┌──────┐ │ ┌───────────┐ ┌────────┐ │
│ │Buffer│ │ │Screen│ │ │LuaRuntime │ │PyRPC   │ │
│ │Mode  │ │ │Editor│ │ │(mlua嵌入) │ │(子进程) │ │
│ │Keymap│ │ │Status│ │ │PluginMgr │ │        │ │
│ │Cmd   │ │ │CmdLn │ │ │EventBus  │ │        │ │
│ └──────┘ │ └──────┘ │ └───────────┘ └────────┘ │
├──────────┴──────────┴───────────────────────────┤
│              Shared: Event types,                │
│              PluginAPI trait, Config types        │
└──────────────────────────────────────────────────┘
```

### 约束

- **Core** 不依赖 UI 和 Plugin，纯数据结构和逻辑
- **Plugin** 通过 EventBus 订阅/发布事件，不直接调用 Core
- **UI** 只负责渲染和转发输入事件到 Core

## 技术选型

| 层 | 选型 |
|---|---|
| TUI 框架 | ratatui |
| 异步运行时 | tokio |
| Lua 绑定 | mlua |
| Python 绑定 | 子进程 RPC（JSON-RPC over stdin/stdout） |
| 序列化 | serde + serde_json |

## 各模块设计

### 1. Core

**Buffer (`core/buffer.rs`)**
- `struct Buffer { id, lines: Vec<Line>, path: Option<PathBuf>, dirty: bool }`
- 行操作：insert, delete, split, join
- undo/redo（基于操作栈，MVP 先不做，预留 trait）

**Mode (`core/mode.rs`)**
- `enum Mode { Normal, Insert, Visual, CommandLine }`
- 每个 Mode 有自己的 keymap 表
- 状态转换由 Core 驱动

**Keymap (`core/keymap.rs`)**
- `struct Keymap { map: HashMap<Vec<Key>, Mapping> }`
- `enum Mapping { Edit(Vec<Action>), Command(String), LuaFn(String) }`
- 前缀键支持（如 `gg`, `dd`）

**Command (`core/command.rs`)**
- `struct Command { name: String, args: Vec<Arg>, run: fn(&mut EditorState) }`
- 内置命令：`:w`, `:q`, `:wq`, `:e`, `:set`
- 插件可注册新命令

### 2. UI（ratatui）

**Screen (`ui/screen.rs`)**
- 主渲染循环，每 tick 调用 ratatui::Terminal::draw
- 从 Core 读取 EditorState，渲染到 buffer

**组件：**
- `EditorWidget` — 文本编辑区，带行号、光标、语法高亮（MVP 用简单高亮，后续接入 tree-sitter）
- `StatusLineWidget` — mode 指示器、文件名、光标位置、dirty 标记
- `CommandLineWidget` — `:` 模式输入

**输入处理：**
- 从 stdin/term 读取原始按键事件（crossterm::event）
- 转为内部 `Key` 枚举，发送到 Core
- Core 返回 `ActionResult`，UI 据此更新状态

### 3. Plugin 系统

**EventBus (`plugin/event_bus.rs`)**
- `struct EventBus { subscribers: HashMap<EventType, Vec<Box<dyn Plugin>>> }`
- 事件类型：BufWritePost, BufReadPost, ModeChanged, KeyPressed, CmdRun

**Plugin trait (`plugin/mod.rs`)**
```rust
trait Plugin {
    fn name(&self) -> &str;
    fn on_event(&mut self, event: &Event, state: &mut EditorState) -> Result<()>;
}
```

**LuaRuntime (`plugin/lua_runtime.rs`)**
- 基于 mlua 嵌入 Lua 5.4
- 提供 `vim` 全局表：`vim.api`, `vim.keymap`, `vim.cmd`, `vim.buf`, `vim.opt`
- 插件目录 `~/.config/fim/lua/*.lua`，自动加载
- 安全沙箱：限制内存/CPU 使用

**PyRPC (`plugin/py_rpc.rs`)**
- MVP 先定义协议，不做实现
- Python 插件作为子进程启动，通过 stdin/stdout 交换 JSON 消息
- 协议：`{"method": "on_event", "params": {...}, "id": 1}`
- 超时和崩溃恢复机制

## MVP 范围

- [x] 打开/编辑/保存文件
- [x] Normal / Insert / Visual / CommandLine 四种模式
- [x] 基础光标移动 (h/j/k/l, w/b, 0/$, gg/G)
- [x] 基础编辑 (i, a, o, x, dd, yy, p)
- [x] Undo (一次)
- [x] `:w`, `:q`, `:wq`, `:e`, `:q!`
- [x] Lua 插件加载与基本 API (`vim.keymap.set`, `vim.cmd.add`)
- [x] 配置文件 `~/.config/fim/init.lua`

## 非 MVP（后续迭代）

- tree-sitter 语法高亮
- LSP 支持
- 多窗口/分屏
- Python 插件
- 增量搜索 (`/`)
- Undo 树
- 配色方案系统
- 自动补全
- 内置终端

## 目录结构

```
fim/
├── Cargo.toml
├── src/
│   ├── main.rs              # 入口：初始化、事件循环
│   ├── core/
│   │   ├── mod.rs
│   │   ├── buffer.rs
│   │   ├── mode.rs
│   │   ├── keymap.rs
│   │   ├── command.rs
│   │   └── state.rs         # EditorState
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── screen.rs
│   │   ├── editor.rs        # EditorWidget
│   │   ├── status.rs        # StatusLineWidget
│   │   └── cmdline.rs       # CommandLineWidget
│   ├── plugin/
│   │   ├── mod.rs           # Plugin trait, PluginManager
│   │   ├── event_bus.rs
│   │   ├── lua_runtime.rs
│   │   └── py_rpc.rs        # 协议定义，MVP 占位
│   └── shared/
│       ├── mod.rs
│       ├── event.rs         # Event enum
│       ├── key.rs           # Key enum
│       └── config.rs        # Config struct
└── runtime/
    ├── init.lua             # 默认配置
    └── lua/                 # 用户插件目录 (说明文档)
```

## 事件循环

```
loop {
    event = poll_input()     // crossterm 原始按键
    core.handle_key(event)   // 根据当前 mode 匹配 keymap
    plugins.on_event(event)  // Lua 插件收到事件
    render(state)            // ratatui draw
}
```
