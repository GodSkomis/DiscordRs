use sqlx::PgPool;

pub mod autoroom;


pub type CommandError = Box<dyn std::error::Error + Send + Sync>;
pub type CommandContext<'a> = poise::Context<'a, CommandData, CommandError>;

pub struct CommandData {
    pool: PgPool
}

pub async fn generate_commands_framework(pool: PgPool) -> poise::Framework<CommandData, CommandError> {
    let framework: poise::Framework<CommandData, CommandError> = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            // prefix_options: PrefixFrameworkOptions {
            //     prefix: Some("!".into()),
            //     ..Default::default()
            // },
            commands: vec![autoroom::autoroom()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(CommandData { pool: pool })
            })
        })
        .build();
    return framework;
}