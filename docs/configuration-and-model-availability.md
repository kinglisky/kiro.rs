# Kiro RS 配置与模型可用性说明

本文说明如何配置 `kiro-rs` 的 `config.json` 与 `credentials.json`，以及如何判断 Kiro 上游真正允许当前账号使用哪些模型。

> 本文的本机实测基线为 2026-07-15、Kiro Enterprise IdC、KIRO POWER、`us-east-1`。模型目录会随账号、组织策略、Region、订阅和 Kiro 发布节奏变化，不能把某一次返回结果视为所有账号的固定目录。

## 1. 配置文件的职责

`kiro-rs` 默认从当前工作目录加载两个文件：

- `config.json`：服务监听、本地 API Key、Region、TLS、代理和管理端配置。
- `credentials.json`：访问 Kiro 上游所需的 OAuth、IdC、Profile 和凭据级覆盖配置。

也可以在启动时指定其他路径：

```powershell
.\target\release\kiro-rs.exe `
  --config C:\path\to\config.json `
  --credentials C:\path\to\credentials.json
```

`config.json` 中的 `apiKey` 是客户端访问本地代理时使用的入站密钥，不是 Kiro 的 `accessToken`、`refreshToken` 或 `kiroApiKey`。不要混用这些字段。

## 2. 推荐的最小 `config.json`

```json
{
  "host": "127.0.0.1",
  "port": 8990,
  "apiKey": "sk-kiro-local-请替换为足够长的随机值",
  "region": "us-east-1",
  "authRegion": "us-east-1",
  "apiRegion": "us-east-1",
  "tlsBackend": "rustls",
  "defaultEndpoint": "ide"
}
```

推荐保持 `host` 为 `127.0.0.1`。只有明确需要让其他机器访问，并且已配置防火墙、强随机 API Key 和可信网络时，才考虑改为 `0.0.0.0`。

可以用 PowerShell 生成随机 API Key：

```powershell
$bytes = [byte[]]::new(24)
[System.Security.Cryptography.RandomNumberGenerator]::Fill($bytes)
$suffix = [Convert]::ToBase64String($bytes).TrimEnd('=').Replace('+', '-').Replace('/', '_')
"sk-kiro-local-$suffix"
```

## 3. `config.json` 字段详解

字段使用 camelCase。下表默认值以当前源码为准。

| 字段 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `host` | string | `127.0.0.1` | HTTP 服务监听地址。仅本机使用时不要改为公网地址。 |
| `port` | number | `8080` | HTTP 服务监听端口。示例使用 `8990`。 |
| `apiKey` | string | 无 | 本地客户端认证密钥。程序启动时缺失会退出。支持 `x-api-key` 和 Bearer 认证。 |
| `region` | string | `us-east-1` | Auth/API Region 的全局回退值。 |
| `authRegion` | string | 回退到 `region` | OAuth/IdC Token 刷新使用的 Region。 |
| `apiRegion` | string | 回退到 `region` | Kiro API 请求使用的 Region。 |
| `kiroVersion` | string | `0.11.107` | 上游请求中的 Kiro 客户端版本标识。通常使用默认值。 |
| `machineId` | string | 自动生成 | 64 位十六进制机器标识。未配置时由凭据稳定派生。 |
| `systemVersion` | string | 自动选择 | 上游 User-Agent 使用的系统版本标识。通常使用默认值。 |
| `nodeVersion` | string | `22.22.0` | 上游 User-Agent 使用的 Node.js 版本标识。通常使用默认值。 |
| `tlsBackend` | string | `rustls` | `rustls` 或 `native-tls`。代理证书或 Token 刷新出现 TLS 问题时可尝试 `native-tls`。 |
| `proxyUrl` | string | 无 | 全局 HTTP、HTTPS 或 SOCKS5 代理，例如 `http://127.0.0.1:7890`。 |
| `proxyUsername` | string | 无 | 全局代理用户名。 |
| `proxyPassword` | string | 无 | 全局代理密码。 |
| `countTokensApiUrl` | string | 无 | 外部 `count_tokens` 服务地址。未配置时使用项目内置估算。 |
| `countTokensApiKey` | string | 无 | 外部 `count_tokens` 服务密钥。 |
| `countTokensAuthType` | string | `x-api-key` | 外部 Token 统计服务认证方式：`x-api-key` 或 `bearer`。 |
| `adminApiKey` | string | 无 | 配置非空值后启用 Admin API 和 Admin UI。应与 `apiKey` 使用不同的强随机值。 |
| `loadBalancingMode` | string | `priority` | `priority` 按优先级使用凭据；`balanced` 在可用凭据间均衡分配。 |
| `extractThinking` | boolean | `true` | 非流式响应是否将 `<thinking>` 内容解析为独立内容块。 |
| `defaultEndpoint` | string | `ide` | 凭据未指定 `endpoint` 时使用的 Kiro 端点。当前注册端点为 `ide`。 |
| `endpoints` | object | `{}` | 端点特定扩展配置。没有明确需求时保持为空。 |

