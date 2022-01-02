pub struct Config {
    pub preview: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config { preview: false }
    }
}
