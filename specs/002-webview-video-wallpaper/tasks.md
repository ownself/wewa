# WebView 视频壁纸：分步任务与验收

## 第一版范围

- 使用现有 Wry/WebView，不新增原生播放器、转码库或 GPU 渲染库。
- 本地 `.mp4`、`.m4v`、`.webm`、`.mov`、`.ogv` 文件自动生成播放器页面；扩展名仅识别输入，不保证其编码可播放。重点验收 H.264 MP4。
- 静音、自动循环；默认 `contain`（等比完整显示、黑色背景、居中），`--video-fit cover`（等比放大、居中裁剪）。视频内嵌黑边不自动去除。
- 复用当前显示器选择、桌面嵌入、IPC 和退出逻辑。每屏独立播放，不保证帧同步或共享解码。
- 视频使用专用 HTTP 路由，直接读取原文件；不复制视频、不写入视频所在目录、不把整个文件读入内存。
- 页面不使用 Canvas、逐帧 JavaScript 或定时重试。硬件解码由 WebView/系统选择，不承诺硬件解码一定启用。
- 远程视频、播放列表、音频开关、严格无缝循环、自动暂停、原生后端不在本版范围。

## 实现方案

`src/video.rs` 负责识别、显示模式、最小 HTML 和媒体事件。`src/server.rs` 提供静态目录模式和单视频模式。单视频模式只公开播放器 HTML 与指定视频，不公开视频所在目录。`src/main.rs` 将本地视频分支接入现有启动流程。平台 WebView 使用现有自动播放策略并为视频接入 IPC 错误反馈。

HTTP 约定：GET 完整响应 200；单字节区间响应 206，支持 `start-end`、`start-`、`-suffix`；有效但无交集的区间返回 416 及 `Content-Range: bytes */length`。畸形/未知单位/多区间 Range 忽略并返回 200（不伪造 multipart）；HEAD 忽略 Range，返回完整资源头且无正文。含 If-Range 的请求保守回退 200，不宣称支持验证器。设置正确 MIME、长度和 Accept-Ranges；保持 no-store，不实现媒体缓存。播放期间应保持原文件不变，本版不提供文件变化检测或跨请求快照一致性。普通静态目录仍需阻止路径穿越及符号链接逃逸。

## 状态说明

勾选表示对应代码和自动测试完成；真实桌面、解码和功耗验收独立记录，不能以编译或 HTML 文本测试替代。

## 阶段 1：输入和播放器（可独立验收）

- [x] T001 在 `src/video.rs` 添加大小写无关扩展名识别和 `VideoFit`（contain / cover）。
- [x] T002 在 `src/cli.rs` 增加 `--video-fit`；非法值报错，不能与停止/内置模式组合，显式用于非本地视频时报错。
- [x] T003 构造黑底、无边距、无滚动条、静音自动循环页面；默认 contain，cover 不拉伸。媒体 URL 固定，不插入原文件名。
- [x] T004 添加识别、CLI 分发和 HTML 单元测试。

验收：默认及显式两种模式正确分发；shader 的 --scale/--time-scale 含义不变；路径、文件名不成为 HTML/JS 注入入口。

## 阶段 2：文件服务（可独立验收）

- [x] T005 将 `fs::read` 全量响应改成 File + Seek + Take 的流式响应。
- [x] T006 实现单范围、完整 GET、HEAD、416、正确长度/MIME；明确不支持多区间及 If-Range 验证器的回退行为。
- [x] T007 添加单视频专用路由，只暴露内存中的 HTML 和原视频；补全视频 MIME。
- [x] T008 正确处理 UTF-8/百分号、字面量 +、查询串、路径穿越、符号链接逃逸；构造 URL 时编码文件名。
- [x] T009 限制应用层请求工作线程数量（默认 4，按屏数增加，最多 64）；空闲线程阻塞等待请求，不定时轮询，由 shutdown 显式唤醒退出。活动网络传输退出仍依赖客户端断开或响应结束。
- [x] T010 添加真实 loopback HTTP 测试，验证状态、响应头、正文、并发 range、HEAD、错误区间、空文件和路由隔离；用稀疏大文件验证小范围读取。

验收：HTTP 小范围只返回对应内容；响应不按视频大小分配内存；中文及特殊字符文件名正确；原有 HTML/CSS/JS/shader 纹理仍可加载。

## 阶段 3：启动与错误反馈

- [x] T011 在 `src/main.rs` 接入本地视频流程，无临时视频副本，启动前验证视频文件存在且是普通文件；记录实际服务器端口。
- [x] T012 在 `src/platform/{windows,linux,macos}/wallpaper.rs` 接入媒体 IPC；播放失败输出可读错误，页面显示错误提示，verbose 输出播放信息。
- [x] T013 保持原有 WebView2/WKWebView/WebKitGTK 后端与自动播放配置；不添加默认 GPU 禁用开关或盲目 Chromium 参数。
- [x] T014 在 `README.md` 中更新中英文用法、格式限制、服务器行为、性能说明。
- [x] T015 运行 cargo test、cargo clippy 和格式检查，记录结果及已有告警。

验收：缺失文件/非法模式不会先停止现有壁纸；端口冲突提示仍有效；错误提示不能只依赖开发者控制台；不把 canPlayType 或 playing 事件当作硬件解码证明。

