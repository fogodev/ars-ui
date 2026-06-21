//! Public widgets browser smoke harnesses.

use std::{
    fmt::{self, Display},
    fs::OpenOptions,
    net::{SocketAddr, TcpStream},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use clap::ValueEnum;
use serde_json::Value;
use thirtyfour::{ChromeCapabilities, ChromiumLikeCapabilities, LoggingPrefsLogLevel, prelude::*};
use tokio::time;

use crate::{
    Error,
    browser::{ChildGuard, maybe_spawn_chromedriver, wait_for_tcp, webdriver_url},
    fixtures,
    input::{assert_attr, dispatch_pointer_sequence, element_by_id, open_input_panel},
};

/// Public widgets example covered by a browser smoke check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Example {
    /// Plain Leptos widgets example.
    Leptos,

    /// Plain Dioxus widgets example.
    Dioxus,

    /// Leptos CSS widgets example.
    LeptosCss,

    /// Dioxus CSS widgets example.
    DioxusCss,

    /// Leptos Tailwind widgets example.
    LeptosTailwind,

    /// Dioxus Tailwind widgets example.
    DioxusTailwind,
}

impl Example {
    const fn package(self) -> &'static str {
        match self {
            Self::Leptos => "widgets-leptos",
            Self::Dioxus => "widgets-dioxus",
            Self::LeptosCss => "widgets-leptos-css",
            Self::DioxusCss => "widgets-dioxus-css",
            Self::LeptosTailwind => "widgets-leptos-tailwind",
            Self::DioxusTailwind => "widgets-dioxus-tailwind",
        }
    }

    const fn path(self) -> &'static str {
        match self {
            Self::Leptos => "examples/widgets-leptos",
            Self::Dioxus => "examples/widgets-dioxus",
            Self::LeptosCss => "examples/widgets-leptos-css",
            Self::DioxusCss => "examples/widgets-dioxus-css",
            Self::LeptosTailwind => "examples/widgets-leptos-tailwind",
            Self::DioxusTailwind => "examples/widgets-dioxus-tailwind",
        }
    }

    const fn default_port(self) -> u16 {
        match self {
            Self::Leptos => 5300,
            Self::Dioxus => 5301,
            Self::LeptosCss => 5302,
            Self::DioxusCss => 5303,
            Self::LeptosTailwind => 5304,
            Self::DioxusTailwind => 5305,
        }
    }

    const fn is_dioxus(self) -> bool {
        matches!(self, Self::Dioxus | Self::DioxusCss | Self::DioxusTailwind)
    }

    const fn is_styled(self) -> bool {
        matches!(
            self,
            Self::LeptosCss | Self::DioxusCss | Self::LeptosTailwind | Self::DioxusTailwind
        )
    }
}

impl Display for Example {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.package())
    }
}

/// Runtime options for public widget smoke checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// Public widget example to exercise.
    pub example: Example,

    /// Port used by the example server.
    pub port: Option<u16>,

    /// `WebDriver` endpoint. Defaults to `WEBDRIVER_URL`, then local
    /// `ChromeDriver` on port 9515.
    pub webdriver_url: Option<String>,

    /// Use an already-running example server instead of spawning one.
    pub no_server: bool,

    /// Whether Chrome should run without a visible browser window.
    pub headless: bool,
}

/// Run the public widgets browser smoke harness.
///
/// # Errors
///
/// Returns an error when the example server, browser session, or assertions
/// fail.
pub async fn run(options: Options) -> Result<(), Error> {
    let session = start_widget_session(&options).await?;

    let run = async {
        run_checkbox_smoke(&session.driver, &session.url, options.example).await?;
        run_tabs_smoke(&session.driver, &session.url, options.example).await
    }
    .await;
    let quit = session.quit().await;

    run?;
    quit?;

    Ok(())
}

struct WidgetSession {
    driver: WebDriver,
    url: String,
    _server: Option<ChildGuard>,
    _chromedriver: Option<ChildGuard>,
}

impl WidgetSession {
    async fn quit(self) -> Result<(), Error> {
        self.driver.quit().await?;

        Ok(())
    }
}

