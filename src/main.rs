use reqwest::{header, redirect, Client};
use scraper::{Html, Selector};
use serde::Deserialize;
use std::error::Error;
use std::fs;

const FILENAME: &str = "accounts.json";
const BASE_URL: &str = "URL";

#[derive(Debug, Deserialize, Clone)]
struct Account {
    email: String,
    password: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let accounts = get_accounts()?;

    for account in accounts {
        let cookies = get_cookies(account.clone()).await?;
        let platforms = get_platforms(cookies).await?;

        println!("{}: {}", account.email, platforms);
    }

    Ok(())
}

fn get_accounts() -> Result<Vec<Account>, Box<dyn Error>> {
    let file = fs::read_to_string(FILENAME)?;
    let accounts: Vec<Account> = serde_json::from_str(&file)?;
    Ok(accounts)
}

async fn get_cookies(account: Account) -> Result<String, Box<dyn Error>> {
    let client = Client::builder()
        .redirect(redirect::Policy::none())
        .build()?;

    let mut headers = header::HeaderMap::new();
    headers.insert("content-type", "application/x-www-form-urlencoded".parse()?);

    let mut params = std::collections::HashMap::new();
    params.insert("email", account.email);
    params.insert("password", account.password);

    let request = client
        .request(reqwest::Method::POST, format!("{}/{}", BASE_URL, "login"))
        .headers(headers)
        .form(&params);

    let response = request.send().await?;
    let a = response.headers();

    let mut session_hash = String::new();
    let mut session_user = String::new();

    for set_cookie_value in a.get_all("set-cookie") {
        let cookie_str = set_cookie_value.to_str()?;

        if cookie_str.contains("session_hash") {
            let hash = cookie_str.split(';').next().unwrap();
            session_hash = hash.to_string();
        }

        if cookie_str.contains("session_user") {
            let user = cookie_str.split(';').next().unwrap();
            session_user = user.to_string();
        }
    }

    let result = format!("{}; {}", session_hash, session_user);

    Ok(result)
}

async fn get_platforms(cookies: String) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::builder().build()?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Cookie", cookies.parse()?);

    let request = client
        .request(
            reqwest::Method::GET,
            format!("{}/{}", BASE_URL, "dashboard"),
        )
        .headers(headers);

    let response = request.send().await?;
    let body = response.text().await?;

    let document = Html::parse_document(&body);
    let selector = Selector::parse(r#"div.card_title"#).unwrap();
    let input = document.select(&selector).next().unwrap();

    if input.inner_html() == "Brak aktywnych pakietów".to_string() {
        Ok("".to_string())
    } else {
        let fragment = Html::parse_document(&body);
        let selector = Selector::parse("div.card-body").unwrap();
        let card = fragment.select(&selector).next().unwrap();
        let card_html = card.html();
        let fragment_card = Html::parse_fragment(&card_html);
        let strong_selector = Selector::parse("strong").unwrap();
        let mut vec = Vec::new();
        for element in fragment_card.select(&strong_selector) {
            vec.push(element.inner_html());
        }
        let result = vec.join(", ");
        Ok(result)
    }
}
