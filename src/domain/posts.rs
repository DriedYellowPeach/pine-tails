use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub struct Post {
    pub metadata: PostMetadata,
    pub content: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostMetadata {
    pub title: String,
    pub slug: String,
    pub date: DateTime<Utc>,
}

impl TryFrom<&str> for PostMetadata {
    type Error = anyhow::Error;
    fn try_from(metadata: &str) -> Result<Self, Self::Error> {
        serde_yml::from_str(metadata).context(format!("Failed to parse metadata from: {metadata}"))
    }
}

#[derive(Default, Deserialize)]
pub struct PostBuilder {
    title: Option<String>,
    slug: Option<String>,
    date: Option<DateTime<Utc>>,
    #[serde(skip)]
    content: Option<String>,
}

impl PostBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    fn try_from_raw_post(raw: &str) -> Result<Self> {
        let re_meta_seg = Regex::new(r"-{3,}\n").expect("Invalid regex");

        let segments = re_meta_seg.find_iter(raw).take(2).collect::<Vec<_>>();
        if segments.len() != 2 {
            bail!("metadata segmentation not found");
        }

        let metadata = &raw[segments[0].end()..segments[1].start()];
        let content = &raw[segments[1].end()..];

        serde_yml::from_str(metadata)
            .context("Failed to parse metadata")
            .map(|mut pb: PostBuilder| {
                pb.content = Some(content.to_string());
                pb
            })
    }

    /// try parse metadata from a raw string
    pub fn from_raw_post(raw: &str) -> Self {
        // when try_from_post failed, we treat raw all as content
        Self::try_from_raw_post(raw).unwrap_or_else(|_| Self::default().with_content(raw))
    }

    // Setter for the title field
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    // Setter for the content field
    pub fn with_content(mut self, content: &str) -> Self {
        self.content = Some(content.to_string());
        self
    }

    pub fn with_datetime(mut self, date: DateTime<Utc>) -> Self {
        self.date = Some(date);
        self
    }

    // Build method to construct the Post object, setting default values if fields are None
    pub fn build(self) -> Post {
        let id = Uuid::new_v4();
        let title = self.title.unwrap_or_else(|| format!("Post {id}"));
        // let slug = self.slug.unwrap_or_else(|| slug::slugify(&title));
        let slug = self
            .slug
            .map_or_else(|| slug::slugify(&title), |x| slug::slugify(&x));
        let date = self.date.unwrap_or_else(Utc::now);
        let content = self
            .content
            .filter(|x| !x.is_empty())
            .unwrap_or_else(|| "hello".to_string());

        Post {
            metadata: PostMetadata { title, slug, date },
            content,
        }
    }
}

impl std::fmt::Display for Post {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let metadata = serde_yml::to_string(&self.metadata).unwrap_or_default();
        write!(f, "---\n{}---\n{}", metadata, self.content)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::*;

    #[test]
    fn test_regex_get_right_section_of_metadata() {
        let raw = r#"------
        title: "My first post"
        "#;

        let re_meta_seg = Regex::new(r"-{3,}\n").expect("Invalid regex");
        let matched_head = re_meta_seg.find(raw).unwrap();
        assert_eq!(matched_head.start(), 0);
        assert_eq!(matched_head.end(), 7);
    }

    #[test]
    fn get_post_ok_from_valid_metadata() {
        let raw = r#"
------
title: "My first post"
slug: "my-first-post"
date: "2021-09-07T12:00:00Z"
------

# Hello world

What a wonderful world!
        "#;

        let bd = PostBuilder::from_raw_post(raw);
        let post = bd.build();
        assert_eq!(post.metadata.title, "My first post".to_string());
        assert_eq!(post.metadata.slug, "my-first-post".to_string());
        assert_eq!(
            post.metadata.date,
            DateTime::parse_from_rfc3339("2021-09-07T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc)
        );
    }

    #[test]
    fn get_post_ok_from_valid_metadata_but_with_fields_missing() {
        let raw = r#"
------
title: "My first post"
------

# Hello world

What a wonderful world!
        "#;

        // WARN: This could failed if I get expected_date at 11:59:59PM and the post is constructed
        // at tomorrow
        let expected_date = Utc::now();

        let bd = PostBuilder::from_raw_post(raw);
        let post = bd.build();
        assert_eq!(post.metadata.title, "My first post".to_string());
        assert_eq!(post.metadata.slug, "my-first-post".to_string());
        assert_eq!(post.metadata.date.year(), expected_date.year(),);
        assert_eq!(post.metadata.date.month(), expected_date.month(),);
        assert_eq!(post.metadata.date.day(), expected_date.day(),);
    }

    #[test]
    fn parse_metadata_should_fix_invalid_slug() {
        let raw = r#"
------
slug: "My first post"
------
        "#;

        let bd = PostBuilder::from_raw_post(raw);
        let post = bd.build();
        assert_eq!(post.metadata.slug, "my-first-post".to_string());
    }

    #[test]
    fn try_get_post_return_err_with_invalid_metadata() {
        let raw = r#"
------
definitely_not_metadata
------
        "#;

        let bd = PostBuilder::try_from_raw_post(raw);
        assert!(bd.is_err());
    }

    #[test]
    fn can_get_post_even_with_metadata_missing() {
        let raw = r#"content directly"#;

        let bd = PostBuilder::from_raw_post(raw);
        let post = bd.build();

        assert_eq!(post.content, "content directly");
    }

    #[test]
    fn post_display_gives_right_format() {
        let post = Post {
            metadata: PostMetadata {
                title: "My first post".to_string(),
                slug: "my-first-post".to_string(),
                date: Utc::now(),
            },
            content: "Hello world".to_string(),
        };

        /*
        ------
        tile: "xxxx"
        slug: "xxx"
        date: "xxx"
        ------
        xxxx
        */

        let expected = post.to_string();
        assert_eq!(expected.split('\n').count(), 6);
    }
}
