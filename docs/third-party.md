# 第三方组件与许可证

## 7-Zip 26.05（内嵌）

| 项 | 值 |
| --- | --- |
| 文件 | `7z.exe`、`7z.dll`、`7zFM.exe`、`7zG.exe`、`Lang/zh-cn.txt` |
| 位置 | `src-tauri/assets/7zip/`（构建时 `include_bytes!` 进 exe） |
| 运行位置 | `%LOCALAPPDATA%\net.miyin.launcher\tools\7zip-2605\` |
| 上游 | https://www.7-zip.org/ |
| 来源包 | `7z2605-x64.exe`（官方安装包，用 Windows `tar` 解开后取文件；未修改） |
| 许可证 | LGPL-2.1+（`src-tauri/assets/7zip/License.txt` 随包附带） |

用途：

1. `campaign/contents.rs` 解压 7z / rar / tar 等压缩包时优先使用内嵌的 `7z.exe`
   （通过环境变量 `MIYIN_7Z`）；
2. 设置页「7-Zip 解压工具」把 `7zFM.exe` 窗口 `SetParent` 嵌入启动器子窗口，
   压缩 / 解压 / 测试都用 7-Zip 自己的界面。

7-Zip 以 LGPL 分发，本项目未修改其二进制；源码可从 https://www.7-zip.org/source.html 获取。

## react-custom-roulette 1.4.1（前端依赖，大转盘）

| 项 | 值 |
| --- | --- |
| 位置 | `ui/package.json` 的 dependencies；打包进用户侧 bundle |
| 使用点 | `ui/src/react/views/wheel/WheelView.tsx` |
| 上游 | https://github.com/effectussoftware/react-custom-roulette |
| 许可证 | MIT（Copyright (c) 2020 Effectus Software） |
| 随包内容 | 该包的 `dist/bundle.js` 内含 webfontloader 1.6.28（Apache-2.0）与 styled-components 5.x（MIT） |
| 许可全文 | `docs/licenses/react-custom-roulette-MIT.txt`、`docs/licenses/webfontloader-Apache-2.0.txt`、`docs/licenses/styled-components-MIT.txt` |

本项目不改这个库的源码。运行时不向 Google Fonts 发请求：WheelView 只传 WEB_FONTS
白名单里的 `sans-serif`，库只有在字体不在白名单时才调用 `WebFont.load`。