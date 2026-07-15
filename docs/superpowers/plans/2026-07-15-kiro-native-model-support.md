# Kiro 原生模型支持实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让七个新增 Kiro 原生模型通过 `/v1/messages` 精确透传，并由同一注册表驱动上下文窗口和 `/v1/models`。

**Architecture:** 新建无网络依赖的原生模型注册表，集中保存 ID、展示信息和 Token 限制。转换器优先查询注册表，再沿用 Claude 启发式；模型列表端点把注册表转换成现有 API 类型后追加 Claude 静态列表。

**Tech Stack:** Rust、Axum、Serde、Tokio、Cargo 内置测试框架、curl、jq、ripgrep。

## Global Constraints

- 精确支持 `gpt-5.6-sol`、`gpt-5.6-terra`、`gpt-5.6-luna`、`deepseek-3.2`、`minimax-m2.5`、`minimax-m2.1`、`glm-5`、`qwen3-coder-next`。
- 保留现有 Claude 映射、WebSearch 提前分流和 thinking 标签注入行为。
- 不增加 `deepseek-chat`、`deepseek-reasoner`、供应商别名或新增模型的 `-thinking` 别名。
- 最大输入使用 Kiro 控制面值；`/v1/models.max_tokens` 使用最大输出。
- 不增加依赖，不读取或输出认证 Token。
- 使用 Early Return；不得新增冗长 `if / else if / else` 分支。
- 规格和计划文档不提交 Git；代码与用户文档提交使用中文标准格式。
- 当前基线有八个使用过期 Claude 名称的 converter 测试失败；先只更新测试输入，不放宽 Claude 生产映射。
- macOS 沙箱可能使 `http_client::tests::test_build_client_without_proxy` 以 `Attempted to create a NULL object` 失败；完整测试需在允许 SystemConfiguration 访问的环境复跑。
- 仓库基线未通过全量 `cargo fmt --check`；只对新建注册表运行 `rustfmt --edition 2024`，现有文件的局部改动手工遵循周边格式，禁止机械改动无关行。

---

## File Structure

- Create: `src/anthropic/models.rs` — Kiro 原生模型规格和精确查询。
- Modify: `src/anthropic/mod.rs` — 注册内部模型模块。
- Modify: `src/anthropic/converter.rs` — 复用注册表完成模型透传和上下文查询。
- Modify: `src/anthropic/handlers.rs` — 由注册表生成 `/v1/models` 原生模型项。
- Modify: `README.md` — 公开原生模型映射。
- Modify: `docs/configuration-and-model-availability.md` — 更新精确透传说明。

---

### Task 1: 修正过期 Claude 测试输入

**Files:**
- Modify: `src/anthropic/converter.rs:914-937,1001,1105,1155,1218,1317,1349,1756`

**Interfaces:**
- Consumes: 现有 Claude 4.5/4.6 映射规则。
- Produces: 不改变生产行为的绿色 converter 测试基线。

- [ ] **Step 1: 将模型映射测试改为当前支持的精确版本**

把 `test_map_model_sonnet` 改为：

```rust
#[test]
fn test_map_model_sonnet() {
    assert_eq!(
        map_model("claude-sonnet-4-6"),
        Some("claude-sonnet-4.6".to_string())
    );
    assert_eq!(
        map_model("claude-sonnet-4-5-20250929"),
        Some("claude-sonnet-4.5".to_string())
    );
}
```

把 `test_map_model_opus` 改为：

```rust
#[test]
fn test_map_model_opus() {
    assert_eq!(
        map_model("claude-opus-4-5-20251101"),
        Some("claude-opus-4.5".to_string())
    );
}
```

- [ ] **Step 2: 更新请求转换测试的陈旧 fixture**

把 `src/anthropic/converter.rs` 中七处：

```rust
model: "claude-sonnet-4".to_string(),
```

统一替换为：

```rust
model: "claude-sonnet-4-5-20250929".to_string(),
```

