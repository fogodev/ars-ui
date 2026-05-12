//! Command-line entrypoint for ars-ui E2E harnesses.

use std::process;

use ars_e2e::{desktop, navigation, utility};
use clap::{Parser, Subcommand};

/// ars-ui E2E harness runner.
#[derive(Parser)]
#[command(name = "ars-e2e", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run all navigation component E2E harnesses against an internal fixture.
    Navigation {
        /// Adapter fixture to exercise.
        #[arg(long, value_enum, default_value_t = navigation::Adapter::Leptos)]
        adapter: navigation::Adapter,

        /// Port for the fixture server.
        #[arg(long)]
        port: Option<u16>,

        /// `WebDriver` endpoint. Defaults to `WEBDRIVER_URL` or local `ChromeDriver`
        /// on port 9515.
        #[arg(long)]
        webdriver_url: Option<String>,

        /// Use an already-running fixture server instead of spawning one.
        #[arg(long)]
        no_server: bool,

        /// Run Chrome with a visible browser window.
        #[arg(long)]
        headed: bool,
    },

    /// Run all utility component E2E harnesses against an internal fixture.
    Utility {
        /// Adapter fixture to exercise.
        #[arg(long, value_enum, default_value_t = utility::Adapter::Leptos)]
        adapter: utility::Adapter,

        /// Port for the fixture server.
        #[arg(long)]
        port: Option<u16>,

        /// `WebDriver` endpoint. Defaults to `WEBDRIVER_URL` or local `ChromeDriver`
        /// on port 9515.
        #[arg(long)]
        webdriver_url: Option<String>,

        /// Use an already-running fixture server instead of spawning one.
        #[arg(long)]
        no_server: bool,

        /// Run Chrome with a visible browser window.
        #[arg(long)]
        headed: bool,
    },

    /// Run Dioxus desktop-mode E2E smoke checks.
    Desktop {
        /// Dioxus desktop example to exercise.
        #[arg(long, value_enum, default_value_t = desktop::Example::DioxusTailwind)]
        example: desktop::Example,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Navigation {
            adapter,
            port,
            webdriver_url,
            no_server,
            headed,
        } => {
            navigation::run(navigation::Options {
                adapter,
                port,
                webdriver_url,
                no_server,
                headless: !headed,
            })
            .await
        }
        Command::Utility {
            adapter,
            port,
            webdriver_url,
            no_server,
            headed,
        } => {
            utility::run(utility::Options {
                adapter,
                port,
                webdriver_url,
                no_server,
                headless: !headed,
            })
            .await
        }
        Command::Desktop { example } => desktop::run(&desktop::Options { example }),
    };

    if let Err(error) = result {
        eprintln!("error: {error}");

        process::exit(1);
    }
}
