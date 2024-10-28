// use chrono::Utc;
// use fake::Fake;
use chrono::{DateTime, Utc};
use fake::{faker::chrono::en::DateTimeBetween, Fake};
use reqwest::multipart::{Form, Part};
use tempfile::NamedTempFile;

use std::io::Write;

use pine_tails::domain::posts::PostBuilder;

const API_ADDR: &str = "http://localhost:8000";

const CONTENT_TEMPLATE: &str = r#"
# Markdown Basics

## Headers

### More Headers

#### Yet another Headers

> Some quote
> Some quote
> Some quote

```rust
// Code block
fn main() {
  println!("Hello world")
}
```

`println!` is a `macro` in rust to write to the `STDOUT`

- List
  - Item 1
  - Item 2
    - Item 2.1
    - Item 2.2
  - Item 3
"#;

#[tokio::test]
#[ignore]
async fn test() {
    let mut handles = vec![];

    for i in 0..50 {
        handles.push(tokio::spawn(upload_one_post(i)));
    }

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|res| res.unwrap()) // Handle errors if necessary
        .collect();

    println!("Results: {:?}", results);
}

fn fake_date() -> DateTime<Utc> {
    // let start = DateTime::<Utc>::from_utc(
    //     chrono::NaiveDate::from_ymd(2020, 1, 1).and_hms(0, 0, 0),
    //     Utc,
    // );
    let start = DateTime::parse_from_rfc3339("2020-01-01T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let end = DateTime::parse_from_rfc3339("2025-01-01T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);

    DateTimeBetween(start, end).fake()
}

async fn upload_one_post(idx: usize) {
    let pb = PostBuilder::default()
        .with_title(&format!("and then {idx}"))
        .with_content(CONTENT_TEMPLATE)
        .with_datetime(fake_date());

    let post = pb.build();

    let api_addr = format!("{}/posts/", API_ADDR);
    let temp_file = tokio::task::spawn_blocking(move || {
        let mut temp_file = NamedTempFile::new().expect("Failed to create tempfile");
        temp_file
            .write_all(post.to_string().as_bytes())
            .expect("Failed to write to tempfile");
        temp_file
    })
    .await
    .expect("Failed to await joinhandle");

    let to_upload = Part::file(temp_file.path()).await.unwrap();
    let form = Form::new().part("file", to_upload);

    reqwest::Client::new()
        .post(&api_addr)
        .multipart(form)
        .send()
        .await
        .expect("Failed to send request")
        .error_for_status()
        .unwrap();
}