- [ ] **Step 3: 运行 converter 基线测试**

Run: `cargo test anthropic::converter::tests -- --nocapture`

Expected: 39 个 converter 测试全部 PASS；生产代码没有变化。

- [ ] **Step 4: 提交测试基线修正**

```bash
git add src/anthropic/converter.rs
git commit -m "test: 更新 Claude 模型测试用例"
```

---

### Task 2: 建立 Kiro 原生模型注册表

**Files:**
- Create: `src/anthropic/models.rs`
- Modify: `src/anthropic/mod.rs`

**Interfaces:**
- Consumes: 无。
- Produces: `NativeModelSpec`、`NATIVE_MODELS: &[NativeModelSpec]`、`find_native_model(&str) -> Option<&'static NativeModelSpec>`。

- [ ] **Step 1: 写入失败测试并注册模块**

在 `src/anthropic/mod.rs` 的 `mod middleware;` 前加入：

```rust
mod models;
```

创建 `src/anthropic/models.rs`，先只写测试：

```rust
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{NATIVE_MODELS, find_native_model};

    const EXPECTED_MODELS: [(&str, i32, i32); 8] = [
        ("gpt-5.6-sol", 272_000, 128_000),
        ("gpt-5.6-terra", 272_000, 128_000),
        ("gpt-5.6-luna", 272_000, 128_000),
        ("deepseek-3.2", 164_000, 64_000),
        ("minimax-m2.5", 196_000, 64_000),
        ("minimax-m2.1", 196_000, 64_000),
        ("glm-5", 200_000, 64_000),
        ("qwen3-coder-next", 256_000, 64_000),
    ];

    #[test]
    fn native_models_have_unique_ids_and_expected_limits() {
        let ids: HashSet<_> = NATIVE_MODELS.iter().map(|model| model.id).collect();
        assert_eq!(ids.len(), NATIVE_MODELS.len());
        assert_eq!(NATIVE_MODELS.len(), EXPECTED_MODELS.len());

        for (id, max_input_tokens, max_output_tokens) in EXPECTED_MODELS {
            let model = find_native_model(id).expect("native model must exist");
            assert_eq!(model.max_input_tokens, max_input_tokens);
            assert_eq!(model.max_output_tokens, max_output_tokens);
        }
    }

    #[test]
    fn native_model_lookup_normalizes_ascii_case_only() {
        assert_eq!(
            find_native_model("MiniMax-M2.5").map(|model| model.id),
            Some("minimax-m2.5")
        );
        assert!(find_native_model("deepseek-chat").is_none());
        assert!(find_native_model("glm-5-thinking").is_none());
    }
}
```

- [ ] **Step 2: 运行测试，确认因注册表尚未实现而失败**

Run: `cargo test anthropic::models::tests -- --nocapture`

Expected: 编译失败，提示 `NATIVE_MODELS` 和 `find_native_model` 未定义。

- [ ] **Step 3: 写入最小注册表实现**

在测试模块前加入：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeModelSpec {
    pub(crate) id: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) owned_by: &'static str,
    pub(crate) created: i64,
    pub(crate) max_input_tokens: i32,
    pub(crate) max_output_tokens: i32,
}

