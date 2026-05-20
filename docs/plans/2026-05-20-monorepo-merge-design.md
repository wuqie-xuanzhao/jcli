# J Monorepo 合并设计

**日期**: 2026-05-20
**状态**: 已确认，执行中

## 决策记录

| 决策 | 选择 |
|------|------|
| Base repo | jcli |
| Phase 1 策略 | 只合并不拆分 crate |
| 前端资源 | web/ 和 remote/ 全部保留 |
| 项目名 | j |

## 目录结构

```
j/                              # Monorepo root（原 jcli repo）
├── crates/
│   ├── j-agent/                # Agent 引擎库（从根目录移入）
│   └── j-cli/                  # CLI 二进制 + TUI（从根目录移入）
│
├── apps/
│   ├── desktop/                # Tauri 桌面端（原 j-gui）
│   │   ├── src-tauri/
│   │   ├── src/
│   │   ├── packages/           # @jgui/shared, @jgui/core, @jgui/ui
│   │   └── ...
│   ├── docs/                   # 文档网站（原 jcli/web/）
│   └── remote/                 # 远程控制前端（原 jcli/assets/remote/）
│
├── Cargo.toml                  # Workspace root
├── Makefile                    # 统一构建入口
└── ...
```

## Cargo Workspace

```toml
[workspace]
members = [
    "crates/j-agent",
    "crates/j-cli",
    "apps/desktop/src-tauri",
]
resolver = "2"
```

- `j-cli` → `j-agent = { path = "../j-agent" }`
- `src-tauri` → `j-cli = { path = "../../../crates/j-cli" }`

## 构建系统

根 Makefile 委托调用子目录 Makefile，各子项目可独立开发。

## CI

路径过滤：`crates/**` 触发 CLI 构建，`apps/desktop/**` 触发桌面端构建，tag 发布走 crates.io。

## 执行步骤

1. 创建目录结构（crates/、apps/）
2. 移动 j-agent/ → crates/j-agent/
3. 移动 j-cli 根目录文件 → crates/j-cli/
4. 导入 j-gui 到 apps/desktop/
5. 移动 web/ → apps/docs/、assets/remote/ → apps/remote/
6. 更新 Cargo.toml（workspace members + path deps）
7. 更新构建配置（Makefile、CI）
8. 验证构建

## 风险点

- src-tauri kernel adapter 的 `j_cli::` import 需验证路径变更后是否正常
- assets/remote/ 构建产物输出路径需调整
- j-cli 内嵌资源路径（rust-embed）可能受影响
