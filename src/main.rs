use anyhow::Result;
use dotenvy::dotenv;
use std::env;
use std::path::PathBuf;
use teloxide::net::Download;
use teloxide::prelude::*;
use teloxide::types::{Audio, InputFile};
use tokio::fs::File;
use tokio::process::Command;

// Import the logging macros
use log::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from `.env` file
    dotenv().ok();

    // Initialize the logger
    pretty_env_logger::init();

    // Get the bot token from the environment variables
    let bot_token = env::var("BOT_TOKEN").expect("BOT_TOKEN not found in environment variables");

    // Initialize the bot with the token
    let bot = Bot::new(bot_token).auto_send();

    info!("Starting the voice converter bot...");

    // Start the bot's event loop
    teloxide::repl(bot, |bot: AutoSend<Bot>, message: Message| async move {
        if let Err(err) = handle_message(bot, message).await {
            error!("Error handling message: {:?}", err);
        }
        respond(())
    })
    .await;

    Ok(())
}

async fn handle_message(bot: AutoSend<Bot>, message: Message) -> Result<()> {
    // Check if the message contains an audio file
    if let Some(audio) = message.audio() {
        process_audio(bot, &message, audio).await?;
    } else {
        bot.send_message(message.chat.id, "Please send me an audio file.")
            .await?;
    }
    Ok(())
}

async fn process_audio(bot: AutoSend<Bot>, message: &Message, audio: &Audio) -> Result<()> {
    let file_id = &audio.file_id;

    // Get the file path on Telegram's server
    let file = bot.get_file(file_id.clone()).await?;
    let file_path = file.file_path;

    // Define local paths for the original and converted files
    let filename = format!("{}.mp3", audio.file_unique_id);
    let filepath = PathBuf::from(&filename);
    let output_filename = format!("{}.ogg", audio.file_unique_id);
    let output_filepath = PathBuf::from(&output_filename);

    // Download the audio file to the local filesystem
    let mut dest_file = File::create(&filepath).await?;
    bot.download_file(&file_path, &mut dest_file).await?;

    // Convert the audio file to a voice message using ffmpeg
    let status = Command::new("ffmpeg")
        .args(&[
            "-i",
            &filepath.to_string_lossy(),
            "-acodec",
            "libopus",
            &output_filepath.to_string_lossy(),
            "-y",
        ])
        .status()
        .await?;

    if !status.success() {
        bot.send_message(message.chat.id, "Failed to convert audio file.")
            .await?;
        return Ok(());
    }

    // Send the converted voice message back to the user
    let voice = InputFile::file(output_filepath.clone());
    bot.send_voice(message.chat.id, voice).await?;

    // Clean up temporary files
    tokio::fs::remove_file(&filepath).await?;
    tokio::fs::remove_file(&output_filepath).await?;

    Ok(())
}
