# Kiro 原生模型支持设计

## 背景

当前 `/v1/messages` 只精确透传 `gpt-5.6-sol`。`gpt-5.6-terra`、
`gpt-5.6-luna`、`deepseek-3.2`、`minimax-m2.5`、`minimax-m2.1`、
`glm-5` 和 `qwen3-coder-next` 会在本地 `map_model` 阶段被拒绝，
请求不会到达 Kiro 上游。

本机 Kiro IDE 于 2026-07-14 调用 `ListAvailableModels`，返回了上述七个
精确 `modelId`。这些 ID 只代表当前 Kiro 账号的控制面目录，不代表供应商
直连接口的通用别名。

## 目标

- 让七个新增 Kiro `modelId` 可通过不含 WebSearch 的 `/v1/messages`
  精确透传。
- 将这些模型加入 `/v1/models`。
- 按 Kiro 控制面返回值配置最大输入和最大输出。
- 保留现有 GPT 5.6 Sol 和 Claude 映射行为。
- 通过真实请求验证普通调用及可见思考内容。

## 非目标

- 不新增 `-thinking` 模型别名。
- 不接受 `deepseek-chat`、`deepseek-reasoner` 等供应商别名。
- 不动态调用 Kiro `ListAvailableModels`。
- 不新增认证、缓存或控制面客户端。
- 不承诺 Kiro 会透传供应商原生的思考字段。
- 不修改现有 Claude 模型的账号可用性策略。
- 不修改 WebSearch 的提前分流和模型处理行为。

## 模型规格

注册表使用 Kiro 的精确 ID。匹配前沿用现有小写标准化，因此大小写不同但
字符相同的输入可用；前缀、后缀及供应商别名不可用。

| 显示名 | Kiro ID | 最大输入 | 最大输出 | 所有者 |
|---|---|---:|---:|---|
| GPT 5.6 Sol | `gpt-5.6-sol` | 272,000 | 128,000 | `openai` |
| GPT 5.6 Terra | `gpt-5.6-terra` | 272,000 | 128,000 | `openai` |
| GPT 5.6 Luna | `gpt-5.6-luna` | 272,000 | 128,000 | `openai` |
| DeepSeek V3.2 | `deepseek-3.2` | 164,000 | 64,000 | `deepseek` |
| MiniMax M2.5 | `minimax-m2.5` | 196,000 | 64,000 | `minimax` |
| MiniMax M2.1 | `minimax-m2.1` | 196,000 | 64,000 | `minimax` |
| GLM-5 | `glm-5` | 200,000 | 64,000 | `z-ai` |
| Qwen3 Coder Next | `qwen3-coder-next` | 256,000 | 64,000 | `qwen` |

最大输入采用 Kiro `ListAvailableModels.tokenLimits.maxInputTokens`，不使用
供应商宣传的总上下文长度。`/v1/models.max_tokens` 表示最大输出，因此使用
表中的“最大输出”。

## 架构

新增一个仅负责 Kiro 原生模型事实的注册表模块。注册表暴露不可变模型规格
切片和按 ID 查询函数，不包含网络、认证或请求转换逻辑。

模型规格包含：

- `id`
- `display_name`
- `owned_by`
- `created`
- `max_input_tokens`
- `max_output_tokens`

`created` 只用于兼容现有 `/v1/models` 响应。GPT 5.6 Sol、Terra 和 Luna
统一使用 Kiro 于 2026-07-14 上线该系列的 UTC 时间戳；五个开权重模型使用
`0` 表示时间未知，避免混用供应商发布日期和 Kiro 上线日期。

三个消费者共享该注册表：

1. `map_model` 先查原生模型注册表；命中后原样返回标准化 ID。
2. `get_context_window_size` 从注册表读取最大输入；未命中时沿用 Claude 规则。
3. `get_models` 将注册表转换成现有 `Model` 响应，再追加现有 Claude 列表。

现有 Claude 名称启发式保持不变。注册表不承担 Claude 别名解析，防止精确
Kiro ID 和兼容别名两种语义混在同一层。

## 请求数据流

不含 WebSearch 的 `POST /v1/messages` 保持以下处理顺序：

1. 解析 Anthropic `MessagesRequest`。
2. 将请求模型标准化为小写。
3. 查找 Kiro 原生模型规格。
4. 原生模型命中时，将精确 ID 写入 Kiro `UserInputMessage.model_id`。
5. 未命中时，执行现有 Claude 映射。
6. 两类映射均失败时，返回现有 HTTP 400 `invalid_request_error`。
7. 映射成功后，调用现有 Kiro `GenerateAssistantResponse`。

包含 WebSearch 工具的请求会在模型转换前进入现有 WebSearch 处理器。本次不
移动该分流，也不把 WebSearch 成功视为对应 Kiro 模型可用。

