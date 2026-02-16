use crate::usecases::context::AppContext;

pub fn start(context: &AppContext) {
    tracing::info!(
        log_level = %context.config.logging.level,
        telegram_adapter = ?context.telegram,
        cache_adapter = ?context.cache,
        "starting TUI shell (placeholder)"
    );

    println!("RTG TUI shell started (placeholder)");
}