## 阶段 4：真实平台功能验收（发布前）

- [ ] T016 Windows/WebView2：单屏、双屏、contain/cover、循环至少 20 次、Ctrl+C/--stopall、实例替换、端口重新使用。
- [ ] T017 Linux/WebKitGTK：分别验收 layer-shell 和 GNOME 扩展；记录 WebKitGTK/GStreamer/插件/驱动版本。
- [ ] T018 macOS/WKWebView：桌面层、Space 切换、单屏/双屏、自动播放和停止。
- [ ] T019 三平台回归 URL、HTML 文件/目录、本地 Shader、内置 Shader 和纹理。
- [ ] T020 用中文/空格/+/#/%/& 文件名、只读视频目录、损坏 MP4 和不支持编码验证加载/错误提示。

验收：保比例且无变形；cover 消除外部比例黑边；无控件/白边/滚动条/鼠标捕获；每屏独立适配。记录循环停顿，不要求严格无缝。不扩大现有各平台 --stop 单屏行为的承诺。

## 阶段 5：性能验收与后续决策

- [ ] T021 准备 H.264 MP4：1080p30、4K30、4K60；不同显示比例测试；保留样本编码参数和时长。
- [ ] T022 单屏/双屏各预热 1 分钟后采样至少 5 分钟，记录总进程树 CPU/内存、GPU Video Decode/3D、掉帧、功耗（若可测）。
- [ ] T023 通过平台媒体诊断/日志确认实际解码路径，记录软件回退；不得从 GPU 3D 占用推断硬解。
- [ ] T024 大视频连续循环至少 30 分钟，观察 Rust 服务进程内存不随文件大小/循环次数持续增长，记录 WebView 缓冲内存。
- [ ] T025 与空白 WebView 和系统播放器对比；若额外 CPU/功耗不可接受，先定位解码、拷贝、多屏或服务器问题，再决定是否增加原生后端。

记录表：

| 系统/后端/版本 | GPU/驱动 | 编码/分辨率/FPS | 屏数/模式 | 实际解码器证据 | CPU | 进程树内存 | Video Decode/3D | 掉帧 | 功耗 | 循环/停止结果 |
|---|---|---|---|---|---|---|---|---|---|---|
| 待实测 | | | | | | | | | | |

## 初版自动验证记录

- 环境：Windows x86_64，Rust 1.92.0；没有新增 Cargo 依赖。
- `cargo build --locked` 成功；CLI 帮助展示 contain/cover，缺失视频和非本地视频的 --video-fit 均提前报错，退出码为 1。
- `cargo test --locked`：41 项通过。包含 loopback HTTP、并发 range、HEAD、路由隔离、大文件小范围及空闲服务器端口释放测试；Unix 符号链接测试尚未在本机执行。
- `cargo clippy --locked --all-targets`：命令成功，保留 9 项既有类型的告警（builtin/IPC、窗口样式、main 的路径前缀和参数过多）；没有声称零告警。
- 所有变更 Rust 文件通过独立 rustfmt 检查（`skip_children=true`）。全仓库 `cargo fmt -- --check` 仍受未修改的 `src/builtin.rs`、`src/config.rs`、`src/ipc.rs`、`src/platform/linux/mod.rs` 既有格式差异影响，没有为本功能改动这些文件。
- 生成的播放器 JS 经 Node VM 模拟检查：首次播放消息、自动播放拒绝、媒体编码错误均通过；此项不是实际 WebView 解码或 GPU 测试。
- `git diff --check` 通过。
- Linux/macOS 编译、真实桌面播放、格式兼容性、硬件解码和功耗均未验收；T016–T025 保持未完成。

## 循环反馈澄清与局部回退

用户报告的 release 程序实际是遗留的 `target/release/webwallpaper.exe`，当前 Cargo 包名为 `wewa`，新构建产物是 `target/release/wewa.exe`。用户据此撤回“新版无法自动循环”的问题反馈；目前没有确认第一版原生 loop 失效的证据。

因此撤销临时的 T026–T028 手动重播方案：恢复媒体元素原生 `loop`，删除 ended/seek/play 状态机、重入保护和循环计数日志，删除仅随此方案新增的 `tests/video-player.test.cjs`。保留第一版输入识别、contain/cover、播放错误提示、IPC 和文件服务；Rust 测试改为验证原生 loop 且不存在手动重播或定时轮询。

回退后 `cargo test --locked` 42 项通过，`cargo build --release --locked` 成功；本轮未运行真实桌面播放。

真实 WebView 连续循环验收仍按 T016–T018/T024 执行，不以页面属性测试替代。复测应运行 `cargo run --release --locked -- <视频路径>` 或 `target/release/wewa.exe`，不要运行遗留名称的程序。

## 拆分顺序

1. 阶段 1 和阶段 2 可以独立开发、分别自动验收。
2. 阶段 3 依赖阶段 1/2，形成可手动试用版本。
3. 阶段 4 是跨平台功能门槛；阶段 5 决定是否继续优化，不能预先标记完成。
4. 后续自动暂停/多屏共享/原生播放器另立任务，不阻塞这一版最小实现。
