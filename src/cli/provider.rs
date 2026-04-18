//! Provider chain status, add, and list command handlers.

use std::io::{self, Write};

use anyhow::Result;
use hmanlab::config::{Config, ProviderConfig};
use hmanlab::providers::{configured_provider_names, resolve_runtime_providers, QuotaStore};

use super::ProviderSubcommand;

pub(crate) async fn cmd_provider(action: ProviderSubcommand) -> Result<()> {
    match action {
        ProviderSubcommand::Status => print_provider_status()?,
        ProviderSubcommand::Add => cmd_provider_add().await?,
        ProviderSubcommand::List => print_provider_list()?,
    }
    Ok(())
}

fn redact_key(key: &str) -> String {
    if key.len() <= 8 {
        return "****".to_string();
    }
    let prefix = &key[..key.len().min(8)];
    format!("{}...****", prefix)
}

fn print_provider_status() -> Result<()> {
    let config = Config::load()?;
    let selections = resolve_runtime_providers(&config);

    if selections.is_empty() {
        println!("No providers configured.");
        println!("Set an API key in ~/.hmanlab/config.json or via environment variable.");
        return Ok(());
    }

    let default_model = &config.agents.defaults.model;

    println!("\nResolved Providers:");
    println!(
        "{:<15} {:<12} {:<30} {:<20}",
        "Name", "Backend", "Model", "API Key"
    );
    println!("{}", "-".repeat(77));

    for s in &selections {
        let model = s.model.as_deref().unwrap_or(default_model);
        let key = redact_key(&s.api_key);
        println!("{:<15} {:<12} {:<30} {:<20}", s.name, s.backend, model, key);
        if let Some(ref base) = s.api_base {
            println!("  api_base: {}", base);
        }
    }

    println!("\nWrappers:");
    println!(
        "  retry:    {} (max {}, base {}ms, budget {}ms)",
        if config.providers.retry.enabled {
            "enabled"
        } else {
            "disabled"
        },
        config.providers.retry.max_retries,
        config.providers.retry.base_delay_ms,
        config.providers.retry.retry_budget_ms,
    );
    println!(
        "  fallback: {}{}",
        if config.providers.fallback.enabled {
            "enabled"
        } else {
            "disabled"
        },
        config
            .providers
            .fallback
            .provider
            .as_ref()
            .map(|p| format!(" (preferred: {})", p))
            .unwrap_or_default(),
    );

    let store = QuotaStore::load_or_default();
    let snapshot = store.snapshot();
    if !snapshot.is_empty() {
        println!("\nQuota Usage:");
        println!(
            "{:<15} {:<12} {:<15} {:<15}",
            "Provider", "Period", "Cost Used", "Tokens Used"
        );
        println!("{}", "-".repeat(62));
        let mut entries: Vec<_> = snapshot.iter().collect();
        entries.sort_by_key(|(name, _)| name.as_str());
        for (name, usage) in entries {
            println!(
                "{:<15} {:<12} ${:<14.4} {:<15}",
                name, usage.period_key, usage.cost_usd, usage.tokens,
            );
        }
    }

    println!();
    Ok(())
}

fn print_provider_list() -> Result<()> {
    let config = Config::load()?;
    let configured = configured_provider_names(&config);

    let all_providers: &[(&str, &str)] = &[
        ("anthropic", "Anthropic (Claude)"),
        ("openai", "OpenAI (GPT-4, o3, etc.)"),
        ("openrouter", "OpenRouter (400+ models)"),
        ("groq", "Groq (fast inference)"),
        ("zhipu", "Zhipu (GLM)"),
        ("vllm", "vLLM (local inference)"),
        ("gemini", "Google Gemini"),
        ("vertex", "Google Vertex AI"),
        ("ollama", "Ollama (local models)"),
        ("nvidia", "Nvidia NIM"),
        ("deepseek", "DeepSeek"),
        ("kimi", "Kimi (Moonshot AI)"),
        ("azure", "Azure OpenAI"),
        ("bedrock", "Amazon Bedrock"),
        ("xai", "xAI (Grok)"),
        ("qianfan", "Baidu Qianfan"),
        ("novita", "Novita AI"),
    ];

    println!("\nProviders:");
    println!("{:<15} {:<12} Description", "Name", "Status");
    println!("{}", "-".repeat(70));

    for (name, label) in all_providers {
        let status = if configured.contains(name) {
            "configured"
        } else {
            "available"
        };
        println!("{:<15} {:<12} {}", name, status, label);
    }

    println!();
    println!("Use 'hmanlab provider add' to configure a provider.");
    println!();
    Ok(())
}

