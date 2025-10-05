use crate::error::{Error, Result};
use crate::types::{Row, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct PoubelleClient {
    stream: Option<TcpStream>,
    host: String,
    port: u16,
    username: String,
    password: String,
}

impl PoubelleClient {
    pub fn new(connection_string: &str) -> Result<Self> {
        let parsed = Self::parse_connection_string(connection_string)?;
        Ok(Self {
            stream: None,
            host: parsed.0,
            port: parsed.1,
            username: parsed.2,
            password: parsed.3,
        })
    }

    fn parse_connection_string(conn_str: &str) -> Result<(String, u16, String, String)> {
        let parts: Vec<&str> = conn_str.split("://").collect();
        if parts.len() != 2 || parts[0] != "poubelle" {
            return Err(Error::Parse(
                "Invalid connection string format. Expected: poubelle://username:password@host:port"
                    .to_string(),
            ));
        }

        let rest = parts[1];
        let auth_host: Vec<&str> = rest.split('@').collect();
        if auth_host.len() != 2 {
            return Err(Error::Parse("Missing @ separator".to_string()));
        }

        let auth: Vec<&str> = auth_host[0].split(':').collect();
        if auth.len() != 2 {
            return Err(Error::Parse("Invalid username:password format".to_string()));
        }

        let host_port: Vec<&str> = auth_host[1].split(':').collect();
        if host_port.len() != 2 {
            return Err(Error::Parse("Invalid host:port format".to_string()));
        }

        let port = host_port[1]
            .parse::<u16>()
            .map_err(|_| Error::Parse("Invalid port number".to_string()))?;

        Ok((
            host_port[0].to_string(),
            port,
            auth[0].to_string(),
            auth[1].to_string(),
        ))
    }

    pub async fn connect(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.host, self.port);
        let stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| Error::Connection(e.to_string()))?;

        let (mut reader, mut writer) = stream.into_split();

        Self::wait_for_prompt(&mut reader, "Username: ").await?;
        writer
            .write_all(format!("{}\n", self.username).as_bytes())
            .await?;
        writer.flush().await?;

        Self::wait_for_prompt(&mut reader, "Password: ").await?;
        writer
            .write_all(format!("{}\n", self.password).as_bytes())
            .await?;
        writer.flush().await?;

        Self::wait_for_prompt(&mut reader, "Connected to Poubelle DB").await?;

        let stream = reader
            .reunite(writer)
            .map_err(|e| Error::Connection(e.to_string()))?;
        self.stream = Some(stream);

        Ok(())
    }

    pub async fn query(&mut self, sql: &str) -> Result<String> {
        let stream = self
            .stream
            .take()
            .ok_or_else(|| Error::Connection("Not connected".to_string()))?;

        let (mut reader, mut writer) = stream.into_split();

        Self::wait_for_prompt(&mut reader, "poubelle> ").await?;

        writer.write_all(format!("{}\n", sql).as_bytes()).await?;
        writer.flush().await?;

        let result = Self::read_until_prompt(&mut reader, "poubelle> ").await?;

        let stream = reader
            .reunite(writer)
            .map_err(|e| Error::Connection(e.to_string()))?;
        self.stream = Some(stream);

        Ok(result)
    }

    pub async fn execute(&mut self, sql: &str) -> Result<Vec<Row>> {
        let result = self.query(sql).await?;
        Self::parse_rows(&result)
    }

    pub async fn close(&mut self) -> Result<()> {
        if let Some(mut stream) = self.stream.take() {
            stream.write_all(b"exit\n").await?;
            stream.flush().await?;
        }
        Ok(())
    }

    fn parse_rows(result: &str) -> Result<Vec<Row>> {
        if result.is_empty() || result == "No rows" {
            return Ok(Vec::new());
        }

        if !result.contains('{') {
            return Ok(Vec::new());
        }

        let mut rows = Vec::new();
        for line in result.lines() {
            if let Some(row) = Self::parse_row(line) {
                rows.push(row);
            }
        }

        Ok(rows)
    }

    fn parse_row(line: &str) -> Option<Row> {
        let line = line.trim();
        if !line.starts_with('{') || !line.ends_with('}') {
            return None;
        }

        let inner = &line[1..line.len() - 1];
        let mut row = Row::new();

        let parts: Vec<&str> = inner.split(", ").collect();
        for part in parts {
            if let Some((key, value)) = part.split_once(": ") {
                let clean_key = key.trim_matches('"');
                let parsed_value = Self::parse_value(value);
                row.insert(clean_key.to_string(), parsed_value);
            }
        }

        if row.is_empty() {
            None
        } else {
            Some(row)
        }
    }

    fn parse_value(value: &str) -> Value {
        let value = value.trim();

        if value == "Null" {
            return Value::Null;
        }

        if let Some(num_str) = value.strip_prefix("Int(").and_then(|s| s.strip_suffix(')')) {
            if let Ok(num) = num_str.parse::<i64>() {
                return Value::Int(num);
            }
        }

        if let Some(text) = value
            .strip_prefix("Text(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let text = text.trim_matches('"');
            return Value::Text(text.to_string());
        }

        Value::Text(value.to_string())
    }

    async fn wait_for_prompt<R: AsyncReadExt + Unpin>(reader: &mut R, prompt: &str) -> Result<()> {
        let mut buffer = Vec::new();
        let mut byte = [0u8; 1];

        loop {
            reader.read_exact(&mut byte).await?;
            buffer.push(byte[0]);

            let s = String::from_utf8_lossy(&buffer);
            if s.contains(prompt) {
                break;
            }
        }
        Ok(())
    }

    async fn read_until_prompt<R: AsyncReadExt + Unpin>(
        reader: &mut R,
        prompt: &str,
    ) -> Result<String> {
        let mut buffer = Vec::new();
        let mut byte = [0u8; 1];

        loop {
            reader.read_exact(&mut byte).await?;
            buffer.push(byte[0]);

            let s = String::from_utf8_lossy(&buffer);
            if s.contains(prompt) {
                let result = s.trim_end_matches(prompt).trim().to_string();
                return Ok(result);
            }
        }
    }
}