async fn start_widget_session(options: &Options) -> Result<WidgetSession, Error> {
    let port = options
        .port
        .unwrap_or_else(|| options.example.default_port());

    let url = format!("http://127.0.0.1:{port}/");
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let server = if options.no_server {
        None
    } else {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
            return Err(Error::Command(format!(
                "widgets server port {port} is already in use; stop the existing server or pass --port"
            )));
        }

        Some(spawn_widget_server(options.example, port)?)
    };

    wait_for_tcp(addr, Duration::from_secs(90), "widgets server")?;

    let webdriver_url = webdriver_url(options.webdriver_url.clone());
    let chromedriver = maybe_spawn_chromedriver(&webdriver_url)?;
    let driver = WebDriver::new(&webdriver_url, chrome_capabilities(options.headless)?).await?;

    Ok(WidgetSession {
        driver,
        url,
        _server: server,
        _chromedriver: chromedriver,
    })
}

fn spawn_widget_server(example: Example, port: u16) -> Result<ChildGuard, Error> {
    let mut command = widget_server_command(example, port).map_err(Error::Command)?;

    let log_path = std::env::temp_dir().join(format!(
        "ars-ui-widget-smoke-{}-{}-{}.log",
        example.package(),
        port,
        std::process::id()
    ));

    let log_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&log_path)
        .map_err(|error| Error::Command(format!("failed to open widget log: {error}")))?;

    command
        .stdout(Stdio::from(log_file.try_clone().map_err(|error| {
            Error::Command(format!("failed to clone widget log: {error}"))
        })?))
        .stderr(Stdio::from(log_file));

    let child = command
        .spawn()
        .map_err(|error| Error::Command(format!("failed to spawn {example}: {error}")))?;

    fixtures::wait_for_fixture_server(
        child,
        log_path,
        SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_secs(90),
        example.package(),
    )
}

/// Build the command used to serve a public widgets example.
///
/// # Errors
///
/// Returns an error when the current directory cannot be resolved.
pub fn widget_server_command(example: Example, port: u16) -> Result<Command, String> {
    let mut command = if example.is_dioxus() {
        let mut command = Command::new("dx");

        command
            .arg("serve")
            .arg("--web")
            .arg("--hot-reload")
            .arg("false")
            .arg("--open")
            .arg("false")
            .arg("--port")
            .arg(port.to_string());

        command
    } else {
        let mut command = Command::new("trunk");

        command
            .arg("serve")
            .arg("--open")
            .arg("false")
            .arg("--port")
            .arg(port.to_string());

        command
    };

    command
        .env("CARGO_TARGET_DIR", examples_target_dir()?)
        .env_remove("NO_COLOR")
        .current_dir(Path::new(example.path()))
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    Ok(command)
}

async fn run_checkbox_smoke(driver: &WebDriver, url: &str, example: Example) -> Result<(), Error> {
    open_input_panel(driver, url).await?;
    assert_clean_console(driver).await?;

    for id in [
        "checkbox-unchecked",
        "checkbox-checked",
        "checkbox-indeterminate",
        "checkbox-disabled",
        "checkbox-invalid",
        "checkbox-demo-form",
    ] {
        element_by_id(driver, id).await?;
    }

    let unchecked = element_by_id(driver, "checkbox-unchecked").await?;
    let unchecked_control = checkbox_control(&unchecked).await?;

    assert_attr(&unchecked_control, "aria-checked", "false").await?;
    if example.is_styled() {
        assert_checkbox_visual_deltas(driver).await?;
    }

    dispatch_pointer_sequence(driver, &unchecked_control).await?;

    assert_attr(&unchecked_control, "aria-checked", "true").await?;

    let indeterminate = element_by_id(driver, "checkbox-indeterminate").await?;
    let indeterminate_control = checkbox_control(&indeterminate).await?;

    assert_attr(&indeterminate_control, "aria-checked", "mixed").await?;

    let invalid = element_by_id(driver, "checkbox-invalid").await?;
    let invalid_control = checkbox_control(&invalid).await?;

    assert_attr(&invalid_control, "aria-invalid", "true").await?;
    assert_control_box_stable(driver, &invalid_control).await?;
    assert_checkbox_hidden_inputs_are_visually_hidden(driver).await?;

    let form = element_by_id(driver, "checkbox-demo-form").await?;

    let submit = form.find(By::Css("[type='submit']")).await?;
    let reset = form.find(By::Css("[type='reset']")).await?;

    assert_form_buttons_have_visible_gap(driver, &submit, &reset).await?;

    submit.click().await?;
    reset.click().await?;

    assert_locale_switch(driver).await?;
    assert_clean_console(driver).await
}

