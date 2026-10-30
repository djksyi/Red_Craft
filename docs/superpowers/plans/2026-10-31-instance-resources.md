# 实例资源管理实施计划

目标：侧边栏与启动页均可进入当前实例的管理页面，顶部按模组、存档、光影、材质包、截图切换，选中分类使用主题色背景，分割线下显示本地资源。

方案：沿用 Svelte 页面状态和主题变量，复用 InstalledMods；其余资源由 Tauri 读取实例实际游戏目录，兼容版本隔离与共用目录。截图用受限文件授权预览。启动页入口宽度为启动按钮的 30%，高度保留可点击尺寸，文字换行。

- [x] 新增 resources.rs：按固定分类扫描本地资源，仅枚举对应类型，目录缺失返回空列表，拒绝越界路径；添加扫描测试。
- [x] 在 api.ts 注册资源类型和调用，在 lib.rs 注册命令。
- [x] 新建 Manage.svelte：显示当前实例和五个分类、分割线、资源列表、加载/空/错误状态、刷新和打开目录；模组沿用现有组件。
- [x] 在 Page、App、Sidebar 注册管理页面，在 Launch 添加入口并保持 30% 宽度比例。
- [x] 执行前端检查、构建、已有测试，检查布局与跳转。
- [x] Rust 编译及扫描测试：安装工具链后执行 `cargo test --workspace --release --locked`，130 项通过、0 失败、17 项按项目设置忽略；新增 3 项资源扫描测试全部通过。

重点验证：目录不存在、分类过滤、排序、目录越界、实例切换时旧请求不覆盖当前列表、最小窗口下按钮可用。

验证结果：`pnpm check` 0 错误 0 警告；`pnpm build` 成功；`pnpm test` 11/11 通过。使用 Edge 无头浏览器及模拟 Tauri IPC 验证两个入口、五个分类、主题色与透明背景、分类切换的旧请求丢弃、当前实例参数、截图预览、两种布局下 880px 窗口按钮宽度比例、深色/毛玻璃、错误重试和无实例状态。该交互检查不能替代真实 Tauri 后端验证。

2026-10-31 补充：Rust 正式版编译成功，`cargo test --workspace --release --locked` 通过。Windows x64 主程序为 `target/release/red-craft.exe`；NSIS 安装包为 `target/release/bundle/nsis/RCL_1.1.1_x64-setup.exe`；MSI 安装包为 `target/release/bundle/msi/RCL_1.1.1_x64_zh-CN.msi`。`pnpm tauri bundle --bundles nsis,msi` 退出码为 0。尚未实际安装运行这些安装包。
