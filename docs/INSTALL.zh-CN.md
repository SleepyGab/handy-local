# nibi 简单安装说明

当前源码版本为 nibi v0.0.1，适用于 Apple Silicon Mac。
建议使用较新的 macOS；尚未验证旧版 macOS 和 Intel 机型。

[Release 下载](https://github.com/SleepyGab/nibi/releases)目前仍是旧版
**Handy Local v0.9.8-local.1**，应用名称为 **Handy Local.app**，不包含 nibi 品牌、
自定义词汇和 DeepSeek 文字润色。旧包请参照随包提供的安装说明；以下步骤针对当前 nibi
构建，构建方法见 [Build nibi](../README.md#build-nibi)。

1. 将构建输出中的 **nibi.app** 复制到 **Applications（应用程序）**；若后续提供 nibi
   DMG，也可从 DMG 拖入后推出安装磁盘。从应用程序打开 nibi。
   先退出占用 Fn 的原版 Handy/Wispr Flow。
2. 当前是本地签名的预览版，尚未经过 Apple 公证。若系统阻止打开，确认文件来自维护者或
   本项目 Release 后，由你本人到 **系统设置 → 隐私与安全性 → 仍要打开（Open Anyway）**。
   不需要关闭整个系统的安全检查。若提示文件损坏或含恶意软件，请停止并重新核对下载文件。
   Apple 说明：https://support.apple.com/en-us/102445
3. 允许 **麦克风** 和 **辅助功能**。部分 macOS 将后者称作“Device Control and Data Access”。
   它们用于录音、读取刚输入的文本框和填入文字。完成后退出并重新打开 nibi。
4. 在 **Models（模型）** 下载 **Qwen3-ASR 1.7B**，如可选量化则选 **Q5_K_M**，并选为当前模型。
   模型约 1.5 GB，安装包不包含它。首次下载需要网络，下载完成后的语音转写在本机运行。
   也可以下载多个本地模型，再选择当前使用哪一个。
5. 在 **General → Transcribe Shortcut** 设置 **Fn**，**Shortcut Behavior** 选择 **Hold（按住）**。
   若 Fn 同时弹出系统表情/切换语言，到 macOS 键盘设置将“按下 🌐 键以”设为“不执行操作”。
   中英文混说时，识别语言保持 **Auto Detect**。界面语言可以选 English。
6. 点进文本框，按住 Fn 讲话，松开后文字会自动填入。首次转写加载模型可能稍慢。
7. **Dictionary → Replacement rules（词典 → 替换规则）** 中可管理已记住的词条；改字提示出现后点 **Add? / 记住？**
   才会保存。未能自动检测时可用菜单栏的“纠正上一条”。换电脑时点击 **Export dictionary**
   导出 JSON，在新电脑同一页点击 **Import dictionary** 导入即可，重复词条会跳过。

建议按需开启登录后启动、隐藏窗口、闲置 5 分钟卸载模型。新电脑需自行设置这些偏好；
安装包不含原用户的设置、词典、语音或历史。导入词典只迁移已确认的替换规则，
不包含自定义词汇、模型或历史记录，模型仍需单独下载。
在 **Dictionary → Custom vocabulary（词典 → 自定义词汇）** 中可直接添加人名、术语的正确
拼写，作为本地识别提示，不必先输入错词；提示不能保证识别结果。

语音识别与词典纠正在本地处理。“文字润色”默认关闭；在设置中保存自己的 DeepSeek API key
后，主动开启才会发送转录文字，录音不会上传。较长口述可按内容整理为笔记、任务清单，
或分开呈现讨论、决定和后续行动；重要内容请核对结果。
“已保存的 DeepSeek key”区域显示遮罩和末尾四位，可点击“显示 key”查看全文，或用
“移除 key”清除并关闭润色。本机只保存一个 DeepSeek key，保存新 key 会替换旧 key。
润色失败时返回原文。“最近一次整理”和
历史记录中的“恢复原文”会复制原文，供你粘贴替换。模型下载需要网络，目标聊天软件也可能自行发送你输入的文字。
若不希望备份同步上云，请把导出文件保存到非 iCloud/Dropbox 的本地文件夹。

从旧的临时签名切换到固定证书时，可能需要重新授予一次权限。如果辅助功能开关已开启但仍不工作，可移除旧的
nibi 条目，重新添加应用程序中的 nibi，然后重启应用。
更新后若保存窗口按钮一直灰色，也可先退出并重新打开应用再试。

## 本机构建与权限恢复

`bun run build:local` 现在使用 macOS 钥匙串中的固定代码签名证书。只有一张可用证书时
自动选用；有多张时，用 `HANDY_MACOS_SIGNING_IDENTITY` 指定完整名称或 SHA-1 指纹。
可用 `security find-identity -v -p codesigning` 查看列表。后续更新继续使用同一签名身份。
仓库不保存证书或私钥。Apple Development 可用于自己的 Mac；对外分发应使用 Developer ID，
公证仍需另外配置。

旧的临时签名以每个版本的文件哈希识别应用。第一次切换到固定证书后，可能需要在系统设置
中用减号移除旧的 Handy Local 或 nibi 权限条目，再添加 **/Applications/nibi.app** 并开启。
返回应用时会自动检查，权限页打开期间也会持续检查。等待 15 秒仍未获准时，会显示恢复步骤、
“重新检查”和“重启 nibi”，不再无限转圈。

缺少证书时，构建会停止并给出配置说明。仅做临时测试时可明确设置
`HANDY_ALLOW_ADHOC_SIGNING=1` 和 `HANDY_MACOS_SIGNING_IDENTITY=-`，但这种构建仍会丢失
稳定的权限身份。
