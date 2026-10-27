# 同步官方 Handy 正式版本

本仓库的默认分支是 `nibi`（原名 `handy-local`）；新功能与上游正式版更新均合入这里。
`main` 保留上游 Handy 代码，不是 Nibi 的开发主线。

Handy Local 跟随 `cjpais/Handy` 的正式 GitHub Release，不跟随 `main`，也不引入
预发布版、草稿或只有 tag 而没有正式 Release 的版本。
[`.github/upstream-sync.json`](../.github/upstream-sync.json) 记录已合入的版本和
准确 commit；应用里的版本号不作为同步依据。

## 日常使用

GitHub Actions 的 **Check official Handy releases** 每周一 16:23 UTC
（洛杉矶夏令时 09:23、冬令时 08:23）检查一次。也可以在 Actions 页面手动点击
**Run workflow**，选择 `nibi` 分支。

- 没有新版：成功结束，不创建分支，也不重新构建。
- 有正式新版：从 `nibi` 创建 `codex/upstream-vX.Y.Z` 同步分支，保留
  Git 合并历史，并运行 **Handy Local validation**。
- 分支已存在：保留已有人工修改，不覆盖或强推；重新验证该分支当前 commit。
- 合并冲突、保护检查失败、上游 tag 被移动或历史分叉：停止，Actions 标为失败。
  在任务摘要和 `upstream-sync-report` artifact 中查看原因；默认分支不受影响。

同步任务不会自动合入、发布应用或打开 PR。nibi 的
[PR 模板](../.github/PULL_REQUEST_TEMPLATE.md) 允许贡献者授权的自动 PR 创建，
不要求单独由本人撰写说明，仍需如实披露 AI 辅助。成功准备分支后，Actions 摘要提供
GitHub 比较入口和 artifact 中的 `pull-request.md`；核对改动说明与检查项后，可通过
网页、CLI 或授权的工具提交 PR。不要把未执行的测试标记为完成。

GitHub 的通知取决于账号的 Actions 通知设置。需要获知每次发现新版，可为这个
workflow 启用完成通知；失败运行用于提示冲突或测试失败。公开仓库的定时任务可能在
长期没有仓库活动后被 GitHub 停用，届时在 Actions 页面重新启用即可。

## 本地命令

需要 Git、Python 3.10+、GitHub CLI（`gh`）及 Bun。先 `gh auth login`。

```sh
bun run upstream:check
bun run upstream:prepare
```

`check` 只读取正式发布信息、获取对应 Git 对象和写本地报告，不改工作文件。
`prepare` 要求工作区干净，使用临时 worktree 合并；成功后留下独立本地分支，
当前分支和工作区不变。报告默认在 `.git/upstream-sync/`。

```sh
# 检查自动准备的分支；把版本号替换成报告给出的真实版本
git switch codex/upstream-vX.Y.Z
bun install --frozen-lockfile
bun run check:local
bun run upstream:test
bun run build
bun run lint
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo test --locked --manifest-path src-tauri/Cargo.toml --features local-only local_corrections --lib
bunx playwright test tests/corrections.spec.ts tests/dictionary-transfer.spec.ts --reporter=line
git push -u origin codex/upstream-vX.Y.Z
```

需要指定历史正式版时，`bun run upstream:prepare --tag vX.Y.Z` 仍然会查询 Release
并拒绝预发布或降级。自动任务的 `--publish` 只允许发布到这个 fork，并要求从远端
当前 `nibi` 开始；不会把修改推到官方 Handy。

## 保留什么、怎样验证

自动合并普通业务代码，保留双方不冲突的改动。同一段代码冲突时不使用
`-X ours`、`-X theirs` 或全量覆盖解决。

以下属于 fork 自己维护的文件，会保持本地版本：独立应用配置、README、AGENTS.md、本地维护说明、
同步工具和正在运行的 CI 工作流。官方 README 更新到 `README.upstream.md`；官方工作流
更新到 `.github/upstream-workflows/`，不会进入活动目录。自动同步脚本内的 `OWNED`
列表是这项策略的明确清单；增加新的 fork 专属工具时应同步维护。

`check:local` 对本地纠错接入点、本地构建入口、应用及数据目录标识、禁用云处理、
禁用官方更新、界面翻译和测试文件进行结构检查。上游重构导致检查失败时，需要人工
确认新接入方式后更新检查；不要简单删掉保护。结构检查不能代替行为测试。

CI 另行运行 Rust 本地纠错测试（含词典持久化、导入导出、替换规则、云调用禁用），
前端构建、ESLint、浏览器纠错/词典测试，以及 Apple Silicon 上的 Clippy 检查。
同步写入任务与代码测试分开，测试任务只有只读权限，不持有持久 Git 凭据。

## 遇到冲突

失败报告列出冲突文件，未解决的临时 worktree 会清理，不产生半成品提交。
在新的维护分支里手动重现合并：

```sh
git switch nibi
git pull --ff-only origin nibi
git switch -c codex/resolve-upstream-vX.Y.Z
git fetch --no-tags upstream refs/tags/vX.Y.Z
git merge --no-commit --no-ff FETCH_HEAD
```

先按照上述 `OWNED` 策略保留 fork 文件、归档官方工作流，再逐项解决业务代码冲突。
`git add` 后运行完整验证。把 `.github/upstream-sync.json` 更新为已验证的正式版本和
`git rev-parse FETCH_HEAD^{commit}`，然后提交这个 merge。不要用 squash/rebase 代替
合并：上游 commit 的祖先关系是后续增量同步的依据。

## 合入与给用户发布

合入 PR 时选择 **Create a merge commit**，保留上游历史。同步分支包含的 baseline
只有合入默认分支后才算生效。

CI 通过后，在原生 Mac 上确认：Fn 录音、粘贴、编辑后的纠错提示、点击确认才记忆、
重启后词典持久化、导入导出及冲突备份正常。浏览器测试模拟 IPC，不能证明真实麦克风
和 macOS Accessibility 行为。

使用 `CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run build:local` 构建 Handy Local，
验证后按现有方式发布自己的安装包。应用内自动更新仍保持关闭；这个流程维护的是代码，
不会替用户更新已经安装的 app，也不会接触用户词典文件。