pub(crate) const NATIVE_MODELS: &[NativeModelSpec] = &[
    NativeModelSpec { id: "gpt-5.6-sol", display_name: "GPT 5.6 Sol", owned_by: "openai", created: 1_783_987_200, max_input_tokens: 272_000, max_output_tokens: 128_000 },
    NativeModelSpec { id: "gpt-5.6-terra", display_name: "GPT 5.6 Terra", owned_by: "openai", created: 1_783_987_200, max_input_tokens: 272_000, max_output_tokens: 128_000 },
    NativeModelSpec { id: "gpt-5.6-luna", display_name: "GPT 5.6 Luna", owned_by: "openai", created: 1_783_987_200, max_input_tokens: 272_000, max_output_tokens: 128_000 },
    NativeModelSpec { id: "deepseek-3.2", display_name: "DeepSeek V3.2", owned_by: "deepseek", created: 0, max_input_tokens: 164_000, max_output_tokens: 64_000 },
    NativeModelSpec { id: "minimax-m2.5", display_name: "MiniMax M2.5", owned_by: "minimax", created: 0, max_input_tokens: 196_000, max_output_tokens: 64_000 },
    NativeModelSpec { id: "minimax-m2.1", display_name: "MiniMax M2.1", owned_by: "minimax", created: 0, max_input_tokens: 196_000, max_output_tokens: 64_000 },
    NativeModelSpec { id: "glm-5", display_name: "GLM-5", owned_by: "z-ai", created: 0, max_input_tokens: 200_000, max_output_tokens: 64_000 },
    NativeModelSpec { id: "qwen3-coder-next", display_name: "Qwen3 Coder Next", owned_by: "qwen", created: 0, max_input_tokens: 256_000, max_output_tokens: 64_000 },
];

pub(crate) fn find_native_model(model: &str) -> Option<&'static NativeModelSpec> {
    let normalized = model.to_ascii_lowercase();
    NATIVE_MODELS
        .iter()
        .find(|candidate| candidate.id == normalized.as_str())
}
```

- [ ] **Step 4: 格式化并运行注册表测试**

Run: `rustfmt --edition 2024 src/anthropic/models.rs && cargo test anthropic::models::tests -- --nocapture`

Expected: 两个注册表测试均 PASS。

- [ ] **Step 5: 提交注册表**

```bash
git add src/anthropic/models.rs src/anthropic/mod.rs
git commit -m "feat: 增加 Kiro 原生模型注册表"
```

---

### Task 3: 接入请求映射和上下文窗口

**Files:**
- Modify: `src/anthropic/converter.rs:80-132`
- Test: `src/anthropic/converter.rs:914-975`

**Interfaces:**
- Consumes: `find_native_model(&str) -> Option<&'static NativeModelSpec>`。
- Produces: 八个原生 ID 的精确 `map_model` 结果和对应上下文窗口。

- [ ] **Step 1: 先扩展转换器测试**

用下面两个测试替换现有 `test_map_model_gpt_5_6_sol_passthrough`，并保留其他测试：

```rust
#[test]
fn test_map_native_models_passthrough_and_contexts() {
    let models = [
        ("gpt-5.6-sol", 272_000),
        ("gpt-5.6-terra", 272_000),
        ("gpt-5.6-luna", 272_000),
        ("deepseek-3.2", 164_000),
        ("minimax-m2.5", 196_000),
        ("minimax-m2.1", 196_000),
        ("glm-5", 200_000),
        ("qwen3-coder-next", 256_000),
    ];

    for (model, context_window) in models {
        assert_eq!(map_model(model).as_deref(), Some(model));
        assert_eq!(get_context_window_size(model), context_window);
    }
}

#[test]
fn test_map_native_models_rejects_unsupported_aliases() {
    for model in [
        "deepseek-chat",
        "deepseek-reasoner",
        "gpt-5.6-terra-thinking",
        "minimax-m2.5-thinking",
        "glm-5-thinking",
        "qwen3-coder-next-thinking",
    ] {
        assert!(map_model(model).is_none(), "unexpected mapping for {model}");
    }
}
```

- [ ] **Step 2: 运行新增测试，确认七个新增 ID 失败**

Run: `cargo test anthropic::converter::tests::test_map_native_models -- --nocapture`

Expected: `gpt-5.6-terra` 首个断言失败，实际结果为 `None`。

- [ ] **Step 3: 在转换器中接入注册表**

在 `use super::types` 前加入：

```rust
use super::models::find_native_model;
```

把 `map_model` 开头改为：