async fn run_tabs_smoke(driver: &WebDriver, url: &str, example: Example) -> Result<(), Error> {
    open_navigation_panel(driver, url).await?;
    assert_clean_console(driver).await?;

    for label in ["Overview", "Keyboard", "Closable", "Disabled"] {
        visible_tab(driver, label).await?;
    }

    let overview = visible_tab(driver, "Overview").await?;
    let keyboard = visible_tab(driver, "Keyboard").await?;
    let disabled = visible_tab(driver, "Disabled").await?;
    let close_affordance = visible_close_affordance(driver).await?;

    assert_attr(&overview, "aria-selected", "true").await?;
    assert_attr(&keyboard, "aria-selected", "false").await?;
    assert_attr(&disabled, "aria-disabled", "true").await?;
    assert_attr(&close_affordance, "aria-hidden", "true").await?;

    assert_nonzero_box(driver, &overview, "tabs Overview tab").await?;
    assert_nonzero_box(driver, &keyboard, "tabs Keyboard tab").await?;
    assert_nonzero_box(driver, &close_affordance, "tabs close affordance").await?;

    if example.is_styled() {
        assert_tabs_visual_deltas(driver).await?;
    }

    keyboard.click().await?;
    wait_for_tab_attr(driver, "Keyboard", "aria-selected", "true").await?;

    keyboard.focus().await?;
    driver
        .action_chain()
        .send_keys(Key::Right)
        .perform()
        .await?;
    wait_for_active_tab_contains(driver, "Closable").await?;

    driver
        .action_chain()
        .send_keys(Key::Delete)
        .perform()
        .await?;
    wait_for_tab_absent(driver, "Closable").await?;
    wait_for_active_tab_contains(driver, "Keyboard").await?;

    select_locale(driver, "pt-BR").await?;
    wait_for_body_text(driver, "Abas").await?;
    visible_tab(driver, "Teclado").await?;

    assert_clean_console(driver).await
}

async fn open_navigation_panel(driver: &WebDriver, url: &str) -> Result<(), Error> {
    driver.goto(url).await?;
    select_locale(driver, "en-US").await?;

    let navigation = visible_tab(driver, "Navigation").await?;

    driver
        .execute("arguments[0].click();", vec![navigation.to_json()?])
        .await?;

    wait_for_body_text(driver, "Live demo of the Tabs adapter").await
}

async fn checkbox_control(root: &WebElement) -> Result<WebElement, Error> {
    Ok(root.find(By::Css("[data-ars-part='control']")).await?)
}

async fn visible_close_affordance(driver: &WebDriver) -> Result<WebElement, Error> {
    let close_affordances = driver
        .find_all(By::Css(
            "[data-ars-scope='tabs'][data-ars-part='tab-close-trigger']",
        ))
        .await?;

    for close_affordance in close_affordances {
        if close_affordance.is_displayed().await? {
            return Ok(close_affordance);
        }
    }

    Err(Error::Assertion(
        "tabs widget must render a visible close affordance".to_string(),
    ))
}

async fn visible_tab(driver: &WebDriver, label: &str) -> Result<WebElement, Error> {
    let deadline = Instant::now() + Duration::from_secs(10);

    while Instant::now() < deadline {
        let tabs = driver.find_all(By::Css("[role='tab']")).await?;

        for tab in tabs.into_iter().rev() {
            let text = tab.text().await?;

            if text.contains(label) && tab.is_displayed().await? {
                return Ok(tab);
            }
        }

        time::sleep(Duration::from_millis(50)).await;
    }

    Err(Error::Timeout(format!(
        "timed out waiting for visible widget tab {label:?}"
    )))
}

async fn wait_for_tab_attr(
    driver: &WebDriver,
    label: &str,
    name: &str,
    expected: &str,
) -> Result<(), Error> {
    let deadline = Instant::now() + Duration::from_secs(5);

    while Instant::now() < deadline {
        let tab = visible_tab(driver, label).await?;
        let value = tab.attr(name).await?;

        if value.as_deref() == Some(expected) {
            return Ok(());
        }

        time::sleep(Duration::from_millis(50)).await;
    }

    Err(Error::Assertion(format!(
        "widget tab {label:?} must have {name:?}={expected:?}"
    )))
}

