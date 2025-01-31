use snafu::Snafu;

#[derive(Debug, Snafu)]
enum ServerError {
    #[snafu(display("No such entity!"))]
    Nope,
    #[snafu(display("I/O error $kind"))]
    IoError {
        kind: std::io::ErrorKind,
    },
}

impl From<std::io::Error> for ServerError {
    fn from(e: std::io::Error) -> Self {
        ServerError::IoError { kind: e.kind() }
    }
}
