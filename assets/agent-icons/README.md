# 智能体图标

侧栏 Agents 列表与窗格边框标题的内置智能体图标，由 `src/client/agent_icons.rs` 通过 `include_bytes!` 嵌入。

来源：npm 包 `@lobehub/icons-static-png` 2.10.0（[LobeHub Icons](https://github.com/lobehub/lobe-icons)，MIT 许可，见同目录 `LICENSE`）。原图 640×640，统一缩放为 64×64 RGBA PNG。商标归各自所有者。

- `<agent>.png`：彩色图标，深浅主题共用。
- `<agent>-dark.png` / `<agent>-light.png`：单色图标的深色主题版（浅色字形）与浅色主题版（深色字形），分别取自上游 `dark/` 与 `light/` 目录。

| 文件名 | 上游图标 |
| --- | --- |
| amp | amp-color |
| antigravity | antigravity-color |
| claude | claude-color |
| cline | cline |
| codex | codex-color |
| copilot | githubcopilot |
| cursor | cursor |
| devin | devin-color |
| gemini | gemini-color |
| grok | grok |
| hermes | hermesagent |
| kilo | kilocode |
| kimi | kimi-color |
| kiro | kiro-color |
| mastracode | mastra |
| opencode | opencode |
| qodercli | qoder-color |
| qwen | qwen-color |

`pi.png` 不在上游，按 [pi.dev](https://pi.dev/press-kit) 彩色标志（MIT，Earendil Inc. & Contributors）的三块矢量图形裁去留白后栅格化为 64×64。

omp、droid、letta、maki、muse 在上游没有对应图标，侧栏为它们保留同宽空白。
