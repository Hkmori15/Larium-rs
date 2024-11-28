use std::error::Error;
use std::time::Duration;
use dotenvy::dotenv;
use ::mongodb::{options::ClientOptions, Client, Database};
use teloxide::{prelude::*, utils::command::BotCommands};
use tokio;
use bson::doc;
use futures::TryStreamExt;

mod mongodb;
mod api;
mod keep_alive;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    Start,
    Subscribe(String),
    Unsubscribe(String),
    List,
    Info(String),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    pretty_env_logger::init();
    keep_alive::keep_alive();

    let bot = Bot::new(std::env::var("TELOXIDE_TOKEN")?);
    let mongo_uri = std::env::var("MONGODB_URI")?;

    let client_options = ClientOptions::parse(&mongo_uri).await?;
    let client = Client::with_options(client_options)?;
    let db = client.database("larium-rs");

    println!("MongoDB connected..");

    // Clone bot and db for background tasks
    let bot_clone = bot.clone();
    let db_clone = db.clone();

    // Start background tasks for checking new episodes
    tokio::spawn(async move {
        loop {
            api::check_new_episodes(&bot_clone, &db_clone).await;
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });

    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint(move |bot: Bot, msg: Message, cmd: Command| {
                    answer(bot, msg, cmd, db.clone())
                }),   
        );

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

async fn answer(
    bot: Bot,
    msg: Message,
    cmd: Command,
    db: Database,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match cmd {
        Command::Start => {
            let user_name = msg.from.unwrap().first_name.clone();
            let welcome_message = format!(
                "Привет 👋🏻, {}! Добро пожаловать в LariumBot.\n\
                Я помогу тебе отслеживать выход новых серий твоих любимых аниме.☕️\n\n\
                Чтобы подписаться на обновления серий аниме, используй команду /subscribe, вот так: \n\
                /subscribe Название аниме\n\n\
                Пример: /subscribe Этот глупый свин не понимает мечту девочки зайки\n\n\
                Только будь уверен, что ты пишешь название тайтла который находится в онгоинге.\n\n\
                Чтобы отписаться от обновлений серий аниме, используй команду /unsubscribe\n\n\
                Пример: /unsubscribe Этот глупый свин не понимает мечту девочки зайки\n\n\
                Чтобы посмотреть список отслеживаемых тобой аниме, используй команду /list\n\n\
                Чтобы получить информацию об аниме, используй команду /info\n\n\
                Пример: /info Наруто\n\n\
                Попробуй прямо сейчас! ⛩",
                user_name
            );

            bot.send_message(msg.chat.id, welcome_message).await?;
        }

        Command::Info(anime_name) => {
            let anime = api::check_anime_exists(&anime_name).await?;

            match anime {
                Some(anime) => {
                    let genres = anime.genres
                        .as_ref()
                        .map(|genres| {
                            genres.iter()
                                .map(|g|
                                g.russian.clone().unwrap_or(g.name.clone()))
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_else(|| "Жанры не указаны".to_string());

                    let info_message = format!(
                        "Информация об аниме \"{}\":\n\
                        Количество серий: {}\n\
                        Жанры: {}\n\
                        Описание: {}\n\
                        Рейтинг: {}",
                        anime.russian.unwrap_or(anime.name),
                        anime.episodes.map_or("Неизвестно".to_string(), |e| e.to_string()),
                        genres,
                        anime.description.unwrap_or_else(|| "Описание отсутсвует".to_string()),
                        anime.score.map_or("Нет рейтинга".to_string(), |s| s.to_string())
                    );

                    bot.send_message(msg.chat.id, info_message).await?;
                }

                None => {
                    bot.send_message(msg.chat.id, "Аниме не найдено. Проверьте правильность названия.").await?;
                }
            }
        }

        Command::Subscribe(anime_name) => {
            if anime_name.trim().is_empty() {
                bot.send_message(msg.chat.id, "Вы не указали название аниме.").await?;
                return Ok(())
            }

            let user_id = msg.from.unwrap().id.0 as i64;
            let anime = api::check_anime_exists(&anime_name).await?;

            match anime {
                Some(anime) => {
                    let collection = db.collection::<mongodb::Subscription>("subscriptions");
                    let subscription = mongodb::Subscription {
                        user_id,
                        anime_id: anime.id,
                        anime_name: anime.russian.unwrap_or(anime.name),
                        last_episode: anime.episodes_aired.unwrap_or(0),
                    };

                    collection.insert_one(subscription).await?;
                    bot.send_message(
                        msg.chat.id,
                        format!("Вы успешно подписались на обновления аниме \"{}\"", anime_name)
                    ).await?;
                }

                None => {
                    bot.send_message(msg.chat.id, "Аниме не найдено. Проверьте правильность названия.").await?;
                }
            }
        }

        Command::Unsubscribe(anime_name) => {
            if anime_name.trim().is_empty() {
                bot.send_message(msg.chat.id, "Вы не указали название аниме.").await?;
                return Ok(())
            }

            let user_id = msg.from.unwrap().id.0 as i64;
            let collection = db.collection::<mongodb::Subscription>("subscriptions");

            let res = collection
                .delete_one(doc! { "user_id": user_id, "anime_name": &anime_name })
                .await?;

            if res.deleted_count > 0 {
                bot.send_message(
                    msg.chat.id,
                    format!("Вы успешно отписались от обновлений аниме \"{}\"", anime_name)
                ).await?;
            } else {
                bot.send_message(
                    msg.chat.id,
                    format!("Вы не были подписаны на аниме \"{}\" или оно не было найдено.", anime_name)
                ).await?;
            }
        }

        Command::List => {
            let user_id = msg.from.unwrap().id.0 as i64;
            let collection = db.collection::<mongodb::Subscription>("subscriptions");

            let mut cursor = collection
                .find(doc! { "user_id": user_id })
                .await?;

            let mut anime_list = String::new();
            let mut index = 1;

            while let Some(subscription) = cursor.try_next().await? {
                anime_list.push_str(&format!("{}. {}\n", index, subscription.anime_name));
                index += 1;
            }

            let message = if anime_list.is_empty() {
                "Вы еще не отслеживаете ни одного аниме.".to_string()
            } else {
                format!("Вот список отслеживаемых вами аниме:\n\n{}", anime_list)
            };

            bot.send_message(msg.chat.id, message).await?;
        }
    }

    Ok(())
}