async fn wait_for_active_tab_contains(driver: &WebDriver, label: &str) -> Result<(), Error> {
    let deadline = Instant::now() + Duration::from_secs(5);

    while Instant::now() < deadline {
        let active = driver.active_element().await?;
        let role = active.attr("role").await?;
        let text = active.text().await?;

        if role.as_deref() == Some("tab") && text.contains(label) {
            return Ok(());
        }

        time::sleep(Duration::from_millis(50)).await;
    }

    Err(Error::Assertion(format!(
        "active widget tab must contain {label:?}"
    )))
}

async fn wait_for_tab_absent(driver: &WebDriver, label: &str) -> Result<(), Error> {
    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        match visible_tab(driver, label).await {
            Ok(_) if Instant::now() >= deadline => {
                return Err(Error::Assertion(format!(
                    "widget tab {label:?} is still present"
                )));
            }

            Ok(_) => time::sleep(Duration::from_millis(50)).await,

            Err(Error::Timeout(_)) | Err(Error::WebDriver(_)) => return Ok(()),

            Err(error) => return Err(error),
        }
    }
}

async fn assert_control_box_stable(driver: &WebDriver, element: &WebElement) -> Result<(), Error> {
    let value = driver
        .execute(
            r#"
            const rect = arguments[0].getBoundingClientRect();
            const style = getComputedStyle(arguments[0]);
            return {
                width: rect.width,
                height: rect.height,
                borderColor: style.borderColor
            };
            "#,
            vec![element.to_json()?],
        )
        .await?;

    let value = value.json();

    for key in ["width", "height"] {
        if value.get(key).and_then(Value::as_f64).unwrap_or_default() <= 0.0 {
            return Err(Error::Assertion(format!(
                "checkbox widget control must have nonzero {key}"
            )));
        }
    }

    Ok(())
}

async fn assert_nonzero_box(
    driver: &WebDriver,
    element: &WebElement,
    label: &str,
) -> Result<(), Error> {
    let value = driver
        .execute(
            r#"
            const rect = arguments[0].getBoundingClientRect();
            return {
                width: rect.width,
                height: rect.height
            };
            "#,
            vec![element.to_json()?],
        )
        .await?;

    let value = value.json();

    for key in ["width", "height"] {
        if number_field(value, key)? <= 0.0 {
            return Err(Error::Assertion(format!("{label} must have nonzero {key}")));
        }
    }

    Ok(())
}

async fn assert_checkbox_visual_deltas(driver: &WebDriver) -> Result<(), Error> {
    let unchecked = control_style(driver, "checkbox-unchecked").await?;

    for (id, expected) in [
        ("checkbox-checked", "checked"),
        ("checkbox-indeterminate", "indeterminate"),
        ("checkbox-invalid", "invalid"),
    ] {
        let style = control_style(driver, id).await?;

        if style.border_color == unchecked.border_color
            && style.background_color == unchecked.background_color
            && style.color == unchecked.color
        {
            return Err(Error::Assertion(format!(
                "{expected} checkbox visual style must differ from unchecked"
            )));
        }
    }

    let disabled = control_style(driver, "checkbox-disabled").await?;

    if disabled.opacity == unchecked.opacity {
        return Err(Error::Assertion(
            "disabled checkbox opacity must differ from unchecked".to_string(),
        ));
    }

    Ok(())
}

async fn assert_tabs_visual_deltas(driver: &WebDriver) -> Result<(), Error> {
    let overview = tab_style(driver, "Overview").await?;
    let keyboard = tab_style(driver, "Keyboard").await?;
    let disabled = tab_style(driver, "Disabled").await?;

    if overview.background_color == keyboard.background_color && overview.color == keyboard.color {
        return Err(Error::Assertion(
            "selected tabs widget style must differ from unselected tabs".to_string(),
        ));
    }

    if disabled.opacity == keyboard.opacity && disabled.cursor == keyboard.cursor {
        return Err(Error::Assertion(
            "disabled tabs widget style must differ from enabled tabs".to_string(),
        ));
    }

    Ok(())
}

