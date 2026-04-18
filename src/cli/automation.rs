//! Automation setup command — connect social media accounts for auto-posting.

use std::io::{self, Write};

use anyhow::Result;

use hmanlab::config::Config;

use super::common::{read_line, read_secret};
use hmanlab::tools::{social_threads, social_x};

pub(crate) async fn cmd_automation(action: AutomationAction) -> Result<()> {
    match action {
        AutomationAction::Setup => automation_setup(),
        AutomationAction::Status => automation_status(),
        AutomationAction::Test => automation_test(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum AutomationAction {
    #[default]
    Status,
    Setup,
    Test,
}

fn automation_setup() -> Result<()> {
    let mut config = Config::load()?;

    println!();
    println!("Automation Setup");
    println!("==================");
    println!();

    println!("Current status:");
    let x_status = if config.tools.x.as_ref().map(|c| c.enabled).unwrap_or(false) {
        "configured"
    } else {
        "not configured"
    };
    let threads_status =
        if config.tools.threads.as_ref().map(|c| c.enabled).unwrap_or(false) {
            "configured"
        } else {
            "not configured"
        };
    println!("  X (Twitter):    {}", x_status);
    println!("  Threads:       {}", threads_status);
    println!();

    println!("What would you like to set up?");
    println!("  1. X (Twitter)");
    println!("  2. Threads");
    println!("  3. Both");
    println!("  4. Skip");
    println!();
    print!("Choice [4]: ");
    io::stdout().flush()?;
    let choice = read_line()?;

    let choice = choice.trim();
    match choice {
        "1" => {
            setup_x(&mut config)?;
            config.save()?;
            println!("Configuration saved.");
        }
        "2" => {
            setup_threads(&mut config)?;
            config.save()?;
            println!("Configuration saved.");
        }
        "3" => {
            setup_x(&mut config)?;
            setup_threads(&mut config)?;
            config.save()?;
            println!("Configuration saved.");
        }
        _ => {
            println!("  Skipped.");
        }
    }

    Ok(())
}

fn setup_x(config: &mut Config) -> Result<()> {
    println!();
    println!("X (Twitter) Setup");
    println!("------------------");
    println!("Choose your API tier:");
    println!("  1. Free (OAuth 1.0a, 1,500 posts/month)");
    println!("  2. Basic ($200/mo, OAuth 2.0, 3,000 posts/month)");
    println!();
    println!("Get your credentials at: https://developer.x.com/en/portal/dashboard");
    println!();

    print!("Choice [1]: ");
    io::stdout().flush()?;
    let _tier = read_line()?;

    print!("Enter API Key: ");
    io::stdout().flush()?;
    let api_key = read_secret()?;
    print!("Enter API Secret: ");
    io::stdout().flush()?;
    let api_secret = read_secret()?;
    print!("Enter Access Token: ");
    io::stdout().flush()?;
    let access_token = read_secret()?;
    print!("Enter Access Token Secret: ");
    io::stdout().flush()?;
    let access_token_secret = read_secret()?;

    if !api_key.is_empty() && !api_secret.is_empty() && !access_token.is_empty() && !access_token_secret.is_empty() {
        let x_cfg = config.tools.x.get_or_insert_with(Default::default);
        x_cfg.enabled = true;
        x_cfg.api_key = Some(api_key);
        x_cfg.api_secret = Some(api_secret);
        x_cfg.access_token = Some(access_token);
        x_cfg.access_token_secret = Some(access_token_secret);
        x_cfg.tier = Some("free".to_string());
        println!("  X (Twitter) configured.");
    } else {
        println!("  Missing credentials. Skipped.");
    }

    Ok(())
}

fn setup_threads(config: &mut Config) -> Result<()> {
    println!();
    println!("Threads Setup");
    println!("-------------");
    println!("1. Go to: https://developers.facebook.com/apps/");
    println!("2. Create an app → Add \"Threads API\" product");
    println!("3. Generate Access Token with threads_basic_write permission");
    println!();

    print!("Enter Threads User ID: ");
    io::stdout().flush()?;
    let user_id = read_line()?;
    print!("Enter Threads Access Token: ");
    io::stdout().flush()?;
    let access_token = read_secret()?;

    if !user_id.is_empty() && !access_token.is_empty() {
        let threads_cfg = config.tools.threads.get_or_insert_with(Default::default);
        threads_cfg.enabled = true;
        threads_cfg.user_id = Some(user_id);
        threads_cfg.access_token = Some(access_token);
        println!("  Threads configured.");
    } else {
        println!("  Missing credentials. Skipped.");
    }

    Ok(())
}

fn automation_status() -> Result<()> {
    let config = Config::load()?;

    println!();
    println!("Automation Status");
    println!("=================");
    println!();

    let x_enabled = config.tools.x.as_ref().map(|c| c.enabled).unwrap_or(false);
    let threads_enabled = config.tools.threads.as_ref().map(|c| c.enabled).unwrap_or(false);

    println!("Configured platforms:");
    println!("  X (Twitter):    {}", if x_enabled { "✓ configured" } else { "✗ not configured" });
    println!("  Threads:       {}", if threads_enabled { "✓ configured" } else { "✗ not configured" });
    println!();

    if !x_enabled && !threads_enabled {
        println!("Run 'hmanlab automation setup' to configure.");
    } else {
        println!("Test your setup: hmanlab automation test");
    }

    Ok(())
}

fn automation_test() -> Result<()> {
    let config = Config::load()?;

    println!();
    println!("Testing automation connections...");
    println!();

    let rt = tokio::runtime::Runtime::new()?;
    let mut any_configured = false;

    if let Some(ref x_cfg) = config.tools.x {
        if x_cfg.enabled {
            any_configured = true;
            print!("  X (Twitter):  ");
            io::stdout().flush()?;

            if let (Some(ak), Some(as_), Some(at), Some(ats)) = (
                x_cfg.api_key.as_deref(),
                x_cfg.api_secret.as_deref(),
                x_cfg.access_token.as_deref(),
                x_cfg.access_token_secret.as_deref(),
            ) {
                match rt.block_on(social_x::test_x_connection(ak, as_, at, ats)) {
                    Ok(username) => {
                        println!("✓ Connected (@{})", username);
                    }
                    Err(e) => {
                        println!("✗ {}", e);
                    }
                }
            } else {
                println!("✗ Missing credentials");
            }
        }
    }

    if let Some(ref threads_cfg) = config.tools.threads {
        if threads_cfg.enabled {
            any_configured = true;
            print!("  Threads:     ");
            io::stdout().flush()?;

            if let (Some(uid), Some(token)) = (
                threads_cfg.user_id.as_deref(),
                threads_cfg.access_token.as_deref(),
            ) {
                match rt.block_on(social_threads::test_threads_connection(uid, token)) {
                    Ok(username) => {
                        println!("✓ Connected ({})", username);
                    }
                    Err(e) => {
                        println!("✗ {}", e);
                    }
                }
            } else {
                println!("✗ Missing credentials");
            }
        }
    }

    if !any_configured {
        println!("No automation platforms configured.");
        println!("Run 'hmanlab automation setup' first.");
    }

    Ok(())
}