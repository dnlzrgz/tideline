use iced::widget::text;

fn main() -> iced::Result {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let pool = tideline::db::connect("tideline.db").await.expect("connect");
        sqlx::migrate!().run(&pool).await.expect("migrations");
    });
    drop(rt);

    iced::run(update, view)
}

fn update(_state: &mut (), _message: ()) {}

fn view(_state: &()) -> iced::Element<'_, ()> {
    text("Hello, world!").into()
}
