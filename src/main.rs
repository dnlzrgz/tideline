use tideline::app::App;

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tideline=debug,warn".into()),
        )
        .init();

    iced::application(App::boot, App::update, App::view)
        .title("Tideline")
        .run()
}
