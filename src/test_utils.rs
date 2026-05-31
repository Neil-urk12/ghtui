use crate::app::App;
use crate::config::Config;

pub fn test_app() -> App {
    App::new(Config::default(), Some("test-token".into())).unwrap()
}