```rust
pub fn map_model(model: &str) -> Option<String> {
    let model_lower = model.to_lowercase();

    if let Some(native_model) = find_native_model(&model_lower) {
        return Some(native_model.id.to_string());
    }

    if model_lower.contains("sonnet") {
```

删除原先只判断 `gpt-5.6-sol` 的 Early Return，其余 Claude 分支保持原样。

用下面实现替换 `get_context_window_size`：

```rust
pub fn get_context_window_size(model: &str) -> i32 {
    if let Some(native_model) = find_native_model(model) {
        return native_model.max_input_tokens;
    }

    match map_model(model) {
        Some(mapped)
            if matches!(
                mapped.as_str(),
                "claude-sonnet-4.6"
                    | "claude-opus-4.6"
                    | "claude-opus-4.7"
                    | "claude-opus-4.8"
            ) =>
        {
            1_000_000
        }
        _ => 200_000,
    }
}
```

- [ ] **Step 4: 运行新增测试和现有转换器回归测试**

Run: `cargo test anthropic::converter::tests -- --nocapture`

Expected: 新增原生模型测试 PASS；现有 Claude 映射和请求转换测试全部 PASS。

- [ ] **Step 5: 提交映射与上下文支持**

```bash
git add src/anthropic/converter.rs
git commit -m "feat: 支持 Kiro 原生模型请求映射"
```

---

### Task 4: 扩展 `/v1/models`

**Files:**
- Modify: `src/anthropic/handlers.rs:1-218`
- Test: `src/anthropic/handlers.rs`

**Interfaces:**
- Consumes: `NativeModelSpec`、`NATIVE_MODELS`。
- Produces: `/v1/models` 中八个原生模型的 API 表示。

- [ ] **Step 1: 写入端点失败测试**