### 3.1 Region 优先级

Auth Region 用于刷新 Token，优先级为：

```text
credentials.authRegion
  > credentials.region
  > config.authRegion
  > config.region
```

API Region 用于请求模型，优先级为：

```text
credentials.apiRegion
  > config.apiRegion
  > config.region
```

如果 Kiro Profile 和 IdC 均位于 `us-east-1`，建议三个字段统一配置为 `us-east-1`，避免 Token 刷新和模型请求落到不同 Region。

### 3.2 代理优先级

```text
credentials.proxyUrl > config.proxyUrl > 直连
```

凭据中的 `proxyUrl: "direct"` 表示该凭据强制直连，即使全局配置了代理。

## 4. IdC `credentials.json` 配置

Enterprise / AWS IAM Identity Center 推荐使用以下结构：

```json
{
  "accessToken": "Kiro 缓存中的 accessToken，可选",
  "refreshToken": "Kiro 缓存中的 refreshToken",
  "profileArn": "arn:aws:codewhisperer:us-east-1:ACCOUNT_ID:profile/PROFILE_ID",
  "expiresAt": "2026-12-31T00:00:00Z",
  "authMethod": "idc",
  "clientId": "AWS SSO OIDC clientId",
  "clientSecret": "AWS SSO OIDC clientSecret",
  "region": "us-east-1",
  "authRegion": "us-east-1",
  "apiRegion": "us-east-1"
}
```

### 4.1 本机凭据来源

Kiro IDE 常见的 Windows 缓存位置：

```text
C:\Users\<用户名>\.aws\sso\cache\kiro-auth-token.json
C:\Users\<用户名>\.aws\sso\cache\<clientIdHash>.json
C:\Users\<用户名>\AppData\Roaming\Kiro\User\globalStorage\kiro.kiroagent\profile.json
```

字段对应关系：

| `credentials.json` 字段 | 来源 |
|---|---|
| `accessToken` | `kiro-auth-token.json.accessToken` |
| `refreshToken` | `kiro-auth-token.json.refreshToken` |
| `expiresAt` | `kiro-auth-token.json.expiresAt` |
| `authMethod` | Enterprise IdC 固定写成 `idc` |
| `region` / `authRegion` / `apiRegion` | `kiro-auth-token.json.region` |
| `clientId` | `<clientIdHash>.json.clientId` |
| `clientSecret` | `<clientIdHash>.json.clientSecret` |
| `profileArn` | Kiro `profile.json.arn` |

`clientIdHash` 的值对应另一个 AWS SSO 缓存文件的文件名。不要使用“遍历后随便取第一个 JSON”的方式关联客户端注册信息，应按 `clientIdHash` 精确定位。

### 4.2 为什么建议配置 `profileArn`

Kiro Enterprise 的模型目录、订阅、额度和管理员策略都绑定到 Kiro Profile。请求带上正确的 `profileArn` 后，上游才能按对应组织 Profile 应用模型授权和配额。

缺少 `profileArn` 不一定立即表现为 401；它也可能表现为模型目录不完整、额度信息不正确或请求参数被上游拒绝。因此，Enterprise IdC 场景不应把它当作可有可无的字段。

### 4.3 其他凭据字段

| 字段 | 说明 |
|---|---|
| `id` | 多凭据和 Admin API 使用的本地唯一 ID。手工单凭据配置可省略。 |
| `priority` | 数字越小优先级越高，默认 `0`。 |
| `machineId` | 凭据级机器标识，优先于全局 `machineId`。 |
| `email` | 可选展示信息，不参与认证。 |
| `subscriptionTitle` | 服务查询额度后记录的订阅名称。 |
| `proxyUrl` / `proxyUsername` / `proxyPassword` | 凭据级代理，优先于全局代理。 |
| `disabled` | 是否禁用该凭据，默认 `false`。 |
| `kiroApiKey` | Headless API Key 凭据。配置后无需 OAuth `refreshToken`。 |
| `endpoint` | 凭据使用的端点名称，默认继承 `config.defaultEndpoint`。 |

`credentials.json` 支持单对象和数组。数组格式可使用 `priority` 或 `balanced` 实现故障转移和负载均衡。

## 5. 凭据安全

- `config.json`、`credentials.json` 已在仓库 `.gitignore` 中忽略，不要使用 `git add -f` 强制提交。
- 不要在 Issue、日志、截图或聊天中暴露 `accessToken`、`refreshToken`、`clientSecret`、`kiroApiKey` 或完整本地 API Key。
- 本地 API Key 与 Admin API Key 应使用不同的随机值。
- 如果凭据已经泄露，应在 Kiro/AWS 侧重新登录或撤销客户端注册，而不是只删除本地文件。

