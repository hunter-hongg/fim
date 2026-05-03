# Fim MVP 计划

## 目标

可用的模态编辑器，能打开/编辑/保存文件，支持基础 Vim 操作和 Lua 插件加载。

## 实现顺序

### Phase 1: 骨架（核心数据结构 + 事件循环）

**产物：** 能启动、显示空白屏幕、按 q 退出的二进制文件

- [ ] 项目初始化：Cargo.toml（ratatui, crossterm, mlua, tokio, serde, serde_json）
- [ ] `src/shared/key.rs` — `Key` 枚举（Char, Esc, Enter, Backspace, Tab, Ctrl, Alt, Fn, 方向键）
- [ ] `src/shared/event.rs` — `Event` 枚举（KeyPressed, BufWritePost, BufReadPost, ModeChanged, CmdRun）
- [ ] `src/shared/config.rs` — `Config` 结构体（占位，硬编码默认值）
- [ ] `src/core/state.rs` — `EditorState` 聚合所有核心状态
- [ ] `src/core/mode.rs` — `Mode` 枚举 + 转换逻辑
- [ ] `src/ui/screen.rs` — 最小化渲染循环，只显示空白区域和 "NORMAL"
- [ ] `src/main.rs` — 事件循环：poll_input → handle_key → render（目前 key 只处理 Ctrl-C/q）

### Phase 2: Buffer + 光标移动

**产物：** 能打开文件（命令行参数），在 Normal 模式下移动光标

- [ ] `src/core/buffer.rs` — `Buffer` 结构体（lines, cursor pos, path, dirty），支持 `open(path)` 和基本行操作
- [ ] `src/ui/editor.rs` — `EditorWidget` 显示文件内容 + 光标位置
- [ ] `src/core/keymap.rs` — `Keymap` 结构体，支持 Normal 模式绑定
- [ ] 注册 Normal 模式移动 keymap：`h/j/k/l`, `w/b`, `0/$`, `gg/G`
- [ ] 滚动：光标移出可视区域时自动 scroll

### Phase 3: 编辑操作

**产物：** 能修改文件内容并保存

- [ ] 进入/退出 Insert 模式（`i`, `Esc`）
- [ ] Insert 模式：输入字符、Backspace、Enter 换行
- [ ] Normal 模式操作：`x` 删字符, `dd` 删行, `yy` 复制行, `p` 粘贴
- [ ] 单级 Undo（每次进入 Insert 模式前保存快照）
- [ ] Visual 模式：`v` 进入，移动扩展选区，`d` 删除选中
- [ ] `src/core/buffer.rs` 添加 `dirty`/`save()`

### Phase 4: 命令行模式

**产物：** 能执行基础 Ex 命令

- [ ] `src/ui/cmdline.rs` — `CommandLineWidget` 底部输入行
- [ ] `:` 进入 CommandLine 模式
- [ ] `src/core/command.rs` — 命令注册和执行框架
- [ ] 内置命令：`:w`, `:q`, `:wq`, `:e <path>`, `:q!`
- [ ] 命令历史（当前会话内 ↑/↓ 浏览）

### Phase 5: 状态栏

**产物：** 底部信息栏显示模式、文件名、位置

- [ ] `src/ui/status.rs` — `StatusLineWidget`
- [ ] 左侧：mode 名称（带颜色标识）
- [ ] 中间：当前文件名
- [ ] 右侧：行列号 `12:34` + dirty 标记 `[+]`

### Phase 6: Lua 插件系统

**产物：** 能加载 Lua 插件，插件可注册 keymap、添加命令

- [ ] `src/plugin/mod.rs` — `Plugin` trait, `PluginManager`（加载、事件分发）
- [ ] `src/plugin/event_bus.rs` — 事件注册与广播
- [ ] `src/plugin/lua_runtime.rs` — mlua 初始化，提供 `vim` 全局 API
- [ ] `vim.keymap.set(mode, key, callback)` — Lua 插件注册 keymap
- [ ] `vim.cmd.add(name, callback)` — Lua 插件注册命令
- [ ] `vim.buf` 表 — `get_lines()`, `set_lines()`, `get_cursor()`, `set_cursor()`
- [ ] `vim.opt` 表 — 读取/设置选项
- [ ] 自动加载 `~/.config/fim/init.lua`
- [ ] 自动加载 `~/.config/fim/lua/*.lua`
- [ ] `src/shared/config.rs` — 从 `~/.config/fim/init.lua` 读取配置

### Phase 7: 打磨

- [ ] 错误处理（打开文件失败、保存失败、插件崩溃 → 不影响编辑器）
- [ ] 信号处理（SIGWINCH 大小改变重绘）
- [ ] 光标形状切换（Insert 模式竖线 vs Normal 方块）
- [ ] 最小化测试覆盖

## 非 MVP（延迟到 v0.2+）

| 特性 | 原因 |
|---|---|
| tree-sitter 语法高亮 | 依赖大，MVP 用纯黑白色 |
| LSP 支持 | 需要异步协议栈 |
| 多窗口/分屏 | 需要 layout 系统 |
| Python 插件 | 仅定义协议，不做运行时 |
| 增量搜索 `/` | 需要搜索 UI 组件 |
| 配色方案 | 先硬编码，后抽象 |
| 自动补全 | 依赖 LSP |
| 内置终端 | 纯功能扩展 |

## 质量目标（MVP）

- 打开 50MB 文件不崩溃（行数限制或懒加载）
- 所有操作不吃 CPU（无 busy loop）
- 插件 Lua 错误不传播到编辑器核心
- `:w`/`:q` 等高频操作延迟 < 1ms