async fn assert_form_buttons_have_visible_gap(
    driver: &WebDriver,
    submit: &WebElement,
    reset: &WebElement,
) -> Result<(), Error> {
    let value = driver
        .execute(
            r#"
            const submit = arguments[0].getBoundingClientRect();
            const reset = arguments[1].getBoundingClientRect();
            const horizontalGap = reset.left - submit.right;
            const verticalGap = reset.top - submit.bottom;

            return {
                horizontalGap,
                verticalGap,
                sameRow: Math.abs(submit.top - reset.top) < 2,
                submitRight: submit.right,
                resetLeft: reset.left,
                submitBottom: submit.bottom,
                resetTop: reset.top
            };
            "#,
            vec![submit.to_json()?, reset.to_json()?],
        )
        .await?;

    let value = value.json();

    let same_row = value
        .get("sameRow")
        .and_then(Value::as_bool)
        .ok_or_else(|| Error::Assertion("button gap check missing `sameRow`".to_string()))?;

    let gap = if same_row {
        number_field(value, "horizontalGap")?
    } else {
        number_field(value, "verticalGap")?
    };

    if gap < 8.0 {
        return Err(Error::Assertion(format!(
            "checkbox form submit/reset buttons must have at least 8px visible gap, got {gap}px"
        )));
    }

    Ok(())
}

async fn assert_checkbox_hidden_inputs_are_visually_hidden(
    driver: &WebDriver,
) -> Result<(), Error> {
    let value = driver
        .execute(
            r#"
            return Array.from(document.querySelectorAll(
                '[data-ars-scope="checkbox"][data-ars-part="hidden-input"]'
            )).map((input) => {
                const rect = input.getBoundingClientRect();
                const style = getComputedStyle(input);

                return {
                    id: input.id,
                    className: input.className,
                    position: style.position,
                    overflow: style.overflow,
                    width: rect.width,
                    height: rect.height
                };
            });
            "#,
            Vec::new(),
        )
        .await?;

    let inputs = value
        .json()
        .as_array()
        .ok_or_else(|| Error::Assertion("hidden input query must return an array".to_string()))?;

    if inputs.is_empty() {
        return Err(Error::Assertion(
            "checkbox widgets must render hidden inputs".to_string(),
        ));
    }

    for input in inputs {
        let id = string_field(input, "id")?;
        let class_name = string_field(input, "className")?;
        let position = string_field(input, "position")?;
        let overflow = string_field(input, "overflow")?;
        let width = number_field(input, "width")?;
        let height = number_field(input, "height")?;

        if !class_token_present(Some(&class_name), "ars-sr-input")
            || position != "absolute"
            || overflow != "hidden"
            || width > 2.0
            || height > 2.0
        {
            return Err(Error::Assertion(format!(
                "checkbox hidden input {id:?} must be visually hidden, got class={class_name:?} position={position:?} overflow={overflow:?} size={width}x{height}"
            )));
        }
    }

    Ok(())
}

async fn control_style(driver: &WebDriver, root_id: &str) -> Result<ControlStyle, Error> {
    let root = element_by_id(driver, root_id).await?;
    let control = checkbox_control(&root).await?;

    let value = driver
        .execute(
            r#"
            const style = getComputedStyle(arguments[0]);
            return {
                borderColor: style.borderColor,
                backgroundColor: style.backgroundColor,
                color: style.color,
                opacity: style.opacity
            };
            "#,
            vec![control.to_json()?],
        )
        .await?;

    let value = value.json();

    Ok(ControlStyle {
        border_color: string_field(value, "borderColor")?,
        background_color: string_field(value, "backgroundColor")?,
        color: string_field(value, "color")?,
        opacity: string_field(value, "opacity")?,
    })
}

