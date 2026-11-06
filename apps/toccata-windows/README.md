# Toccata for Windows

Windows 客户端的独立开发目录。当前提供可运行的最小任务管理版本：任务、子任务、完成/恢复、置顶、计划日期和完成回顾。键盘活跃采集、托盘与安装包尚未实现。

## 运行

环境要求：Windows 10/11 x64，系统自带或已安装 WebView2 Runtime。

双击仓库根目录的 `run.bat`。首次运行会自动：

1. 在 `.tools/rustup`、`.tools/cargo` 安装项目内 Rust 工具链；
2. 从本机的 Visual Studio Build Tools 与 Windows SDK 复制 x64 编译所需文件到 `.tools/msvc`；
3. 构建并启动 `toccata-studio.exe`。

工具链、构建产物与任务数据全部位于项目内（`.tools/`、`data/`），不写入用户目录。首次构建需要联网下载 Rust 与 crates，耗时较长；之后可离线复用。

首次准备工具链要求本机已安装 Visual Studio Build Tools 的「使用 C++ 的桌面开发」组件。脚本只复制所需文件，不修改原安装。

手动构建与运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\windows\build.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\windows\run.ps1
```

## 已实现

- `data/tasks.sqlite` 使用与 macOS 端一致的 schema、WAL 和外键，旧任务库自动补齐 `is_pinned`、`subtask_order`。
- 任务与子任务：新增、编辑标题与 Markdown 详情、计划日期与时间、删除（级联子任务）；子任务最多支持 5 层。
- 完成任务会同时完成未完成后代；恢复任务会清除自身与已完成祖先；新增子任务会恢复已完成的祖先。
- 置顶作用于主任务，子任务跟随整组；列表按完成状态、置顶、手动顺序、创建时间排列。
- 新建任务支持自然语言：`11.10去校医院`、`这周六爬山`、`明天8:30开会`、`下午五点提醒我吃饭`、`周日上午十点`、`2026.11.10 8:30:00 发布` 等。缺省年份/月份使用当前值；只有时间时会自动顺延到下一次。
- 计划日期可只选日期，也可补充具体时间；详情栏默认渲染 Markdown，可切换编辑。
- 任务提醒：可单独开启、设置提前分钟；全局默认不提醒、全屏应用时静默，均可在「设置 → 提醒」中修改。到期会显示应用内弹窗并发送 Windows 通知。
- 关闭窗口：首次关闭会询问「收起到系统托盘 / 退出应用」，默认收起并默认勾选「不再询问」；之后可在「设置 → 窗口」修改。收起后任务栏不保留窗口，可从系统托盘图标重新打开，键盘活动与提醒继续运行。
- 开机自启动：首次运行默认开启，登录 Windows 后自动启动 Toccata；可在「设置 → 通用」关闭。启动项指向当前可执行文件，移动项目目录后需要重新开启一次。
- 界面按 macOS 工作台结构：侧边栏分为「任务」和「时间与活动」，主区域为任务列表 + 详情分栏。
- 视图：任务（本周 / 最近 7 天）、收集箱、任务日历、任务回顾；回顾支持本周 / 最近 7 天并复制为周报素材。
- 任务和收集箱列表只显示未完成任务；已完成任务仍保留在任务日历和任务回顾中。
- 任务日历按月展示创建、计划与完成日期：创建 `#39C5BB`、计划 `#EE0000`、完成 `#66CCFF`；逾期未完成任务的日期格整块标为 `#FFC0CB`。
- 键盘活跃：`WH_KEYBOARD_LL` 采集按键和前台应用，`WH_MOUSE_LL` 记录鼠标左右键次数；显示活跃时间、按键次数、输入速度（键/分钟）、左/右键次数、键盘热力图和 15 分钟时间线。
- 应用与趋势：最近 26 周活跃日历（可按「键盘 / 使用时间」切换）、应用与网页活跃排行（时长、按键次数）与范围切换。浏览器通过地址栏读取完整 URL 并保存在本机，站点统计显示在浏览器下方（如 Edge → 哔哩哔哩 / ChatGPT）；其他窗口记录标题并按应用归类。
- AI 汇总站点：在「应用与网页活跃度」点击「AI 汇总站点」，会把**未识别的域名**（不含完整 URL、窗口标题）发给 DeepSeek 归类（模型 `deepseek-flash`），结果保存在本机 `site_aliases` 表并自动应用到后续统计。API Key 读取 `%APPDATA%\xiaoman\Toccata\cookie\deepseek.txt`，也可用 `DEEPSEEK_API_KEY` 环境变量覆盖。
- 定时任务：官方任务默认每 3 小时自动 AI 汇总站点，可调整间隔或关闭；用户任务支持立即执行，或按间隔、每天、每周执行本地脚本。脚本通过 `cmd.exe` 在项目根目录运行，最长 5 分钟，状态与输出保存在本机。
- 搜索任务名称与详情；列表可切换「仅标题 / 含详情」，并按手动、创建时间、名称或计划时间排序。
- 快捷键：`Enter` 创建任务或子任务，`Ctrl/Cmd+S` 保存，`Esc` 关闭确认框。

