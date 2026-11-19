# MAGI Launcher

<p align="center">
  <img src="MAGI Launcher.png" alt="MAGI Launcher 截图">
</p>

*[Read in English](README.md) · [Lire en français](README.fr.md) · [日本語で読む](README.jp.md)*

一个适用于 Windows 和 Linux 的快速、功能繁多、纯键盘操作的应用启动器：按下全局快捷键，输入几个字母，回车。它可以启动应用、文件夹和快捷方式；还能当计算器和十六进制颜色预览器；在打开的窗口之间切换；设置快速计时器；保存便签；自动重启你想保持运行的程序；浏览并清空回收站；按名称查找 emoji；控制媒体播放；保留一份可选的、仅存于内存的剪贴板历史；弹出 Windows 自己有时拒绝弹出的 USB 驱动器；并自带 100 多种配色主题。一个小巧轻量的可执行文件，无需安装，除了你自己的快捷方式列表外没有任何需要配置的东西。Linux 上的不同之处见 [Linux](#linux)。

## 功能

- 全局快捷键，可在任何地方呼出／关闭启动器（默认 `Ctrl+Space`，可修改）
- 对你的应用、文件夹和快捷方式列表进行模糊搜索
- 可执行文件旁的 `shortcuts/` 文件夹——把任意文件（`.lnk`、`.bat`、`.cmd`、`.vbs`，什么都行）放进去，它就会自动成为可启动的条目，排在 `apps.json` 自己的条目之后
- 内置计算器（`2*(3+4)` → `= 14`，回车复制结果）
- 十六进制颜色预览（`#3a8ea0` 会把列表填成该颜色，回车复制十六进制代码）
- Window Switcher——模糊搜索并跳转到任意打开的窗口，也可以当场关闭或强制结束它
- 带时长解析的计时器（`5m`、`90s`、`1h`），到点时会触发一个 DVD 弹跳屏保彩蛋
- Sticky Notes——数量不限、可搜索的快速文本便签列表，可当场复制或删除
- Auto-restart——选择任意要保持运行的目标；一旦它的进程消失（崩溃或手动关闭），会在几秒内自动重新启动
- Auto-kill——Auto-restart 的反面：选择要保持关闭的目标，每隔几秒发现就终止。默认关闭，每次启用后有固定的 10 秒宽限期，并有一条“绝不终止 MAGI 自身”的硬性规则
- 实时显示回收站的项目数和大小，可直接在启动器中浏览其中的内容——复制某个项目的名称或单独删除它——一个按键即可清空
- Emoji 选择器——模糊搜索 Unicode 官方 emoji 名称，以彩色显示，`Enter` 复制 emoji 本身
- Copy History——可选的、仅存于内存的复制记录（仅文本）；可在启动器中浏览并重新复制，绝不写入磁盘
- Eject——可搜索的已连接 USB 驱动器列表，`Enter` 当场弹出（如果有程序正在使用，用 `Shift+Delete` 强制弹出）；即使 Windows 自己的“弹出”选项缺失或变灰也能用
- 无需实体媒体键盘即可控制媒体键（播放／暂停、下一首／上一首、音量）
- 100 多种内置配色主题，可在启动器中实时预览并切换，无需重启
- 用键盘实时调整弹出窗口及其边框的大小（`Ctrl+1`–`Ctrl+0` ／ `Ctrl+-`、`Ctrl+=`），立即保存
- 以单实例运行，带通知区域图标（切换快捷键／切换 Auto-restart／切换 Auto-kill／切换 Copy History／Website／退出）
- 对于不匹配任何应用的内容，会像“运行”（Win+R）一样处理

## 键盘快捷键

| 按键 | 操作 |
|---|---|
| 全局快捷键（默认 `Ctrl+Space`） | 显示／隐藏启动器 |
| 输入 | 实时模糊筛选列表 |
| `↑` / `↓` 或 `Ctrl+W` / `Ctrl+S` | 上移／下移选择 |
| `←` / `→` 或 `Ctrl+A` / `Ctrl+D` | 向后／向前翻一页（10 项） |
| `Enter` | 启动所选条目——*（回收站主条目）* 浏览其中的内容而不是清空它——*（回收站浏览视图）* 把高亮项目的完整名称（含扩展名）复制到剪贴板并关闭启动器——*（Copy History）* 重新复制高亮条目并关闭启动器，不会再次加入历史——*（Eject）* 如果没有程序打开高亮的驱动器就弹出它，否则不做任何事（强制弹出见 `Shift+Delete`）；两种情况下启动器都保持打开，方便连续弹出多个 |
| `Shift+Enter` | 在文件管理器中显示所选条目而不是启动它——*（回收站主条目）* 清空它（启动器保持打开）——*（Sticky Notes）* 用默认编辑器打开 `notes.json` 而不是复制 |
| `Tab` | 跳到 Window Switcher——在启动器中的任何位置都一样，不仅限于主列表 |
| `Escape` | 从任何模式返回主列表——如果已经在主列表则关闭启动器。在计时器的 DVD 弹跳屏保期间，会停止弹跳并返回主列表，而不是关闭 |
| `Delete` | *（Window Switcher）* 温和地关闭高亮窗口（`WM_CLOSE`，等同于点击它的 ×）——*（Sticky Notes，在选择器中）* 删除高亮便签——*（主列表中的“Note”条目上）* 不打开选择器，直接删除最新的便签——*（Auto-restart）* 停止监视高亮目标——*（Copy History）* 删除高亮条目——*（回收站主条目）* 清空整个回收站（启动器保持打开）——*（回收站浏览视图）* 只永久删除高亮项目——*（计时器，已启动或在时长输入中）* 取消倒计时 |
| `Shift+Delete` | *（Window Switcher）* 强制结束高亮窗口的进程，用于不再响应 `Delete` 的窗口——*（Sticky Notes）* 删除所有便签——*（Auto-restart）* 停止监视所有目标——*（Auto-kill）* 清除所有终止目标——*（Copy History）* 清空整个历史——*（回收站浏览视图）* 清空整个回收站（启动器保持打开）——*（Eject）* 即使仍有程序打开，也强制弹出高亮的驱动器；在这里单独按 `Delete` 不做任何事 |
| `Ctrl+1`–`Ctrl+9` / `Ctrl+0` | 实时把弹出窗口调整为屏幕宽度的 10%–90% ／ 100%，并立即保存到 `state.json`。在数字需要 `Shift` 的键盘布局（如 AZERTY）上，请同时按住它 |
| `Ctrl+-` / `Ctrl+=` | 实时把边框减小／增大 1px，立即保存。`Ctrl++` 也可以 |
| 搜索框为空时按 `Backspace` | 退出 Window Switcher／主题选择器／计时器／Sticky Notes／Auto-restart／Auto-kill／回收站浏览视图／Emoji 选择器／Copy History／Eject |
| 左键点击通知区域图标 | 显示／隐藏启动器 |
| 右键点击通知区域图标 | 切换快捷键／切换 Auto-restart／切换 Auto-kill／切换 Copy History／Website／退出 菜单 |

搜索框始终保持键盘焦点——上面的每个按键都直接在那里被截获，所以你永远不需要点击或用 Tab 切到别处才能继续输入。鼠标在启动器自己的窗口内没有任何效果（点击、悬停、光标变化都没有）——只有通知区域图标会响应它。`Alt+F4` 永远不会结束启动器的进程——它只是隐藏弹出窗口，和 `Escape` 一样；启动器只会通过通知区域菜单中的“Quit”退出。

## 搜索模式

在搜索框中输入的内容按以下顺序解释：

1. **十六进制颜色**（`#fff` 或 `#3a8ea0`）——把整个列表填成该颜色作为实时预览；`Enter` 把十六进制代码复制到剪贴板。
2. **数学表达式**（必须包含运算符，例如 `2+2`、`100/3`）——显示 `= <结果>`；`Enter` 复制结果。由一个手写的小型递归下降解析器求值，只允许数字和算术运算符——任何可能查找名称、调用东西或超出表达式本身的内容都绝不会被执行。
3. **应用名称**——模糊匹配：查询中的每个词都必须匹配，顺序不限。它的字母必须按顺序出现在名称中且彼此靠近，或者是名称中各个词的首字母（从 3 个字母起），所以 `vsc` 能找到“Visual Studio Code”。单个字母或数字只看名称的开头，然后看各个词的开头。结果排序：完全前缀匹配优先，其次是普通子串，最后是模糊匹配（越紧凑的匹配排得越前）。同一档次内并列时保持条目在 `apps.json` 中的顺序——所以在同样好的匹配中，`apps.json` 里排得越靠前的越先显示。背后完全没有使用历史或频率排序：MAGI 从不记住你之前启动过什么，`apps.json` 中的位置是唯一的决胜因素，无论你选择某项的频率（或最近程度）如何，都始终不变。
4. **其他任何内容**——如果什么都不匹配，`Enter` 会像 Windows 的“运行”对话框（`Win+R`）一样运行原始文本，方式与 `apps.json` 条目的启动相同——打错字后按 `Enter` 也会运行，如果它恰好是某个命令的名称。形如电子邮件地址的查询（`someone@example.com`）会以 `mailto:` 打开，而不是作为不存在的文件路径失败。

## Window Switcher

在启动器中的任何位置按 `Tab`，即可列出所有打开的顶层窗口（标题，以与应用相同的模糊方式筛选）：

- `Enter` 激活高亮窗口
- `Delete` 温和地关闭它（`WM_CLOSE`，等同于点击 ×），并停留在切换器中以便处理下一个
- `Shift+Delete` 强制结束它的进程（`TerminateProcess`），用于不再响应 `Delete` 的窗口
- `Escape` 退出切换器回到主列表，不触碰任何窗口

`Shift+Delete` 特意设计成只比普通的 `Delete` 关闭多一个修饰键：很多窗口——例如所有文件资源管理器的文件夹窗口——和桌面、任务栏运行在同一个进程中（除非启用了“在单独的进程中打开文件夹窗口”），所以对其中一个使用 `TerminateProcess` 会让整个 `explorer.exe` 崩溃，而不只是高亮的窗口。只有在 `Delete` 确实无效时才使用它。

## Timer

添加一个 `"magi:timer"` 条目（见下文）即可解锁——在其中时，搜索框的占位文字会变为 `Type a duration (5m, 90s, 1h...)`。输入时长（`5m`、`90s`、`1h`，或默认以分钟计的单个数字）并按 `Enter` 启动倒计时，它会实时显示在主列表中“Timer”旁边（空闲时为 `Timer: --:--`）。倒计时归零时，弹出窗口会像 DVD 屏保一样在屏幕上四处弹跳，每次碰到边缘都会切换主题——用全局快捷键、在它上面任意位置点击鼠标，或在它拥有键盘焦点时按任意键（`Escape`、`Tab`…）即可关闭。在提醒之前改变主意了？按 `Delete` 取消倒计时，无论你还在时长输入中，还是在主搜索中高亮了“Timer”。

## Sticky Notes

添加一个 `"magi:notes"` 条目（见下文）即可解锁——一个数量不限的快速文本便签本，保存在 `apps.json`／`themes.json` 旁边的 `notes.json` 中。选择它会列出所有便签（最新的在前，实时显示在主列表中“Notes”旁边），可以像 Window Switcher 一样模糊搜索。在其中时，搜索框的占位文字会变为 `Type a note...`。

- 输入与现有便签匹配的文本，`Enter` 把它复制到剪贴板并关闭启动器
- 输入不匹配任何内容的文本，`Enter` 把它添加为新便签，并保持打开以便继续添加
- `Delete` 删除高亮便签，`Shift+Delete` 删除全部便签
- `Shift+Enter` 用关联的编辑器直接打开 `notes.json` 而不是复制——方便手动编辑便签（多行、调整顺序…）
- 搜索框为空时按 `Tab` / `Escape` / `Backspace` 关闭选择器

要删除最新的便签，甚至不需要打开选择器：直接在主列表中的“Note”条目上按 `Delete`，就像不打开输入框直接取消计时器一样。

## Auto-restart

添加一个 `"magi:auto-restart"` 条目（见下文）即可解锁——一个由 MAGI 在后台保持运行的目标列表（任意 `path`，格式与 `apps.json` 条目相同，可带参数——不会预先拒绝任何东西），保存在 `apps.json`／`themes.json` 旁边的 `restart.json` 中。一个专用线程每隔几秒检查每个被监视目标的进程是否仍在运行（按可执行文件名，而不是持有句柄），一旦不在运行就立即重新启动它——无论是崩溃还是你自己关闭，都没有区别，它都会回来。选择该条目会列出所有当前被监视的目标（以 `Auto-restart: N` 的形式实时显示在主列表中“Auto-restart”旁边，为空时为 `0`），可以像 Sticky Notes 一样模糊搜索，每个目标前面带有 `★`（正在运行）或 `☆`（当前未运行）。在其中时，搜索框的占位文字会变为 `Type a target to watch...`。

- 输入与现有目标匹配的文本，`Enter` 不做任何事（对已有条目，除了移除它之外没有别的有用操作，见下面的 `Delete`）
- 输入不匹配任何内容的路径，`Enter` 把它加入监视列表，并保持打开以便继续添加——按你从资源管理器地址栏粘贴的样子输入即可（单个反斜杠）；这个输入框是纯文本，不是 JSON，所以这里不需要双写。之后打开原始 `restart.json` 时看到的双反斜杠只是写入时正常的 JSON 转义（与 `apps.json` 的 `path` 遵循的规则相同，见下文）——MAGI 两种写法都能正确读取。
- `Delete` 停止监视高亮目标（**不会**关闭或结束应用本身，只是结束监视）
- `Shift+Delete` 一次停止监视所有目标（清空整个列表，与 Sticky Notes 相同）
- 搜索框为空时按 `Tab` / `Escape` / `Backspace` 关闭选择器

目标不需要同时存在于 `apps.json` 中——两个列表完全独立，所以同一个应用可以是普通的可启动条目、被监视的自动重启目标、两者都是，或两者都不是。由于检测纯粹是“这个可执行文件名有没有在运行”，MAGI 无法区分真正的崩溃和你故意关闭窗口——只要在列表里，它就一定会回来。也不会尝试检测卡死但仍在运行（“未响应”）的目标并强制结束它：那样可能会结束一个只是暂时忙碌、马上就会自己恢复的程序，比什么都不做更糟。通知区域菜单中也有自己的“Disable/Enable Auto-restart”开关，可以在不改动监视列表本身的情况下暂停整个监视器（与禁用快捷键的思路相同）。

## Auto-kill

Auto-restart 的镜像，并共用同一个后台线程：一个 `"magi:auto-kill"` 条目会解锁一个目标列表（保存在 `kill.json` 中），MAGI 会让它们保持**关闭**——每隔几秒，它会终止发现正在运行的任何目标（该可执行文件名的所有实例）。选择器的工作方式与 Auto-restart 完全相同——主列表中显示 `Auto-kill: N`，可模糊搜索，每个目标前面带有 `✖`（正在运行，将被终止）或 `·`（未运行）——占位文字为 `Type a target to kill...`。`Enter` 添加新目标，`Delete` 移除高亮目标，`Shift+Delete` 清空列表。

两条安全规则，都不可商量，也不可配置：

- **10 秒宽限期。** 启用 Auto-kill 后的前 10 秒内不会终止任何东西——每次都是：MAGI 启动时，以及每次从通知区域启用它时。如果 `kill.json` 指向了你确实需要的东西，这就是你在第一次终止发生之前打开通知区域并选择“Disable Auto-kill”的时间。
- **MAGI 绝不终止自己。** 无论 `kill.json` 写了什么，`magi_launcher.exe` 都会被排除——否则一个错误的列表加上开机自启，会让 MAGI 每次开机都被终止，连用来禁用它的通知区域图标都没有。

Auto-kill 默认**关闭**（不同于默认开启的 Auto-restart）：终止进程的功能必须有意开启。它的通知区域开关和 `state.json` 中的 `auto_kill_enabled` 都会保存这个选择；无论用哪种方式重新启用，都会重新开始 10 秒宽限期。

## 回收站

内置的 `"magi:empty-recycle-bin"` 条目（见下文）会在名称旁实时显示项目数和大小，清空的那一刻立即刷新——之后马上重新打开启动器也绝不会显示过时的残留。在它上面按 `Enter` 会打开一个可模糊搜索的列表，显示回收站中此刻实际存在的内容（所有驱动器，直接从 `$Recycle.Bin` 读取，在 UI 线程之外进行，所以缓慢或庞大的回收站绝不会让启动器卡住——不涉及任何资源管理器窗口）：

- 输入以模糊筛选已删除项目的列表，与其他地方相同
- 在高亮项目上按 `Enter`，把它的完整名称（含扩展名）复制到剪贴板并关闭启动器
- `Delete` 只把高亮项目从回收站中永久删除——其余不受影响
- `Shift+Delete` 在这个视图中也能清空整个回收站——启动器保持打开，（现已为空的）列表就地刷新
- 搜索框为空时按 `Tab` / `Escape` / `Backspace` 返回主列表

要不打开回收站就直接清空它，在主列表的条目上直接使用 `Shift+Enter` 或 `Delete`——特意与打开浏览视图的按键不同，这样只是看一眼里面有什么时绝不会意外清空它。无论哪种方式，启动器都保持打开；只有条目旁的项目数会更新。

## Emoji

添加一个 `"magi:emoji"` 条目（见下文）即可解锁——模糊搜索 Unicode 官方 emoji 名称（"fire"、"red heart"、"grinning face"…），按 `Enter` 把 emoji 本身复制到剪贴板并关闭启动器。没有内置的 emoji 列表，也没有 JSON：它读取 `emoji-test.txt`，即 Unicode 官方在 [unicode.org/Public/emoji/latest/emoji-test.txt](https://www.unicode.org/Public/emoji/latest/emoji-test.txt) 发布的纯文本参考文件，像 `apps.json`／`themes.json` 一样放在可执行文件旁边。主列表会在它的名称旁实时显示 `Emoji: Version 17.0`（或文件声明的版本）；如果文件不存在，则显示 `Emoji: missing emoji-test.txt`，在它上面按 `Enter` 不做任何事——从上面的链接下载一份放到 `.exe` 旁边（如果启动器已在运行，之后再执行 **Reload**）即可解锁。

- 输入以按名称模糊筛选，与其他地方相同
- 搜索框为空时按 `Tab` / `Escape` / `Backspace` 返回主列表

以后要换成更新的 emoji 集，只需把 `emoji-test.txt` 替换为 Unicode 的新版本并执行 **Reload**——无需重新编译。emoji 使用系统的 emoji 字体以彩色绘制，该字体只在选择器打开时加载。字体画不出来的会以普通文本显示——Windows 上的旗帜（它的 emoji 字体没有旗帜），以及字体尚未跟上的新增 emoji（复制都照常有效）。

## Copy History

默认开启——可以在通知区域菜单中切换（“Enable/Disable Copy History”），或添加一个 `"magi:copy-history"` 条目（见下文）在启动器中直接浏览。开启期间，你在系统任何地方复制的文本都会被记录，最新的在前，并以计数的形式实时显示在主列表中“Copy History”旁边（开关关闭时显示 `disabled`）。

- 输入以模糊筛选过去的复制内容，与其他地方相同
- 在高亮条目上按 `Enter`，把它重新复制到剪贴板并关闭启动器——这次重新复制不会添加重复的条目
- `Delete` 删除高亮条目，`Shift+Delete` 清空整个历史
- 搜索框为空时按 `Tab` / `Escape` / `Backspace` 返回主列表

**保存位置：** 永远不在磁盘上的任何地方。历史只保存在启动器自身进程的内存（RAM）中，总上限为 1,000,000 字节（满了之后先删除最旧的条目）——关掉电脑就无影无踪，没有可找到的文件，也没有遥测。每个条目的内存都用 `VirtualLock`（Linux 上为 `mlock`，在用户的 `ulimit -l` 范围内）锁定，使系统在内存紧张时不会把它交换到磁盘，并在它被丢弃的那一刻（被挤出、被删除，或你从通知区域退出时）用零覆盖。在 Windows 上，会尊重那些把自己复制的内容标记为不进入剪贴板历史的密码管理器。

这是审慎的，但不是绝对的：Copy History 打开时，它的文本也会显示在屏幕上并存在于界面自身的内存中，当前的剪贴板内容由系统持有，读取它时也会产生短暂的副本。特意**没有**加密：同一进程中的解密密钥，挡不住任何已经能读取该进程内存的东西。

通知区域开关的状态（`hotkey_enabled`／`auto_restart_enabled`／`auto_kill_enabled`／`copy_history_enabled`）在你切换的那一刻就会保存到 `state.json`（见下文），因此重启后依然保留。

## Eject

添加一个 `"magi:eject"` 条目（见下文）即可解锁——一个可模糊搜索的列表，列出当前插着的每个通过 USB 连接的驱动器。

- 输入以模糊筛选列表，与其他地方相同
- 在高亮驱动器上按 `Enter`，*仅当当前没有程序打开它时*才弹出——成功后它会从列表中消失，启动器保持打开，所以连续弹出多个驱动器无需重新打开选择器。如果仍有程序打开着它的句柄（正在扫描的杀毒软件／索引器、打开了文件的应用…），`Enter` 什么也不做，驱动器不受影响——仍要强制弹出请见下面的 `Shift+Delete`
- `Shift+Delete` 即使仍有程序打开着，也强制弹出高亮的驱动器——依赖它之前请先阅读下面的警告
- 搜索框为空时按 `Tab` / `Escape` / `Backspace` 返回主列表

只列出 USB 总线上的驱动器（直接对设备本身进行检查，而不是使用 `GetDriveTypeW` 的可移动／固定标志，后者会把很多外置 USB-SATA／UASP 硬盘盒误报为“固定”）——绝不包括系统盘，也绝不包括内置的第二块硬盘。

先锁定并卸载卷（`FSCTL_LOCK_VOLUME`／`FSCTL_DISMOUNT_VOLUME`），再通过即插即用管理器移除设备（`CM_Request_Device_Eject`），与第三方 USB 弹出工具的做法相同。请求针对的是驱动器第一个被标记为可移除的上级设备（最差情况下是 USB 端口），而不是驱动器本身，所以对那些不把自己报告为可移动的外置硬盘盒也有效——也就是 Windows 自己的菜单不显示或变灰的那些。弹出在后台进行：启动器保持响应，期间显示“Ejecting...”。

**`Shift+Delete` 会强制弹出——事先不会征求同意。** 与普通的 `Enter`（卷锁定一被拒绝就干净地退回）不同，强制路径完全跳过这项检查：即使另一个进程仍在驱动器上打开着文件，卸载步骤也会成功，那个进程的句柄只会被作废（它会收到 I/O 错误），而不会阻止弹出。后台扫描（杀毒软件、搜索索引器）在*读取*过程中这样被打断是无害的——没有任何东西被修改，它只是在自己那边报错。但在那一刻确实正在*写入*驱动器的文件（正在进行的复制、正在进行的保存）会在毫无预警的情况下被截断或损坏——没有任何“现在是否有东西正在写入”的检查，强制就是强制。

## Linux

以上所有内容在 Linux 上同样可用，区别如下。文件格式相同；路径使用 `/` 以及 `$VAR` 或 `~`，而不是 `%VAR%`。

**X11 和 Wayland。** X11 上功能完整。Wayland 不允许应用全局监听键盘、查看其他窗口或自行定位，因此在那里：

- 全局快捷键不可用。请改为在桌面设置中把可执行文件绑定到一个快捷键：在它运行时再次启动它，就会显示或隐藏启动器（X11 上也是如此）。
- Window Switcher 保持为空。
- 窗口的位置以及是否出现在任务栏中，由合成器决定。

**依赖什么。** 启动器本身只需要 fontconfig，几乎所有桌面环境都有。下面的每个工具只在使用它的功能中才需要：

| 功能 | 工具 |
|---|---|
| 打开文档、文件夹和地址 | `xdg-open`（`xdg-utils`） |
| Eject | `lsblk` 和 `udisksctl`（`udisks2`） |
| 媒体键 | `playerctl` |
| 音量键 | `pactl`（PulseAudio 或 PipeWire） |
| 彩色 emoji | Noto Color Emoji 字体 |
| 通知区域图标 | 带通知区域（StatusNotifierItem）的桌面；没有的话启动器仍会运行，可以通过结束其进程来退出 |

**其他区别：**

- Copy History 每秒检查一次剪贴板而不是接收通知，看不到密码管理器的排除标记，并且会漏掉在那一秒内被替换的文本。
- 回收站是你主目录的回收站（`~/.local/share/Trash`）；不读取各个磁盘自己的回收站文件夹。
- Auto-restart 和 Auto-kill 比较的是进程命令行中的程序名，所以脚本会以其解释器的名字（`python3`）出现。
- 时钟始终为 24 小时制。
- `hidden` 没有效果：程序的输出本来就不会显示。
- `Shift+Enter` 打开包含目标的文件夹，但不会选中它。

## 配置

`apps.json` 和 `themes.json` 位于可执行文件旁边——绝不打包进可执行文件——所以你可以手动编辑它们而无需重新编译。使用 **Reload**（一个 `"magi:reload"` 条目）即可在不重启应用的情况下应用更改。

`state.json` 也在那里，但它是另一类文件——内部记录（当前主题、字体／窗口／边框偏好，以及通知区域的开关），启动器自己在启动时读取一次，并在你通过键盘／通知区域／主题选择器更改其中某项的那一刻自动重写。出于与下面 `notes.json`／`restart.json` 相同的原因，它不适合在运行期间手动编辑，但如果你想在首次启动前设置初始值，或者只是看看当前保存了什么，请参阅 [`state.json`](#statejson)。

`notes.json`／`restart.json` 也在那里，但它们也是另一类文件：`notes.json` 是一个简单的字符串 JSON 数组（`["note 1", "note 2"]`），`restart.json` 是一个简单的 `path` 字符串 JSON 数组（`["A:\\Apps\\Foo\\Foo.exe"]`）——两者都在你于 Sticky Notes／Auto-restart 中添加或删除条目时由应用自己自动创建和重写，所以通常启动器在内存中的副本始终是准的。它们不适合手动编辑，但 **Reload** 也会从磁盘重新读取这两个文件（连同 `apps.json`／`themes.json`／`state.json`），所以对其中任何一个的手动修改仍然无需重启即可生效。

`emoji-test.txt` 又是另一类：直接来自 Unicode 的纯文本参考文件（见上面的 [Emoji](#emoji)），既不是 JSON，也不是由 MAGI 自己写入的——更新它只需替换整个文件，里面没有需要逐条手动编辑的东西。它是可选的：如果不存在，Emoji 选择器只是保持锁定（主列表中会明确显示 `Emoji: missing emoji-test.txt`）。

可执行文件旁的 `shortcuts/` 文件夹同样是可选的：直接位于其中的每个文件（不含子文件夹）都会成为可启动的条目，在搜索排名相同时排在 `apps.json` 自己的条目之后——`.lnk`、`.bat`、`.cmd`、`.vbs`，什么都行，MAGI 不检查扩展名，只是把路径交给所有其他条目都使用的同一个 `ShellExecute` 调用（它本来就知道如何解析 `.lnk` 或运行 `.bat`／`.vbs`，与在资源管理器中双击完全一样）。条目名称是去掉扩展名的文件名。文件夹不存在或为空：没有任何影响，也不会额外显示任何东西。

### `apps.json`

```json
{
  "apps": [
    { "name": "Notepad", "path": "%windir%\\system32\\notepad.exe" },
    { "name": "Command Prompt", "path": "%windir%\\system32\\cmd.exe", "cwd": "%HOMEDRIVE%%HOMEPATH%" },
    { "name": "Some background script", "path": "A:\\Scripts\\thing.ps1", "hidden": true }
  ]
}
```

- **`name`**（必填）——显示名称，可模糊搜索。
- **`path`**（必填）——普通路径、带参数的路径（`"app.exe --flag"`）、shell URI（`ms-settings:...`、`shell:RecycleBinFolder`…），或下面的[特殊 `magi:` 条目](#特殊条目)。一切都通过 `ShellExecute`，所以资源管理器能打开的任何东西（包括通过文件关联解析的文档类型，例如 `.msc` 文件）都能用。反斜杠必须双写（`"A:\\Apps\\Foo.exe"`），因为 `\` 是 JSON 的转义字符——单个 `\` 是无效的 JSON，并可能悄无声息地破坏路径（`\t`、`\n`… 都是真正的转义序列）。正斜杠（`"A:/Apps/Foo.exe"`）也可以，而且不需要转义——Windows 两者都接受。
- **`cwd`**（可选）——工作目录。默认是目标自己所在的文件夹（与资源管理器中双击一致），但对于 `cmd.exe`／`powershell.exe` 这类条目，你通常会想明确指定它（否则它们会在 `system32` 中启动）。
- **`hidden`**（可选，默认 `false`）——不显示窗口启动（`SW_HIDE`），适用于没有东西要显示的脚本。

#### 特殊条目

| `path` | 效果 |
|---|---|
| `magi:reload` | 就地重新加载 `apps.json`／`themes.json`／`state.json`／`notes.json`／`restart.json`／`kill.json`／`emoji-test.txt`／`shortcuts/`，无需重启 |
| `magi:theme-picker` | 进入实时主题选择器（见下文） |
| `magi:timer` | 进入计时器的时长输入；空闲时显示 `<名称>: --:--`，启动后显示实时倒计时 |
| `magi:notes` | 进入 Sticky Notes 选择器（见上文）；为空时显示 `<名称>:`，否则显示 `<名称>: <最新便签>` |
| `magi:auto-restart` | 进入 Auto-restart 选择器（见上文）；以 `<名称>: N` 显示被监视目标的数量，为空时为 `0` |
| `magi:auto-kill` | 进入 Auto-kill 选择器（见上文）；以 `<名称>: N` 显示终止目标的数量，为空时为 `0` |
| `magi:copy-history` | 进入 Copy History 选择器（见上文）；以 `<名称>: N` 显示已保存条目的数量，通知区域开关关闭时显示 `<名称>: disabled`（此时 Enter 不做任何事） |
| `magi:open-folder` | 在文件管理器中打开 MAGI Launcher 所在的文件夹——每次启动时重新解析，所以文件夹被移动后也会跟随 |
| `magi:empty-recycle-bin` | 非空时显示 `<名称>: N items, X MB`；`Enter` 浏览其内容（见上面的[回收站](#回收站)），`Shift+Enter` 或 `Delete` 清空它 |
| `magi:emoji` | 进入 Emoji 选择器（见上文）；显示来自 `emoji-test.txt` 的 `<名称>: Version X.Y`，文件不存在时显示 `<名称>: missing emoji-test.txt`（Enter 不做任何事） |
| `magi:eject` | 进入 Eject 选择器（见上文） |
| `magi:media-play-pause`、`magi:media-next`、`magi:media-previous`、`magi:media-stop`、`magi:media-volume-mute`、`magi:media-volume-down`、`magi:media-volume-up` | 发送对应的虚拟媒体键，路由方式与真正的硬件按键相同（全局媒体会话，而不只是这个窗口） |

一个真正的“关闭 Windows”对话框（与在桌面上按 `Alt+F4` 得到的相同）也可以作为普通条目添加，无需任何特殊代码：

```json
{
  "name": "Shutdown",
  "path": "%windir%\\system32\\WindowsPowerShell\\v1.0\\powershell.exe -command \"(New-Object -ComObject Shell.Application).ShutdownWindows()\"",
  "hidden": true
}
```

### `themes.json`

```json
{
  "themes": {
    "arc-dark": {
      "search_background": "#404552",
      "search_text": "#7c818c",
      "list_background": "#383c4a",
      "list_text": "#d3dae3",
      "selected_background": "#5294e2",
      "selected_text": "#ffffff",
      "border": "#4b5162"
    }
  }
}
```

这里只有配色方案本身——哪个是当前主题，以及所有其他显示偏好，都放在下面的 [`state.json`](#statejson) 中（分开存放，这样用更新的配色集整体替换 `themes.json` 时，绝不会重置你自己的偏好）。

`themes` 字典中自带 100 多种内置主题（主要是角色／游戏配色）——打开启动器并选择 `Themes` 条目（`magi:theme-picker`），即可实时预览并切换，无需重启。选择器会定位在当前生效的主题上，而不是按字母顺序的第一个。

### `state.json`

MAGI Launcher 自己写入的内部记录——你应该不需要碰它，但如果你想看看，或在首次启动前设置初始值，它就是最普通的 JSON：

```json
{
  "ui": {
    "theme": "arc-dark",
    "font_family": null,
    "placeholder_text": "Type to search...",
    "show_clock": true,
    "window_size": 30,
    "border": 1
  },
  "auto_restart_enabled": true,
  "auto_kill_enabled": false,
  "copy_history_enabled": true,
  "hotkey_enabled": true,
  "hotkey": "Ctrl+Space"
}
```

- **`ui.theme`**——`themes.json` 的 `themes` 字典中当前生效条目的名称。在主题选择器中选中的那一刻写入。
- **`ui.font_family`**——`null` 表示保留操作系统的默认字体。推荐字体：[SGr-Iosevka-Regular.ttc](https://github.com/be5invis/iosevka)
- **`ui.placeholder_text`**——搜索框为空时显示的文字（在 Timer／Sticky Notes／Auto-restart／Auto-kill 中会被该模式专用的占位文字取代，见上面各自的章节）
- **`ui.show_clock`**——在搜索框旁显示当前时间（在 Windows 上使用用户的短时间格式）
- **`ui.window_size`**——弹出窗口占屏幕宽度的百分比（0–100）（高度和字号随之变化，保持 16:9 的比例，始终在光标所在的显示器上居中，并被限制在屏幕范围内）。也可以用键盘实时调整，见上面的 `Ctrl+1`–`Ctrl+0`——每按一次都会立即把新值写回这里。
- **`ui.border`**——模拟边框的粗细（像素）。同样可以实时调整，见上面的 `Ctrl+-`／`Ctrl+=`。
- **`hotkey`**——（Wayland 上不可用，见 [Linux](#linux)）形如 `"ctrl+space"`、`"ctrl+alt+f"`、`"win+e"`、`"f14"` 的写法。支持 `ctrl`／`control`、`alt`、`shift`、`win`／`super`、`space`、`enter`／`return`、`tab`、`esc`／`escape`、`f1`–`f24`，以及单个字符。
- **`hotkey_enabled`** / **`auto_restart_enabled`** / **`auto_kill_enabled`** / **`copy_history_enabled`**——对应通知区域菜单的开关，启动时读取一次，在你从通知区域切换其中一个的那一刻自动重写。`auto_kill_enabled` 默认为 `false`（其他为 `true`）——终止进程的功能需要主动开启；启用它（在这里或从通知区域）会在第一次终止前开始固定的 10 秒宽限期。

## 许可证

个人非商业用途免费。Copyright (C) 2026 Nyaldee，保留所有权利——完整条款见 [LICENSE](../LICENSE)。请仅从本仓库的 [Releases](https://github.com/Nyaldee/nyaruhodo/releases) 下载。

Anthropic 的 AI [Claude](https://claude.com) 参与了本项目的代码编写。