async fn tab_style(driver: &WebDriver, label: &str) -> Result<TabStyle, Error> {
    let tab = visible_tab(driver, label).await?;

    let value = driver
        .execute(
            r#"
            const style = getComputedStyle(arguments[0]);
            return {
                backgroundColor: style.backgroundColor,
                color: style.color,
                cursor: style.cursor,
                opacity: style.opacity
            };
            "#,
            vec![tab.to_json()?],
        )
        .await?;

    let value = value.json();

    Ok(TabStyle {
        background_color: string_field(value, "backgroundColor")?,
        color: string_field(value, "color")?,
        cursor: string_field(value, "cursor")?,
        opacity: string_field(value, "opacity")?,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ControlStyle {
    border_color: String,
    background_color: String,
    color: String,
    opacity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TabStyle {
    background_color: String,
    color: String,
    cursor: String,
    opacity: String,
}

fn string_field(value: &Value, field: &str) -> Result<String, Error> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| Error::Assertion(format!("computed style missing `{field}`")))
}

fn number_field(value: &Value, field: &str) -> Result<f64, Error> {
    value
        .get(field)
        .and_then(Value::as_f64)
        .ok_or_else(|| Error::Assertion(format!("computed style missing numeric `{field}`")))
}

fn class_token_present(class: Option<&str>, token: &str) -> bool {
    class
        .into_iter()
        .flat_map(str::split_whitespace)
        .any(|candidate| candidate == token)
}

async fn assert_locale_switch(driver: &WebDriver) -> Result<(), Error> {
    let switcher = driver.find(By::Css(".locale-switcher")).await?;

    let portuguese = switcher
        .find(By::Css("button[aria-pressed='false']"))
        .await?;

    portuguese.click().await?;

    wait_for_body_text(driver, "Estados de checkbox").await
}

async fn select_locale(driver: &WebDriver, locale: &str) -> Result<(), Error> {
    let switcher = wait_for_locale_switcher(driver).await?;
    let xpath = format!(".//button[normalize-space()='{locale}']");
    let button = switcher.find(By::XPath(xpath.as_str())).await?;

    button.click().await?;

    Ok(())
}

async fn wait_for_locale_switcher(driver: &WebDriver) -> Result<WebElement, Error> {
    let deadline = Instant::now() + Duration::from_secs(10);

    while Instant::now() < deadline {
        match driver.find(By::Css(".locale-switcher")).await {
            Ok(switcher) => return Ok(switcher),
            Err(_) => time::sleep(Duration::from_millis(50)).await,
        }
    }

    Err(Error::Timeout(
        "timed out waiting for widget locale switcher".to_string(),
    ))
}

async fn wait_for_body_text(driver: &WebDriver, needle: &str) -> Result<(), Error> {
    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        let result = driver
            .execute(
                "return document.body.innerText.includes(arguments[0]);",
                vec![Value::String(needle.to_string())],
            )
            .await?;

        if result.json().as_bool() == Some(true) {
            return Ok(());
        }

        if Instant::now() >= deadline {
            return Err(Error::Assertion(format!(
                "body text must include translated text {needle:?}"
            )));
        }

        time::sleep(Duration::from_millis(50)).await;
    }
}

async fn assert_clean_console(driver: &WebDriver) -> Result<(), Error> {
    let entries = driver.browser_log().await?;

    let severe = entries
        .iter()
        .filter(|entry| entry.level == "SEVERE")
        .map(|entry| entry.message.as_str())
        .collect::<Vec<_>>();

    if severe.is_empty() {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "browser console contains severe entries: {}",
            severe.join(" | ")
        )))
    }
}

fn chrome_capabilities(headless: bool) -> WebDriverResult<ChromeCapabilities> {
    let mut caps = DesiredCapabilities::chrome();

    caps.set_browser_log_level(LoggingPrefsLogLevel::Severe)?;

    if headless {
        caps.add_arg("--headless=new")?;
    }

    Ok(caps)
}

fn examples_target_dir() -> Result<std::path::PathBuf, String> {
    Ok(std::env::current_dir()
        .map_err(|error| error.to_string())?
        .join("target/examples"))
}

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command};

    use super::{Example, widget_server_command};

    fn args(command: &Command) -> Vec<String> {
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn widget_server_command_targets_leptos_tailwind_example() {
        let command =
            widget_server_command(Example::LeptosTailwind, 5304).expect("command should build");

        assert_eq!(command.get_program().to_string_lossy(), "trunk");
        assert_eq!(
            command.get_current_dir(),
            Some(Path::new("examples/widgets-leptos-tailwind"))
        );
        assert_eq!(
            args(&command),
            ["serve", "--open", "false", "--port", "5304"]
        );
    }

    #[test]
    fn widget_server_command_targets_dioxus_tailwind_example() {
        let command =
            widget_server_command(Example::DioxusTailwind, 5305).expect("command should build");

        assert_eq!(command.get_program().to_string_lossy(), "dx");
        assert_eq!(
            command.get_current_dir(),
            Some(Path::new("examples/widgets-dioxus-tailwind"))
        );
        assert_eq!(
            args(&command),
            [
                "serve",
                "--web",
                "--hot-reload",
                "false",
                "--open",
                "false",
                "--port",
                "5305",
            ]
        );
    }
}
