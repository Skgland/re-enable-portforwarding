use std::{io::ErrorKind, time::Duration};

use thirtyfour::prelude::*;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Config {
    #[serde(default = "default_selenium_url")]
    selenium_url: String,
    #[serde(default = "default_router_url")]
    router_url: String,
    password: String,
}

#[derive(Debug, thiserror::Error)]
enum ConfigLoadError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Deserialize(#[from] ron::error::SpannedError),
}

impl Config {
    fn template_config() -> Self {
        Config {
            selenium_url: default_selenium_url(),
            router_url: default_router_url(),
            password: "CHANGE-ME".into(),
        }
    }

    async fn load() -> Result<Self, ConfigLoadError> {
        match tokio::fs::read_to_string("./config.ron").await {
            Ok(data) => Ok(ron::from_str(&data)?),
            Err(err) if err.kind() == ErrorKind::NotFound => {
                tokio::fs::write(
                    "./config.ron",
                    ron::to_string(&Config::template_config()).unwrap(),
                )
                .await?;

                return Err(err.into());
            }
            Err(err) => {
                eprintln!("{err}");
                return Err(err.into());
            }
        }
    }
}

fn default_selenium_url() -> String {
    "http://localhost:9515".into()
}

fn default_router_url() -> String {
    "http://192.168.0.1/".into()
}

#[derive(Debug, thiserror::Error)]
enum MainError {
    #[error("{0}")]
    Element34(#[from] WebDriverError),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Config(#[from] ConfigLoadError),
}

#[tokio::main]
async fn main() -> Result<(), MainError> {
    let config = Config::load().await?;

    let mut caps = DesiredCapabilities::chrome();
    caps.set_headless()?;
    caps.set_disable_gpu()?;

    let mut driver = WebDriver::new(&config.selenium_url, caps).await?;

    driver
        .set_implicit_wait_timeout(Duration::from_secs(3))
        .await?;

    if let Err(err) = run(&config, &mut driver).await {
        eprintln!("{err}");

        // Always explicitly close the browser.
        driver.quit().await?;

        return Err(err.into());
    };

    // Always explicitly close the browser.
    driver.quit().await?;

    Ok(())
}

async fn run(config: &Config, driver: &mut WebDriver) -> WebDriverResult<()> {
    driver.goto(&config.router_url).await?;

    login(driver, config).await?;

    goto_host_exposure(driver).await?;

    toggle_portforwading_enabled_state(driver, true).await?;
    toggle_portforwading_enabled_state(driver, false).await?;

    Ok(())
}

async fn toggle_portforwading_enabled_state(
    driver: &mut WebDriver,
    from: bool,
) -> Result<(), WebDriverError> {
    let port_forwardings = driver
        .find_all(By::Css(if from {
            ".content-port-mapping .table-row > td > .button.button-on"
        } else {
            ".content-port-mapping .table-row > td > .button.button-off"
        }))
        .await?;

    println!(
        "Found {} Port Forwardings to {}",
        port_forwardings.len(),
        if from { "disable" } else { "enable" }
    );

    for port_forwarding in port_forwardings {
        port_forwarding.click().await?;
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    driver
        .find(By::Css(
            ".content-port-mapping input[type='button'][value='Apply'].button.button-apply#applyButton",
        ))
        .await?
        .click()
        .await?;

    let background = driver.find(By::Css(".blackBackground")).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    background.wait_until().not_displayed().await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    Ok(())
}

async fn goto_host_exposure(driver: &mut WebDriver) -> Result<(), WebDriverError> {
    driver
        .find(By::Css("div.hamburger-menu"))
        .await?
        .click()
        .await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let nav_item = driver
        .find(By::Css("li.main-item.mobile-navigation-item-3"))
        .await?;
    let nav_title = nav_item
        .find(By::Css("a.mobile-navigation-item-title"))
        .await?;
    assert_eq!("Internet", nav_title.inner_html().await?);
    nav_title.click().await?;
    let nav_item = nav_item.find(By::Css("ul > li:nth-child(3)")).await?;
    let nav_title = nav_item
        .find(By::Css("a.mobile-sub-navigation-item-title"))
        .await?;
    assert_eq!("IPv6 Host Exposure", nav_title.inner_html().await?);
    nav_title.click().await?;
    Ok(())
}

async fn login(driver: &mut WebDriver, config: &Config) -> Result<(), WebDriverError> {
    let elem_text = driver
        .find(By::Css("input[type='password']#Password_m"))
        .await?;

    elem_text.click().await?;

    elem_text.send_keys(&config.password).await?;

    driver
        .find(By::Css("input[type='button']#LoginBtn_m"))
        .await?
        .click()
        .await?;
    Ok(())
}