在 `src/anthropic/handlers.rs` 末尾加入：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_models_contains_every_native_model_with_output_limit() {
        let response = get_models().await.into_response();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("models response body");
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("valid models response");
        let models = payload["data"].as_array().expect("models data array");

        for spec in crate::anthropic::models::NATIVE_MODELS {
            let model = models
                .iter()
                .find(|model| model["id"] == spec.id)
                .unwrap_or_else(|| panic!("missing model {}", spec.id));
            assert_eq!(model["max_tokens"], spec.max_output_tokens);
            assert_eq!(model["owned_by"], spec.owned_by);
        }
    }
}
```

- [ ] **Step 2: 运行端点测试，确认新增模型缺失**

Run: `cargo test anthropic::handlers::tests::get_models_contains_every_native_model_with_output_limit -- --nocapture`

Expected: FAIL，错误为 `missing model gpt-5.6-terra`。

- [ ] **Step 3: 用注册表生成原生模型响应**

在 handler 的模块导入区加入：

```rust
use super::models::{NATIVE_MODELS, NativeModelSpec};
```

在 `get_models` 前加入：

```rust
fn native_model_to_response(spec: &NativeModelSpec) -> Model {
    Model {
        id: spec.id.to_string(),
        object: "model".to_string(),
        created: spec.created,
        owned_by: spec.owned_by.to_string(),
        display_name: spec.display_name.to_string(),
        model_type: "chat".to_string(),
        max_tokens: spec.max_output_tokens,
    }
}
```

在 `get_models` 中，把 `let models = vec![` 和紧随其后的 GPT Sol `Model` 块替换为：

```rust
let mut models: Vec<Model> = NATIVE_MODELS
    .iter()
    .map(native_model_to_response)
    .collect();

models.extend([
```

保留从 `claude-opus-4-8` 开始的所有 Claude `Model` 块。把 Claude 列表末尾的
`];` 改为 `]);`。`Json(ModelsResponse { ... })` 保持原样。

- [ ] **Step 4: 运行端点测试和 Anthropic 模块回归测试**

Run: `cargo test anthropic::handlers::tests -- --nocapture && cargo test anthropic:: -- --nocapture`

Expected: 端点测试 PASS；Anthropic 模块全部测试 PASS。

- [ ] **Step 5: 提交模型列表支持**

```bash
git add src/anthropic/handlers.rs
git commit -m "feat: 扩展原生模型列表接口"
```

---

### Task 5: 更新用户文档并完成静态验证

**Files:**
- Modify: `README.md:44,438-447`
- Modify: `docs/configuration-and-model-availability.md:191-197,338`

**Interfaces:**
- Consumes: 最终原生模型注册表。
- Produces: 与代码一致的用户可见模型映射说明。

- [ ] **Step 1: 更新 README 功能说明和映射表**

把“多模型支持”功能项改为：

```markdown
- **多模型支持**: 精确支持 GPT 5.6、DeepSeek V3.2、MiniMax M2.5/M2.1、GLM-5、Qwen3 Coder Next，并保留 Claude 系列兼容映射
```

在“模型映射”表头后、Claude 行前加入：

```markdown
| `gpt-5.6-sol` | `gpt-5.6-sol` |
| `gpt-5.6-terra` | `gpt-5.6-terra` |
| `gpt-5.6-luna` | `gpt-5.6-luna` |
| `deepseek-3.2` | `deepseek-3.2` |
| `minimax-m2.5` | `minimax-m2.5` |
| `minimax-m2.1` | `minimax-m2.1` |
| `glm-5` | `glm-5` |
| `qwen3-coder-next` | `qwen3-coder-next` |
```

- [ ] **Step 2: 更新配置与模型可用性文档**

把 6.2 节中的单模型说明替换为：

```markdown
以下 Kiro 原生 ID 在本项目中采用精确透传，不会伪装成 Claude：
`gpt-5.6-sol`、`gpt-5.6-terra`、`gpt-5.6-luna`、`deepseek-3.2`、
`minimax-m2.5`、`minimax-m2.1`、`glm-5`、`qwen3-coder-next`。
供应商别名和新增模型的 `-thinking` 后缀不会映射到这些 ID。
```

把结论中的“`gpt-5.6-sol` 已验证成功”改为：

```markdown
- 当前账号真实可用目录只包含 GPT/DeepSeek/MiniMax/GLM/Qwen；本地精确映射不等于账号永久授权，实际可用性以最新真实调用为准。
```

- [ ] **Step 3: 运行完整静态验证**

Run: `rustfmt --edition 2024 --check src/anthropic/models.rs && git diff --check && cargo check && cargo test`

Expected: 三条命令均以状态码 0 完成，所有测试 PASS。若完整测试只有
`http_client::tests::test_build_client_without_proxy` 因 macOS SystemConfiguration
沙箱报错，则在允许系统配置访问的环境原样复跑 `cargo test`，复跑必须 PASS。

- [ ] **Step 4: 检查差异只包含本次范围**

Run: `git diff --check && git status --short`

Expected: 无空白错误；只出现 README、模型可用性文档以及未提交的 `docs/superpowers/` 计划/规格目录。

- [ ] **Step 5: 提交用户文档**

```bash
git add README.md docs/configuration-and-model-availability.md
git commit -m "docs: 更新 Kiro 原生模型支持说明"
```

---

### Task 6: 构建并真实验证七个新增模型

**Files:**
- No file changes.

**Interfaces:**
- Consumes: `target/release/kiro-rs`、本地 `config.json`、本地 `credentials.json`。
- Produces: 七个模型的普通可用性和可见 thinking 观测结果。

- [ ] **Step 1: 构建 Release 二进制并重启服务**

Run: `cargo build --release`

Expected: 状态码 0，生成 `target/release/kiro-rs`。

Run: `zsh -ic 'kiro-stop'`

Expected: 现有 `kiro-rs` 进程收到 TERM 并退出。

Run: `zsh -ic 'kiro-start'`（在长运行 PTY 会话中执行）

Expected: 服务监听 `127.0.0.1:8990`，进程保持运行。

确认健康状态：

Run: `curl -sS http://127.0.0.1:8990/health`

