#[derive(Debug)]
pub enum JevError {
    BuildError,
    MissingApiKey,
    InvalidUrl,
    InvalidProxy,
    ClientBuild(reqwest::Error),
}
