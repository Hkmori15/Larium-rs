use std::error::Error;
use bson::doc;
use futures::TryStreamExt;
use ::mongodb::Database;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use teloxide::{prelude::Requester, types::ChatId, Bot};

use crate::mongodb::{self};

#[derive(Debug, Serialize, Deserialize)]
pub struct AnimeResponse {
   pub id: i32,
   pub name: String,
   pub russian: Option<String>,
   pub episodes: Option<i32>,
   pub episodes_aired: Option<i32>,
   pub status: String,
   pub description: Option<String>,
   pub score: Option<String>,
   pub genres: Option<Vec<Genre>>,
   pub image: ImageData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Genre {
   pub name: String,
   pub russian: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageData {
   pub original: String,
}

pub async fn check_anime_exists(anime_name: &str) -> Result<Option<AnimeResponse>, Box<dyn Error + Send + Sync>> {
   let client = Client::new();
   let response = client
      .get("https://shikimori.one/api/animes")
      .query(&[("search", anime_name), ("limit", "1")])
      .header("User-Agent", "Larium-rs/1.0")
      .send()
      .await?;

   let animes: Vec<AnimeResponse> = response.json().await?;

   if animes.is_empty() {
      return Ok(None);
   }

   let anime_id = animes[0].id;
   let details = client
      .get(&format!("https://shikimori.one/api/animes/{}", anime_id))
      .header("User-Agent", "Larium-rs/1.0")
      .send()
      .await?
      .json()
      .await?;

   Ok(Some(details))
}

pub async fn check_new_episodes(bot: &Bot, db: &Database) {
   let collection = db.collection::<mongodb::Subscription>("subscriptions");

   let mut cursor = match collection.find(doc! {}).await {
       Ok(cursor) => cursor,
       Err(_) => return,
   };

   while let Ok(Some(subscription)) = cursor.try_next().await {
       if let Ok(Some(anime)) = check_anime_exists(&subscription.anime_name).await {
         if anime.status == "released" {
            // Delete subs if anime is finished
            let _ = collection
               .delete_one(doc! {
                  "user_id": subscription.user_id,
                  "anime_id": subscription.anime_id
               })
               .await;

            let _ = bot
               .send_message(
                  ChatId(subscription.user_id as i64),
                  format!(
                     "Аниме \"{}\" завершилось и было удалено из вашего списка подписок.",
                     subscription.anime_name
                  ),
               )
               .await;

            continue;
         }

         if let Some(episodes_aired) = anime.episodes_aired {
            if episodes_aired > subscription.last_episode {
               // Update last episode count
               let _ = collection
                  .update_one(
                     doc! {
                        "user_id": subscription.user_id,
                        "anime_id": subscription.anime_id
                     },

                     doc! { "$set": { "last_episode": episodes_aired }}
                  )
                  .await;

               // Notify user about new episode
               let _ = bot
                  .send_message(
                     ChatId(subscription.user_id as i64),
                     format!(
                        "Вышла {} серия аниме \"{}\".",
                        episodes_aired,
                        subscription.anime_name
                     ),
                  )
                  .await;
            }
         }
       }
   }
}