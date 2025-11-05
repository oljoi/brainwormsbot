#![feature(str_split_whitespace_remainder)]
#![feature(iter_collect_into)]

use std::{
    env,
    time::Duration,
    fmt::Debug,
};
use teloxide::{
    prelude::*,
    RequestError,
    adaptors::DefaultParseMode,
    types::{
        ChatAction, InlineQueryResult, InlineQueryResultArticle, InputFile, InputMessageContent,
        InputMessageContentText, Me, ParseMode,
    },
};
use sqlx::{
    PgPool, postgres::PgPoolOptions
};

type Bot = DefaultParseMode<teloxide::Bot>;

const DICTIONARY: &'static [u8] = include_bytes!("brainworms.pdf");
macro_rules! FORMAT {
    (MSG) => (r#"<b>{readable_name}</b>
<blockquote expandable>{readable_name} — {desc}</blockquote>
Source: <a href="https://t.me/brainwormsbot?start=source">{added_by}</a>
"#);
    (INFO) => (r#"Developed by <a href="https://puppy.support/">oljoi</a>
Source code avaiable at <a href="https://toys.puppy.support/oljoi/brainwormsbot">toys.puppy.support</a>
"#);
}

#[derive(Debug)]
#[derive(sqlx::FromRow)]
struct Word {
    id: i64,
    #[allow(dead_code)] name: String,
    readable_name: String,
    desc: String,
    added_by: String,
    #[allow(dead_code)] lang: String
}

async fn db_find_all<'p>(db: PgPool, name: &'p str, lang: &'p str) -> Result<Vec<Word>, sqlx::Error> {
    log::debug!("Quering db for {name}");
    sqlx::query_as::<_, Word>("SELECT * FROM slurs WHERE name LIKE $1 AND lang = $2")
        .bind(format!("%{name}%"))
        .bind(lang)
        .fetch_all(&db)
        .await
}

async fn db_find_one<'p>(db: PgPool, name: &'p str, lang: &'p str) -> Result<Word, sqlx::Error> {
    log::debug!("Quering db for {name}");
    sqlx::query_as::<_, Word>("SELECT * FROM slurs WHERE name LIKE $1 AND lang = $2")
        .bind(format!("%{name}%"))
        .bind(lang)
        .fetch_one(&db)
        .await
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let dictionary_file = InputFile::memory(DICTIONARY).file_name("brainworms.pdf");

    let conn = env::var("DATABASE_URL").expect("ENV DATABASE_URL is not set");

    let db = PgPoolOptions::new()
        .max_connections(3)
        .connect(&conn)
        .await
        .expect("Error while creating database object");

    log::info!("Starting the bot");

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .expect("Could not build HTTP client");

    let bot = teloxide::Bot::from_env_with_client(http_client).parse_mode(ParseMode::Html);

    let result_nothing = InlineQueryResult::Article(
        InlineQueryResultArticle::new(
            "noting_found",
            "No such word found",
            InputMessageContent::Text(InputMessageContentText::new("Found nothing :(")),
        )
        .description("Found nothing :("),
    );

    let handler = dptree::entry()
        .inspect(|u: Update| {
            log::debug!("{u:#?}");
        })
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_inline_query().endpoint(cmd_search_inline));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![dictionary_file, db, result_nothing])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn cmd_source(bot: Bot, msg: Message, file: InputFile) -> Result<(), RequestError> {
    bot.send_chat_action(msg.chat.id, ChatAction::UploadDocument).await?;
    bot.send_document(msg.chat.id, file).await?;

    respond(())
}

async fn cmd_search_one<'p>(bot: Bot, msg: Message, db: PgPool, query: &'p str) -> Result<(), RequestError> {
    bot.send_chat_action(msg.chat.id, ChatAction::Typing).await?;

    match db_find_one(db.clone(), query, "en").await {
        Ok(w) => {
            log::debug!("Found values for {query:?}");

            let message = format!(FORMAT!(MSG),
                readable_name=w.readable_name,
                desc=w.desc,
                added_by=w.added_by
            );
            bot.send_message(msg.chat.id, message).await?;
        },
        Err(e) => {
            log::error!("Error querying database for '{query:?}': {:?}", e);
            bot.send_message(msg.chat.id, "No results found.").await?;
        }
    };

    respond(())
}

async fn cmd_search_inline<'p>(bot: Bot, query: InlineQuery, db: PgPool, result_nothing: InlineQueryResult) -> Result<(), RequestError> {
    let mut results: Vec<InlineQueryResult> = Vec::with_capacity(20);

    db_find_all(db.clone(), &query.query, "en").await
        .unwrap_or_default()
        .iter()
        .take(20)
        .map( |word| {
            InlineQueryResult::Article(
                InlineQueryResultArticle::new(
                    word.id.to_string(),
                    word.readable_name.clone(),
                    InputMessageContent::Text(
                        InputMessageContentText::new(
                            format!(FORMAT!(MSG),
                                readable_name=word.readable_name,
                                desc=word.desc,
                                added_by=word.added_by
                            )
                        )
                    )
                ).description(word.desc.clone())
            )
        })
        .collect_into(&mut results);

    if results.is_empty() {
        log::info!("Found nothing for {query:?}");
        results.push(result_nothing);
    }

    bot
        .answer_inline_query(query.id, results)
        .send()
        .await
        .map_err(|e| log::error!("Error in handler: {e:?}"))
        .unwrap();

    respond(())
}

async fn message_handler(
    bot: Bot,
    msg: Message,
    #[allow(unused_variables)] me: Me,
    file: InputFile,
    db: PgPool,
) -> Result<(), RequestError> {
    let input = msg.text()
        .unwrap_or("")
        .to_lowercase();
    let mut text = input.split_whitespace();

    let cmd = text.nth(0).unwrap_or("");
    let query = text.remainder().unwrap_or("");

    match cmd {
        "/start" => {
            if query == "source" {
                cmd_source(bot, msg, file).await?;
            } else if query == "" {
                // first start maybe
            }
        }
        "/source" | "source" => {
            cmd_source(bot, msg, file).await?;
        }
        "/search" | "search" | "s" => {
            if query != "" {
                cmd_search_one(bot, msg, db, query).await?;
            } else {
                bot.send_message(msg.chat.id, "Please provide a word to search.").await?;
            }
        }
        "/info" | "info" => {
            bot.send_message(msg.chat.id, FORMAT!(INFO)).await?;
        }
        _ => {}
    };

    respond(())
}