本次保留现有 thinking 处理：转换器根据 Anthropic `thinking` 生成
`<thinking_mode>`、`<max_thinking_length>` 或 `<thinking_effort>` 控制标签，
再把标签注入 Kiro 历史消息。流式响应解析器在请求启用 thinking 时，将
`<thinking>...</thinking>` 转换成 Anthropic `thinking_delta`。这些标签属于
项目现有的提示级控制，不是供应商私有 API 字段，也不保证每个 Kiro 模型遵循。

## 思考能力边界

供应商原生能力与 Kiro 代理行为分开记录：

| 模型 | 供应商原生能力 | 本次对 Kiro 的承诺 |
|---|---|---|
| GPT 5.6 Sol/Terra/Luna | 支持 reasoning effort | Kiro 隐藏思维链，只保证最终文本 |
| DeepSeek V3.2 | 支持思考、非思考和 thinking tool use | 放开调用后实测，不预设透传 |
| MiniMax M2.5/M2.1 | 支持 reasoning 和 interleaved thinking | 放开调用后实测，不预设透传 |
| GLM-5 | 支持可配置 thinking，默认开启 | 放开调用后实测，不预设透传 |
| Qwen3 Coder Next | 不同官方接入文档结论冲突 | 放开调用后实测，不新增别名 |

“支持可见思考”的判定必须满足以下至少一项：

- 非流式响应含 `content[].type == "thinking"`。
- 流式响应含 `thinking_delta`。

最终文本包含分析语气不算可见思考支持。

## 错误处理

- 未注册模型继续返回本地 HTTP 400，不发送上游请求。
- 已注册但当前账号不可用的模型由上游返回错误，沿用现有错误映射。
- 注册表不等于账号授权清单；文档和日志不得宣称模型永久可用。
- 真实验证出现单个模型失败时，保留其他已验证模型，并记录上游错误类型。
- 思考块缺失不视为普通调用失败，只记录为“未观察到可见思考”。

## 测试设计

### 单元测试

- 注册表包含八个唯一 Kiro 原生 ID。
- 八个原生 ID 经 `map_model` 后精确透传。
- 每个模型返回表中定义的最大输入。
- `deepseek-chat`、`deepseek-reasoner` 和新增模型的 `-thinking` 后缀仍被拒绝。
- `/v1/models` 包含八个原生模型，且 `max_tokens` 等于最大输出。
- 现有 Claude 和 GPT Sol 测试继续通过。

### 静态验证

- `rustfmt --edition 2024 --check src/anthropic/models.rs`
- `git diff --check`
- `cargo check`
- `cargo test`

仓库基线未通过全量 `cargo fmt --check`，因此不机械格式化无关文件。现有 Rust
文件中的局部修改沿用周边格式，新建注册表文件必须独立通过 rustfmt 检查。

### 真实验证

对七个新增模型逐一执行不含 WebSearch 的非流式 `/v1/messages` 请求，要求
HTTP 200 且返回非空文本。随后逐一执行流式 thinking 请求；请求设置
`stream = true`、`max_tokens = 2048`、`thinking.type = "enabled"` 和
`thinking.budget_tokens = 1024`，再检查 SSE 事件。选择流式请求是为了绕开
非流式 `extract_thinking` 运行配置，确保所有模型使用同一判定路径。

每个模型记录：

- HTTP 状态。
- 返回模型 ID。
- 是否出现 `thinking_delta`。
- 是否只有最终文本。
- 上游错误类型（若失败）。

真实验证使用短提示，避免无意义的 Credit 消耗。验证结果写入交付说明，不写入
注册表能力字段，避免将一次观测固化成长期保证。

## 验收标准

- 七个新增模型在不含 WebSearch 的 `/v1/messages` 中不再被本地
  `UnsupportedModel` 拒绝。
- `/v1/models` 返回 GPT 5.6 Sol、七个新增模型及原有 Claude 模型。
- 上下文换算使用注册表中的 Kiro 最大输入。
- 供应商旧别名和新增 `-thinking` 别名仍不可用。
- 全部静态检查和单元测试通过。
- 真实请求结果按模型逐项报告，明确区分普通可用性和可见思考能力。

## 资料来源

- 本机模型目录记录：`docs/configuration-and-model-availability.md`
- Kiro 模型文档：https://kiro.dev/docs/models/
- Kiro GPT 5.6 说明：https://kiro.dev/changelog/
- OpenAI 模型文档：https://developers.openai.com/api/docs/models
- DeepSeek 变更记录：https://api-docs.deepseek.com/updates/
- MiniMax Anthropic API：https://platform.minimax.io/docs/api-reference/text-anthropic-api
- GLM Thinking Mode：https://docs.z.ai/guides/capabilities/thinking-mode
- Alibaba Cloud Coding Plan FAQ：https://help.aliyun.com/en/model-studio/coding-plan-faq
