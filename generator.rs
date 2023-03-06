//# reqwest = "^0.9.20"
//# serde = { version = "^1.0.92", features = ["derive"] }
//# serde_json = "1.0"
//# Inflector = "^0.11.4"
//# base64 = "0.10.1"

extern crate base64;
extern crate serde;
extern crate serde_json;
extern crate reqwest;
extern crate inflector;

use serde::{Serialize, Deserialize};
use inflector::cases::pascalcase::to_pascal_case;
use std::collections::HashMap;
use std::env;

const RAW_CSS_PROPERTIES_ENDPOINT: &str = "https://raw.githubusercontent.com/mdn/data/master/css/properties.json";
const REPOSITORY_OWNER: &str = "MartinKavik";
const REPOSITORY_NAME: &str = "html-css-db";
const CSS_PROPERTIES_PATH: &str = "css_properties.json";
const COMMIT_MESSAGE: &str = "Updated CSS properties";
const BRANCH: &str = "master";
const GITHUB_API_URL: &str = "https://api.github.com";
const COMITTER_NAME: &str = "CRON_JOB";
const COMITTER_EMAIL: &str = "CRON_JOB";
const GITHUB_TOKEN_ENV_VAR: &str = "GITHUB_TOKEN";

type RawCssPropertyName = String;

#[derive(Serialize, Debug)]
struct CssProperty {
    name: CssPropertyName
}

#[derive(Serialize, Debug)]
struct CssPropertyName {
    original: String,
    pascal_case: String
}

#[derive(Deserialize, Debug)]
struct RawCssProperty {
    // not required
}

pub fn main() {
    println!("Fetching raw CSS properties...");
    let raw_css_properties = fetch_raw_css_properties();

    println!("Remodeling raw CSS properties...");
    let mut css_properties = remodel_raw_css_properties(raw_css_properties);

    println!("Sorting CSS properties...");
    sort_css_properties(&mut css_properties);

    println!("Saving CSS properties...");
    save_css_properties(css_properties);
}

fn fetch_raw_css_properties() -> HashMap<RawCssPropertyName, RawCssProperty> {
    reqwest::get(RAW_CSS_PROPERTIES_ENDPOINT)
        .expect("Request to raw css properties endpoint failed.")
        .json()
        .expect("Problem parsing CSS properties as JSON.")
}

fn remodel_raw_css_properties(
    raw_css_properties: HashMap<RawCssPropertyName,RawCssProperty>
) -> Vec<CssProperty> {
    raw_css_properties
        .into_iter()
        .map(|(original_name, _)| {
            CssProperty {
                name: CssPropertyName {
                    pascal_case: to_pascal_case(&original_name),
                    original: original_name
                }
            }
        })
        .collect()
}

fn sort_css_properties(css_properties: &mut Vec<CssProperty>) {
    css_properties.sort_by(|a, b| a.name.original.cmp(&b.name.original));
}

fn save_css_properties(css_properties: Vec<CssProperty>) {
    let github_client = create_github_client();
    let css_properties_file_info = fetch_css_properties_file_info(&github_client);

    let content = serde_json::to_string_pretty(&css_properties)
        .expect("Cannot serialize CSS properties");

    push_css_properties(&github_client, &content, &css_properties_file_info.sha);
}

// ------ save_css_properties helpers ------

#[derive(Deserialize, Debug)]
struct GithubContentInfo {
    sha: String,
    // not required
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
enum RequestBodyValue<'a> {
    Text(&'a str),
    Object(HashMap<&'a str, &'a str>)
}

fn create_github_client() -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static("application/vnd.github.v3+json")
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .expect("Cannot build client")
}

fn fetch_css_properties_file_info(github_client: &reqwest::Client) -> GithubContentInfo {
    github_client
        .get(&format!("{}/repos/{}/{}/contents/{}",
            GITHUB_API_URL, REPOSITORY_OWNER, REPOSITORY_NAME, CSS_PROPERTIES_PATH
        ))
        .query(&[("ref", BRANCH)])
        .send()
        .expect("Request to get file info failed")
        .json()
        .expect("Cannot deserialize file info")
}

fn push_css_properties(
    github_client: &reqwest::Client, content: &str, old_css_properties_file_sha: &str
) {
    let github_token = env::var(GITHUB_TOKEN_ENV_VAR)
        .expect(&format!("Cannot read env. variable '{}'", GITHUB_TOKEN_ENV_VAR));
    let encoded_content = base64::encode(content);

    let mut committer = HashMap::new();
    committer.insert("name", COMITTER_NAME);
    committer.insert("email", COMITTER_EMAIL);

    let mut request_body = HashMap::new();
    request_body.insert("message", RequestBodyValue::Text(COMMIT_MESSAGE));
    request_body.insert("content", RequestBodyValue::Text(&encoded_content));
    request_body.insert("sha", RequestBodyValue::Text(old_css_properties_file_sha));
    request_body.insert("committer", RequestBodyValue::Object(committer));
    request_body.insert("branch", RequestBodyValue::Text(BRANCH));
    let request_body = serde_json::to_string_pretty(&request_body)
        .expect("Cannot serialize request body");

    let mut response = github_client
        .put(&format!("{}/repos/{}/{}/contents/{}",
            GITHUB_API_URL, REPOSITORY_OWNER, REPOSITORY_NAME, CSS_PROPERTIES_PATH
        ))
        .bearer_auth(github_token)
        .body(request_body)
        .send()
        .expect("Cannot push to repository.");

    match response.status() {
        reqwest::StatusCode::OK => println!("Pushed."),
        _ => {
            let response_text = response.text();
            panic!("{}", format!("{:#?}\n Response body: {:#?}", response, response_text))
        }
    }
}
