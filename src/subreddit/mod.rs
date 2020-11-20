//! # Subreddit
//! A read-only module to read data from a specific subreddit.
//!
//! # Basic Usage
//! ```rust
//! use roux::Subreddit;
//! use tokio;
//!
//! #[tokio::main]
//! async fn main() {
//!     let subreddit = Subreddit::new("rust");
//!     // Now you are able to:
//!
//!     // Get moderators.
//!     let moderators = subreddit.moderators().await;
//!
//!     // Get hot posts with limit = 25.
//!     let hot = subreddit.hot(25, None).await;
//!
//!     // Get rising posts with limit = 30.
//!     let rising = subreddit.rising(30, None).await;
//!
//!     // Get top posts with limit = 10.
//!     let top = subreddit.top(10, None).await;
//!
//!     // Get latest comments.
//!     // `depth` and `limit` are optional.
//!     let latest_comments = subreddit.latest_comments(None, Some(25)).await;
//!
//!     // Get comments from a submission.
//!     let article_id = &hot.unwrap().data.children.first().unwrap().data.id.clone();
//!     let article_comments = subreddit.article_comments(article_id, None, Some(25));
//! }
//! ```
//!
//! # Usage with feed options
//!
//! ```rust
//! use roux::Subreddit;
//! use roux::util::FeedOption;
//! use tokio;
//!
//! #[tokio::main]
//! async fn main() {
//!     let subreddit = Subreddit::new("astolfo");
//!
//!     // Gets hot 10
//!     let hot = subreddit.hot(25, None).await;
//!
//!     // Get after param from `hot`
//!     let after = hot.unwrap().data.after.unwrap();
//!     let options = FeedOption::new().after(&after);
//!
//!     // Gets next 25
//!     let next_hot = subreddit.hot(25, Some(options)).await;
//! }
//! ```

extern crate reqwest;
extern crate serde_json;

use crate::util::{FeedOption, RouxError};
use reqwest::Client;

pub mod responses;
use responses::{SubredditComments, Moderators, Submissions};

/// Subreddit.
pub struct Subreddit {
    /// Name of subreddit.
    pub name: String,
    url: String,
    client: Client,
}

impl Subreddit {
    /// Create a new `Subreddit` instance.
    pub fn new(name: &str) -> Subreddit {
        Self::new_with_http_client(name, Client::new())
    }

    /// Create a new `Subreddit` instance with a provided HTTP client.
    pub fn new_with_http_client(name: &str, http_client: Client) -> Subreddit {
        let subreddit_url = format!("https://www.reddit.com/r/{}", name);

        Subreddit {
            name: name.to_owned(),
            url: subreddit_url,
            client: http_client,
        }
    }

    /// Get moderators.
    pub async fn moderators(&self) -> Result<Moderators, RouxError> {
        Ok(self
            .client
            .get(&format!("{}/about/moderators/.json", self.url))
            .send()
            .await?
            .json::<Moderators>()
            .await?)
    }

    async fn get_feed(
        &self,
        ty: &str,
        limit: u32,
        options: Option<FeedOption>,
    ) -> Result<Submissions, RouxError> {
        let url = &mut format!("{}/{}.json?limit={}", self.url, ty, limit);

        if !options.is_none() {
            let option = options.unwrap();

            if !option.after.is_none() {
                url.push_str(&mut format!("&after={}", option.after.unwrap().to_owned()));
            } else if !option.before.is_none() {
                url.push_str(&mut format!(
                    "&before={}",
                    option.before.unwrap().to_owned()
                ));
            }

            if !option.count.is_none() {
                url.push_str(&mut format!("&count={}", option.count.unwrap()));
            }
        }

        Ok(self
            .client
            .get(&url.to_owned())
            .send()
            .await?
            .json::<Submissions>()
            .await?)
    }

    async fn get_comment_feed(
        &self,
        ty: &str,
        depth: Option<u32>,
        limit: Option<u32>,
    ) -> Result<SubredditComments, RouxError> {
        let url = &mut format!("{}/{}.json?", self.url, ty);

        if !depth.is_none() {
            url.push_str(&mut format!("&depth={}", depth.unwrap()));
        }

        if !limit.is_none() {
            url.push_str(&mut format!("&limit={}", limit.unwrap()));
        }

        // This is one of the dumbest APIs I've ever seen.
        // The comments for a subreddit are stored in a normal hash map
        // but for posts the comments are in an array with the ONLY item
        // being same hash map as the one for subreddits...
        if url.contains("comments/") {
            let mut comments = self
                .client
                .get(&url.to_owned())
                .send()
                .await?
                .json::<Vec<SubredditComments>>()
                .await?;

            Ok(comments.pop().unwrap())
        } else {
            Ok(self
                .client
                .get(&url.to_owned())
                .send()
                .await?
                .json::<SubredditComments>()
                .await?)
        }
    }

    /// Get hot posts.
    pub async fn hot(
        &self,
        limit: u32,
        options: Option<FeedOption>,
    ) -> Result<Submissions, RouxError> {
        self.get_feed("hot", limit, options).await
    }

    /// Get rising posts.
    pub async fn rising(
        &self,
        limit: u32,
        options: Option<FeedOption>,
    ) -> Result<Submissions, RouxError> {
        self.get_feed("rising", limit, options).await
    }

    /// Get top posts.
    pub async fn top(
        &self,
        limit: u32,
        options: Option<FeedOption>,
    ) -> Result<Submissions, RouxError> {
        // TODO: time filter
        self.get_feed("top", limit, options).await
    }

    /// Get latest posts.
    pub async fn latest(
        &self,
        limit: u32,
        options: Option<FeedOption>,
    ) -> Result<Submissions, RouxError> {
        self.get_feed("new", limit, options).await
    }

    /// Get latest comments.
    pub async fn latest_comments(
        &self,
        depth: Option<u32>,
        limit: Option<u32>,
    ) -> Result<SubredditComments, RouxError> {
        self.get_comment_feed("comments", depth, limit).await
    }

    /// Get comments from article.
    pub async fn article_comments(
        &self,
        article: &str,
        depth: Option<u32>,
        limit: Option<u32>,
    ) -> Result<SubredditComments, RouxError> {
        self.get_comment_feed(&format!("comments/{}", article), depth, limit)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::Subreddit;
    use tokio;

    #[tokio::test]
    async fn test_no_auth() {
        let subreddit = Subreddit::new("astolfo");

        // Test moderators
        let moderators = subreddit.moderators().await;
        assert!(moderators.is_ok());

        // Test feeds
        let hot = subreddit.hot(25, None).await;
        assert!(hot.is_ok());

        let rising = subreddit.rising(25, None).await;
        assert!(rising.is_ok());

        let top = subreddit.top(25, None).await;
        assert!(top.is_ok());

        let latest_comments = subreddit.latest_comments(None, Some(25)).await;
        assert!(latest_comments.is_ok());

        let article_id = &hot.unwrap().data.children.first().unwrap().data.id.clone();
        let article_comments = subreddit.article_comments(article_id, None, Some(25)).await;
        assert!(article_comments.is_ok());
    }
}
