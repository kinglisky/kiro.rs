# kiro-rs：macOS 与 Windows 快速入门

本文说明如何通过 Kiro IDE 登录 AWS IAM Identity Center（IdC），从本机缓存手动整理凭据，并在不使用 Docker 的情况下构建、启动和验证 `kiro-rs`。

## 1. 适用范围与准备工作

本文仅适用于以下环境：

- 操作系统为 macOS 或 Windows。
- 组织已分配 Kiro Enterprise 订阅。
- 使用 AWS IAM Identity Center 登录 Kiro IDE。
- 在本机直接构建和运行 `kiro-rs`，不使用 Docker。

本文不适用于 GitHub、Google、AWS Builder ID、外部身份提供商和 Headless API Key 登录方式。上述方式的缓存结构可能不同，请勿套用本文字段。

开始前请准备：

- Kiro IDE。
- 组织管理员提供的 IAM Identity Center Start URL 和 IdC Region。
- Git、Rust 稳定版工具链和 Cargo。
- Node.js 和项目使用的 pnpm。
- macOS 需要 Xcode Command Line Tools。
- Windows 需要 PowerShell 和 Visual Studio C++ Build Tools。

后续项目命令均在 `kiro-rs` 仓库根目录执行。

`credentials.json` 和 `config.json` 都包含敏感信息。不要输出、提交或分享文件内容，并将文件权限限制为仅当前系统用户可读。

## 2. 安装并登录 Kiro IDE

