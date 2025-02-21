use std::time::Duration;

use async_trait::async_trait;
use sqlx::query;
use types::{UrlEmbed, UserId};
use url::Url;
use uuid::Uuid;

use super::Postgres;

use crate::{data::DataUrlEmbed, Result};

#[async_trait]
impl DataUrlEmbed for Postgres {
    async fn url_embed_insert(&self, user_id: UserId, embed: UrlEmbed) -> Result<()> {
        let id = Uuid::now_v7();
        query!(
            r#"
            INSERT INTO url_embed (
                id,
                url,
                canonical_url,
                title,
                description,
                color,
                media,
                media_is_thumbnail,
                author_url,
                author_name,
                author_avatar,
                site_name,
                site_avatar,
                user_id
            )
    	    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        "#,
            id,
            embed.url.to_string(),
            embed.canonical_url.map(|u| u.to_string()),
            embed.title,
            embed.description,
            embed.color,
            embed.media.map(|m| m.id.into_inner()),
            embed.media_is_thumbnail,
            embed.author_url.map(|u| u.to_string()),
            embed.author_name,
            embed.author_avatar.map(|m| m.id.into_inner()),
            embed.site_name,
            embed.site_avatar.map(|m| m.id.into_inner()),
            user_id.into_inner(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn url_embed_find(&self, url: Url, max_age: Duration) -> Result<Option<UrlEmbed>> {
        let min_ts = time::OffsetDateTime::now_utc() - max_age;
        let min_ts = time::PrimitiveDateTime::new(min_ts.date(), min_ts.time());
        let row = query!(
            r#"
            SELECT
                u.canonical_url,
                u.title,
                u.description,
                u.color,
                row_to_json(m) as media,
                u.media_is_thumbnail,
                u.author_url,
                u.author_name,
                row_to_json(a) as author_avatar,
                u.site_name,
                row_to_json(s) as site_avatar
            FROM url_embed u
            JOIN media_json m ON m.id = u.media
            JOIN media_json a ON a.id = u.author_avatar
            JOIN media_json s ON s.id = u.site_avatar
            WHERE u.url = $1 AND u.created_at > $2
            "#,
            url.to_string(),
            min_ts,
        )
        .fetch_optional(&self.pool)
        .await?;
        let embed = row.map(|r| UrlEmbed {
            url,
            canonical_url: r
                .canonical_url
                .map(|i| i.parse().expect("invalid data in db")),
            title: r.title,
            description: r.description,
            color: r.color,
            media: r
                .media
                .map(|m| serde_json::from_value(m).expect("invalid data in db")),
            media_is_thumbnail: r.media_is_thumbnail.expect("invalid data in db"),
            author_url: r.author_url.map(|i| i.parse().expect("invalid data in db")),
            author_name: r.author_name,
            author_avatar: r
                .author_avatar
                .map(|m| serde_json::from_value(m).expect("invalid data in db")),
            site_name: r.site_name,
            site_avatar: r
                .site_avatar
                .map(|m| serde_json::from_value(m).expect("invalid data in db")),
        });
        Ok(embed)
    }
}
