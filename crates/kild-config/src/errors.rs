#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to parse config file: {message}")]
    ConfigParseError { message: String },

    #[error("Invalid agent '{agent}'. Supported agents: {supported_agents}")]
    InvalidAgent {
        agent: String,
        supported_agents: String,
    },

    #[error("Invalid configuration: {message}")]
    InvalidConfiguration { message: String },

    #[error("IO error reading config: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },
}