## 6. 模型列表的三个层次

理解以下三层，可以避免把“代理显示了模型”误判为“上游允许调用模型”。

### 6.1 `kiro-rs` 的 `/v1/models`

当前 `/v1/models` 是本地静态模型声明，主要用于兼容 Anthropic 客户端。它不会实时调用 Kiro 的模型目录服务。

因此，即使本地接口列出了 Claude，也不代表当前 Kiro Profile 有权使用 Claude。

### 6.2 `kiro-rs` 的模型映射

模型映射只负责将客户端模型名称转换为 Kiro 请求中的 `modelId`。映射代码能够生成某个 Claude `modelId`，也不代表上游授权该 ID。

以下 Kiro 原生 ID 在本项目中采用精确透传，不会伪装成 Claude：
`gpt-5.6-sol`、`gpt-5.6-terra`、`gpt-5.6-luna`、`deepseek-3.2`、
`minimax-m2.5`、`minimax-m2.1`、`glm-5`、`qwen3-coder-next`。
供应商别名和新增模型的 `-thinking` 后缀不会映射到这些 ID。

### 6.3 Kiro 上游 `ListAvailableModels`

Kiro IDE 根据当前登录身份、Profile、订阅、Region 和组织策略获取实际模型目录。这个结果才是当前账号可调用模型的权威依据。

模型不在该目录中时，即使本地代理接受了名称，上游仍可能返回：

```json
{
  "message": "Invalid model. Please select a different model to continue.",
  "reason": "INVALID_MODEL_ID"
}
```

## 7. 本机为什么只有 GPT/DeepSeek/MiniMax/GLM/Qwen

### 7.1 本机实测结果

2026-07-14 23:13（Asia/Shanghai），当前 Enterprise IdC / KIRO POWER / `us-east-1` Profile 的 Kiro IDE `ListAvailableModels` 返回：

| 模型 ID | 本机返回的最大输入 | 最大输出 | Credit 倍率 |
|---|---:|---:|---:|
| `gpt-5.6-sol` | 272k | 128k | 2.4x |
| `gpt-5.6-terra` | 272k | 128k | 1.2x |
| `gpt-5.6-luna` | 272k | 128k | 0.6x |
| `deepseek-3.2` | 164k | 64k | 0.25x |
| `minimax-m2.5` | 196k | 64k | 0.25x |
| `minimax-m2.1` | 196k | 64k | 0.15x |
| `glm-5` | 200k | 64k | 0.5x |
| `qwen3-coder-next` | 256k | 64k | 0.05x |

默认模型为 `gpt-5.6-sol`，目录中没有任何 Claude 模型。实际请求 `claude-sonnet-4.5`、`claude-sonnet-4.6` 和 `claude-haiku-4.5` 均被上游以 `INVALID_MODEL_ID` 拒绝；`gpt-5.6-sol` 已真实请求成功并返回 HTTP 200。

### 7.2 最可能原因：Enterprise 模型 allow list

Kiro 官方模型文档仍列出 Claude Opus、Sonnet 和 Haiku，并说明这些模型通常支持 Power、IdC 和 `us-east-1`。因此，本机缺少 Claude 不能解释为“Kiro 已在全球移除 Claude”。

Kiro Enterprise 支持管理员启用模型可用性控制并维护 approved list。启用后：

- 只有 approved list 中的模型会出现在客户端。
- 新发布模型不会自动提供给组织用户，必须由管理员加入列表。
- 管理员可以设置组织默认模型。

本机满足 Claude 的常见订阅、认证和 Region 条件，但实际目录只返回一组非 Claude 模型，并将 `gpt-5.6-sol` 设为默认模型。综合这些证据，最可能的解释是：当前 Enterprise Kiro Profile 的模型 allow list 未批准 Claude，或明确只批准了当前返回的模型。

这是基于官方机制与本机响应做出的推断；只有组织 Kiro 管理员能在控制台确认最终配置。

相关官方资料：

