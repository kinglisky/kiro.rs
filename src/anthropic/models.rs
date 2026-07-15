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
    NativeModelSpec {
        id: "gpt-5.6-sol",
        display_name: "GPT 5.6 Sol",
        owned_by: "openai",
        created: 1_783_987_200,
        max_input_tokens: 272_000,
        max_output_tokens: 128_000,
    },
    NativeModelSpec {
        id: "gpt-5.6-terra",
        display_name: "GPT 5.6 Terra",
        owned_by: "openai",
        created: 1_783_987_200,
        max_input_tokens: 272_000,
        max_output_tokens: 128_000,
    },
    NativeModelSpec {
        id: "gpt-5.6-luna",
        display_name: "GPT 5.6 Luna",
        owned_by: "openai",
        created: 1_783_987_200,
        max_input_tokens: 272_000,
        max_output_tokens: 128_000,
    },
    NativeModelSpec {
        id: "deepseek-3.2",
        display_name: "DeepSeek V3.2",
        owned_by: "deepseek",
        created: 0,
        max_input_tokens: 164_000,
        max_output_tokens: 64_000,
    },
    NativeModelSpec {
        id: "minimax-m2.5",
        display_name: "MiniMax M2.5",
        owned_by: "minimax",
        created: 0,
        max_input_tokens: 196_000,
        max_output_tokens: 64_000,
    },
    NativeModelSpec {
        id: "minimax-m2.1",
        display_name: "MiniMax M2.1",
        owned_by: "minimax",
        created: 0,
        max_input_tokens: 196_000,
        max_output_tokens: 64_000,
    },
    NativeModelSpec {
        id: "glm-5",
        display_name: "GLM-5",
        owned_by: "z-ai",
        created: 0,
        max_input_tokens: 200_000,
        max_output_tokens: 64_000,
    },
    NativeModelSpec {
        id: "qwen3-coder-next",
        display_name: "Qwen3 Coder Next",
        owned_by: "qwen",
        created: 0,
        max_input_tokens: 256_000,
        max_output_tokens: 64_000,
    },
];

pub(crate) fn find_native_model(model: &str) -> Option<&'static NativeModelSpec> {
    let normalized = model.to_ascii_lowercase();
    NATIVE_MODELS
        .iter()
        .find(|candidate| candidate.id == normalized.as_str())
}

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
