## 项目上下文摘要（macos-icon）
生成时间：2026-03-17 14:00:00 +0800

### 1. 相似实现分析
- **实现1**: `script/bundle-macos.sh:30-40`
  - 模式：macOS 打包时直接把 `resources/macos/OnetCli.icns` 拷入 `.app`
  - 可复用：`.app` 资源目录和 `Info.plist` 生成逻辑
  - 需注意：如果 `OnetCli.icns` 过期，打包不会自动从 `logo.svg` 更新

- **实现2**: `.github/workflows/release.yml:115-121`
  - 模式：CI 在 macOS 上执行 `bundle-macos.sh` 和 `bundle-macos-dmg.sh`
  - 可复用：当前发布链完全依赖仓库脚本，不依赖额外打包工具
  - 需注意：只要修正本地脚本，CI 发布链会同步生效

- **实现3**: `logo.svg:53-55`
  - 模式：图标视觉源是 SVG，圆角背景位于透明画布上
  - 可复用：`logo.svg` 可以直接作为 `icns` 的单一真源
  - 需注意：不同系统渲染链对 SVG 透明角处理不一致，Quick Look 会把角烘成白底

### 2. 项目约定
- **命名约定**: 脚本文件使用 kebab-case，资源文件沿用 `OnetCli.icns`
- **文件组织**: macOS 资源位于 `resources/macos/`，打包脚本位于 `script/`
- **代码风格**: Shell 脚本使用 `set -euo pipefail`，路径通过 `SCRIPT_DIR/PROJECT_DIR` 计算
- **导入/依赖**: 优先使用系统自带的 `sips` 与 `iconutil`

### 3. 可复用组件清单
- `script/bundle-macos.sh`: 现有 `.app` 打包入口
- `script/bundle-macos-dmg.sh`: 现有 `.dmg` 打包入口
- `resources/macos/Info.plist`: App bundle 元数据
- `logo.svg`: 图标源文件

### 4. 测试策略
- **验证方式**: 本地脚本执行 + `iconutil` 解包 + 像素级透明度检查
- **关键检查**:
  - `generate-macos-icon.sh` 能输出合法的 `OnetCli.icns`
  - `iconutil -c iconset` 能反解成功
  - 反解后的 `icon_512x512.png` 四角 alpha 为 0

### 5. 依赖和集成点
- **外部依赖**: `/usr/bin/sips`、`/usr/bin/iconutil`
- **内部依赖**: `bundle-macos.sh` 依赖新的图标生成脚本
- **集成方式**: 打包前自动重建 `resources/macos/OnetCli.icns`

### 6. 技术选型理由
- **为什么用这个方案**: 直接用仓库现有 `logo.svg` 作为真源，消除手工维护旧 `.icns` 的偏差
- **优势**: 本地和 CI 行为一致；不需要引入第三方图形工具
- **风险**: `sips` 从 SVG 渲染出的母图是 512，再放大生成 1024 规格；对当前简洁图标足够，但复杂图标可能需要更高精度渲染链

### 7. 关键风险点
- **边界条件**: `qlmanage` 会把透明角渲染成白底，不能用于生成 `.icns`
- **兼容性**: 该脚本依赖 macOS 自带工具，只适合在 macOS 运行
- **工具说明**: 当前会话没有 `desktop-commander`、`context7`、`github.search_code`，本次使用本地源码检索和系统命令完成分析与验证