Expected: HTTP 200 健康响应。

- [ ] **Step 2: 验证 `/v1/models` 返回七个新增模型**

```bash
API_KEY="$(jq -r '.apiKey' config.json)"
curl -sS http://127.0.0.1:8990/v1/models \
  -H "x-api-key: ${API_KEY}" \
  | jq -e '[.data[].id] as $ids | [
      "gpt-5.6-terra", "gpt-5.6-luna", "deepseek-3.2",
      "minimax-m2.5", "minimax-m2.1", "glm-5", "qwen3-coder-next"
    ] as $expected | all($expected[]; . as $id | ($ids | index($id)) != null)'
```

Expected: 输出 `true`，状态码 0。命令不得打印 API Key。

- [ ] **Step 3: 逐一验证普通非流式消息**

```bash
API_KEY="$(jq -r '.apiKey' config.json)"
MODELS=(gpt-5.6-terra gpt-5.6-luna deepseek-3.2 minimax-m2.5 minimax-m2.1 glm-5 qwen3-coder-next)

for model in "${MODELS[@]}"; do
  body="/tmp/kiro-${model}.json"
  status="$(curl -sS -o "${body}" -w '%{http_code}' \
    http://127.0.0.1:8990/v1/messages \
    -H "x-api-key: ${API_KEY}" \
    -H 'content-type: application/json' \
    -d "$(jq -nc --arg model "${model}" '{
      model: $model,
      max_tokens: 64,
      messages: [{role: "user", content: "仅返回 OK"}]
    }')")"
  [[ "${status}" == 200 ]] || {
    print -u2 -r -- "${model} HTTP ${status}"
    exit 1
  }
  jq -e --arg model "${model}" \
    '.model == $model and any(.content[]; .type == "text" and (.text | length > 0))' \
    "${body}" >/dev/null || {
      jq . "${body}" >&2
      exit 1
    }
  print -r -- "${model} HTTP ${status} text=true"
done
```

Expected: 七行均为 `HTTP 200 text=true`。

- [ ] **Step 4: 逐一探测流式可见 thinking**

```bash
API_KEY="$(jq -r '.apiKey' config.json)"
MODELS=(gpt-5.6-terra gpt-5.6-luna deepseek-3.2 minimax-m2.5 minimax-m2.1 glm-5 qwen3-coder-next)

for model in "${MODELS[@]}"; do
  body="/tmp/kiro-${model}-thinking.sse"
  status="$(curl -sS -N -o "${body}" -w '%{http_code}' \
    http://127.0.0.1:8990/v1/messages \
    -H "x-api-key: ${API_KEY}" \
    -H 'content-type: application/json' \
    -d "$(jq -nc --arg model "${model}" '{
      model: $model,
      max_tokens: 2048,
      stream: true,
      thinking: {type: "enabled", budget_tokens: 1024},
      messages: [{role: "user", content: "计算 12345 × 6789，只输出最终数字"}]
    }')")"
  [[ "${status}" == 200 ]] || {
    print -u2 -r -- "${model} HTTP ${status}"
    exit 1
  }
  thinking=false
  rg -q 'thinking_delta' "${body}" && thinking=true
  print -r -- "${model} HTTP ${status} visible_thinking=${thinking}"
done
```

Expected: 七个请求均为 HTTP 200；`visible_thinking` 按实际 SSE 记录，不把 `false` 判为普通调用失败。

- [ ] **Step 5: 汇总交付证据**

报告每个模型的普通 HTTP 状态、非空文本、`thinking_delta` 观测结果。明确说明：

- GPT 5.6 在 Kiro 中只应观察到最终文本。
- 其他模型的结果是本次 Kiro 运行观测，不是供应商长期能力保证。
- WebSearch 未纳入模型透传验证。
- `git status --short` 中规格和计划文件保持未提交。