- [Kiro 模型与可用性](https://kiro.dev/docs/models/)
- [Kiro Enterprise 模型治理与 approved list](https://kiro.dev/docs/enterprise/governance/model/)
- [Kiro Enterprise Profile 概念](https://kiro.dev/docs/enterprise/concepts/)
- [GPT 5.6 Sol/Terra/Luna 发布说明](https://kiro.dev/changelog/models/gpt-5-6/)

### 7.3 其他可能原因

| 可能性 | 如何判断 | 本机可能性 |
|---|---|---|
| Region 或国家限制 | 对照官方模型 Region，并确认 `apiRegion` 与 Profile Region | 较低：当前为官方支持的 `us-east-1` |
| 订阅等级不足 | 检查 Kiro 订阅信息 | 较低：当前为 KIRO POWER |
| 分阶段发布 | 对照模型发布时间和账号发布范围 | 中低：可能影响单个新模型，难以解释所有 Claude 同时缺失 |
| IDE/会话缓存 | 修改 allow list 后重启 Kiro IDE、CLI 和代理 | 修改策略后必须排查 |
| `profileArn` 错误或缺失 | 对照 Kiro `profile.json.arn` | 已修复并精确配置 |
| 本地模型映射错误 | 查看发送到上游的最终 `modelId` | Claude ID 能生成，但上游目录没有对应模型 |

## 8. 如何恢复 Claude 可用性

如果你是 Kiro Enterprise 管理员：

1. 打开 AWS Console 中的 Kiro 控制台。
2. 进入 **Settings**。
3. 在 **Shared settings** 中检查 **Model availability**。
4. 如果启用了 **Control which models are available to users**，打开 approved list。
5. 加入需要的 Claude Opus、Sonnet 或 Haiku，并设置合适的默认模型。
6. 保存配置。
7. 重启 Kiro IDE/CLI 和 `kiro-rs`，重新获取模型目录。

如果你不是管理员，应把当前 Profile ARN、Region、订阅等级和缺失模型名称提供给组织管理员，但不要发送 Token 或 Secret。

不要为了让 Claude 名称“看起来能用”而静默映射到 `gpt-5.6-sol`。这种做法只能实现客户端名称兼容，实际推理模型仍是 GPT，会造成能力、费用、上下文窗口和审计信息不一致。如确实需要别名回退，应显式配置并向调用方公开真实上游模型。

## 9. 启动与验证

### 9.1 启动服务

```powershell
.\target\release\kiro-rs.exe
```

### 9.2 获取本地模型声明

```powershell
$config = Get-Content .\config.json -Raw | ConvertFrom-Json
$headers = @{ 'x-api-key' = $config.apiKey }
Invoke-RestMethod http://127.0.0.1:8990/v1/models -Headers $headers
```

注意：这个接口返回的是 `kiro-rs` 本地声明，不是 Kiro 上游实时授权目录。

### 9.3 验证 `gpt-5.6-sol`

```powershell
$config = Get-Content .\config.json -Raw | ConvertFrom-Json
$headers = @{ 'x-api-key' = $config.apiKey }
$body = @{
  model = 'gpt-5.6-sol'
  max_tokens = 32
  stream = $false
  messages = @(
    @{ role = 'user'; content = 'Reply with exactly: OK' }
  )
} | ConvertTo-Json -Depth 6

Invoke-RestMethod `
  -Uri http://127.0.0.1:8990/v1/messages `
  -Method Post `
  -Headers $headers `
  -ContentType 'application/json' `
  -Body $body
```

成功响应应满足：

- HTTP 状态码为 `200`。
- `model` 为 `gpt-5.6-sol`。
- `type` 为 `message`。
- `content` 中包含模型返回文本。

### 9.4 常见错误

| 错误 | 最可能原因 | 处理方式 |
|---|---|---|
| `401 Unauthorized` | 本地 `apiKey` 错误 | 从 `config.json` 读取正确的本地 API Key。 |
| `INVALID_MODEL_ID` | 模型不在当前 Kiro Profile 实际目录中 | 检查 Enterprise approved list、Region、订阅和会话缓存。 |
| Token 刷新 400/401 | `refreshToken`、`clientId` 或 `clientSecret` 失效 | 重新登录 Kiro IDE，并重新合并 AWS SSO 缓存。 |
| TLS/证书错误 | 代理证书与 `rustls` 不兼容 | 安装可信 CA，或将 `tlsBackend` 改为 `native-tls`。 |
| 模型列表有 Claude，但调用失败 | `/v1/models` 是本地静态声明 | 以 Kiro IDE 的实际上游目录和真实调用结果为准。 |

## 10. 结论

- `config.json` 管理本地代理；`credentials.json` 管理 Kiro 上游身份，两者不可混用。
- Enterprise IdC 应完整配置 `refreshToken`、`clientId`、`clientSecret`、Region 和 `profileArn`。
- 当前账号真实可用目录只包含 GPT/DeepSeek/MiniMax/GLM/Qwen；本地精确映射不等于账号永久授权，实际可用性以最新真实调用为准。
- Claude 缺失最可能由 Enterprise Kiro Profile 的模型 allow list 导致，不是 `kiro-rs` 映射本身，也不是 Kiro 全球下架 Claude。
- 恢复真实 Claude 的正确方式是由组织管理员批准 Claude，并在客户端重启后确认它重新出现在 `ListAvailableModels` 中。