## 尚未实现与限制

- Windows 不采集 Fn 键，因此不提供语音输出估算；键盘活跃页改用输入速度（按键次数 ÷ 键盘活跃时间，单位键/分钟）。
- 键盘采集在应用运行期间生效；以普通权限运行，管理员权限窗口内的输入无法捕获。
- 浏览器地址栏需要提供可访问性信息才能读取 URL；读取失败时会回退为按浏览器应用归类。窗口标题与 URL 只保存在本机数据库，界面只显示归类后的站点或应用名称。
- 菜单栏小窗、全局快捷键、喝水提醒、应用内更新。
- 用户定时脚本以当前用户权限执行，运行前请自行确认命令内容；单个脚本最长运行 5 分钟。
- 拖拽排序（列表行已显示把手，但尚未接入拖动）、外观与语言设置、安装包。

## 数据位置

- 任务库：`data/tasks.sqlite`，可用 `TOCCATA_DATA_DIR` 覆盖。
- 键盘与调度库：`data/toccata.db`（`key_events` / `mouse_events` / `site_aliases` / `scheduled_scripts` / `scheduler_config` 表），保存键码、鼠标按键、时间、按住时长、前台进程名、窗口标题、浏览器地址栏完整 URL，以及定时任务配置；不保存输入文本、鼠标坐标或滚轮。界面只显示聚合后的站点/应用名称。
- WebView2 用户数据（包括提醒设置、关闭行为、自启动状态和列表偏好的 localStorage）：`data/webview`。
- 开机自启动使用当前用户的 Windows 启动项（`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`），这是系统状态，不随项目目录保存；关闭自启动会移除该项。
- 数据路径由 `run.bat` 固定到项目内；删除项目目录会同时删除任务与键盘记录，请自行备份。

## 工程结构

```
apps/toccata-windows/
  src-tauri/        Rust + Tauri：窗口、命令、SQLite 任务存储与测试
  ui/               静态 HTML/CSS/JS 界面，无需 Node 或打包器
scripts/windows/
  setup-toolchain.ps1  安装项目内 Rust、复制 MSVC/SDK
  build.ps1            构建
  run.ps1              构建并启动
run.bat                 一键入口（GBK 编码）
```

运行测试：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command ". scripts\windows\env.ps1; cargo test --manifest-path apps\toccata-windows\src-tauri\Cargo.toml"
```

## English

The Windows client is now a minimal runnable task manager. Double-click `run.bat` at the repository root; it installs a project-local Rust toolchain, copies the required MSVC/Windows SDK subset into `.tools/`, builds, and launches the app. All toolchains, build output, and task data stay inside the repository.

Implemented: task/subtask CRUD, Markdown details, planned dates, completion cascade and restore semantics, root pinning, manual ordering, inbox/7-day/week views, and weekly review export. Keyboard capture, tray integration, installer, drag reordering, and calendar are not implemented yet.

Task data uses the same `tasks.sqlite` schema as the macOS app (see `shared/protocol.md`).