1. 安装并启动 Kiro IDE。
2. 根据 [Kiro 官方认证说明](https://kiro.dev/docs/getting-started/authentication/) 选择 **Sign in with AWS IAM Identity Center**。
3. 输入管理员提供的 **Start URL** 和 **IdC Region**。
4. 在浏览器中完成授权，然后返回 Kiro IDE。
5. 在 IDE 中发起一次普通对话，确认可以正常收到回复。

只有 Kiro IDE 对话成功后，才继续读取缓存。完成登录但未成功使用 IDE 时，Token、客户端注册信息或 Profile 缓存可能尚未完整写入。

Start URL 和 IdC Region 必须使用管理员提供的值。不要根据示例或 Kiro Profile ARN 猜测 IdC Region。

## 3. macOS 缓存位置与字段来源

Kiro IDE 登录成功后，需要以下三份文件：

```text
~/.aws/sso/cache/kiro-auth-token.json
~/.aws/sso/cache/<clientIdHash>.json
~/Library/Application Support/Kiro/User/globalStorage/kiro.kiroagent/profile.json
```

先打开 `kiro-auth-token.json`，找到 `clientIdHash`。假设值为 `abc123`，对应的客户端缓存就是：

```text
~/.aws/sso/cache/abc123.json
```

必须按 `clientIdHash` 精确定位同名文件。AWS SSO 缓存目录可能包含其他应用或历史登录产生的 JSON，不能随便选择第一个文件。

最后打开 `profile.json`，读取 `arn`。该值是 Kiro Enterprise Profile ARN，不是 IAM Identity Center Start URL。

## 4. Windows 缓存位置与字段来源

Windows 上对应的三份文件是：

```text
%USERPROFILE%\.aws\sso\cache\kiro-auth-token.json
%USERPROFILE%\.aws\sso\cache\<clientIdHash>.json
%APPDATA%\Kiro\User\globalStorage\kiro.kiroagent\profile.json
```

可以在文件资源管理器地址栏直接输入带环境变量的路径。`%APPDATA%` 通常指向当前用户的 `AppData\Roaming`，无需手动替换用户名。

先从 `kiro-auth-token.json` 读取 `clientIdHash`，再打开同目录下名称完全相同的 `<clientIdHash>.json`。

不要按文件时间或排列顺序选择客户端缓存。Token 与 `clientId`、`clientSecret` 不匹配时，刷新请求通常会失败。

## 5. 手动创建 `credentials.json`

macOS 与 Windows 使用相同的字段映射：

| `credentials.json` 字段 | 来源或填写规则 |
|---|---|
| `accessToken` | `kiro-auth-token.json.accessToken`；没有时与 `expiresAt` 一起省略 |
| `refreshToken` | `kiro-auth-token.json.refreshToken` |
| `expiresAt` | `kiro-auth-token.json.expiresAt`；仅与对应的 `accessToken` 一起填写 |
| `clientId` | `<clientIdHash>.json.clientId` |
| `clientSecret` | `<clientIdHash>.json.clientSecret` |
| `profileArn` | `profile.json.arn` |
| `authMethod` | Enterprise IdC 固定填写 `idc` |
| `region` | `kiro-auth-token.json.region` |
| `authRegion` | 与 `kiro-auth-token.json.region` 相同 |
| `apiRegion` | 将 `profileArn` 按 `:` 分隔后取第 4 段 |

例如，下面 ARN 的第 4 段是 `us-east-1`：

```text
arn:aws:codewhisperer:us-east-1:123456789012:profile/example
```

IdC Region 用于认证，Profile Region 用于调用 Kiro API，两者可能不同。因此，`authRegion` 应来自 Token 缓存，`apiRegion` 应来自 Profile ARN。

在项目根目录创建 `credentials.json`。将各来源文件中的字段值填到对应占位符；JSON 中的双引号必须保留，也不要把示例占位符当成真实值。

文件必须使用数组格式，即使目前只有一组凭据。下面示例省略了 `accessToken` 和 `expiresAt`，服务会在首次请求时通过 Refresh Token 获取新的访问令牌：

```json
[
  {
    "refreshToken": "<kiro-auth-token.json 中的 refreshToken>",
    "profileArn": "<profile.json 中的 arn>",
    "authMethod": "idc",
    "clientId": "<clientIdHash 对应文件中的 clientId>",
    "clientSecret": "<clientIdHash 对应文件中的 clientSecret>",
    "region": "<kiro-auth-token.json 中的 region>",
    "authRegion": "<kiro-auth-token.json 中的 region>",
    "apiRegion": "<profileArn 第 4 段的 Region>"
  }
]
```

如果需要沿用缓存中的访问令牌，必须同时填写相互对应的 `accessToken` 和 `expiresAt`。不要只填写其中一个字段。

保存后检查 JSON 语法，重点留意逗号、双引号和数组外层的方括号。不要把真实凭据粘贴到在线 JSON 校验网站。

## 6. 创建 `config.json`

在项目根目录创建 `config.json`。配置字段使用 camelCase：

```json
{
  "host": "127.0.0.1",
  "port": 8990,
  "apiKey": "<本地客户端使用的强随机密钥>",
  "adminApiKey": "<Admin UI 使用的另一条强随机密钥>",
  "region": "<credentials.json 中的 region>",
  "authRegion": "<credentials.json 中的 authRegion>",
  "apiRegion": "<credentials.json 中的 apiRegion>",
  "tlsBackend": "rustls",
  "defaultEndpoint": "ide"
}
```

`apiKey` 只用于客户端访问本地代理，不是 Kiro Token。`adminApiKey` 只用于 Admin API 和 Admin UI，必须与 `apiKey` 不同。

不需要 Admin UI 时，可以删除 `adminApiKey`。仅本机使用时保持 `host` 为 `127.0.0.1`，不要改成 `0.0.0.0`。

两条 Key 都应使用密码管理器或可靠的随机生成器创建，建议至少包含 32 个随机字符。不要使用文档中的占位符。

更多字段说明见 [配置与模型可用性说明](configuration-and-model-availability.md)。

## 7. 编译、启动与接口验证

先构建 Admin UI，再构建 Rust 服务。顺序如下：

```text
pnpm --dir admin-ui install
pnpm --dir admin-ui build
cargo build --release
```

macOS 启动命令：

```bash
./target/release/kiro-rs \
  --config ./config.json \
  --credentials ./credentials.json
```

Windows PowerShell 启动命令：

```powershell
.\target\release\kiro-rs.exe `
  --config .\config.json `
  --credentials .\credentials.json
```

保持启动终端运行。日志显示服务监听 `127.0.0.1:8990` 后，另开终端验证接口。

macOS 使用 `curl`：

```bash
curl --fail-with-body --silent --show-error \
  -H 'x-api-key: <config.json 中的 apiKey>' \
  http://127.0.0.1:8990/v1/models

curl --fail-with-body --silent --show-error \
  -H 'content-type: application/json' \
  -H 'x-api-key: <config.json 中的 apiKey>' \
  -d '{"model":"gpt-5.6-sol","max_tokens":32,"stream":false,"messages":[{"role":"user","content":"Reply with exactly: OK"}]}' \
  http://127.0.0.1:8990/v1/messages
```

Windows PowerShell 使用系统自带的 `curl.exe`：

```powershell
curl.exe --fail-with-body --silent --show-error `
  -H "x-api-key: <config.json 中的 apiKey>" `
  http://127.0.0.1:8990/v1/models

$body = '{"model":"gpt-5.6-sol","max_tokens":32,"stream":false,"messages":[{"role":"user","content":"Reply with exactly: OK"}]}'
curl.exe --fail-with-body --silent --show-error `
  -H "content-type: application/json" `
  -H "x-api-key: <config.json 中的 apiKey>" `
  -d $body `
  http://127.0.0.1:8990/v1/messages
```

`/v1/models` 返回的是 `kiro-rs` 的静态模型声明，不代表当前账号已获上游授权。模型是否可用，最终以 `/v1/messages` 返回成功为准。

如果组织未授权 `gpt-5.6-sol`，请将示例模型 ID 换成 Kiro IDE 中当前身份实际可用的模型。

## 8. Admin UI 与常见问题

配置了非空 `adminApiKey` 后，启动服务并访问：

```text
http://127.0.0.1:8990/admin
```

登录时填写 `config.json` 中的 `adminApiKey`，不要填写 `apiKey`、`accessToken` 或 `refreshToken`。

| 现象 | 最可能原因 | 处理方式 |
|---|---|---|
| 找不到 `kiro-auth-token.json` 或 `profile.json` | IdC 登录或 IDE 初始化尚未完成 | 回到 Kiro IDE 重新登录，并确认至少一次对话成功。 |
| 找不到 `<clientIdHash>.json` | 客户端注册缓存缺失或已被清理 | 重新登录 Kiro IDE，再按最新 `clientIdHash` 精确定位；不要改用第一个 AWS SSO JSON。 |
| Token 刷新返回 `invalid_grant` | `refreshToken` 过期、撤销，或与客户端注册不匹配 | 在 Kiro IDE 退出后重新登录，再同时更新 Token 和对应客户端缓存中的字段。 |
| 本地接口返回 401 | 请求 Key 与 `config.json.apiKey` 不一致 | 从本地配置重新复制 `apiKey`，不要使用 Kiro Token 或 `adminApiKey`。 |
| 上游返回 401 或 403 | 凭据失效、Profile 不正确、订阅或组织策略拒绝访问 | 先确认同一账号和 Profile 在 Kiro IDE 中可正常对话，再核对授权。 |
| Token 刷新或模型请求落到错误 Region | IdC Region 与 Profile Region 被混用 | `authRegion` 使用 Token Region；`apiRegion` 使用 Profile ARN 第 4 段。 |
| 上游返回 `INVALID_MODEL_ID` | 当前账号、Profile、Region 或组织 approved list 未授权该模型 | 以 Kiro IDE 的实际模型目录为准，并联系管理员核对模型授权。 |

若配置仍有疑问，请阅读 [配置与模型可用性说明](configuration-and-model-availability.md)。Kiro Enterprise 的模型范围和 Region 也应以组织策略与 Kiro 官方文档为准。