async fn cmd_provider_add() -> Result<()> {
    let mut config = Config::load()?;

    println!("Add Provider");
    println!("============");
    println!();
    println!("Which provider would you like to add?");
    println!("  1. Anthropic (Claude)");
    println!("  2. OpenAI (GPT-4, o3, etc.)");
    println!("  3. OpenRouter (400+ models)");
    println!("  4. Google Gemini");
    println!("  5. Groq");
    println!("  6. DeepSeek");
    println!("  7. Zhipu (GLM)");
    println!("  8. Ollama (local models)");
    println!("  9. Other (enter provider name)");
    println!();
    print!("Choice: ");
    io::stdout().flush()?;

    let input = super::common::read_line()?;
    let provider = match input.trim() {
        "1" => "anthropic",
        "2" => "openai",
        "3" => "openrouter",
        "4" => "gemini",
        "5" => "groq",
        "6" => "deepseek",
        "7" => "zhipu",
        "8" => "ollama",
        "9" => {
            print!("Enter provider name: ");
            io::stdout().flush()?;
            let name = super::common::read_line()?;
            let name = name.trim().to_string();
            if name.is_empty() {
                println!("  No provider name entered. Aborting.");
                return Ok(());
            }
            return configure_generic_provider(&mut config, &name);
        }
        other => {
            println!("  Invalid choice '{}'. Aborting.", other);
            return Ok(());
        }
    };

    println!();
    match provider {
        "anthropic" => super::onboard::configure_anthropic_pub(&mut config).await?,
        "openai" => super::onboard::configure_openai_pub(&mut config).await?,
        "openrouter" => super::onboard::configure_openrouter_pub(&mut config).await?,
        "gemini" => super::onboard::configure_gemini_pub(&mut config).await?,
        _ => configure_generic_provider(&mut config, provider)?,
    }

    config.save()?;
    println!();
    println!("Configuration saved to {:?}", Config::path());
    println!("Run 'hmanlab provider status' to verify.");
    Ok(())
}

fn configure_generic_provider(config: &mut Config, name: &str) -> Result<()> {
    println!("{} Setup", name);
    println!("{}", "-".repeat(40));
    println!();
    print!("Enter API key (or press Enter if not required): ");
    io::stdout().flush()?;
    let api_key = super::common::read_secret()?;

    let mut provider_config = ProviderConfig::default();
    if !api_key.is_empty() {
        provider_config.api_key = Some(api_key);
    }

    print!("Enter custom API base URL (or press Enter for default): ");
    io::stdout().flush()?;
    let base_url = super::common::read_line()?;
    if !base_url.trim().is_empty() {
        provider_config.api_base = Some(base_url.trim().to_string());
    }

    print!("Enter default model (or press Enter for global default): ");
    io::stdout().flush()?;
    let model = super::common::read_line()?;
    if !model.trim().is_empty() {
        provider_config.model = Some(model.trim().to_string());
    }

    set_provider_config(config, name, provider_config)?;

    println!("  {} provider configured.", name);
    Ok(())
}

fn set_provider_config(config: &mut Config, name: &str, pc: ProviderConfig) -> Result<()> {
    match name {
        "anthropic" => config.providers.anthropic = Some(pc),
        "openai" => config.providers.openai = Some(pc),
        "openrouter" => config.providers.openrouter = Some(pc),
        "groq" => config.providers.groq = Some(pc),
        "zhipu" => config.providers.zhipu = Some(pc),
        "vllm" => config.providers.vllm = Some(pc),
        "gemini" => config.providers.gemini = Some(pc),
        "vertex" => config.providers.vertex = Some(pc),
        "ollama" => config.providers.ollama = Some(pc),
        "nvidia" => config.providers.nvidia = Some(pc),
        "deepseek" => config.providers.deepseek = Some(pc),
        "kimi" => config.providers.kimi = Some(pc),
        "azure" => config.providers.azure = Some(pc),
        "bedrock" => config.providers.bedrock = Some(pc),
        "xai" => config.providers.xai = Some(pc),
        "qianfan" => config.providers.qianfan = Some(pc),
        "novita" => config.providers.novita = Some(pc),
        other => {
            anyhow::bail!(
                "Unknown provider '{}'. Edit ~/.hmanlab/config.json manually.",
                other
            )
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_key_short() {
        assert_eq!(redact_key("abc"), "****");
    }

    #[test]
    fn test_redact_key_normal() {
        assert_eq!(redact_key("sk-ant-api03-abcdefghijk"), "sk-ant-a...****");
    }

    #[test]
    fn test_redact_key_exact_boundary() {
        assert_eq!(redact_key("12345678"), "****");
        assert_eq!(redact_key("123456789"), "12345678...****");
    }
}
