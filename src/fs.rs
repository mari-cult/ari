use {
    crate::tokenizer::Tokenizer,
    serde::de::DeserializeOwned,
    std::{
        fs::{self, File},
        io::{self, Read},
        path::Path,
    },
    ubyte::{ByteUnit, ToByteUnit},
};

#[derive(Clone, Debug)]
pub struct Options {
    limit: Option<ByteUnit>,
}

impl Options {
    pub fn new() -> Self {
        Self {
            limit: Some(8.megabytes()),
        }
    }

    pub fn limit<L: Into<Option<ByteUnit>>>(mut self, limit: L) -> Self {
        self.limit = limit.into();
        self
    }

    pub fn open<P: AsRef<Path>>(&self, path: P) -> io::Result<File> {
        let file = File::open(path)?;

        if let Some(limit) = self.limit {
            if u128::from(file.metadata()?.len()) > limit.as_u128() {
                return Err(io::Error::other("file is too large"));
            }
        }

        Ok(file)
    }

    pub fn read<P: AsRef<Path>>(&self, path: P) -> io::Result<Vec<u8>> {
        if self.limit.is_none() {
            return fs::read(path);
        }

        let mut bytes = Vec::new();

        self.open(path)?.read_to_end(&mut bytes)?;

        Ok(bytes)
    }

    pub fn read_to_string<P: AsRef<Path>>(&self, path: P) -> io::Result<String> {
        if self.limit.is_none() {
            return fs::read_to_string(path);
        }

        let mut string = String::new();

        self.open(path)?.read_to_string(&mut string)?;

        Ok(string)
    }

    pub fn json<T: DeserializeOwned, P: AsRef<Path>>(&self, path: P) -> io::Result<T> {
        serde_json::from_reader(self.open(path)?).map_err(io::Error::other)
    }

    pub fn toml<T: DeserializeOwned, P: AsRef<Path>>(&self, path: P) -> io::Result<T> {
        toml::from_str(&self.read_to_string(path)?).map_err(io::Error::other)
    }

    pub fn tokenizer<P: AsRef<Path>>(&self, path: P) -> io::Result<Tokenizer> {
        Ok(Tokenizer {
            tokenizer: tokenizers::Tokenizer::from_file(path).map_err(io::Error::other)?,
        })
    }
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}
